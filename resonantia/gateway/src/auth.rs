use std::env;
use std::time::{Duration, Instant};

use axum::http::HeaderMap;
use chrono::Utc;
use jsonwebtoken::errors::ErrorKind;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::{AppError, GatewayContext};

pub(crate) struct UserContext {
    pub(crate) tenant_id: String,
    /// Present only when auth mode is Clerk and JWT was verified.
    pub(crate) user_id: Option<String>,
}

enum GatewayAuthMode {
    Off,
    Clerk(ClerkAuth),
}

pub(crate) struct AuthResolver {
    mode: GatewayAuthMode,
}

struct ClerkAuth {
    issuer: String,
    audience: Option<String>,
    tenant_claim: String,
    allow_tenant_header_fallback: bool,
    token_leeway_seconds: u64,
    jwks_url: String,
    jwks_cache_ttl: Duration,
    http: reqwest::Client,
    jwks_cache: RwLock<Option<CachedJwks>>,
}

struct CachedJwks {
    fetched_at: Instant,
    keys: Vec<Jwk>,
}

#[derive(Deserialize)]
struct JwksResponse {
    keys: Vec<Jwk>,
}

#[derive(Clone, Deserialize)]
struct Jwk {
    kid: Option<String>,
    kty: String,
    n: String,
    e: String,
    alg: Option<String>,
    #[serde(rename = "use")]
    use_field: Option<String>,
}

pub(crate) async fn resolve_user_context(
    context: &GatewayContext,
    headers: &HeaderMap,
) -> Result<UserContext, AppError> {
    #[cfg(test)]
    if let Some(test_user_id) = test_user_id_from_headers(headers) {
        let tenant_id = test_tenant_id_from_headers(headers)
            .unwrap_or_else(|| sanitize_tenant_id(&test_user_id));
        let user_ctx = UserContext {
            tenant_id,
            user_id: Some(test_user_id),
        };

        if let Some(user_id) = user_ctx.user_id.as_deref() {
            if let Err(err) = context.accounts.provision(user_id).await {
                error!(%err, "account provisioning failed");
            }
        }

        return Ok(user_ctx);
    }

    let user_ctx = context
        .auth
        .resolve_user_context(headers, &context.tenant_pool.default_tenant)
        .await?;

    // Auto-provision the account record when we have a real user identity.
    if let Some(ref user_id) = user_ctx.user_id {
        if let Err(err) = context.accounts.provision(user_id).await {
            // Non-fatal: log but don't block the request.
            error!(%err, "account provisioning failed");
        }
    }

    Ok(user_ctx)
}

impl AuthResolver {
    pub(crate) fn from_env() -> Result<Self, String> {
        let mode = env::var("RESONANTIA_GATEWAY_AUTH_MODE")
            .unwrap_or_else(|_| "off".to_string())
            .trim()
            .to_ascii_lowercase();

        if mode.is_empty() || mode == "off" {
            info!("gateway auth mode: off (header/default tenant mode)");
            return Ok(Self {
                mode: GatewayAuthMode::Off,
            });
        }

        if mode != "clerk" {
            return Err(format!(
                "unsupported RESONANTIA_GATEWAY_AUTH_MODE: {mode} (expected off|clerk)"
            ));
        }

        let issuer = env::var("RESONANTIA_GATEWAY_CLERK_ISSUER")
            .map_err(|_| "RESONANTIA_GATEWAY_CLERK_ISSUER is required when auth mode is clerk".to_string())?
            .trim()
            .trim_end_matches('/')
            .to_string();
        if issuer.is_empty() {
            return Err("RESONANTIA_GATEWAY_CLERK_ISSUER cannot be empty".to_string());
        }

        let audience = env::var("RESONANTIA_GATEWAY_CLERK_AUDIENCE")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        let jwks_url = env::var("RESONANTIA_GATEWAY_CLERK_JWKS_URL")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| format!("{issuer}/.well-known/jwks.json"));

        let tenant_claim = env::var("RESONANTIA_GATEWAY_CLERK_TENANT_CLAIM")
            .unwrap_or_else(|_| "org_id".to_string())
            .trim()
            .to_string();
        if tenant_claim.is_empty() {
            return Err("RESONANTIA_GATEWAY_CLERK_TENANT_CLAIM cannot be empty".to_string());
        }

        let jwks_ttl_seconds = env::var("RESONANTIA_GATEWAY_CLERK_JWKS_CACHE_SECONDS")
            .ok()
            .and_then(|value| value.trim().parse::<u64>().ok())
            .unwrap_or(300);

        let allow_tenant_header_fallback =
            env_flag("RESONANTIA_GATEWAY_ALLOW_TENANT_HEADER_FALLBACK", false);

        let token_leeway_seconds = env::var("RESONANTIA_GATEWAY_CLERK_TOKEN_LEEWAY_SECONDS")
            .ok()
            .and_then(|value| value.trim().parse::<u64>().ok())
            .unwrap_or(60);

