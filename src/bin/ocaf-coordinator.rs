//! Authenticated capability coordinator for a trusted network.
//! HMAC provides integrity, NOT confidentiality. An operator must configure
//! Zenoh TLS/mTLS before using this on non-loopback addresses.
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use ocaf_runtime::cluster::jobs::PgJobQueue;
use ocaf_runtime::cluster::leadership::PgLeader;
use ocaf_runtime::coordinator::leadership::LeaderGuard;
use ocaf_runtime::ops::secrets::{load_env_secret, read_private_text};
use ocaf_runtime::rpc::{PeerPolicy, RpcGateway};
use ocaf_runtime::runtime::OcafRuntime;
use ocaf_runtime::transport::rpc::ZenohRpcServer;
#[cfg(feature = "tls-bound")]
use ocaf_runtime::transport::tls_bound::{server_config, TlsBoundRpcServer};
use ocaf_runtime::transport_policy::{read_zenoh_config, validate_zenoh_config};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PeerConfig {
    key_b64: String,
    // Optional credential overlap schedule, evaluated against server UTC
    // milliseconds. All three fields must be present together.
    #[serde(default)]
    next_key_b64: Option<String>,
    #[serde(default)]
    activate_at_ms: Option<u64>,
    #[serde(default)]
    retire_old_at_ms: Option<u64>,
    /// Opt-in pinned leaf certificate proof for this RPC sender.
    #[serde(default)]
    certificate_pem_path: Option<PathBuf>,
    #[serde(default)]
    certificate_sha256: Option<String>,
    #[serde(default)]
    tls_dns_san: Option<String>,
    allowed_tools: Vec<String>,
    #[serde(default)]
    submit_jobs: bool,
    #[serde(default)]
    worker_jobs: bool,
    /// An additional, separately audited capability (default deny).
    #[serde(default)]
    write_effects: bool,
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
    let raw = read_private_text(path, 32_768)?;
    let parsed: HashMap<String, PeerConfig> = serde_json::from_str(&raw)
        .map_err(|_| anyhow::anyhow!("invalid peer configuration JSON"))?;
    if parsed.is_empty() || parsed.len() > 64 {
        anyhow::bail!("configure between 1 and 64 authenticated peers");
    }
    let mut peers = HashMap::with_capacity(parsed.len());
    let mut seen_keys = HashSet::new();
    let mut seen_certificates = HashSet::new();
    let require_certificate_proof = env::var("OCAF_REQUIRE_PEER_CERT_POP").as_deref() == Ok("1");
    let tls_mode = env::var("OCAF_RPC_INGRESS").as_deref() == Ok("tls-bound");
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
        match (
            entry.next_key_b64,
            entry.activate_at_ms,
            entry.retire_old_at_ms,
        ) {
            (None, None, None) => {}
            (Some(encoded), Some(activate), Some(retire)) => {
                let decoded = B64
                    .decode(encoded.as_bytes())
                    .map_err(|_| anyhow::anyhow!("invalid next peer key for {node}"))?;
                let next_key: [u8; 32] = decoded
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("next peer key for {node} must be 32 bytes"))?;
                if !seen_keys.insert(next_key) {
                    anyhow::bail!("duplicate coordinator peer credential across rotations");
                }
                policy = policy
                    .with_key_rotation(next_key, activate, retire)
                    .map_err(|_| anyhow::anyhow!("invalid peer rotation schedule for {node}"))?;
            }
            _ => anyhow::bail!("incomplete peer rotation configuration for {node}"),
        }
        match (entry.certificate_pem_path, entry.certificate_sha256) {
            (None, None) => {
                if require_certificate_proof {
                    anyhow::bail!("peer certificate binding required for {node}");
                }
            }
            (Some(cert_path), Some(pin)) => {
                if !cert_path.is_absolute() {
                    anyhow::bail!("peer certificate path must be absolute for {node}");
                }
                let pem = read_private_text(&cert_path, 16_384)?;
                policy = policy
                    .with_pinned_certificate(pem.as_bytes(), &pin)
                    .map_err(|_| anyhow::anyhow!("invalid pinned certificate for {node}"))?;
                if !seen_certificates.insert(pin) {
                    anyhow::bail!("duplicate certificate identity across peers");
                }
            }
            _ => anyhow::bail!("incomplete certificate identity configuration for {node}"),
        }
        match entry.tls_dns_san {
            Some(dns) => {
                if !tls_mode {
                    anyhow::bail!("TLS-bound peer identity cannot use legacy Zenoh RPC ingress");
                }
                #[cfg(feature = "tls-bound")]
                {
                    policy = policy
                        .with_tls_dns_identity(&dns)
                        .map_err(|_| anyhow::anyhow!("invalid TLS-bound SAN for {node}"))?;
                }
                #[cfg(not(feature = "tls-bound"))]
                {
                    let _ = dns;
                    anyhow::bail!("TLS-bound ingress requires compile-time feature tls-bound");
                }
            }
            None if tls_mode => anyhow::bail!("TLS-bound ingress requires peer certificate and DNS SAN for {node}"),
            None => {}
        }
        if entry.submit_jobs {
            policy = policy.allow_job_submission();
        }
        if entry.worker_jobs {
            policy = policy.allow_job_worker();
        }
        if entry.write_effects {
            if !entry.worker_jobs {
                anyhow::bail!("write_effects requires worker_jobs for {node}");
            }
            policy = policy.allow_fenced_effect_writes();
        }
        peers.insert(node, policy);
    }
    Ok(peers)
}

