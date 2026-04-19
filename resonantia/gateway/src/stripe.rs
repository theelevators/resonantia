use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use tracing::{error, info};

use crate::{
    auth::resolve_user_context, AppError, GatewayContext,
};

#[derive(Clone)]
pub struct StripeConfig {
    pub secret_key: String,
    pub webhook_secret: String,
    pub price_id_resonant: String,
    pub price_id_soulful: String,
    pub success_url: String,
    pub cancel_url: String,
    pub api_base_url: String,
}

#[derive(Deserialize)]
pub(crate) struct CheckoutRequest {
    tier: String,
}

#[derive(Serialize)]
pub(crate) struct CheckoutSessionResponse {
    url: String,
}

pub async fn checkout_handler(
    State(context): State<GatewayContext>,
    headers: HeaderMap,
    Json(request): Json<CheckoutRequest>,
) -> Result<Json<CheckoutSessionResponse>, AppError> {
    let stripe = context
        .stripe
        .as_ref()
        .ok_or_else(|| AppError::bad_request("stripe is not configured on this gateway".to_string()))?;

    if !matches!(request.tier.as_str(), "resonant" | "soulful") {
        return Err(AppError::bad_request("tier must be 'resonant' or 'soulful'".to_string()));
    }

    let user_ctx = resolve_user_context(&context, &headers).await?;
    let user_id = user_ctx
        .user_id
        .ok_or_else(|| AppError::unauthorized("checkout requires clerk auth".to_string()))?;

    let price_id = if request.tier == "soulful" {
        stripe.price_id_soulful.as_str()
    } else {
        stripe.price_id_resonant.as_str()
    };
    if !looks_like_stripe_price_id(price_id) {
        return Err(AppError::internal(format!(
            "invalid stripe price id configured for tier '{}': '{}'. Use a Price ID (price_...), not a Product ID (prod_...).",
            request.tier, price_id
        )));
    }
    let tier_str = request.tier.as_str();
    let user_id_str = user_id.as_str();

    // Build form body for Stripe Checkout Session creation.
    let params = [
        ("mode", "subscription"),
        ("allow_promotion_codes", "true"),
        ("line_items[0][price]", price_id),
        ("line_items[0][quantity]", "1"),
        ("success_url", stripe.success_url.as_str()),
        ("cancel_url", stripe.cancel_url.as_str()),
        ("metadata[clerk_user_id]", user_id_str),
        ("metadata[tier]", tier_str),
        ("client_reference_id", user_id_str),
        ("subscription_data[metadata][clerk_user_id]", user_id_str),
        ("subscription_data[metadata][tier]", tier_str),
    ];

    let http = reqwest::Client::new();
    let checkout_url = format!(
        "{}/v1/checkout/sessions",
        stripe.api_base_url.trim_end_matches('/')
    );
    let response = http
        .post(checkout_url)
        .basic_auth(&stripe.secret_key, None::<&str>)
        .form(&params)
        .send()
        .await
        .map_err(|err| AppError::internal(format!("stripe request failed: {err}")))?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::internal(format!("stripe checkout session failed: {body}")));
    }

    let payload: serde_json::Value = response
        .json()
        .await
        .map_err(|err| AppError::internal(format!("stripe response parse failed: {err}")))?;

    // Persist the Stripe customer ID from the session so the portal handler can reuse it.
    if let Some(customer_id) = payload["customer"].as_str().filter(|s| !s.is_empty()) {
        if let Err(err) = context.accounts.set_stripe_customer_id(&user_id, customer_id).await {
            error!(%user_id, %err, "failed to persist stripe customer id");
        }
    }

    let url = payload["url"]
        .as_str()
        .ok_or_else(|| AppError::internal("stripe response missing url".to_string()))?
        .to_string();

    Ok(Json(CheckoutSessionResponse { url }))
}

