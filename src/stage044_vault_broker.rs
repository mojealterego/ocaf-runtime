//! Vault-backed secret broker: no application-managed AES master key.
//! Secrets are remotely encrypted/decrypted under a tenant/user/route context.
//! Only operator-provisioned routes are accepted; no caller-controlled URLs.
use crate::stage044_oauth::{private_database, AuthorizedGrant};
use crate::stage044_secret_broker::{https_post, BrokerError, BrokerResponse, BrokerRoute};
use crate::stage044_vault::{KmsError, VaultTransit};
use rusqlite::{params, Connection, OpenFlags, OptionalExtension, TransactionBehavior};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VaultBrokerError {
    #[error("invalid vault broker configuration")]
    Configuration,
    #[error("broker access denied")]
    Denied,
    #[error("secret missing")]
    Missing,
    #[error("durable secret store unavailable")]
    Storage,
    #[error("remote KMS operation rejected")]
    Kms(#[from] KmsError),
    #[error("HTTPS provider call rejected")]
    Transport(#[from] BrokerError),
}

pub struct VaultSecretBroker {
    db_path: PathBuf,
    kms: VaultTransit,
    routes: HashMap<String, BrokerRoute>,
}

impl VaultSecretBroker {
    pub fn open(
        db_path: &Path,
        kms: VaultTransit,
        routes: Vec<BrokerRoute>,
    ) -> Result<Self, VaultBrokerError> {
        if !private_database(db_path) || routes.is_empty() || routes.len() > 1024 {
            return Err(VaultBrokerError::Configuration);
        }
        let mut registered = HashMap::new();
        for route in routes {
            route
                .validate()
                .map_err(|_| VaultBrokerError::Configuration)?;
            if registered.insert(route.route_id.clone(), route).is_some() {
                return Err(VaultBrokerError::Configuration);
            }
        }
        let broker = Self {
            db_path: db_path.to_owned(),
            kms,
            routes: registered,
        };
        broker
            .connection()?
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS oauth_vault_secrets (
                tenant TEXT NOT NULL, user_id TEXT NOT NULL, route TEXT NOT NULL,
                ciphertext TEXT NOT NULL,
                PRIMARY KEY(tenant,user_id,route)
            );",
            )
            .map_err(|_| VaultBrokerError::Storage)?;
        Ok(broker)
    }

    fn connection(&self) -> Result<Connection, VaultBrokerError> {
        let conn = Connection::open_with_flags(
            &self.db_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )
        .map_err(|_| VaultBrokerError::Storage)?;
        conn.busy_timeout(Duration::from_secs(5))
            .map_err(|_| VaultBrokerError::Storage)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")
            .map_err(|_| VaultBrokerError::Storage)?;
        Ok(conn)
    }

    /// Operator-only provisioning, NEVER exposed to an agent HTTP endpoint.
    pub fn provision(
        &self,
        tenant: &str,
        user: &str,
        route_id: &str,
        secret: &[u8],
    ) -> Result<(), VaultBrokerError> {
        if !valid_id(tenant)
            || !valid_id(user)
            || !self.routes.contains_key(route_id)
            || secret.is_empty()
            || secret.len() > 8192
            || secret.iter().any(|b| *b <= 32 || *b >= 127)
        {
            return Err(VaultBrokerError::Configuration);
        }
        let ciphertext = self.kms.encrypt(tenant, user, route_id, secret)?;
        self.connection()?
            .execute(
                "INSERT INTO oauth_vault_secrets(tenant,user_id,route,ciphertext)
             VALUES(?1,?2,?3,?4)
             ON CONFLICT(tenant,user_id,route) DO UPDATE SET ciphertext=excluded.ciphertext",
                params![tenant, user, route_id, ciphertext],
            )
            .map_err(|_| VaultBrokerError::Storage)?;
        Ok(())
    }

    pub fn execute(
        &self,
        grant: &AuthorizedGrant,
        route_id: &str,
        tool_id: &str,
        json_body: &[u8],
    ) -> Result<BrokerResponse, VaultBrokerError> {
        let route = self.routes.get(route_id).ok_or(VaultBrokerError::Denied)?;
        if route.tool_id != tool_id
            || route.resource != grant.resource()
            || grant.assurance_level() < route.min_assurance
            || !route.required_scopes.is_subset(grant.scopes())
            || json_body.len() > 65536
            || serde_json::from_slice::<serde_json::Value>(json_body).is_err()
        {
            return Err(VaultBrokerError::Denied);
        }
        let ciphertext: Option<String> = self
            .connection()?
            .query_row(
                "SELECT ciphertext FROM oauth_vault_secrets
             WHERE tenant=?1 AND user_id=?2 AND route=?3",
                params![grant.tenant(), grant.user(), route_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|_| VaultBrokerError::Storage)?;
        let ciphertext = ciphertext.ok_or(VaultBrokerError::Missing)?;
        let mut secret = self
            .kms
            .decrypt(grant.tenant(), grant.user(), route_id, &ciphertext)?;
        if secret.is_empty() || secret.len() > 8192 || secret.iter().any(|b| *b <= 32 || *b >= 127)
        {
            secret.fill(0);
            return Err(VaultBrokerError::Denied);
        }
        let response = https_post(route, &secret, json_body);
        secret.fill(0);
        response.map_err(VaultBrokerError::Transport)
    }

    /// Rewrap all rows against the current Vault key version. Operator-only;
    /// serialize with provisioning and block tool execution during migration.
    pub fn rewrap_all(&self) -> Result<usize, VaultBrokerError> {
        let mut conn = self.connection()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| VaultBrokerError::Storage)?;
        type Entry = (String, String, String, String);
        let entries: Vec<Entry> = {
            let mut stmt = tx
                .prepare("SELECT tenant,user_id,route,ciphertext FROM oauth_vault_secrets")
                .map_err(|_| VaultBrokerError::Storage)?;
            let rows = stmt
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
                .map_err(|_| VaultBrokerError::Storage)?;
            rows.collect::<Result<Vec<Entry>, _>>()
                .map_err(|_| VaultBrokerError::Storage)?
        };
        let mut count = 0;
        for (tenant, user, route, ciphertext) in entries {
            let next = self.kms.rewrap(&tenant, &user, &route, &ciphertext)?;
            tx.execute(
                "UPDATE oauth_vault_secrets SET ciphertext=?1
                WHERE tenant=?2 AND user_id=?3 AND route=?4",
                params![next, tenant, user, route],
            )
            .map_err(|_| VaultBrokerError::Storage)?;
            count += 1;
        }
        tx.commit().map_err(|_| VaultBrokerError::Storage)?;
        Ok(count)
    }

    pub fn revoke(&self, tenant: &str, user: &str, route: &str) -> Result<(), VaultBrokerError> {
        if !valid_id(tenant) || !valid_id(user) || !self.routes.contains_key(route) {
            return Err(VaultBrokerError::Denied);
        }
        self.connection()?
            .execute(
                "DELETE FROM oauth_vault_secrets WHERE tenant=?1 AND user_id=?2 AND route=?3",
                params![tenant, user, route],
            )
            .map_err(|_| VaultBrokerError::Storage)?;
        Ok(())
    }
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value != "."
        && value != ".."
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-.".contains(&b))
}