fn ingress_config(allow_loopback: bool) -> anyhow::Result<zenoh::Config> {
    match env::var("OCAF_RPC_INGRESS").as_deref() {
        Ok("tls-bound") => {
            // A tls-bound coordinator never creates a Zenoh session.
            Ok(zenoh::Config::default())
        }
        Ok("zenoh") | Err(env::VarError::NotPresent) => {
            let source = read_zenoh_config(&absolute_path("OCAF_ZENOH_CONFIG")?)?;
            validate_zenoh_config(&source, allow_loopback)?;
            zenoh::Config::from_json5(&source)
                .map_err(|_| anyhow::anyhow!("cannot parse vetted Zenoh config"))
        }
        _ => anyhow::bail!("OCAF_RPC_INGRESS must be zenoh or tls-bound"),
    }
}

/// Select exactly ONE ingress for the process. In TLS-bound mode Zenoh
/// queryables are never advertised, preventing legacy HMAC-only fallback.
async fn serve_gateway(
    gateway: Arc<RpcGateway>,
    config: zenoh::Config,
) -> anyhow::Result<()> {
    match env::var("OCAF_RPC_INGRESS").as_deref() {
        Ok("tls-bound") => {
            #[cfg(feature = "tls-bound")]
            {
                let cert = read_private_text(&absolute_path("OCAF_TLS_SERVER_CERT_FILE")?, 32_768)?;
                let key = read_private_text(&absolute_path("OCAF_TLS_SERVER_KEY_FILE")?, 16_384)?;
                let ca = read_private_text(&absolute_path("OCAF_TLS_CLIENT_CA_FILE")?, 131_072)?;
                let config = server_config(cert.as_bytes(), key.as_bytes(), ca.as_bytes())?;
                let bind = env_required("OCAF_TLS_RPC_BIND")?
                    .parse::<std::net::SocketAddr>()
                    .map_err(|_| anyhow::anyhow!("OCAF_TLS_RPC_BIND must be an explicit socket address"))?;
                let listener = tokio::net::TcpListener::bind(bind).await?;
                tracing::info!(address = %listener.local_addr()?, "Exclusive TLS 1.3-bound RPC ingress started");
                TlsBoundRpcServer::new(listener, config, gateway)?.serve().await
            }
            #[cfg(not(feature = "tls-bound"))]
            {
                let _ = gateway;
                let _ = config;
                anyhow::bail!("TLS-bound ingress requires compile-time feature tls-bound");
            }
        }
        Ok("zenoh") | Err(env::VarError::NotPresent) => {
            let session = Arc::new(zenoh::open(config).await?);
            ZenohRpcServer::new(session, gateway).serve().await
        }
        _ => anyhow::bail!("OCAF_RPC_INGRESS must be zenoh or tls-bound"),
    }
}