pub async fn customer_portal_handler(
    State(context): State<GatewayContext>,
    headers: HeaderMap,
) -> Result<Json<CheckoutSessionResponse>, AppError> {
    let stripe = context
        .stripe
        .as_ref()
        .ok_or_else(|| AppError::bad_request("stripe is not configured on this gateway".to_string()))?;

    let user_ctx = resolve_user_context(&context, &headers).await?;
    let user_id = user_ctx
        .user_id
        .ok_or_else(|| AppError::unauthorized("customer portal requires clerk auth".to_string()))?;

    let customer_id = context
        .accounts
        .get_stripe_customer_id(&user_id)
        .await
        .map_err(AppError::internal)?
        .ok_or_else(|| {
            AppError::bad_request("no stripe customer found for this account — subscribe first".to_string())
        })?;

    let return_url = format!(
        "{}",
        stripe
            .success_url
            .split('?')
            .next()
            .unwrap_or("https://account.resonantia.me")
    );

    let params = [
        ("customer", customer_id.as_str()),
        ("return_url", return_url.as_str()),
    ];

    let http = reqwest::Client::new();
    let portal_url = format!(
        "{}/v1/billing_portal/sessions",
        stripe.api_base_url.trim_end_matches('/')
    );
    let response = http
        .post(portal_url)
        .basic_auth(&stripe.secret_key, None::<&str>)
        .form(&params)
        .send()
        .await
        .map_err(|err| AppError::internal(format!("stripe billing portal request failed: {err}")))?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::internal(format!("stripe billing portal session failed: {body}")));
    }

    let payload: serde_json::Value = response
        .json()
        .await
        .map_err(|err| AppError::internal(format!("stripe portal response parse failed: {err}")))?;

    let url = payload["url"]
        .as_str()
        .ok_or_else(|| AppError::internal("stripe portal response missing url".to_string()))?
        .to_string();

    Ok(Json(CheckoutSessionResponse { url }))
}

fn verify_stripe_signature(body: &[u8], sig_header: &str, secret: &str) -> bool {
    // Stripe-Signature: t=<timestamp>,v1=<hex_sig>[,v1=<hex_sig2>...]
    let mut timestamp: Option<&str> = None;
    let mut signatures: Vec<&str> = Vec::new();

    for part in sig_header.split(',') {
        if let Some(t) = part.strip_prefix("t=") {
            timestamp = Some(t);
        } else if let Some(sig) = part.strip_prefix("v1=") {
            signatures.push(sig);
        }
    }

    let Some(t) = timestamp else {
        return false;
    };
    if signatures.is_empty() {
        return false;
    }

    let signed_payload = format!("{}.{}", t, String::from_utf8_lossy(body));

    let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(signed_payload.as_bytes());
    let expected = mac.finalize().into_bytes();
    let expected_hex = hex::encode(expected);

    // Constant-time comparison: check if any v1 signature matches.
    signatures.iter().any(|sig| {
        sig.len() == expected_hex.len()
            && sig
                .bytes()
                .zip(expected_hex.bytes())
                .fold(0u8, |acc, (a, b)| acc | (a ^ b))
                == 0
    })
}

pub fn looks_like_stripe_price_id(value: &str) -> bool {
    value.trim().starts_with("price_")
}

