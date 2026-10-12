//! PostgreSQL session advisory lock for **cross-host leader exclusion**.
//!
//! A leader is authoritative only while its dedicated PostgreSQL session is
//! alive. A standby can acquire the same advisory key after connection loss.
//! No SQLite state is magically replicated by this module: full cluster
//! failover requires replacing every coordinator journal with shared
//! transactional storage. Never combine this leader with per-host SQLite
//! databases and claim production high availability.
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::task::JoinHandle;
use tokio_postgres::config::{Host, SslMode};
use tokio_postgres::{Client, Config, NoTls};

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PgLeaderError {
    #[error("invalid cluster identifier")]
    InvalidCluster,
    #[error("PostgreSQL transport must use verified TLS or explicitly permitted loopback")]
    UnsafeTransport,
    #[error("PostgreSQL leadership database unavailable")]
    Database,
}

/// A dedicated connection: session advisory locks cannot be safely used
/// with transaction-pooling proxies or a connection pool that can swap
/// the backend session.
pub struct PgLeader {
    client: Client,
    connection: JoinHandle<()>,
    key: i64,
    held: AtomicBool,
}

fn loopback_hosts(config: &Config) -> bool {
    if config.get_hosts().is_empty() || config.get_hostaddrs().iter().any(|addr| !addr.is_loopback()) {
        return false;
    }
    config.get_hosts().iter().all(|host| {
        match host {
            Host::Tcp(name) => name == "localhost" || name.parse::<IpAddr>().is_ok_and(|addr| addr.is_loopback()),
            #[cfg(unix)]
            Host::Unix(_) => true,
        }
    })
}

fn lock_key(cluster: &str) -> i64 {
    // Fixed namespace, deterministic across hosts/architectures.
    let hash = Sha256::digest(format!("ocaf.cluster.leader.v1:{cluster}").as_bytes());
    i64::from_be_bytes(hash[..8].try_into().expect("digest length >= 8"))
}

impl PgLeader {
    pub async fn connect(
        dsn: &str,
        cluster: &str,
        allow_insecure_loopback: bool,
    ) -> Result<Self, PgLeaderError> {
        if cluster.is_empty()
            || cluster.len() > 64
            || !cluster.bytes().all(|ch| ch.is_ascii_alphanumeric() || ch == b'-' || ch == b'_')
        {
            return Err(PgLeaderError::InvalidCluster);
        }
        let config = Config::from_str(dsn).map_err(|_| PgLeaderError::UnsafeTransport)?;
        let local = config.get_ssl_mode() == SslMode::Disable
            && allow_insecure_loopback
            && loopback_hosts(&config);
        let secure = config.get_ssl_mode() == SslMode::Require;
        if !local && !secure {
            return Err(PgLeaderError::UnsafeTransport);
        }

        let (client, task) = if local {
            let (client, connection) = tokio::time::timeout(
                Duration::from_secs(5),
                config.connect(NoTls),
            )
            .await
            .map_err(|_| PgLeaderError::Database)?
            .map_err(|_| PgLeaderError::Database)?;
            let task = tokio::spawn(async move {
                if let Err(err) = connection.await {
                    tracing::warn!(error = %err, "PostgreSQL leader connection lost");
                }
            });
            (client, task)
        } else {
            // Native TLS verifies the server's chain and DNS name by default.
            // PostgreSQL sslmode=require prevents fallback to plaintext.
            let connector = native_tls::TlsConnector::builder()
                .build()
                .map_err(|_| PgLeaderError::UnsafeTransport)?;
            let connector = postgres_native_tls::MakeTlsConnector::new(connector);
            let (client, connection) = tokio::time::timeout(
                Duration::from_secs(5),
                config.connect(connector),
            )
            .await
            .map_err(|_| PgLeaderError::Database)?
            .map_err(|_| PgLeaderError::Database)?;
            let task = tokio::spawn(async move {
                if let Err(err) = connection.await {
                    tracing::warn!(error = %err, "PostgreSQL leader connection lost");
                }
            });
            (client, task)
        };

        Ok(Self {
            client,
            connection: task,
            key: lock_key(cluster),
            held: AtomicBool::new(false),
        })
    }

    /// Claims exclusive leadership from the PostgreSQL *backend session*.
    /// Independent coordinator hosts compete for the same key.
    pub async fn try_acquire(&self) -> Result<bool, PgLeaderError> {
        if self.held.load(Ordering::SeqCst) {
            self.verify().await?;
            return Ok(true);
        }
        let row = tokio::time::timeout(
            Duration::from_secs(5),
            self.client.query_one("SELECT pg_try_advisory_lock($1)", &[&self.key]),
        )
        .await
        .map_err(|_| PgLeaderError::Database)?
        .map_err(|_| PgLeaderError::Database)?;
        let acquired: bool = row.get(0);
        if acquired {
            self.held.store(true, Ordering::SeqCst);
        }
        Ok(acquired)
    }

    /// A leader must keep proving the original session is alive. If this
    /// returns an error, immediately cease serving: the advisory lock may
    /// already have been handed to a standby.
    pub async fn verify(&self) -> Result<(), PgLeaderError> {
        if !self.held.load(Ordering::SeqCst) || self.connection.is_finished() {
            return Err(PgLeaderError::Database);
        }
        tokio::time::timeout(
            Duration::from_secs(5),
            self.client.simple_query("SELECT 1"),
        )
        .await
        .map_err(|_| PgLeaderError::Database)?
        .map_err(|_| PgLeaderError::Database)?;
        Ok(())
    }

    pub async fn release(&self) -> Result<(), PgLeaderError> {
        if !self.held.swap(false, Ordering::SeqCst) {
            return Ok(());
        }
        let row = tokio::time::timeout(
            Duration::from_secs(5),
            self.client.query_one("SELECT pg_advisory_unlock($1)", &[&self.key]),
        )
        .await
        .map_err(|_| PgLeaderError::Database)?
        .map_err(|_| PgLeaderError::Database)?;
        if !row.get::<_, bool>(0) {
            return Err(PgLeaderError::Database);
        }
        Ok(())
    }

    pub async fn acquire_standby(&self, interval: Duration) -> Result<(), PgLeaderError> {
        if interval.is_zero() || interval > Duration::from_secs(5) {
            return Err(PgLeaderError::InvalidCluster);
        }
        loop {
            if self.try_acquire().await? {
                return Ok(());
            }
            tokio::time::sleep(interval).await;
        }
    }
}

impl Drop for PgLeader {
    fn drop(&mut self) {
        // Dropping the connection task closes the socket/session and frees
        // PostgreSQL advisory locks even after an abrupt process exit.
        self.connection.abort();
    }
}
