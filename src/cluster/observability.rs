//! Operator-only read-only PostgreSQL health snapshots.
//!
//! This module never exposes a network listener. Access is controlled by
//! PostgreSQL credentials and its existing TLS policy. Snapshots contain
//! only aggregate counts, never job contents, identities or lease tokens.
//! PostgreSQL is the monitoring source of truth, not a proof of end-to-end
//! reachability of every Zenoh participant.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;
use tokio_postgres::Client;

use super::leadership::{cluster_key, open_database, PgLeaderError};

/// Coarse health state for operator automation and alert routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClusterHealth {
    Healthy,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClusterAlertCode {
    NoLeader,
    MultipleLeaders,
    NoLiveAgents,
    ExpiredAgents,
    PendingBacklog,
    UncertainJobs,
    ExpiredRunningJobs,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterAlert {
    pub code: ClusterAlertCode,
    pub severity: ClusterHealth,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterSnapshot {
    pub observed_at_ms: u64,
    pub leader_sessions: u64,
    pub agents_live: u64,
    pub agents_expired: u64,
    pub jobs_pending: u64,
    pub jobs_running: u64,
    pub jobs_succeeded: u64,
    pub jobs_failed: u64,
    pub jobs_uncertain: u64,
    pub jobs_running_expired: u64,
}

/// Owner-restricted operator view of one registered node; no tool ACLs,
/// signing credentials, addresses or job contents are included.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeHealth {
    pub node_id: String,
    pub observed_at_ms: u64,
    pub expires_at_ms: u64,
    pub alive: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterReport {
    pub health: ClusterHealth,
    pub snapshot: ClusterSnapshot,
    pub alerts: Vec<ClusterAlert>,
}

/// Thresholds are inclusive. A value of zero is prohibited to avoid
/// accidental alert storms and misleading always-true comparisons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlertPolicy {
    pub pending_threshold: u64,
    pub uncertain_threshold: u64,
    pub expired_agents_threshold: u64,
}

impl Default for AlertPolicy {
    fn default() -> Self {
        Self {
            pending_threshold: 100,
            uncertain_threshold: 1,
            expired_agents_threshold: 1,
        }
    }
}

impl AlertPolicy {
    pub fn new(
        pending_threshold: u64,
        uncertain_threshold: u64,
        expired_agents_threshold: u64,
    ) -> Result<Self, PgLeaderError> {
        if [
            pending_threshold,
            uncertain_threshold,
            expired_agents_threshold,
        ]
        .iter()
        .any(|v| !(1..=1_000_000).contains(v))
        {
            return Err(PgLeaderError::InvalidCluster);
        }
        Ok(Self {
            pending_threshold,
            uncertain_threshold,
            expired_agents_threshold,
        })
    }

    pub fn evaluate(&self, snapshot: ClusterSnapshot) -> ClusterReport {
        let mut alerts = Vec::new();
        if snapshot.leader_sessions == 0 {
            alerts.push(ClusterAlert {
                code: ClusterAlertCode::NoLeader,
                severity: ClusterHealth::Critical,
            });
        } else if snapshot.leader_sessions > 1 {
            alerts.push(ClusterAlert {
                code: ClusterAlertCode::MultipleLeaders,
                severity: ClusterHealth::Critical,
            });
        }
        if snapshot.jobs_running_expired > 0 {
            alerts.push(ClusterAlert {
                code: ClusterAlertCode::ExpiredRunningJobs,
                severity: ClusterHealth::Critical,
            });
        }
        if snapshot.jobs_uncertain >= self.uncertain_threshold {
            alerts.push(ClusterAlert {
                code: ClusterAlertCode::UncertainJobs,
                severity: ClusterHealth::Critical,
            });
        }
        if snapshot.jobs_pending >= self.pending_threshold {
            alerts.push(ClusterAlert {
                code: ClusterAlertCode::PendingBacklog,
                severity: ClusterHealth::Warning,
            });
        }
        if snapshot.agents_live == 0 {
            alerts.push(ClusterAlert {
                code: ClusterAlertCode::NoLiveAgents,
                severity: ClusterHealth::Warning,
            });
        }
        if snapshot.agents_expired >= self.expired_agents_threshold {
            alerts.push(ClusterAlert {
                code: ClusterAlertCode::ExpiredAgents,
                severity: ClusterHealth::Warning,
            });
        }
        let health = if alerts
            .iter()
            .any(|alert| alert.severity == ClusterHealth::Critical)
        {
            ClusterHealth::Critical
        } else if alerts.is_empty() {
            ClusterHealth::Healthy
        } else {
            ClusterHealth::Warning
        };
        ClusterReport {
            health,
            snapshot,
            alerts,
        }
    }
}

/// Privileged, read-only, dedicated PostgreSQL session. Monitoring is not
/// a leader candidate and never obtains or releases an advisory lock.
pub struct PgClusterMonitor {
    client: Client,
    connection: JoinHandle<()>,
    leader_key: i64,
    cluster_id: String,
}

impl PgClusterMonitor {
    pub async fn connect(
        dsn: &str,
        cluster_id: &str,
        allow_insecure_loopback: bool,
    ) -> Result<Self, PgLeaderError> {
        let leader_key = cluster_key(cluster_id)?;
        let (client, connection) = open_database(dsn, allow_insecure_loopback).await?;
        Ok(Self {
            client,
            connection,
            leader_key,
            cluster_id: cluster_id.to_owned(),
        })
    }

    /// One read-only PostgreSQL query obtains all counters using a database
    /// clock. A missing table or dead database makes the command fail closed.
    /// Advisory lock discovery via pg_locks is a snapshot, not an external
    /// fencing guarantee; no raw client-supplied time can skew lease counts.
    pub async fn snapshot(&self) -> Result<ClusterSnapshot, PgLeaderError> {
        let row = tokio::time::timeout(
            Duration::from_secs(5),
            self.client.query_one(
                "WITH clock AS (
                    SELECT (EXTRACT(EPOCH FROM clock_timestamp())*1000)::BIGINT AS now_ms
                 )
                 SELECT
                    clock.now_ms,
                    (SELECT count(*)::BIGINT FROM pg_locks
                     WHERE locktype='advisory' AND granted
                       AND mode='ExclusiveLock' AND objsubid=1
                       AND classid=(($1::BIGINT >> 32) & 4294967295)::OID
                       AND objid=($1::BIGINT & 4294967295)::OID),
                    (SELECT count(*)::BIGINT FROM ocaf_pg_nodes_v1
                     WHERE expires_at_ms>clock.now_ms),
                    (SELECT count(*)::BIGINT FROM ocaf_pg_nodes_v1
                     WHERE expires_at_ms<=clock.now_ms),
                    (SELECT count(*)::BIGINT FROM ocaf_pg_jobs_v1 WHERE cluster_id=$2 AND status='pending'),
                    (SELECT count(*)::BIGINT FROM ocaf_pg_jobs_v1 WHERE cluster_id=$2 AND status='running'),
                    (SELECT count(*)::BIGINT FROM ocaf_pg_jobs_v1 WHERE cluster_id=$2 AND status='succeeded'),
                    (SELECT count(*)::BIGINT FROM ocaf_pg_jobs_v1 WHERE cluster_id=$2 AND status='failed'),
                    (SELECT count(*)::BIGINT FROM ocaf_pg_jobs_v1 WHERE cluster_id=$2 AND status='uncertain'),
                    (SELECT count(*)::BIGINT FROM ocaf_pg_jobs_v1
                     WHERE cluster_id=$2 AND status='running' AND lease_expires_at_ms<=clock.now_ms)
                 FROM clock",
                &[&self.leader_key, &self.cluster_id],
            ),
        )
        .await
        .map_err(|_| PgLeaderError::Database)?
        .map_err(|_| PgLeaderError::Database)?;

        fn read_count(row: &tokio_postgres::Row, index: usize) -> Result<u64, PgLeaderError> {
            u64::try_from(row.get::<usize, i64>(index)).map_err(|_| PgLeaderError::Database)
        }
        Ok(ClusterSnapshot {
            observed_at_ms: read_count(&row, 0)?,
            leader_sessions: read_count(&row, 1)?,
            agents_live: read_count(&row, 2)?,
            agents_expired: read_count(&row, 3)?,
            jobs_pending: read_count(&row, 4)?,
            jobs_running: read_count(&row, 5)?,
            jobs_succeeded: read_count(&row, 6)?,
            jobs_failed: read_count(&row, 7)?,
            jobs_uncertain: read_count(&row, 8)?,
            jobs_running_expired: read_count(&row, 9)?,
        })
    }
    /// Operator report, using only DB-owned clock and per-cluster job counts.
    /// Missing leadership is an unhealthy result, not an empty-success.
    pub async fn report(&self, policy: AlertPolicy) -> Result<ClusterReport, PgLeaderError> {
        Ok(policy.evaluate(self.snapshot().await?))
    }

    /// Resolve a single node's signed-heartbeat expiry from the shared
    /// directory. Uses the database clock; no user-controlled time input.
    pub async fn node_health(&self, node_id: &str) -> Result<Option<NodeHealth>, PgLeaderError> {
        crate::rpc::validate_node_id(node_id).map_err(|_| PgLeaderError::InvalidCluster)?;
        let row = tokio::time::timeout(
            Duration::from_secs(5),
            self.client.query_opt(
                "SELECT node_id,expires_at_ms,
                    (EXTRACT(EPOCH FROM clock_timestamp())*1000)::BIGINT
                 FROM ocaf_pg_nodes_v1 WHERE node_id=$1",
                &[&node_id],
            ),
        )
        .await
        .map_err(|_| PgLeaderError::Database)?
        .map_err(|_| PgLeaderError::Database)?;
        row.map(|row| {
            let expires: i64 = row.get(1);
            let observed: i64 = row.get(2);
            Ok(NodeHealth {
                node_id: row.get(0),
                observed_at_ms: u64::try_from(observed).map_err(|_| PgLeaderError::Database)?,
                expires_at_ms: u64::try_from(expires).map_err(|_| PgLeaderError::Database)?,
                alive: expires > observed,
            })
        })
        .transpose()
    }
}

impl Drop for PgClusterMonitor {
    fn drop(&mut self) {
        self.connection.abort();
    }
}