pub async fn stripe_webhook_handler(
    State(context): State<GatewayContext>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let stripe = match context.stripe.as_ref() {
        Some(s) => s,
        None => return (StatusCode::NOT_FOUND, "stripe not configured").into_response(),
    };

    let sig_header = match headers.get("stripe-signature").and_then(|v| v.to_str().ok()) {
        Some(s) => s.to_string(),
        None => {
            return (StatusCode::BAD_REQUEST, "missing stripe-signature header").into_response();
        }
    };

    if !verify_stripe_signature(&body, &sig_header, &stripe.webhook_secret) {
        return (StatusCode::UNAUTHORIZED, "invalid stripe signature").into_response();
    }

    let event: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid json").into_response(),
    };

    let event_type = event["type"].as_str().unwrap_or("");
    info!(%event_type, "stripe webhook received");

    let object = &event["data"]["object"];

    // Resolve user identity from event metadata. For checkout sessions, Stripe may only
    // provide client_reference_id, so use it as a fallback.
    let user_id = object["metadata"]["clerk_user_id"]
        .as_str()
        .or_else(|| object["subscription_details"]["metadata"]["clerk_user_id"].as_str())
        .or_else(|| object["client_reference_id"].as_str());

    // Persist Stripe customer mapping from webhook events as canonical source of truth.
    if let (Some(user_id), Some(customer_id)) = (
        user_id,
        object["customer"].as_str().filter(|value| !value.is_empty()),
    ) {
        if let Err(err) = context
            .accounts
            .set_stripe_customer_id(user_id, customer_id)
            .await
        {
            error!(%user_id, %customer_id, %err, "failed to persist stripe customer id from webhook");
        }
    }

    let new_tier: Option<String> = match event_type {
        // Do not grant privileges on checkout completion. Checkout can complete before
        // a durable paid invoice is finalized for subscription flows.
        "checkout.session.completed" => None,
        "invoice.payment_succeeded" => {
            let paid = object["paid"].as_bool().unwrap_or(false);
            let amount_paid = object["amount_paid"].as_i64().unwrap_or(0);
            if paid && amount_paid > 0 {
                let tier = object["subscription_details"]["metadata"]["tier"]
                    .as_str()
                    .or_else(|| object["metadata"]["tier"].as_str())
                    .unwrap_or("resonant");
                Some(tier.to_string())
            } else {
                info!(%paid, amount_paid, "ignoring invoice.payment_succeeded without captured payment");
                None
            }
        }
        "customer.subscription.deleted" | "invoice.payment_failed" => Some("free".to_string()),
        _ => None,
    };

    let Some(tier) = new_tier else {
        return (StatusCode::OK, "event ignored").into_response();
    };

    let Some(user_id) = user_id else {
        error!(%event_type, "stripe webhook missing clerk_user_id in metadata");
        return (StatusCode::OK, "no clerk_user_id in metadata, skipped").into_response();
    };

    match context.accounts.update_tier(user_id, &tier).await {
        Ok(Some(_)) => info!(%user_id, %tier, "account tier updated via stripe webhook"),
        Ok(None) => {
            // User hasn't signed in to the gateway yet — provision and set tier.
            if let Err(err) = context.accounts.provision(user_id).await {
                error!(%user_id, %err, "failed to provision account from stripe webhook");
            } else if let Err(err) = context.accounts.update_tier(user_id, &tier).await {
                error!(%user_id, %err, "failed to set tier after provisioning from stripe webhook");
            }
        }
        Err(err) => {
            error!(%user_id, %err, "failed to update tier from stripe webhook");
            return (StatusCode::INTERNAL_SERVER_ERROR, "tier update failed").into_response();
        }
    }

    (StatusCode::OK, "ok").into_response()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Duration;

    use async_trait::async_trait;
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use axum::routing::post;
    use axum::{Json, Router};
    use hmac::{Hmac, Mac};
    use serde_json::{json, Value};
    use sha2::Sha256;
    use tokio::sync::Mutex;
    use tower::ServiceExt;
    use uuid::Uuid;

    use crate::accounts::{AccountRecord, AccountsRepo};
    use crate::auth::AuthResolver;
    use crate::tenant_pool::TenantPool;
    use crate::{GatewayContext, ObservabilityConfig};

    use super::{
        checkout_handler, customer_portal_handler, looks_like_stripe_price_id,
        stripe_webhook_handler, verify_stripe_signature, StripeConfig,
    };

    #[derive(Default, Clone)]
    struct MockAccountsState {
        stripe_customer_by_user: HashMap<String, String>,
        tier_by_user: HashMap<String, String>,
        set_customer_calls: Vec<(String, String)>,
        update_tier_calls: Vec<(String, String)>,
        provision_calls: Vec<String>,
        get_customer_calls: Vec<String>,
    }

    #[derive(Default)]
    struct MockAccounts {
        state: Mutex<MockAccountsState>,
    }

    impl MockAccounts {
        async fn seed_customer(&self, user_id: &str, customer_id: &str) {
            let mut state = self.state.lock().await;
            state
                .stripe_customer_by_user
                .insert(user_id.to_string(), customer_id.to_string());
        }

        async fn snapshot(&self) -> MockAccountsState {
            self.state.lock().await.clone()
        }
    }

    #[async_trait]
    impl AccountsRepo for MockAccounts {
        async fn set_stripe_customer_id(&self, user_id: &str, customer_id: &str) -> Result<(), String> {
            let mut state = self.state.lock().await;
            state
                .stripe_customer_by_user
                .insert(user_id.to_string(), customer_id.to_string());
            state
                .set_customer_calls
                .push((user_id.to_string(), customer_id.to_string()));
            Ok(())
        }

        async fn get_stripe_customer_id(&self, user_id: &str) -> Result<Option<String>, String> {
            let mut state = self.state.lock().await;
            state.get_customer_calls.push(user_id.to_string());
            Ok(state.stripe_customer_by_user.get(user_id).cloned())
        }

        async fn update_tier(&self, user_id: &str, tier: &str) -> Result<Option<AccountRecord>, String> {
            let mut state = self.state.lock().await;
            state.update_tier_calls.push((user_id.to_string(), tier.to_string()));
            state
                .tier_by_user
                .insert(user_id.to_string(), tier.to_string());
            Ok(Some(AccountRecord {
                user_id: user_id.to_string(),
                created_at: "2026-01-01T00:00:00Z".to_string(),
                tier: tier.to_string(),
            }))
        }

        async fn provision(&self, user_id: &str) -> Result<(), String> {
            let mut state = self.state.lock().await;
            state.provision_calls.push(user_id.to_string());
            state
                .tier_by_user
                .entry(user_id.to_string())
                .or_insert_with(|| "free".to_string());
            Ok(())
        }

        async fn get(&self, user_id: &str) -> Result<Option<AccountRecord>, String> {
            let state = self.state.lock().await;
            Ok(state.tier_by_user.get(user_id).map(|tier| AccountRecord {
                user_id: user_id.to_string(),
                created_at: "2026-01-01T00:00:00Z".to_string(),
                tier: tier.clone(),
            }))
        }
    }

    fn build_test_context(accounts: Arc<dyn AccountsRepo>, stripe: Arc<StripeConfig>) -> GatewayContext {
        let tenant_pool = Arc::new(TenantPool::new(
            std::env::temp_dir().join(format!("resonantia-gateway-test-{}", Uuid::new_v4())),
            "public".to_string(),
            8,
            Duration::from_secs(60),
            None,
        ));

        GatewayContext {
            tenant_pool,
            auth: Arc::new(AuthResolver::off_for_tests()),
            accounts,
            admin_secret: None,
            stripe: Some(stripe),
            ai: None,
            observability: Arc::new(ObservabilityConfig {
                request_log_sample_rate: 1.0,
            }),
        }
    }

    fn build_test_stripe_config(api_base_url: String) -> Arc<StripeConfig> {
        Arc::new(StripeConfig {
            secret_key: "sk_test_123".to_string(),
            webhook_secret: "whsec_test_123".to_string(),
            price_id_resonant: "price_resonant_test".to_string(),
            price_id_soulful: "price_soulful_test".to_string(),
            success_url: "https://account.resonantia.me?payment=success".to_string(),
            cancel_url: "https://account.resonantia.me?payment=cancelled".to_string(),
            api_base_url,
        })
    }

    async fn spawn_mock_stripe_server() -> String {
        let app = Router::new()
            .route(
                "/v1/checkout/sessions",
                post(|| async {
                    Json(json!({
                        "url": "https://checkout.mock/session",
                        "customer": "cus_mock_123"
                    }))
                }),
            )
            .route(
                "/v1/billing_portal/sessions",
                post(|| async {
                    Json(json!({
                        "url": "https://portal.mock/session"
                    }))
                }),
            );

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("mock stripe listener should bind");
        let addr = listener
            .local_addr()
            .expect("mock stripe listener should have local addr");

        tokio::spawn(async move {
            if let Err(err) = axum::serve(listener, app).await {
                panic!("mock stripe server failed: {err}");
            }
        });

        format!("http://{addr}")
    }

    #[tokio::test]
    async fn checkout_route_returns_url_and_persists_customer_id() {
        let mock_server_base = spawn_mock_stripe_server().await;
        let stripe = build_test_stripe_config(mock_server_base);
        let mock_accounts = Arc::new(MockAccounts::default());
        let accounts: Arc<dyn AccountsRepo> = mock_accounts.clone();

        let app = Router::new()
            .route("/api/v1/checkout", post(checkout_handler))
            .with_state(build_test_context(accounts, stripe));

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/checkout")
                    .header("content-type", "application/json")
                    .header("x-resonantia-test-user-id", "user_123")
                    .body(Body::from(r#"{"tier":"resonant"}"#))
                    .expect("checkout request should build"),
            )
            .await
            .expect("checkout request should complete");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("checkout response body should read");
        let payload: Value = serde_json::from_slice(&body).expect("checkout response should be json");
        assert_eq!(payload["url"], "https://checkout.mock/session");

        let state = mock_accounts.snapshot().await;
        assert_eq!(
            state.set_customer_calls,
            vec![("user_123".to_string(), "cus_mock_123".to_string())]
        );
        assert_eq!(state.provision_calls, vec!["user_123".to_string()]);
    }

    #[tokio::test]
    async fn customer_portal_route_returns_url_for_existing_customer() {
        let mock_server_base = spawn_mock_stripe_server().await;
        let stripe = build_test_stripe_config(mock_server_base);
        let mock_accounts = Arc::new(MockAccounts::default());
        mock_accounts.seed_customer("user_456", "cus_existing_456").await;
        let accounts: Arc<dyn AccountsRepo> = mock_accounts.clone();

        let app = Router::new()
            .route("/api/v1/customer-portal", post(customer_portal_handler))
            .with_state(build_test_context(accounts, stripe));

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/customer-portal")
                    .header("x-resonantia-test-user-id", "user_456")
                    .body(Body::empty())
                    .expect("customer portal request should build"),
            )
            .await
            .expect("customer portal request should complete");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("customer portal response body should read");
        let payload: Value = serde_json::from_slice(&body)
            .expect("customer portal response should be json");
        assert_eq!(payload["url"], "https://portal.mock/session");

        let state = mock_accounts.snapshot().await;
        assert_eq!(state.get_customer_calls, vec!["user_456".to_string()]);
        assert_eq!(state.provision_calls, vec!["user_456".to_string()]);
    }

    #[tokio::test]
    async fn webhook_route_updates_tier_and_persists_customer_for_paid_invoice() {
        let stripe = build_test_stripe_config("http://127.0.0.1:9".to_string());
        let mock_accounts = Arc::new(MockAccounts::default());
        let accounts: Arc<dyn AccountsRepo> = mock_accounts.clone();

        let app = Router::new()
            .route("/stripe/webhook", post(stripe_webhook_handler))
            .with_state(build_test_context(accounts, stripe.clone()));

        let body = json!({
            "type": "invoice.payment_succeeded",
            "data": {
                "object": {
                    "paid": true,
                    "amount_paid": 1500,
                    "customer": "cus_webhook_789",
                    "subscription_details": {
                        "metadata": {
                            "clerk_user_id": "user_789",
                            "tier": "soulful"
                        }
                    },
                    "metadata": {
                        "clerk_user_id": "user_789",
                        "tier": "soulful"
                    }
                }
            }
        });
        let body_bytes = serde_json::to_vec(&body).expect("webhook body should serialize");

        let timestamp = "1710000000";
        let signed_payload = format!("{}.{}", timestamp, String::from_utf8_lossy(&body_bytes));
        let mut mac = Hmac::<Sha256>::new_from_slice(stripe.webhook_secret.as_bytes())
            .expect("hmac init should succeed");
        mac.update(signed_payload.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());
        let sig_header = format!("t={timestamp},v1={signature}");

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/stripe/webhook")
                    .header("stripe-signature", sig_header)
                    .header("content-type", "application/json")
                    .body(Body::from(body_bytes))
                    .expect("webhook request should build"),
            )
            .await
            .expect("webhook request should complete");

        assert_eq!(response.status(), StatusCode::OK);
        let state = mock_accounts.snapshot().await;
        assert_eq!(
            state.set_customer_calls,
            vec![("user_789".to_string(), "cus_webhook_789".to_string())]
        );
        assert_eq!(
            state.update_tier_calls,
            vec![("user_789".to_string(), "soulful".to_string())]
        );
    }

    #[test]
    fn looks_like_stripe_price_id_only_accepts_price_prefix() {
        assert!(looks_like_stripe_price_id("price_123"));
        assert!(looks_like_stripe_price_id("  price_abc  "));
        assert!(!looks_like_stripe_price_id("prod_123"));
        assert!(!looks_like_stripe_price_id(""));
    }

    #[test]
    fn verify_stripe_signature_accepts_valid_signature() {
        let body = br#"{"id":"evt_123","type":"invoice.payment_succeeded"}"#;
        let secret = "whsec_test_secret";
        let timestamp = "1710000000";
        let payload = format!("{}.{}", timestamp, String::from_utf8_lossy(body));

        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
            .expect("hmac init should succeed");
        mac.update(payload.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());
        let sig_header = format!("t={},v1={}", timestamp, signature);

        assert!(verify_stripe_signature(body, &sig_header, secret));
    }

    #[test]
    fn verify_stripe_signature_rejects_invalid_signature() {
        let body = br#"{"id":"evt_123"}"#;
        let secret = "whsec_test_secret";
        let sig_header = "t=1710000000,v1=deadbeef";

        assert!(!verify_stripe_signature(body, sig_header, secret));
    }
}