        info!(
            %issuer,
            %jwks_url,
            tenant_claim = %tenant_claim,
            token_leeway_seconds,
            "gateway auth mode: clerk"
        );

        Ok(Self {
            mode: GatewayAuthMode::Clerk(ClerkAuth {
                issuer,
                audience,
                tenant_claim,
                allow_tenant_header_fallback,
                token_leeway_seconds,
                jwks_url,
                jwks_cache_ttl: Duration::from_secs(jwks_ttl_seconds.max(15)),
                http: reqwest::Client::new(),
                jwks_cache: RwLock::new(None),
            }),
        })
    }

    #[cfg(test)]
    pub(crate) fn off_for_tests() -> Self {
        Self {
            mode: GatewayAuthMode::Off,
        }
    }

    async fn resolve_user_context(
        &self,
        headers: &HeaderMap,
        default_tenant: &str,
    ) -> Result<UserContext, AppError> {
        match &self.mode {
            GatewayAuthMode::Off => Ok(UserContext {
                tenant_id: tenant_id_from_headers(headers)
                    .unwrap_or_else(|| sanitize_tenant_id(default_tenant)),
                user_id: None,
            }),
            GatewayAuthMode::Clerk(clerk) => clerk.resolve_user_context(headers).await,
        }
    }
}

impl ClerkAuth {
    async fn resolve_user_context(&self, headers: &HeaderMap) -> Result<UserContext, AppError> {
        let token = bearer_token_from_headers(headers)
            .ok_or_else(|| AppError::unauthorized("missing bearer token".to_string()))?;

        let claims = self.verify_token(token).await?;

        // `sub` is always the stable Clerk user ID — use as account identity.
        let user_id = claim_string(&claims, "sub").ok_or_else(|| {
            AppError::unauthorized("token missing sub claim".to_string())
        })?;
        let sanitized_user_id = sanitize_tenant_id(&user_id);

        // Tenant can be org_id (or the configured claim) for multi-org setups,
        // falling back to the user's own sub so solo users get their own space.
        let tenant_id = if let Some(tenant) = claim_string(&claims, &self.tenant_claim) {
            let sanitized = sanitize_tenant_id(&tenant);
            if sanitized != "public" {
                sanitized
            } else {
                sanitized_user_id.clone()
            }
        } else {
            sanitized_user_id.clone()
        };

        if tenant_id == "public" {
            if self.allow_tenant_header_fallback {
                if let Some(header_tenant) = tenant_id_from_headers(headers) {
                    return Ok(UserContext {
                        tenant_id: header_tenant,
                        user_id: Some(sanitized_user_id),
                    });
                }
            }
            return Err(AppError::unauthorized(
                "token did not include a usable tenant claim".to_string(),
            ));
        }

        Ok(UserContext {
            tenant_id,
            user_id: Some(sanitized_user_id),
        })
    }

    async fn verify_token(&self, token: &str) -> Result<Value, AppError> {
        let header = decode_header(token)
            .map_err(|err| AppError::unauthorized(format!("invalid token header: {err}")))?;

        if header.alg != Algorithm::RS256 {
            return Err(AppError::unauthorized(
                "unsupported token algorithm; expected RS256".to_string(),
            ));
        }

        let kid = header
            .kid
            .ok_or_else(|| AppError::unauthorized("token missing kid".to_string()))?;
        let jwk = self
            .jwk_for_kid(&kid)
            .await
            .map_err(AppError::unauthorized)?;

        let decoding_key = DecodingKey::from_rsa_components(&jwk.n, &jwk.e)
            .map_err(|err| AppError::unauthorized(format!("invalid jwk key material: {err}")))?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[self.issuer.as_str()]);
        validation.leeway = self.token_leeway_seconds;
        if let Some(audience) = &self.audience {
            validation.set_audience(&[audience.as_str()]);
        }

        let data = decode::<Value>(token, &decoding_key, &validation).map_err(|err| {
            if matches!(err.kind(), ErrorKind::ExpiredSignature) {
                // Re-parse claims with exp/nbf checks disabled so we can log token timing details.
                let mut relaxed = validation.clone();
                relaxed.validate_exp = false;
                relaxed.validate_nbf = false;

                if let Ok(relaxed_data) = decode::<Value>(token, &decoding_key, &relaxed) {
                    let now = Utc::now().timestamp();
                    let exp = relaxed_data.claims.get("exp").and_then(|v| v.as_i64());
                    let iat = relaxed_data.claims.get("iat").and_then(|v| v.as_i64());
                    let nbf = relaxed_data.claims.get("nbf").and_then(|v| v.as_i64());

                    warn!(
                        now_ts = now,
                        exp_ts = ?exp,
                        iat_ts = ?iat,
                        nbf_ts = ?nbf,
                        leeway_seconds = self.token_leeway_seconds,
                        "clerk token rejected as expired"
                    );

                    if let Some(exp_ts) = exp {
                        return AppError::unauthorized(format!(
                            "token verification failed: ExpiredSignature (now={now}, exp={exp_ts}, leeway={})",
                            self.token_leeway_seconds
                        ));
                    }
                }
            }

            AppError::unauthorized(format!("token verification failed: {err}"))
        })?;

