//! Authenticated capability coordinator for a trusted network.
//! HMAC provides integrity, NOT confidentiality. An operator must configure
//! Zenoh TLS/mTLS before using this on non-loopback addresses.
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use ocaf_runtime::cluster::leadership::PgLeader;
use ocaf_runtime::coordinator::leadership::LeaderGuard;
use ocaf_runtime::rpc::{PeerPolicy, RpcGateway};
use ocaf_runtime::runtime::OcafRuntime;
use ocaf_runtime::transport::rpc::ZenohRpcServer;
use ocaf_runtime::transport_policy::{read_zenoh_config, validate_zenoh_config};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PeerConfig {
    key_b64: String,
    allowed_tools: Vec<String>,
    #[serde(default)]
    submit_jobs: bool,
    #[serde(default)]
    worker_jobs: bool,
}

fn env_required(name: &str) -> anyhow::Result<String> {
    let value =
        env::var(name).map_err(|_| anyhow::anyhow!("missing required configuration: {name}"))?;
    if value.is_empty() {
        anyhow::bail!("empty configuration: {name}");
    }
    Ok(value)
}

fn absolute_path(name: &str) -> anyhow::Result<PathBuf> {
    let path = PathBuf::from(env_required(name)?);
    if !path.is_absolute() {
        anyhow::bail!("{name} must be an absolute path");
    }
    Ok(path)
}

fn load_peers(path: &Path) -> anyhow::Result<HashMap<String, PeerPolicy>> {
    let meta = fs::symlink_metadata(path)?;
    if !meta.is_file() || meta.len() > 32_768 {
        anyhow::bail!("peer configuration must be a regular file of at most 32 KiB");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if meta.permissions().mode() & 0o077 != 0 {
            anyhow::bail!("peer configuration must be owner-only (chmod 600)");
        }
    }
    let raw = fs::read_to_string(path)?;
    let parsed: HashMap<String, PeerConfig> = serde_json::from_str(&raw)
        .map_err(|_| anyhow::anyhow!("invalid peer configuration JSON"))?;
    if parsed.is_empty() || parsed.len() > 64 {
        anyhow::bail!("configure between 1 and 64 authenticated peers");
    }
    let mut peers = HashMap::with_capacity(parsed.len());
    let mut seen_keys = HashSet::new();
    for (node, entry) in parsed {
        let secret = B64
            .decode(&entry.key_b64)
            .map_err(|_| anyhow::anyhow!("invalid peer key for {node}"))?;
        let key: [u8; 32] = secret
            .try_into()
            .map_err(|_| anyhow::anyhow!("peer key for {node} must be 32 bytes"))?;
        if key == [0; 32] || entry.allowed_tools.is_empty() {
            anyhow::bail!("invalid empty/zero peer credentials or ACL for {node}");
        }
        if !seen_keys.insert(key) {
            anyhow::bail!("each coordinator peer must have a unique secret");
        }
        let mut policy = PeerPolicy::new(key, entry.allowed_tools);
        if entry.submit_jobs {
            policy = policy.allow_job_submission();
        }
        if entry.worker_jobs {
            policy = policy.allow_job_worker();
        }
        peers.insert(node, policy);
    }
    Ok(peers)
}

