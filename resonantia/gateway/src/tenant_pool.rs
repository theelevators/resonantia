use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use resonantia_core::{
    create_app_state, initialize_app_state, initialize_app_state_remote_strict, AppState,
};
use tokio::sync::RwLock;
use tracing::info;

#[derive(Clone)]
pub(crate) struct SurrealConfig {
    pub(crate) endpoint: String,
    pub(crate) namespace: String,
    pub(crate) username: String,
    pub(crate) password: String,
}

pub(crate) struct TenantPool {
    data_root: PathBuf,
    pub(crate) default_tenant: String,
    states: RwLock<HashMap<String, TenantStateEntry>>,
    max_cached_tenants: usize,
    tenant_idle_ttl: Duration,
    surreal: Option<Arc<SurrealConfig>>,
}

struct TenantStateEntry {
    state: Arc<AppState>,
    last_access: Instant,
}

impl TenantPool {
    pub(crate) fn new(
        data_root: PathBuf,
        default_tenant: String,
        max_cached_tenants: usize,
        tenant_idle_ttl: Duration,
        surreal: Option<Arc<SurrealConfig>>,
    ) -> Self {
        Self {
            data_root,
            default_tenant,
            states: RwLock::new(HashMap::new()),
            max_cached_tenants,
            tenant_idle_ttl,
            surreal,
        }
    }

    pub(crate) async fn state_for(&self, tenant_id: &str) -> Result<Arc<AppState>, String> {
        {
            let mut guard = self.states.write().await;
            Self::evict_idle_locked(&mut guard, self.tenant_idle_ttl);
            if let Some(existing) = guard.get_mut(tenant_id) {
                existing.last_access = Instant::now();
                return Ok(existing.state.clone());
            }
            if guard.len() >= self.max_cached_tenants {
                Self::evict_one_lru_locked(&mut guard);
            }
        }

        let state = if let Some(surreal) = &self.surreal {
            // Remote SurrealDB — tenant_id is the database name, no local dir needed.
            let endpoint = surreal.endpoint.clone();
            let namespace = surreal.namespace.clone();
            let database = tenant_id.to_string();
            let username = surreal.username.clone();
            let password = surreal.password.clone();

            tokio::task::spawn_blocking(move || {
                let state = Arc::new(create_app_state());
                initialize_app_state_remote_strict(
                    &state,
                    &endpoint,
                    &namespace,
                    &database,
                    &username,
                    &password,
                )?;
                Ok::<Arc<AppState>, String>(state)
            })
            .await
            .map_err(|err| format!("failed to join remote app-state init task: {err}"))??
        } else {
            // Embedded surrealkv fallback — one file per tenant.
            let tenant_dir = self.data_root.join("tenants").join(tenant_id);

            tokio::task::spawn_blocking(move || {
                let state = Arc::new(create_app_state());
                initialize_app_state(&state, &tenant_dir)?;
                Ok::<Arc<AppState>, String>(state)
            })
            .await
            .map_err(|err| format!("failed to join local app-state init task: {err}"))??
        };

        let mut guard = self.states.write().await;
        Self::evict_idle_locked(&mut guard, self.tenant_idle_ttl);
        if let Some(existing) = guard.get_mut(tenant_id) {
            existing.last_access = Instant::now();
            return Ok(existing.state.clone());
        }
        if guard.len() >= self.max_cached_tenants {
            Self::evict_one_lru_locked(&mut guard);
        }

        guard.insert(
            tenant_id.to_string(),
            TenantStateEntry {
                state: state.clone(),
                last_access: Instant::now(),
            },
        );

        Ok(state)
    }

    async fn evict_idle_tenants(&self) -> usize {
        let mut guard = self.states.write().await;
        Self::evict_idle_locked(&mut guard, self.tenant_idle_ttl)
    }

    fn evict_idle_locked(
        states: &mut HashMap<String, TenantStateEntry>,
        tenant_idle_ttl: Duration,
    ) -> usize {
        let before = states.len();
        states.retain(|_, entry| entry.last_access.elapsed() <= tenant_idle_ttl);
        before.saturating_sub(states.len())
    }

    fn evict_one_lru_locked(states: &mut HashMap<String, TenantStateEntry>) -> Option<String> {
        let oldest_key = states
            .iter()
            .max_by_key(|(_, entry)| entry.last_access.elapsed())
            .map(|(tenant_id, _)| tenant_id.clone())?;
        states.remove(&oldest_key);
        Some(oldest_key)
    }
}

pub(crate) fn start_tenant_cache_cleanup(tenant_pool: Arc<TenantPool>, interval: Duration) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            ticker.tick().await;
            let evicted = tenant_pool.evict_idle_tenants().await;
            if evicted > 0 {
                info!(evicted, "evicted idle tenant states from cache");
            }
        }
    });
}
