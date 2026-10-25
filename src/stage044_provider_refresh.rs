//! Managed OAuth refresh for operator-pinned external providers.
//! Conservative crash/replay policy: any ambiguous refresh outcome requires
//! interactive re-authorization. Never replay a possibly rotated credential.
use crate::stage044_oauth::private_database;
use crate::stage044_secret_broker::{https_post_form_basic, BrokerRoute};
use crate::stage044_vault::VaultTransit;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use rusqlite::{params, Connection, OpenFlags, OptionalExtension, TransactionBehavior};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProviderRefreshError {
    #[error("invalid external provider configuration")]
    Configuration,
    #[error("external provider requires interactive reauthorization")]
    ReauthorizationRequired,
    #[error("refresh credentials unavailable")]
    Missing,
    #[error("external provider or KMS rejected refresh")]
    Unavailable,
    #[error("durable credential state unavailable")]
    Storage,
}

#[derive(Clone)]
pub struct ProviderConfig {
    pub tenant: String,
    pub user: String,
    pub provider: String,
    pub client_id: String,
    pub client_secret_file: PathBuf,
    pub token_route: BrokerRoute,
    pub required_scopes: BTreeSet<String>,
}

pub struct ProviderAccess {
    pub access_token: String,
    pub expires_in: i64,
}
impl std::fmt::Debug for ProviderAccess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderAccess")
            .field("access_token", &"[REDACTED]")
            .field("expires_in", &self.expires_in).finish()
    }
}

pub struct ManagedProviderRefresh {
    db_path: PathBuf,
    kms: VaultTransit,
    config: ProviderConfig,
}