/// Opt-in multi-host coordinator path. All security-critical state lives on
/// the same PostgreSQL primary, and serving is tied to an active advisory
/// leader. PostgreSQL HA, VIP/load balancer and certificate operations are
/// separate operator responsibilities.
async fn run_postgres_cluster(node_id: &str) -> anyhow::Result<()> {
    let dsn = env_required("OCAF_CLUSTER_PG_DSN")?;
    let cluster_id = env_required("OCAF_CLUSTER_ID")?;
    let allow_loopback = env::var("OCAF_ALLOW_INSECURE_LOOPBACK").as_deref() == Ok("1");

    let peers = load_peers(&absolute_path("OCAF_PEERS_FILE")?)?;
    let source = read_zenoh_config(&absolute_path("OCAF_ZENOH_CONFIG")?)?;
    validate_zenoh_config(&source, allow_loopback)?;
    let config = zenoh::Config::from_json5(&source)
        .map_err(|err| anyhow::anyhow!("cannot parse vetted Zenoh config: {err}"))?;

    let leader = Arc::new(PgLeader::connect(&dsn, &cluster_id, allow_loopback).await?);
    let mode = env::var("OCAF_COORDINATOR_MODE").unwrap_or_else(|_| "active".into());
    match mode.as_str() {
        "active" => {
            if !leader.try_acquire().await? {
                anyhow::bail!("another coordinator owns PostgreSQL cluster leadership");
            }
        }
        "standby" => {
            tracing::info!("standby awaits PostgreSQL cluster leadership");
            leader.acquire_standby(Duration::from_millis(250)).await?;
        }
        _ => anyhow::bail!("OCAF_COORDINATOR_MODE must be active or standby"),
    }
    leader.verify().await?;
    let gateway = RpcGateway::new(node_id, OcafRuntime::new(0), peers, 64)?
        .with_postgres_replay(&dsn, allow_loopback)
        .await?
        .with_postgres_coordinator(&dsn, allow_loopback)
        .await?
        .with_postgres_job_db(&dsn, allow_loopback)
        .await?
        .with_pg_leader(Arc::clone(&leader));

    let session = Arc::new(
        zenoh::open(config)
            .await
            .map_err(|err| anyhow::anyhow!("Zenoh open: {err}"))?,
    );
    let server = ZenohRpcServer::new(session, Arc::new(gateway));
    tracing::info!(node=%node_id, cluster=%cluster_id, "PostgreSQL coordinator leader serving");

    // If the session dies or PostgreSQL stops responding, destroy the
    // queryable and terminate rather than serving from a stale host.
    let watch = async {
        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;
            leader.verify().await?;
        }
        #[allow(unreachable_code)]
        Ok::<(), ocaf_runtime::cluster::leadership::PgLeaderError>(())
    };
    tokio::select! {
        result = server.serve() => result,
        result = watch => {
            match result {
                Ok(()) => anyhow::bail!("leader monitor unexpectedly stopped"),
                Err(err) => anyhow::bail!("PostgreSQL leadership lost; coordinator stopped: {err}"),
            }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let node_id = env_required("OCAF_COORDINATOR_NODE_ID")?;
    if env::var("OCAF_CLUSTER_MODE").as_deref() == Ok("postgres") {
        return run_postgres_cluster(&node_id).await;
    }
    if env::var("OCAF_CLUSTER_MODE").is_ok() {
        anyhow::bail!("OCAF_CLUSTER_MODE must be postgres when specified");
    }
    let replay = absolute_path("OCAF_REPLAY_DB_PATH")?;
    let membership = absolute_path("OCAF_COORDINATOR_DB_PATH")?;
    let jobs = absolute_path("OCAF_JOBS_DB_PATH")?;
    let lock_path = absolute_path("OCAF_COORDINATOR_LOCK_PATH")?;
    if replay == membership
        || replay == jobs
        || membership == jobs
        || lock_path == replay
        || lock_path == membership
        || lock_path == jobs
    {
        anyhow::bail!("leadership, coordinator, replay and job paths must be separate");
    }
    // A different lock directory could allow apparently identical leaders
    // to acquire different locks. Canonicalize the *parents* before serving.
    let lock_parent = fs::canonicalize(
        lock_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("lock parent missing"))?,
    )?;
    let jobs_parent = fs::canonicalize(
        jobs.parent()
            .ok_or_else(|| anyhow::anyhow!("jobs parent missing"))?,
    )?;
    if lock_parent != jobs_parent {
        anyhow::bail!(
            "coordinator leader lock must be in the same local directory as the job database"
        );
    }

    let mode = env::var("OCAF_COORDINATOR_MODE").unwrap_or_else(|_| "active".into());
    let _leader = match mode.as_str() {
        "active" => LeaderGuard::try_acquire(&lock_path)?.ok_or_else(|| {
            anyhow::anyhow!("another coordinator already holds the exclusive lock")
        })?,
        "standby" => {
            tracing::info!("coordinator standby waiting for exclusive local leadership");
            LeaderGuard::acquire_standby(&lock_path, Duration::from_millis(250)).await?
        }
        _ => anyhow::bail!("OCAF_COORDINATOR_MODE must be active or standby"),
    };
    let peers = load_peers(&absolute_path("OCAF_PEERS_FILE")?)?;
    let zenoh_config = env_required("OCAF_ZENOH_CONFIG")?;
    let source = read_zenoh_config(Path::new(&zenoh_config))?;
    validate_zenoh_config(
        &source,
        env::var("OCAF_ALLOW_INSECURE_LOOPBACK").as_deref() == Ok("1"),
    )?;
    let config = zenoh::Config::from_json5(&source)
        .map_err(|err| anyhow::anyhow!("cannot parse vetted Zenoh config: {err}"))?;

    let gateway =
        RpcGateway::with_sqlite_replay(&node_id, OcafRuntime::new(0), peers, 64, &replay)?
            .with_coordinator_db(&membership)?;
    let gateway = if let Ok(dsn) = env::var("OCAF_PG_JOBS_DSN") {
        // Only the job journal is shared. Replay and membership still use
        // local SQLite, so this opt-in is NOT cross-host coordinator HA.
        if env::var("OCAF_EXPERIMENTAL_PG_JOBS").as_deref() != Ok("1") {
            anyhow::bail!(
                "PostgreSQL job backend is experimental; OCAF_EXPERIMENTAL_PG_JOBS=1 is required"
            );
        }
        gateway
            .with_postgres_job_db(
                &dsn,
                env::var("OCAF_ALLOW_INSECURE_LOOPBACK").as_deref() == Ok("1"),
            )
            .await?
    } else {
        gateway.with_job_db(&jobs)?
    };
    let session = Arc::new(
        zenoh::open(config)
            .await
            .map_err(|err| anyhow::anyhow!("Zenoh open: {err}"))?,
    );
    tracing::info!(node = %node_id, "OCAF authenticated coordinator active");
    ZenohRpcServer::new(session, Arc::new(gateway))
        .serve()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_shared_identity_secrets_between_two_distinct_peers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("peers.json");
        let encoded = B64.encode([42u8; 32]);
        let config = serde_json::json!({
            "node-a": {"key_b64": encoded, "allowed_tools": ["echo"]},
            "node-b": {"key_b64": encoded, "allowed_tools": ["echo"]}
        });
        std::fs::write(&path, serde_json::to_string(&config).unwrap()).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        assert!(load_peers(&path).is_err());
    }
}
