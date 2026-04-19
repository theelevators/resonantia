use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::{Query, Request, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use opentelemetry::global;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry::KeyValue;
use opentelemetry_http::HeaderExtractor;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::runtime::Tokio;
use opentelemetry_sdk::{trace as sdktrace, Resource};
use sha2::{Digest, Sha256};
use resonantia_core::{
    get_health, get_graph,
    list_nodes, rename_session, store_context,
    GraphResponse, HealthResponse, ListNodesResponse, StoreContextRequest,
    StoreContextResponse, RenameSessionRequest, RenameSessionResponse,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info, warn, Instrument};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

mod accounts;
mod auth;
mod stripe;
mod tenant_context;
mod tenant_pool;

use accounts::{AccountResponse, AccountStore, AccountsRepo};
use auth::{resolve_user_context, AuthResolver};
use stripe::{checkout_handler, customer_portal_handler, looks_like_stripe_price_id, stripe_webhook_handler, StripeConfig};
use tenant_context::{resolve_sync_tenant_state, resolve_tenant_state};
use tenant_pool::{start_tenant_cache_cleanup, SurrealConfig, TenantPool};

// ── Gateway context ───────────────────────────────────────────────────────────

#[derive(Clone)]
struct GatewayContext {
    tenant_pool: Arc<TenantPool>,
    auth: Arc<AuthResolver>,
    accounts: Arc<dyn AccountsRepo>,
    admin_secret: Option<String>,
    stripe: Option<Arc<StripeConfig>>,
    ai: Option<Arc<AiConfig>>,
    observability: Arc<ObservabilityConfig>,
}

#[derive(Clone)]
struct ObservabilityConfig {
    request_log_sample_rate: f64,
}

#[derive(Clone)]
struct AiConfig {
    openai_api_key: String,
    openai_model: String,
    openai_base_url: String,
    require_soulful_for_chat: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListNodesQuery {
    limit: Option<i32>,
    session_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AiChatRequest {
    messages: Vec<AiMessage>,
    purpose: Option<String>,
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct AiMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AiChatResponse {
    content: String,
    provider: String,
    model: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OpenAiChatRequest {
    model: String,
    messages: Vec<AiMessage>,
    stream: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenAiChatResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenAiChoice {
    message: OpenAiAssistantMessage,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenAiAssistantMessage {
    content: String,
}

#[tokio::main]
async fn main() {
    let telemetry = init_tracing();

    let bind_addr = env::var("RESONANTIA_GATEWAY_BIND")
        .unwrap_or_else(|_| "0.0.0.0:8090".to_string())
        .parse::<SocketAddr>()
        .expect("RESONANTIA_GATEWAY_BIND must be a valid socket address");

    let data_root = PathBuf::from(
        env::var("RESONANTIA_GATEWAY_DATA_DIR")
            .unwrap_or_else(|_| "./gateway-data".to_string()),
    );
    let default_tenant = env::var("RESONANTIA_GATEWAY_DEFAULT_TENANT")
        .unwrap_or_else(|_| "public".to_string());
    let max_cached_tenants = env::var("RESONANTIA_GATEWAY_MAX_CACHED_TENANTS")
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .unwrap_or(256)
        .max(1);
    let tenant_idle_ttl_seconds = env::var("RESONANTIA_GATEWAY_TENANT_IDLE_TTL_SECONDS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(1800)
        .max(30);
    let tenant_cache_cleanup_seconds = env::var("RESONANTIA_GATEWAY_TENANT_CACHE_CLEANUP_SECONDS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(300)
        .max(15);

    let surreal_config = {
        let endpoint = env::var("RESONANTIA_SURREALDB_ENDPOINT").ok().filter(|s| !s.trim().is_empty());
        let namespace = env::var("RESONANTIA_SURREALDB_NS").ok().filter(|s| !s.trim().is_empty());
        let username = env::var("RESONANTIA_SURREALDB_USER").ok().filter(|s| !s.trim().is_empty());
        let password = env::var("RESONANTIA_SURREALDB_PASS").ok().filter(|s| !s.trim().is_empty());
        match (endpoint, namespace, username, password) {
            (Some(endpoint), Some(namespace), Some(username), Some(password)) => {
                info!(%endpoint, %namespace, "remote surrealdb configured for node storage");
                Some(Arc::new(SurrealConfig { endpoint, namespace, username, password }))
            }
            _ => {
                info!("no RESONANTIA_SURREALDB_* vars set — falling back to embedded surrealkv per-tenant");
                None
            }
        }
    };

    let account_store = if let Some(ref surreal) = surreal_config {
        AccountStore::open_remote(&surreal.endpoint, &surreal.username, &surreal.password)
            .await
            .expect("failed to open remote account store")
    } else {
        AccountStore::open(&data_root)
            .await
            .expect("failed to open account store")
    };
    let accounts: Arc<dyn AccountsRepo> = Arc::new(account_store);

    let tenant_pool = Arc::new(TenantPool::new(
        data_root.clone(),
        default_tenant.clone(),
        max_cached_tenants,
        Duration::from_secs(tenant_idle_ttl_seconds),
        surreal_config.clone(),
    ));

    info!(
        max_cached_tenants,
        tenant_idle_ttl_seconds,
        tenant_cache_cleanup_seconds,
        "tenant cache configured"
    );

    let admin_secret = env::var("RESONANTIA_GATEWAY_ADMIN_SECRET")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let stripe = {
        let key = env::var("RESONANTIA_STRIPE_SECRET_KEY").ok().filter(|s| !s.trim().is_empty());
        let webhook_secret = env::var("RESONANTIA_STRIPE_WEBHOOK_SECRET").ok().filter(|s| !s.trim().is_empty());
        let price_id_resonant = env::var("RESONANTIA_STRIPE_PRICE_ID_RESONANT").ok().filter(|s| !s.trim().is_empty());
        let price_id_soulful = env::var("RESONANTIA_STRIPE_PRICE_ID_SOULFUL").ok().filter(|s| !s.trim().is_empty());
        match (key, webhook_secret, price_id_resonant, price_id_soulful) {
            (Some(secret_key), Some(webhook_secret), Some(price_id_resonant), Some(price_id_soulful)) => {
                if !looks_like_stripe_price_id(&price_id_resonant) {
                    warn!(price_id_resonant, "stripe resonant tier id does not look like a Price ID (expected prefix 'price_')");
                }
                if !looks_like_stripe_price_id(&price_id_soulful) {
                    warn!(price_id_soulful, "stripe soulful tier id does not look like a Price ID (expected prefix 'price_')");
                }
                let success_url = env::var("RESONANTIA_STRIPE_SUCCESS_URL")
                    .unwrap_or_else(|_| "https://account.resonantia.me?payment=success".to_string());
                let cancel_url = env::var("RESONANTIA_STRIPE_CANCEL_URL")
                    .unwrap_or_else(|_| "https://account.resonantia.me?payment=cancelled".to_string());
                let api_base_url = env::var("RESONANTIA_STRIPE_API_BASE_URL")
                    .ok()
                    .map(|value| value.trim().trim_end_matches('/').to_string())
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| "https://api.stripe.com".to_string());
                info!("stripe integration enabled");
                Some(Arc::new(StripeConfig {
                    secret_key,
                    webhook_secret,
                    price_id_resonant,
                    price_id_soulful,
                    success_url,
                    cancel_url,
                    api_base_url,
                }))
            }
            _ => {
                info!("stripe integration disabled (missing STRIPE_SECRET_KEY / WEBHOOK_SECRET / PRICE_ID_RESONANT / PRICE_ID_SOULFUL)");
                None
            }
        }
    };

    let observability = Arc::new(read_observability_config());

    let context = GatewayContext {
        tenant_pool: tenant_pool.clone(),
        auth: Arc::new(AuthResolver::from_env().expect("invalid gateway auth configuration")),
        accounts,
        admin_secret,
        stripe,
        ai: read_ai_config(),
        observability,
    };

    start_tenant_cache_cleanup(
        tenant_pool,
        Duration::from_secs(tenant_cache_cleanup_seconds),
    );

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/api/v1/account", get(account_handler))
        .route("/api/v1/account/tier", axum::routing::patch(update_tier_handler))
        .route("/api/v1/checkout", post(checkout_handler))
        .route("/api/v1/customer-portal", post(customer_portal_handler))
        .route("/stripe/webhook", post(stripe_webhook_handler))
        .route("/api/v1/ai/chat", post(ai_chat_handler))
        .route("/api/ai/chat", post(ai_chat_handler))
        .route("/ai/chat", post(ai_chat_handler))
        .route("/api/v1/store", post(store_handler))
        .route("/api/store", post(store_handler))
        .route("/store", post(store_handler))
        .route("/api/v1/session/rename", post(rename_session_handler))
        .route("/api/session/rename", post(rename_session_handler))
        .route("/session/rename", post(rename_session_handler))
        .route("/api/v1/nodes", get(list_nodes_handler))
        .route("/api/nodes", get(list_nodes_handler))
        .route("/nodes", get(list_nodes_handler))
        .route("/api/v1/graph", get(graph_handler))
        .route("/api/graph", get(graph_handler))
        .route("/graph", get(graph_handler))
        .layer(axum::middleware::from_fn_with_state(
            context.clone(),
            observability_middleware,
        ))
        .layer(build_cors_layer())
        .with_state(context);

    info!(%bind_addr, "resonantia gateway listening");

    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .expect("failed to bind gateway listener");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("gateway server failed");

    if telemetry.otel_enabled {
        global::shutdown_tracer_provider();
    }
}

async fn health_handler(
    State(context): State<GatewayContext>,
    headers: HeaderMap,
) -> Result<Json<HealthResponse>, AppError> {
    let tenant_state = resolve_tenant_state(&context, &headers).await?;
    let response = get_health(&tenant_state).await.map_err(AppError::internal)?;
    Ok(Json(response))
}

#[derive(Deserialize)]
struct UpdateTierRequest {
    user_id: String,
    tier: String,
}

fn check_admin_secret(headers: &HeaderMap, secret: &str) -> bool {
    headers
        .get("x-admin-secret")
        .and_then(|v| v.to_str().ok())
        .map(|v| v == secret)
        .unwrap_or(false)
}

async fn update_tier_handler(
    State(context): State<GatewayContext>,
    headers: HeaderMap,
    Json(request): Json<UpdateTierRequest>,
) -> Result<Json<AccountResponse>, AppError> {
    let secret = context.admin_secret.as_deref().unwrap_or("");
    if secret.is_empty() || !check_admin_secret(&headers, secret) {
        return Err(AppError::unauthorized("valid x-admin-secret header required".to_string()));
    }

    if !matches!(request.tier.as_str(), "free" | "resonant" | "soulful") {
        return Err(AppError::bad_request(
            "tier must be 'free', 'resonant', or 'soulful'".to_string(),
        ));
    }

    let record = context
        .accounts
        .update_tier(&request.user_id, &request.tier)
        .await
        .map_err(AppError::internal)?
        .ok_or_else(|| AppError::not_found("account not found".to_string()))?;

    Ok(Json(AccountResponse {
        user_id: record.user_id,
        tier: record.tier,
        member_since: record.created_at,
    }))
}

async fn account_handler(
    State(context): State<GatewayContext>,
    headers: HeaderMap,
) -> Result<Json<AccountResponse>, AppError> {
    let user_ctx = resolve_user_context(&context, &headers).await?;
    let user_id = user_ctx.user_id.ok_or_else(|| {
        AppError::unauthorized("account endpoint requires clerk auth".to_string())
    })?;

    let record = context
        .accounts
        .get(&user_id)
        .await
        .map_err(AppError::internal)?
        .ok_or_else(|| AppError::internal("account record not found after provisioning".to_string()))?;

    Ok(Json(AccountResponse {
        user_id: record.user_id,
        tier: record.tier,
        member_since: record.created_at,
    }))
}

async fn list_nodes_handler(
    State(context): State<GatewayContext>,
    headers: HeaderMap,
    Query(query): Query<ListNodesQuery>,
) -> Result<Json<ListNodesResponse>, AppError> {
    let tenant_state = resolve_sync_tenant_state(&context, &headers).await?;
    let response = list_nodes(
        &tenant_state,
        query.limit.unwrap_or(200),
        query.session_id,
    )
    .await
    .map_err(AppError::internal)?;
    Ok(Json(response))
}

async fn graph_handler(
    State(context): State<GatewayContext>,
    headers: HeaderMap,
    Query(query): Query<ListNodesQuery>,
) -> Result<Json<GraphResponse>, AppError> {
    let tenant_state = resolve_sync_tenant_state(&context, &headers).await?;
    let response = get_graph(
        &tenant_state,
        query.limit.unwrap_or(200),
        query.session_id,
    )
    .await
    .map_err(AppError::internal)?;
    Ok(Json(response))
}

async fn store_handler(
    State(context): State<GatewayContext>,
    headers: HeaderMap,
    Json(request): Json<StoreContextRequest>,
) -> Result<Json<StoreContextResponse>, AppError> {
    let tenant_state = resolve_sync_tenant_state(&context, &headers).await?;
    let response = store_context(&tenant_state, request)
        .await
        .map_err(AppError::internal)?;
    Ok(Json(response))
}

async fn rename_session_handler(
    State(context): State<GatewayContext>,
    headers: HeaderMap,
    Json(request): Json<RenameSessionRequest>,
) -> Result<Json<RenameSessionResponse>, AppError> {
    let tenant_state = resolve_sync_tenant_state(&context, &headers).await?;
    let response = rename_session(&tenant_state, request)
        .await
        .map_err(AppError::bad_request)?;
    Ok(Json(response))
}

fn has_cloud_sync_tier(tier: &str) -> bool {
    matches!(tier, "resonant" | "soulful")
}

fn has_soulful_tier(tier: &str) -> bool {
    matches!(tier, "soulful")
}

fn read_ai_config() -> Option<Arc<AiConfig>> {
    let openai_api_key = env::var("RESONANTIA_OPENAI_API_KEY")
        .ok()
        .or_else(|| env::var("OPENAI_API_KEY").ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let Some(openai_api_key) = openai_api_key else {
        info!("managed ai disabled (missing RESONANTIA_OPENAI_API_KEY/OPENAI_API_KEY)");
        return None;
    };

    let openai_model = env::var("RESONANTIA_OPENAI_MODEL")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "gpt-4o-mini".to_string());

    let openai_base_url = env::var("RESONANTIA_OPENAI_BASE_URL")
        .ok()
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "https://api.openai.com/v1".to_string());

    let require_soulful_for_chat = env::var("RESONANTIA_AI_REQUIRE_SOULFUL_FOR_CHAT")
        .ok()
        .and_then(|value| value.trim().parse::<bool>().ok())
        .unwrap_or(true);

    info!(model = %openai_model, base = %openai_base_url, require_soulful_for_chat, "managed ai enabled");
    Some(Arc::new(AiConfig {
        openai_api_key,
        openai_model,
        openai_base_url,
        require_soulful_for_chat,
    }))
}

fn client_kind(headers: &HeaderMap) -> String {
    headers
        .get("x-resonantia-client")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

fn ai_purpose(input: Option<&str>) -> String {
    input
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_else(|| "chat".to_string())
}

async fn enforce_ai_entitlement(
    context: &GatewayContext,
    user_id: Option<&str>,
    purpose: &str,
    client: &str,
) -> Result<(), AppError> {
    let Some(user_id) = user_id else {
        // BYO/auth-off mode: allow.
        return Ok(());
    };

    let account = context
        .accounts
        .get(user_id)
        .await
        .map_err(AppError::internal)?
        .ok_or_else(|| AppError::unauthorized("account record missing for authenticated user".to_string()))?;

    let allowed = if purpose == "transmutation" {
        has_soulful_tier(&account.tier) || client == "tauri"
    } else if context
        .ai
        .as_ref()
        .map(|ai| ai.require_soulful_for_chat)
        .unwrap_or(true)
    {
        has_soulful_tier(&account.tier)
    } else {
        true
    };

    if allowed {
        return Ok(());
    }

    Err(AppError::forbidden(
        "managed ai requires soulful tier for this operation".to_string(),
    ))
}

async fn call_openai_chat(
    http: &reqwest::Client,
    ai: &AiConfig,
    messages: Vec<AiMessage>,
) -> Result<String, AppError> {
    let url = format!("{}/chat/completions", ai.openai_base_url.trim_end_matches('/'));
    let payload = OpenAiChatRequest {
        model: ai.openai_model.clone(),
        messages,
        stream: false,
    };

    let response = http
        .post(&url)
        .bearer_auth(&ai.openai_api_key)
        .json(&payload)
        .send()
        .await
        .map_err(|err| AppError::internal(format!("openai request failed: {err}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::internal(format!("openai response failed: {status} {body}")));
    }

    let parsed = response
        .json::<OpenAiChatResponse>()
        .await
        .map_err(|err| AppError::internal(format!("openai response parse failed: {err}")))?;

    let content = parsed
        .choices
        .into_iter()
        .next()
        .map(|choice| choice.message.content.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::internal("openai returned empty content".to_string()))?;

    Ok(content)
}

async fn ai_chat_handler(
    State(context): State<GatewayContext>,
    headers: HeaderMap,
    Json(request): Json<AiChatRequest>,
) -> Result<Json<AiChatResponse>, AppError> {
    let ai = context
        .ai
        .as_ref()
        .ok_or_else(|| AppError::bad_request("managed ai is not configured on this gateway".to_string()))?;

    if request.messages.is_empty() {
        return Err(AppError::bad_request("messages are required".to_string()));
    }

    let user_ctx = resolve_user_context(&context, &headers).await?;
    let purpose = ai_purpose(request.purpose.as_deref());
    let client = client_kind(&headers);
    enforce_ai_entitlement(&context, user_ctx.user_id.as_deref(), &purpose, &client).await?;

    let http = reqwest::Client::new();
    let content = call_openai_chat(&http, ai, request.messages).await?;
    Ok(Json(AiChatResponse {
        content,
        provider: "openai".to_string(),
        model: ai.openai_model.clone(),
    }))
}

struct TelemetryRuntime {
    otel_enabled: bool,
}

fn parse_sample_rate_env(name: &str, default: f64) -> f64 {
    env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<f64>().ok())
        .map(|value| value.clamp(0.0, 1.0))
        .unwrap_or(default)
}

fn read_observability_config() -> ObservabilityConfig {
    let request_log_sample_rate = parse_sample_rate_env(
        "RESONANTIA_GATEWAY_OBS_REQUEST_LOG_SAMPLE_RATE",
        0.2,
    );

    info!(request_log_sample_rate, "gateway request observability configured");
    ObservabilityConfig {
        request_log_sample_rate,
    }
}

fn extract_request_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
}

fn generate_request_id() -> String {
    Uuid::new_v4().to_string()
}

fn should_sample(seed: &str, sample_rate: f64) -> bool {
    if sample_rate <= 0.0 {
        return false;
    }
    if sample_rate >= 1.0 {
        return true;
    }

    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    let digest = hasher.finalize();

    let mut slice = [0u8; 8];
    slice.copy_from_slice(&digest[..8]);
    let unit = (u64::from_be_bytes(slice) as f64) / (u64::MAX as f64);
    unit < sample_rate
}

async fn observability_middleware(
    State(context): State<GatewayContext>,
    request: Request,
    next: Next,
) -> Response {
    let started = Instant::now();
    let method = request.method().to_string();
    let path = request.uri().path().to_string();
    let client = client_kind(request.headers());
    let request_id = extract_request_id(request.headers()).unwrap_or_else(generate_request_id);

    let traceparent = request
        .headers()
        .get("traceparent")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string());

    let span = tracing::info_span!(
        "http.request",
        request_id = %request_id,
        method = %method,
        path = %path,
        client = %client,
        traceparent = tracing::field::Empty,
    );

    if let Some(traceparent_value) = traceparent.as_deref() {
        span.record("traceparent", tracing::field::display(traceparent_value));
    }

    global::get_text_map_propagator(|propagator| {
        let parent = propagator.extract(&HeaderExtractor(request.headers()));
        span.set_parent(parent);
    });

    let mut response = next.run(request).instrument(span).await;
    let status_code = response.status().as_u16();
    let duration_ms = started.elapsed().as_millis() as u64;

    if let Ok(value) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert("x-request-id", value);
    }

    let should_log = status_code >= 500
        || should_sample(&request_id, context.observability.request_log_sample_rate);

    if should_log {
        if status_code >= 500 {
            error!(
                request_id = %request_id,
                method = %method,
                path = %path,
                client = %client,
                status_code,
                duration_ms,
                "http request completed with server error"
            );
        } else if status_code >= 400 {
            warn!(
                request_id = %request_id,
                method = %method,
                path = %path,
                client = %client,
                status_code,
                duration_ms,
                "http request completed with client error"
            );
        } else {
            info!(
                request_id = %request_id,
                method = %method,
                path = %path,
                client = %client,
                status_code,
                duration_ms,
                "http request completed"
            );
        }
    }

    response
}

fn build_cors_layer() -> CorsLayer {
    let origins = allowed_origins();
    let mut layer = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::OPTIONS])
        .allow_headers(Any);

    if origins.is_empty() {
        layer = layer.allow_origin(Any);
    } else {
        let parsed: Vec<HeaderValue> = origins
            .iter()
            .filter_map(|origin| HeaderValue::from_str(origin).ok())
            .collect();
        layer = layer.allow_origin(parsed);
    }

    layer
}

fn allowed_origins() -> Vec<String> {
    let raw = env::var("RESONANTIA_GATEWAY_ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "https://app.resonantia.me".to_string());

    raw.split(',')
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect()
}

fn init_tracing() -> TelemetryRuntime {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("resonantia_gateway=info,tower_http=info"));

    let otel_service_name = env::var("RESONANTIA_OTEL_SERVICE_NAME")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "resonantia-gateway".to_string());

    let otlp_endpoint = env::var("RESONANTIA_OTEL_EXPORTER_OTLP_ENDPOINT")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let otel_trace_sample_rate = parse_sample_rate_env("RESONANTIA_OTEL_TRACE_SAMPLE_RATE", 0.1);

    global::set_text_map_propagator(TraceContextPropagator::new());

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .compact();

    if let Some(endpoint) = otlp_endpoint {
        let resource = Resource::new(vec![
            KeyValue::new("service.name", otel_service_name.clone()),
            KeyValue::new("service.namespace", "resonantia"),
        ]);

        let exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .with_endpoint(endpoint.clone())
            .build()
            .expect("failed to build OTLP span exporter");

        let tracer_provider = sdktrace::TracerProvider::builder()
            .with_sampler(sdktrace::Sampler::TraceIdRatioBased(otel_trace_sample_rate))
            .with_resource(resource)
            .with_batch_exporter(exporter, Tokio)
            .build();

        let tracer = tracer_provider.tracer(otel_service_name.clone());
        global::set_tracer_provider(tracer_provider);

        tracing_subscriber::registry()
            .with(filter)
            .with(fmt_layer)
            .with(tracing_opentelemetry::layer().with_tracer(tracer))
            .init();

        info!(
            %endpoint,
            service = %otel_service_name,
            otel_trace_sample_rate,
            "opentelemetry exporter enabled"
        );

        return TelemetryRuntime { otel_enabled: true };
    }

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .init();

    info!("opentelemetry exporter disabled (set RESONANTIA_OTEL_EXPORTER_OTLP_ENDPOINT to enable)");
    TelemetryRuntime { otel_enabled: false }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install terminate handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

struct AppError {
    status: StatusCode,
    message: String,
}

impl AppError {
    fn internal(message: String) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message,
        }
    }

    fn unauthorized(message: String) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            message,
        }
    }

    fn bad_request(message: String) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message,
        }
    }

    fn forbidden(message: String) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            message,
        }
    }

    fn not_found(message: String) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        error!(status = %self.status, message = %self.message, "gateway request failed");
        (self.status, Json(serde_json::json!({ "error": self.message }))).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::{check_admin_secret, has_cloud_sync_tier, has_soulful_tier};
    use axum::http::{HeaderMap, HeaderValue};

    #[test]
    fn has_cloud_sync_tier_accepts_paid_tiers() {
        assert!(has_cloud_sync_tier("resonant"));
        assert!(has_cloud_sync_tier("soulful"));
        assert!(!has_cloud_sync_tier("free"));
    }

    #[test]
    fn has_soulful_tier_accepts_only_soulful() {
        assert!(has_soulful_tier("soulful"));
        assert!(!has_soulful_tier("resonant"));
        assert!(!has_soulful_tier("free"));
    }

    #[test]
    fn check_admin_secret_validates_exact_header_value() {
        let mut headers = HeaderMap::new();
        headers.insert("x-admin-secret", HeaderValue::from_static("correct-secret"));

        assert!(check_admin_secret(&headers, "correct-secret"));
        assert!(!check_admin_secret(&headers, "wrong-secret"));
    }
}
