//! Loopback-only PostgreSQL TCP failure injector for CI. This module must
//! never be used as an unauthenticated production network proxy.
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
use std::sync::Arc;

use tokio::io::copy_bidirectional;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tokio::task::JoinHandle;

const MODE_FORWARD: u8 = 0;
const MODE_PARTITION: u8 = 1;
const MODE_BLACKHOLE: u8 = 2;

pub struct PgFaultProxy {
    port: u16,
    mode: Arc<AtomicU8>,
    killed: Arc<AtomicUsize>,
    drop_tx: watch::Sender<u64>,
    accept: JoinHandle<()>,
}

fn pg_port(dsn: &str) -> anyhow::Result<u16> {
    // Test harness is intentionally restricted to the PostgreSQL container
    // on local loopback. In particular never proxy arbitrary operator DSNs.
    if !dsn.split_whitespace().any(|p| p == "host=127.0.0.1")
        || !dsn.split_whitespace().any(|p| p == "sslmode=disable")
    {
        anyhow::bail!("fault proxy requires explicit test-only loopback PostgreSQL");
    }
    let port = dsn
        .split_whitespace()
        .find_map(|p| p.strip_prefix("port="))
        .unwrap_or("5432")
        .parse::<u16>()?;
    if port == 0 {
        anyhow::bail!("invalid test database port");
    }
    Ok(port)
}

impl PgFaultProxy {
    pub async fn bind(dsn: &str) -> anyhow::Result<Self> {
        let backend_port = pg_port(dsn)?;
        let listener = TcpListener::bind(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            0,
        ))
        .await?;
        let port = listener.local_addr()?.port();
        let mode = Arc::new(AtomicU8::new(MODE_FORWARD));
        let killed = Arc::new(AtomicUsize::new(0));
        let (drop_tx, _) = watch::channel(0u64);
        let active_mode = Arc::clone(&mode);
        let count = Arc::clone(&killed);
        let signal = drop_tx.subscribe();
        let accept = tokio::spawn(async move {
            while let Ok((mut inbound, _)) = listener.accept().await {
                let mode = active_mode.load(Ordering::SeqCst);
                if mode == MODE_PARTITION {
                    continue;
                }
                let mut cancel = signal.clone();
                let cancelled = Arc::clone(&count);
                tokio::spawn(async move {
                    if mode == MODE_BLACKHOLE {
                        // Keep the client TCP socket open but do not respond
                        // to PostgreSQL's startup exchange. This simulates a
                        // one-way packet blackhole, not a clean TCP reset.
                        let _ = cancel.changed().await;
                        return;
                    }
                    let Ok(mut upstream) =
                        TcpStream::connect((Ipv4Addr::LOCALHOST, backend_port)).await
                    else {
                        return;
                    };
                    tokio::select! {
                        _ = copy_bidirectional(&mut inbound, &mut upstream) => {},
                        changed = cancel.changed() => {
                            if changed.is_ok() {
                                cancelled.fetch_add(1, Ordering::SeqCst);
                            }
                        }
                    };
                });
            }
        });
        Ok(Self {
            port,
            mode,
            killed,
            drop_tx,
            accept,
        })
    }

    pub fn proxied_dsn(&self, original: &str) -> anyhow::Result<String> {
        pg_port(original)?;
        let mut args = original
            .split_whitespace()
            .filter(|entry| !entry.starts_with("port="))
            .collect::<Vec<_>>()
            .join(" ");
        args.push_str(&format!(" port={}", self.port));
        Ok(args)
    }

    pub fn partition(&self) {
        self.mode.store(MODE_PARTITION, Ordering::SeqCst);
        self.signal_drop();
    }

    pub fn blackhole_new_connections(&self) {
        self.mode.store(MODE_BLACKHOLE, Ordering::SeqCst);
        self.signal_drop();
    }

    pub fn heal(&self) {
        self.mode.store(MODE_FORWARD, Ordering::SeqCst);
    }

    pub fn terminated_connections(&self) -> usize {
        self.killed.load(Ordering::SeqCst)
    }

    fn signal_drop(&self) {
        self.drop_tx.send_modify(|epoch| *epoch = epoch.wrapping_add(1));
    }
}

impl Drop for PgFaultProxy {
    fn drop(&mut self) {
        self.accept.abort();
        // Closing the last sender interrupts all in-flight relay tasks.
    }
}