impl ManagedProviderRefresh {
    pub fn open(
        db_path: &Path, kms: VaultTransit, config: ProviderConfig,
    ) -> Result<Self, ProviderRefreshError> {
        if !private_database(db_path)
            || !safe_id(&config.tenant) || !safe_id(&config.user)
            || !safe_id(&config.provider) || !safe_id(&config.client_id)
            || config.token_route.route_id != config.provider
            || config.token_route.validate().is_err()
            || config.required_scopes.is_empty()
            || config.required_scopes.len() > 64
            || !config.client_secret_file.is_absolute()
        { return Err(ProviderRefreshError::Configuration); }
        let secret = crate::ops::secrets::read_private_text(&config.client_secret_file,4096)
            .map_err(|_| ProviderRefreshError::Configuration)?;
        if !valid_token(secret.trim_end_matches(['\r','\n'])) {
            return Err(ProviderRefreshError::Configuration);
        }
        let service = Self {db_path:db_path.to_owned(),kms,config};
        service.connection()?.execute_batch(
            "CREATE TABLE IF NOT EXISTS oauth_provider_refresh (
                tenant TEXT NOT NULL, user_id TEXT NOT NULL, provider TEXT NOT NULL,
                refresh_ciphertext TEXT NOT NULL, revision INTEGER NOT NULL DEFAULT 0,
                state TEXT NOT NULL CHECK(state IN ('active','refreshing','reauth_required')),
                PRIMARY KEY(tenant,user_id,provider)
            );"
        ).map_err(|_| ProviderRefreshError::Storage)?;
        Ok(service)
    }

    fn connection(&self) -> Result<Connection, ProviderRefreshError> {
        let conn = Connection::open_with_flags(&self.db_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NOFOLLOW)
            .map_err(|_| ProviderRefreshError::Storage)?;
        conn.busy_timeout(Duration::from_secs(5)).map_err(|_| ProviderRefreshError::Storage)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")
            .map_err(|_| ProviderRefreshError::Storage)?;
        Ok(conn)
    }

    /// Operator-only initial grant storage. Existing grants cannot be replaced
    /// without explicit revoke/re-consent; no LLM-callable provisioning route.
    pub fn provision(&self, refresh_token: &[u8]) -> Result<(), ProviderRefreshError> {
        if !valid_token_bytes(refresh_token) {
            return Err(ProviderRefreshError::Configuration);
        }
        let c = &self.config;
        let ciphertext = self.kms.encrypt(&c.tenant,&c.user,&c.provider,refresh_token)
            .map_err(|_| ProviderRefreshError::Unavailable)?;
        self.connection()?.execute(
            "INSERT INTO oauth_provider_refresh
             (tenant,user_id,provider,refresh_ciphertext,state)
             VALUES(?1,?2,?3,?4,'active')",
            params![c.tenant,c.user,c.provider,ciphertext]
        ).map_err(|_| ProviderRefreshError::Storage)?;
        Ok(())
    }

    pub fn refresh_once(&self) -> Result<ProviderAccess, ProviderRefreshError> {
        let (ciphertext,revision) = self.begin_refresh()?;
        let outcome = self.exchange(&ciphertext);
        match outcome {
            Ok((access,next)) => {
                let c = &self.config;
                let encrypted = self.kms.encrypt(&c.tenant,&c.user,&c.provider,next.as_bytes())
                    .map_err(|_| ProviderRefreshError::Unavailable);
                match encrypted {
                    Ok(value) => {
                        let changed = self.connection()?.execute(
                            "UPDATE oauth_provider_refresh SET refresh_ciphertext=?1,
                             revision=revision+1,state='active'
                             WHERE tenant=?2 AND user_id=?3 AND provider=?4
                               AND state='refreshing' AND revision=?5",
                            params![value,c.tenant,c.user,c.provider,revision]
                        ).map_err(|_| ProviderRefreshError::Storage)?;
                        if changed == 1 { return Ok(access); }
                    }
                    Err(_) => {}
                }
                self.require_reauthorization();
                Err(ProviderRefreshError::ReauthorizationRequired)
            }
            Err(_) => {
                self.require_reauthorization();
                Err(ProviderRefreshError::ReauthorizationRequired)
            }
        }
    }

    fn begin_refresh(&self) -> Result<(String,i64), ProviderRefreshError> {
        let c = &self.config;
        let mut conn = self.connection()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| ProviderRefreshError::Storage)?;
        let row: Option<(String,i64,String)> = tx.query_row(
            "SELECT refresh_ciphertext,revision,state FROM oauth_provider_refresh
             WHERE tenant=?1 AND user_id=?2 AND provider=?3",
            params![c.tenant,c.user,c.provider],
            |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))
        ).optional().map_err(|_| ProviderRefreshError::Storage)?;
        let (ciphertext,revision,state) = row.ok_or(ProviderRefreshError::Missing)?;
        if state != "active" { return Err(ProviderRefreshError::ReauthorizationRequired); }
        let changed = tx.execute(
            "UPDATE oauth_provider_refresh SET state='refreshing'
             WHERE tenant=?1 AND user_id=?2 AND provider=?3
               AND state='active' AND revision=?4",
            params![c.tenant,c.user,c.provider,revision]
        ).map_err(|_| ProviderRefreshError::Storage)?;
        if changed != 1 { return Err(ProviderRefreshError::ReauthorizationRequired); }
        tx.commit().map_err(|_| ProviderRefreshError::Storage)?;
        Ok((ciphertext,revision))
    }

    fn exchange(&self, ciphertext: &str) -> Result<(ProviderAccess,String), ProviderRefreshError> {
        let c = &self.config;
        let mut refresh = self.kms.decrypt(&c.tenant,&c.user,&c.provider,ciphertext)
            .map_err(|_| ProviderRefreshError::Unavailable)?;
        if !valid_token_bytes(&refresh) {
            refresh.fill(0); return Err(ProviderRefreshError::Unavailable);
        }
        let mut secret = crate::ops::secrets::read_private_text(&c.client_secret_file,4096)
            .map_err(|_| ProviderRefreshError::Unavailable)?;
        let clean = secret.trim_end_matches(['\r','\n']);
        if !valid_token(clean) {
            refresh.fill(0);return Err(ProviderRefreshError::Unavailable);
        }
        let old_digest = Sha256::digest(&refresh);
        let mut basic = STANDARD.encode(format!("{}:{clean}",c.client_id)).into_bytes();
        secret.clear();
        let form = format!("grant_type=refresh_token&refresh_token={}",percent_encode(&refresh));
        let result = https_post_form_basic(&c.token_route,&basic,form.as_bytes());
        basic.fill(0);
        refresh.fill(0);
        let result = result.map_err(|_| ProviderRefreshError::Unavailable)?;
        let body: Value = serde_json::from_slice(&result.body)
            .map_err(|_| ProviderRefreshError::Unavailable)?;
        if body.get("token_type").and_then(Value::as_str) != Some("Bearer") {
            return Err(ProviderRefreshError::Unavailable);
        }
        let access = body.get("access_token").and_then(Value::as_str)
            .ok_or(ProviderRefreshError::Unavailable)?;
        let next = body.get("refresh_token").and_then(Value::as_str)
            .ok_or(ProviderRefreshError::Unavailable)?;
        let ttl = body.get("expires_in").and_then(Value::as_i64)
            .ok_or(ProviderRefreshError::Unavailable)?;
        if !valid_token(access) || !valid_token(next)
            || Sha256::digest(next.as_bytes()).as_slice() == old_digest.as_slice()
            || !(1..=86400).contains(&ttl) {
            return Err(ProviderRefreshError::Unavailable);
        }
        if let Some(scopes) = body.get("scope").and_then(Value::as_str) {
            let returned: BTreeSet<&str> = scopes.split_ascii_whitespace().collect();
            if !c.required_scopes.iter().all(|v| returned.contains(v.as_str())) {
                return Err(ProviderRefreshError::Unavailable);
            }
        }
        Ok((ProviderAccess {access_token:access.to_owned(),expires_in:ttl},next.to_owned()))
    }

    /// Fail closed on crash/timeout/network ambiguity. Never retry a possibly
    /// consumed refresh token without explicit user re-consent.
    fn require_reauthorization(&self) {
        let c = &self.config;
        if let Ok(conn) = self.connection() {
            let _ = conn.execute(
                "UPDATE oauth_provider_refresh SET state='reauth_required'
                 WHERE tenant=?1 AND user_id=?2 AND provider=?3 AND state='refreshing'",
                params![c.tenant,c.user,c.provider]
            );
        }
    }

    pub fn revoke(&self) -> Result<(), ProviderRefreshError> {
        let c = &self.config;
        self.connection()?.execute(
            "DELETE FROM oauth_provider_refresh
             WHERE tenant=?1 AND user_id=?2 AND provider=?3",
            params![c.tenant,c.user,c.provider]
        ).map_err(|_| ProviderRefreshError::Storage)?;
        Ok(())
    }
}

fn safe_id(v: &str) -> bool {
    !v.is_empty() && v.len() <= 128 && v != "." && v != ".."
        && v.bytes().all(|b| b.is_ascii_alphanumeric() || b"_-.".contains(&b))
}
fn valid_token(v: &str) -> bool {
    !v.is_empty() && v.len() <= 8192 && v.bytes().all(|b| (33..=126).contains(&b))
}
fn valid_token_bytes(v: &[u8]) -> bool {
    !v.is_empty() && v.len() <= 8192 && v.iter().all(|b| (33..=126).contains(b))
}
fn percent_encode(bytes: &[u8]) -> String {
    let mut out=String::with_capacity(bytes.len()*3);
    for b in bytes {
        if b.is_ascii_alphanumeric() || b"-._~".contains(b) {
            out.push(char::from(*b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn form_encoder_never_injects_delimiters() {
        assert_eq!(percent_encode(b"a+b&c=d"),"a%2Bb%26c%3Dd");
        assert!(valid_token_bytes(b"abc-123"));
        assert!(!valid_token_bytes(b"abc\r\nInjected: yes"));
        assert!(!safe_id("../metadata"));
    }
}