        Ok(data.claims)
    }

    async fn jwk_for_kid(&self, kid: &str) -> Result<Jwk, String> {
        if let Some(key) = self.find_cached_jwk(kid).await {
            return Ok(key);
        }

        let fetched = self.fetch_jwks().await?;
        fetched
            .into_iter()
            .find(|key| {
                key.kid.as_deref() == Some(kid)
                    && key.kty.eq_ignore_ascii_case("rsa")
                    && key
                        .alg
                        .as_deref()
                        .map(|value| value.eq_ignore_ascii_case("rs256"))
                        .unwrap_or(true)
                    && key
                        .use_field
                        .as_deref()
                        .map(|value| value.eq_ignore_ascii_case("sig"))
                        .unwrap_or(true)
            })
            .ok_or_else(|| format!("no JWK found for kid={kid}"))
    }

    async fn find_cached_jwk(&self, kid: &str) -> Option<Jwk> {
        let guard = self.jwks_cache.read().await;
        let cache = guard.as_ref()?;
        if cache.fetched_at.elapsed() > self.jwks_cache_ttl {
            return None;
        }

        cache
            .keys
            .iter()
            .find(|key| key.kid.as_deref() == Some(kid))
            .cloned()
    }

    async fn fetch_jwks(&self) -> Result<Vec<Jwk>, String> {
        let response = self
            .http
            .get(&self.jwks_url)
            .send()
            .await
            .map_err(|err| format!("failed to fetch JWKS: {err}"))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("failed to fetch JWKS: {status} {body}"));
        }

        let payload = response
            .json::<JwksResponse>()
            .await
            .map_err(|err| format!("failed to parse JWKS payload: {err}"))?;

        {
            let mut guard = self.jwks_cache.write().await;
            *guard = Some(CachedJwks {
                fetched_at: Instant::now(),
                keys: payload.keys.clone(),
            });
        }

        Ok(payload.keys)
    }
}

fn tenant_id_from_headers(headers: &HeaderMap) -> Option<String> {
    let candidates = ["x-resonantia-tenant", "x-tenant-id", "x-tenant"];

    for key in candidates {
        let value = headers.get(key)?.to_str().ok()?.trim();
        if !value.is_empty() {
            return Some(sanitize_tenant_id(value));
        }
    }

    None
}

fn sanitize_tenant_id(input: &str) -> String {
    let mut sanitized = String::with_capacity(input.len());

    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            sanitized.push(ch.to_ascii_lowercase());
        } else if matches!(ch, '-' | '_') {
            sanitized.push(ch);
        }
    }

    let trimmed = sanitized.trim_matches(['-', '_']);
    if trimmed.is_empty() {
        "public".to_string()
    } else {
        trimmed.to_string()
    }
}

fn env_flag(name: &str, default: bool) -> bool {
    match env::var(name) {
        Ok(value) => matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        Err(_) => default,
    }
}

fn bearer_token_from_headers(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get("authorization")?.to_str().ok()?.trim();
    let (scheme, token) = value.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("bearer") {
        return None;
    }

    let trimmed = token.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn claim_string(claims: &Value, key: &str) -> Option<String> {
    claims
        .get(key)
        .and_then(|value| value.as_str())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
fn test_user_id_from_headers(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-resonantia-test-user-id")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(sanitize_tenant_id)
        .filter(|value| value != "public")
}

#[cfg(test)]
fn test_tenant_id_from_headers(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-resonantia-test-tenant")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(sanitize_tenant_id)
        .filter(|value| value != "public")
}

#[cfg(test)]
mod tests {
    use super::{bearer_token_from_headers, sanitize_tenant_id};
    use axum::http::{HeaderMap, HeaderValue};

    #[test]
    fn sanitize_tenant_id_normalizes_allowed_characters() {
        assert_eq!(sanitize_tenant_id("Acme Inc._Team-01"), "acmeinc_team-01");
    }

    #[test]
    fn sanitize_tenant_id_falls_back_to_public_when_empty() {
        assert_eq!(sanitize_tenant_id("!!!"), "public");
    }

    #[test]
    fn bearer_token_from_headers_accepts_case_insensitive_scheme() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", HeaderValue::from_static("BeAreR token-123"));

        assert_eq!(bearer_token_from_headers(&headers), Some("token-123"));
    }

    #[test]
    fn bearer_token_from_headers_rejects_invalid_values() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", HeaderValue::from_static("Basic abc"));
        assert_eq!(bearer_token_from_headers(&headers), None);

        headers.insert("authorization", HeaderValue::from_static("Bearer   "));
        assert_eq!(bearer_token_from_headers(&headers), None);
    }
}