/// Opt-in multi-host coordinator path. All security-critical state lives on
/// the same PostgreSQL primary, and serving is tied to an active advisory
/// leader. PostgreSQL HA, VIP/load balancer and certificate operations are
/// separate operator responsibilities.
/// Opt-in restartable HA supervisor. For safety, every iteration opens new
/// PostgreSQL connections and a fresh Zenoh queryable only *after* election.
/// The process remains alive during a database outage but does not serve.
async fn run_postgres_cluster(node_id: &str) -> anyhow::Result<()> {
    let dsn = load_env_secret("OCAF_CLUSTER_PG_DSN", 4096)?;
    let cluster_id = env_required("OCAF_CLUSTER_ID")?;
    let allow_loopback = env::var("OCAF_ALLOW_INSECURE_LOOPBACK").as_deref() == Ok("1");

    let peers = load_peers(&absolute_path("OCAF_PEERS_FILE")?)?;
    let config = ingress_config(allow_loopback)?;
    let mode = env::var("OCAF_COORDINATOR_MODE").unwrap_or_else(|_| "active".into());
    match mode.as_str() {
        "active" | "standby" => {
            serve_postgres_generation(
                node_id,
                &dsn,
                &cluster_id,
                allow_loopback,
                &peers,
                &config,
                mode.as_str(),
            )
            .await
        }
        "ha" => {
            let mut delay = Duration::from_millis(250);
            loop {
                let result = serve_postgres_generation(
                    node_id,
                    &dsn,
                    &cluster_id,
                    allow_loopback,
                    &peers,
                    &config,
                    "standby",
                )
                .await;
                // A failed PostgreSQL connection, loss of a leader session,
                // or a failed Zenoh service may never leave the old gateway
                // online while a newly elected leader starts serving.
                match result {
                    Ok(()) => tracing::warn!("HA service ended; reconnecting"),
                    Err(err) => tracing::warn!(error=%err, "HA service unavailable; reconnecting"),
                }
                tokio::time::sleep(delay).await;
                delay = delay.saturating_mul(2).min(Duration::from_secs(5));
            }
        }
        _ => anyhow::bail!("OCAF_COORDINATOR_MODE must be active, standby or ha"),
    }
}

