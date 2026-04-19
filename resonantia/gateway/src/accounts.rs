use std::path::PathBuf;

use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use surrealdb::engine::any::{connect as surreal_connect, Any as SurrealAny};
use surrealdb::Surreal;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountRecord {
    pub user_id: String,
    /// ISO 8601 string — stored as plain string to avoid SurrealDB datetime round-trip issues.
    pub created_at: String,
    pub tier: String,
}

#[derive(Debug, Serialize)]
pub struct AccountResponse {
    #[serde(rename = "userId")]
    pub user_id: String,
    pub tier: String,
    #[serde(rename = "memberSince")]
    pub member_since: String,
}

pub struct AccountStore {
    db: Surreal<SurrealAny>,
}

#[async_trait]
pub(crate) trait AccountsRepo: Send + Sync {
    async fn set_stripe_customer_id(&self, user_id: &str, customer_id: &str) -> Result<(), String>;
    async fn get_stripe_customer_id(&self, user_id: &str) -> Result<Option<String>, String>;
    async fn update_tier(&self, user_id: &str, tier: &str) -> Result<Option<AccountRecord>, String>;
    async fn provision(&self, user_id: &str) -> Result<(), String>;
    async fn get(&self, user_id: &str) -> Result<Option<AccountRecord>, String>;
}

impl AccountStore {
    pub async fn open(data_root: &PathBuf) -> Result<Self, String> {
        let accounts_dir = data_root.join("accounts");
        std::fs::create_dir_all(&accounts_dir)
            .map_err(|err| format!("failed to create accounts dir: {err}"))?;

        let endpoint = format!("surrealkv://{}", accounts_dir.display());
        Self::connect_inner(&endpoint, None, None).await
    }

    pub async fn open_remote(
        endpoint: &str,
        username: &str,
        password: &str,
    ) -> Result<Self, String> {
        Self::connect_inner(endpoint, Some(username), Some(password)).await
    }

    async fn connect_inner(
        endpoint: &str,
        username: Option<&str>,
        password: Option<&str>,
    ) -> Result<Self, String> {
        let db: Surreal<SurrealAny> = surreal_connect(endpoint)
            .await
            .map_err(|err| format!("failed to open accounts db: {err}"))?;

        if let (Some(user), Some(pass)) = (username, password) {
            use surrealdb::opt::auth::Root;
            db.signin(Root { username: user.to_string(), password: pass.to_string() })
                .await
                .map_err(|err| format!("failed to sign in to accounts db: {err}"))?;
        }

        db.use_ns("resonantia")
            .use_db("accounts")
            .await
            .map_err(|err| format!("failed to select accounts namespace: {err}"))?;

        db.query(
            "DEFINE TABLE IF NOT EXISTS account SCHEMAFULL;\
             DEFINE FIELD IF NOT EXISTS user_id ON TABLE account TYPE string;\
             DEFINE FIELD IF NOT EXISTS created_at ON TABLE account TYPE string;\
             DEFINE FIELD IF NOT EXISTS tier ON TABLE account TYPE string DEFAULT 'free';\
             DEFINE FIELD IF NOT EXISTS stripe_customer_id ON TABLE account TYPE option<string>;\
             DEFINE INDEX IF NOT EXISTS idx_account_user_id ON TABLE account FIELDS user_id UNIQUE;",
        )
        .await
        .map_err(|err| format!("failed to define account schema: {err}"))?;

        Ok(Self { db })
    }

    pub async fn set_stripe_customer_id(&self, user_id: &str, customer_id: &str) -> Result<(), String> {
        self.db
            .query("UPDATE account SET stripe_customer_id = $cid WHERE user_id = $user_id")
            .bind(("user_id", user_id.to_string()))
            .bind(("cid", customer_id.to_string()))
            .await
            .map_err(|err| format!("failed to set stripe customer id: {err}"))?;
        Ok(())
    }

    pub async fn get_stripe_customer_id(&self, user_id: &str) -> Result<Option<String>, String> {
        let mut result = self
            .db
            .query("SELECT stripe_customer_id FROM account WHERE user_id = $user_id LIMIT 1")
            .bind(("user_id", user_id.to_string()))
            .await
            .map_err(|err| format!("failed to query stripe customer id: {err}"))?;

        let values: Vec<serde_json::Value> = result
            .take(0)
            .map_err(|err| format!("failed to take stripe customer id result: {err}"))?;

        Ok(values
            .into_iter()
            .next()
            .and_then(|v| v["stripe_customer_id"].as_str().map(str::to_string)))
    }

    pub async fn update_tier(&self, user_id: &str, tier: &str) -> Result<Option<AccountRecord>, String> {
        let user_id = user_id.to_string();
        let tier = tier.to_string();
        self.db
            .query("UPDATE account SET tier = $tier WHERE user_id = $user_id")
            .bind(("user_id", user_id.clone()))
            .bind(("tier", tier))
            .await
            .map_err(|err| format!("failed to update account tier: {err}"))?;

        self.get(&user_id).await
    }

    /// Insert the user account if it does not already exist. No-op on duplicate.
    pub async fn provision(&self, user_id: &str) -> Result<(), String> {
        let now = Utc::now().to_rfc3339();
        self.db
            .query(
                "INSERT INTO account { user_id: $user_id, created_at: $now, tier: 'free' } \
                 ON DUPLICATE KEY UPDATE user_id = user_id",
            )
            .bind(("user_id", user_id.to_string()))
            .bind(("now", now))
            .await
            .map_err(|err| format!("failed to provision account: {err}"))?;
        Ok(())
    }

    pub async fn get(&self, user_id: &str) -> Result<Option<AccountRecord>, String> {
        let user_id = user_id.to_string();
        let mut result = self
            .db
            .query("SELECT user_id, created_at, tier FROM account WHERE user_id = $user_id LIMIT 1")
            .bind(("user_id", user_id))
            .await
            .map_err(|err| format!("failed to query account: {err}"))?;

        // SurrealDB v3 — take as serde_json::Value then deserialize.
        let values: Vec<serde_json::Value> = result
            .take(0)
            .map_err(|err| format!("failed to take account results: {err}"))?;

        values
            .into_iter()
            .next()
            .map(|v| {
                serde_json::from_value::<AccountRecord>(v)
                    .map_err(|err| format!("failed to deserialize account: {err}"))
            })
            .transpose()
    }
}

#[async_trait]
impl AccountsRepo for AccountStore {
    async fn set_stripe_customer_id(&self, user_id: &str, customer_id: &str) -> Result<(), String> {
        AccountStore::set_stripe_customer_id(self, user_id, customer_id).await
    }

    async fn get_stripe_customer_id(&self, user_id: &str) -> Result<Option<String>, String> {
        AccountStore::get_stripe_customer_id(self, user_id).await
    }

    async fn update_tier(&self, user_id: &str, tier: &str) -> Result<Option<AccountRecord>, String> {
        AccountStore::update_tier(self, user_id, tier).await
    }

    async fn provision(&self, user_id: &str) -> Result<(), String> {
        AccountStore::provision(self, user_id).await
    }

    async fn get(&self, user_id: &str) -> Result<Option<AccountRecord>, String> {
        AccountStore::get(self, user_id).await
    }
}
