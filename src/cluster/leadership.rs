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
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::time::Duration;

use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::sync::Mutex;
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
    cluster_id: String,
    held: AtomicBool,
    /// Zero before election, positive while active, -1 after an ambiguous
    /// transition. A poisoned session must be dropped, never reused.
    epoch: AtomicI64,
    transition: Mutex<()>,
}

fn loopback_hosts(config: &Config) -> bool {
    if config.get_hosts().is_empty()
        || config
            .get_hostaddrs()
            .iter()
            .any(|addr| !addr.is_loopback())
    {
        return false;
    }
    config.get_hosts().iter().all(|host| match host {
        Host::Tcp(name) => {
            name == "localhost" || name.parse::<IpAddr>().is_ok_and(|addr| addr.is_loopback())
        }
        #[cfg(unix)]
        Host::Unix(_) => true,
    })
}

/// Validate identity and derive the same advisory key on every host.
pub(crate) fn cluster_key(cluster: &str) -> Result<i64, PgLeaderError> {
    if cluster.is_empty()
        || cluster.len() > 64
        || !cluster
            .bytes()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == b'-' || ch == b'_')
    {
        return Err(PgLeaderError::InvalidCluster);
    }
    let hash = Sha256::digest(format!("ocaf.cluster.leader.v1:{cluster}").as_bytes());
    Ok(i64::from_be_bytes(
        hash[..8].try_into().expect("digest length >= 8"),
    ))
}