async fn serve_postgres_generation(
    node_id: &str,
    dsn: &str,
    cluster_id: &str,
    allow_loopback: bool,
    peers: &HashMap<String, PeerPolicy>,
    config: &zenoh::Config,
    mode: &str,
) -> anyhow::Result<()> {
    let leader = Arc::new(PgLeader::connect(dsn, cluster_id, allow_loopback).await?);
    match mode {
        "active" => {
            if !leader.try_acquire().await? {
                anyhow::bail!("another coordinator owns PostgreSQL cluster leadership");
            }
        }
        "standby" => {
            tracing::info!(cluster=%cluster_id, "standby awaits PostgreSQL cluster leadership");
            leader.acquire_standby(Duration::from_millis(250)).await?;
        }
        _ => anyhow::bail!("invalid internal cluster election mode"),
    }
    leader.verify().await?;

    let jobs = PgJobQueue::connect(dsn, allow_loopback)
        .await?
        .with_leader(Arc::clone(&leader));
    // Promotion must mark all leases from a former generation uncertain
    // immediately, rather than waiting for their full deadline.
    let recovered = jobs.recover_expired(0).await?;
    tracing::info!(
        cluster=%cluster_id,
        epoch=leader.fencing_epoch().await?,
        uncertain=recovered.uncertain,
        "PostgreSQL promotion recovery completed"
    );

    let gateway = RpcGateway::new(node_id, OcafRuntime::new(0), peers.clone(), 64)?
        .with_postgres_replay(dsn, allow_loopback)
        .await?
        .with_postgres_coordinator(dsn, allow_loopback)
        .await?
        .with_postgres_job_db(dsn, allow_loopback)
        .await?
        .with_pg_leader(Arc::clone(&leader));

    tracing::info!(node=%node_id, cluster=%cluster_id, "PostgreSQL coordinator leader serving");

    let watch = async {
        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;
            leader.verify().await?;
        }
        #[allow(unreachable_code)]
        Ok::<(), ocaf_runtime::cluster::leadership::PgLeaderError>(())
    };
    // Periodic recovery works even when workers stop polling entirely.
    // Recovery itself verifies leadership and locks the epoch transaction.
    let sweep = async {
        loop {
            tokio::time::sleep(Duration::from_secs(2)).await;
            jobs.recover_expired(0).await?;
        }
        #[allow(unreachable_code)]
        Ok::<(), ocaf_runtime::jobs::JobError>(())
    };
    let result = tokio::select! {
        result = serve_gateway(Arc::new(gateway), config.clone()) => result,
        result = watch => {
            match result {
                Ok(()) => Err(anyhow::anyhow!("leader monitor unexpectedly stopped")),
                Err(err) => Err(anyhow::anyhow!("PostgreSQL leadership lost: {err}")),
            }
        },
        result = sweep => {
            match result {
                Ok(()) => Err(anyhow::anyhow!("lease recovery sweep unexpectedly stopped")),
                Err(err) => Err(anyhow::anyhow!("PostgreSQL recovery failed: {err}")),
            }
        }
    };
    // Drop the serving queryable first; release the exact advisory session
    // before a new iteration attempts another election.
    if let Err(err) = leader.release().await {
        tracing::warn!(error=%err, "leader session release failed closed");
    }
    result
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
    let config = ingress_config(
        env::var("OCAF_ALLOW_INSECURE_LOOPBACK").as_deref() == Ok("1"),
    )?;

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
    tracing::info!(node = %node_id, "OCAF authenticated coordinator active");
    serve_gateway(Arc::new(gateway), config).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_incomplete_certificate_identity_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("peers.json");
        let config = serde_json::json!({
            "agent": {
                "key_b64": B64.encode([1u8; 32]),
                "certificate_sha256": "a".repeat(64),
                "allowed_tools": ["echo"]
            }
        });
        std::fs::write(&path, config.to_string()).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        assert!(load_peers(&path).is_err());
    }

    #[test]
    fn rejects_incomplete_peer_rotation_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("peers.json");
        let config = serde_json::json!({
            "agent": {
                "key_b64": B64.encode([1u8; 32]),
                "next_key_b64": B64.encode([2u8; 32]),
                "allowed_tools": ["echo"]
            }
        });
        std::fs::write(&path, config.to_string()).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        assert!(load_peers(&path).is_err());
    }

    #[test]
    fn rejects_reused_rotation_key_across_peer_identities() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("peers.json");
        let config = serde_json::json!({
            "agent-a": {
                "key_b64": B64.encode([1u8; 32]),
                "next_key_b64": B64.encode([3u8; 32]),
                "activate_at_ms": 1800000000000u64,
                "retire_old_at_ms": 1800000060000u64,
                "allowed_tools": ["echo"]
            },
            "agent-b": {
                "key_b64": B64.encode([3u8; 32]),
                "allowed_tools": ["echo"]
            }
        });
        std::fs::write(&path, config.to_string()).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        assert!(load_peers(&path).is_err());
    }

    #[test]
    fn accepts_bounded_staged_secret_rotation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("peers.json");
        let config = serde_json::json!({
            "agent": {
                "key_b64": B64.encode([1u8; 32]),
                "next_key_b64": B64.encode([2u8; 32]),
                "activate_at_ms": 1800000000000u64,
                "retire_old_at_ms": 1800000060000u64,
                "allowed_tools": ["echo"]
            }
        });
        std::fs::write(&path, config.to_string()).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        assert_eq!(load_peers(&path).unwrap().len(), 1);
    }

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