pub(crate) async fn open_database(
    dsn: &str,
    allow_insecure_loopback: bool,
) -> Result<(Client, JoinHandle<()>), PgLeaderError> {
    let config = Config::from_str(dsn).map_err(|_| PgLeaderError::UnsafeTransport)?;
    let local = config.get_ssl_mode() == SslMode::Disable
        && allow_insecure_loopback
        && loopback_hosts(&config);
    let secure = config.get_ssl_mode() == SslMode::Require;
    if !local && !secure {
        return Err(PgLeaderError::UnsafeTransport);
    }

    let result = if local {
        let (client, connection) =
            tokio::time::timeout(Duration::from_secs(5), config.connect(NoTls))
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
        let (client, connection) =
            tokio::time::timeout(Duration::from_secs(5), config.connect(connector))
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
    Ok(result)
}

impl PgLeader {
    pub async fn connect(
        dsn: &str,
        cluster: &str,
        allow_insecure_loopback: bool,
    ) -> Result<Self, PgLeaderError> {
        let key = cluster_key(cluster)?;
        let (client, task) = open_database(dsn, allow_insecure_loopback).await?;
        // Epochs are durable on the same PostgreSQL primary as the coordinator
        // security state. Schema provisioning is a separate operator duty in
        // least-privileged production deployments.
        if !matches!(
            tokio::time::timeout(
                Duration::from_secs(5),
                client.batch_execute(
                    "BEGIN;
                     SELECT pg_advisory_xact_lock(574123456);
                     CREATE TABLE IF NOT EXISTS ocaf_pg_leader_epochs_v1 (
                         cluster_id TEXT PRIMARY KEY,
                         epoch BIGINT NOT NULL CHECK (epoch > 0)
                     );
                     COMMIT;",
                ),
            )
            .await,
            Ok(Ok(()))
        ) {
            task.abort();
            return Err(PgLeaderError::Database);
        }

        Ok(Self {
            client,
            connection: task,
            key,
            cluster_id: cluster.to_owned(),
            held: AtomicBool::new(false),
            epoch: AtomicI64::new(0),
            transition: Mutex::new(()),
        })
    }

    /// Acquire exactly once per session, even when called concurrently.
    /// PostgreSQL session advisory locks are re-entrant; without a mutex
    /// parallel callers could stack acquisitions and block future leaders
    /// even after a single release.
    pub async fn try_acquire(&self) -> Result<bool, PgLeaderError> {
        let _transition = self.transition.lock().await;
        if self.epoch.load(Ordering::SeqCst) < 0 {
            return Err(PgLeaderError::Database);
        }
        if self.held.load(Ordering::SeqCst) {
            self.verify_locked().await?;
            return Ok(true);
        }
        if self.connection.is_finished() {
            return Err(PgLeaderError::Database);
        }
        let row = tokio::time::timeout(
            Duration::from_secs(5),
            self.client
                .query_one("SELECT pg_try_advisory_lock($1)", &[&self.key]),
        )
        .await
        .map_err(|_| PgLeaderError::Database)?
        .map_err(|_| PgLeaderError::Database)?;
        if !row.get::<_, bool>(0) {
            return Ok(false);
        }

        // Every successful election increments a durable fencing epoch
        // while this PostgreSQL session still holds the exclusive lock.
        // An epoch-write failure poisons and closes the session; it cannot
        // start serving with a leader lock but without a durable epoch.
        let next = tokio::time::timeout(
            Duration::from_secs(5),
            self.client.query_one(
                "INSERT INTO ocaf_pg_leader_epochs_v1(cluster_id,epoch)
                 VALUES($1,1)
                 ON CONFLICT(cluster_id) DO UPDATE SET
                   epoch=ocaf_pg_leader_epochs_v1.epoch+1
                 RETURNING epoch",
                &[&self.cluster_id],
            ),
        )
        .await;
        let next_epoch = match next {
            Ok(Ok(row)) => row.get::<_, i64>(0),
            _ => {
                self.epoch.store(-1, Ordering::SeqCst);
                self.connection.abort();
                return Err(PgLeaderError::Database);
            }
        };
        if next_epoch <= 0 {
            self.epoch.store(-1, Ordering::SeqCst);
            self.connection.abort();
            return Err(PgLeaderError::Database);
        }
        self.epoch.store(next_epoch, Ordering::SeqCst);
        self.held.store(true, Ordering::SeqCst);
        Ok(true)
    }

    /// Return the durable leader generation after checking the actual lock.
    /// This is a token for future cooperating downstream fencing; obtaining
    /// it does NOT itself fence arbitrary external side effects.
    pub async fn fencing_epoch(&self) -> Result<i64, PgLeaderError> {
        self.verify().await?;
        let epoch = self.epoch.load(Ordering::SeqCst);
        if epoch <= 0 {
            return Err(PgLeaderError::Database);
        }
        Ok(epoch)
    }

    /// Only the dedicated PostgreSQL backend holding the *exact advisory
    /// lock* and the latest durable epoch is allowed to serve. Merely
    /// successfully running SELECT 1 does not prove lock ownership.
    async fn verify_locked(&self) -> Result<(), PgLeaderError> {
        let epoch = self.epoch.load(Ordering::SeqCst);
        if epoch <= 0 || !self.held.load(Ordering::SeqCst) || self.connection.is_finished() {
            return Err(PgLeaderError::Database);
        }
        let row = tokio::time::timeout(
            Duration::from_secs(5),
            self.client.query_one(
                "SELECT COALESCE(
                    (SELECT epoch FROM ocaf_pg_leader_epochs_v1 WHERE cluster_id=$1)=$2,
                    FALSE
                ) AND EXISTS (
                    SELECT 1 FROM pg_locks
                    WHERE locktype='advisory' AND pid=pg_backend_pid()
                      AND granted AND mode='ExclusiveLock' AND objsubid=1
                      AND ((classid::bigint << 32) | objid::bigint)=$3
                )",
                &[&self.cluster_id, &epoch, &self.key],
            ),
        )
        .await
        .map_err(|_| PgLeaderError::Database)?
        .map_err(|_| PgLeaderError::Database)?;
        if !row.get::<_, bool>(0)
            || !self.held.load(Ordering::SeqCst)
            || self.epoch.load(Ordering::SeqCst) != epoch
        {
            return Err(PgLeaderError::Database);
        }
        Ok(())
    }

    /// Report the PostgreSQL server session PID for diagnostics and
    /// controlled failure-injection tests. This value is not a fencing
    /// token and must never authorize a remote workload.
    pub async fn backend_pid(&self) -> Result<i32, PgLeaderError> {
        let row = tokio::time::timeout(
            Duration::from_secs(5),
            self.client.query_one("SELECT pg_backend_pid()", &[]),
        )
        .await
        .map_err(|_| PgLeaderError::Database)?
        .map_err(|_| PgLeaderError::Database)?;
        Ok(row.get(0))
    }

    pub async fn verify(&self) -> Result<(), PgLeaderError> {
        let _transition = self.transition.lock().await;
        self.verify_locked().await
    }

    pub async fn release(&self) -> Result<(), PgLeaderError> {
        let _transition = self.transition.lock().await;
        if !self.held.swap(false, Ordering::SeqCst) {
            return if self.epoch.load(Ordering::SeqCst) < 0 {
                Err(PgLeaderError::Database)
            } else {
                Ok(())
            };
        }
        // Refuse request verification while the unlock is in flight.
        // If acknowledgement is lost, the session becomes permanently
        // poisoned and is closed rather than risking re-entrant locking.
        self.epoch.store(-1, Ordering::SeqCst);
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            self.client
                .query_one("SELECT pg_advisory_unlock($1)", &[&self.key]),
        )
        .await;
        match result {
            Ok(Ok(row)) if row.get::<_, bool>(0) => {
                self.epoch.store(0, Ordering::SeqCst);
                Ok(())
            }
            _ => {
                self.connection.abort();
                Err(PgLeaderError::Database)
            }
        }
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
