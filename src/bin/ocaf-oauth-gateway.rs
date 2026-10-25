//! Loopback-only OAuth protected-resource authorization gateway.
//! Public TLS ingress must be provided by a separately hardened reverse proxy.
//! This process NEVER issues ID tokens, handles passwords, or dispatches OMAt.
use ocaf_runtime::stage044_oidc::OidcTrust;
use serde_json::json;
use std::collections::BTreeSet;
use std::env;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Semaphore;
use tokio::time::{timeout, Duration};

const MAX_HEADERS: usize = 8192;
const MAX_CONNECTIONS: usize = 64;

struct State {
    trust: OidcTrust,
    issuer: String,
    resource: String,
    scopes: BTreeSet<String>,
}

struct Request {
    method: String,
    path: String,
    authorization: Option<String>,
}

fn config(name: &str) -> Result<String, &'static str> {
    let value = env::var(name).map_err(|_| "required operator configuration missing")?;
    if value.is_empty() || value.len() > 2048 || value.contains('\n') || value.contains('\r') {
        return Err("invalid operator configuration");
    }
    Ok(value)
}

fn parse_request(bytes: &[u8]) -> Result<Request, ()> {
    if bytes.len() > MAX_HEADERS || !bytes.ends_with(b"\r\n\r\n") {
        return Err(());
    }
    let raw = std::str::from_utf8(bytes).map_err(|_| ())?;
    let mut lines = raw.split("\r\n");
    let first = lines.next().ok_or(())?;
    let mut start = first.split(' ');
    let method = start.next().ok_or(())?;
    let path = start.next().ok_or(())?;
    if start.next() != Some("HTTP/1.1")
        || start.next().is_some()
        || !matches!(method, "GET" | "POST")
        || !path.starts_with('/')
        || path.len() > 2048
        || path.contains('?')
        || path.contains('#')
        || path.contains('%')
    {
        return Err(());
    }
    let mut host = false;
    let mut authorization = None;
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let (key, value) = line.split_once(':').ok_or(())?;
        if key.is_empty()
            || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || value.bytes().any(|b| b == 0 || b == b'\r' || b == b'\n')
        {
            return Err(());
        }
        let value = value.trim();
        if key.eq_ignore_ascii_case("host") {
            if host || value.is_empty() {
                return Err(());
            }
            host = true;
        } else if key.eq_ignore_ascii_case("authorization") {
            if authorization.is_some() || value.len() > 16_384 {
                return Err(());
            }
            authorization = Some(value.to_owned());
        } else if key.eq_ignore_ascii_case("content-length") {
            if value != "0" {
                return Err(());
            }
        } else if key.eq_ignore_ascii_case("transfer-encoding") {
            return Err(());
        }
    }
    if !host {
        return Err(());
    }
    Ok(Request {
        method: method.into(),
        path: path.into(),
        authorization,
    })
}

fn decision(state: &State, request: Request) -> (u16, &'static str, Vec<u8>) {
    if request.method == "GET" && request.path == "/healthz" {
        return (200, "application/json", b"{\"status\":\"ok\"}".to_vec());
    }
    if request.method == "GET" && request.path == "/.well-known/oauth-protected-resource" {
        let body = json!({
            "resource":state.resource,
            "authorization_servers":[state.issuer],
            "bearer_methods_supported":["header"]
        });
        return (
            200,
            "application/json",
            serde_json::to_vec(&body).unwrap_or_default(),
        );
    }
    if request.method != "POST" {
        return (404, "application/json", b"{}".to_vec());
    }
    let segments: Vec<&str> = request.path.split('/').collect();
    if segments.len() != 6
        || segments[0] != ""
        || segments[1] != "omat"
        || segments[3] != "users"
        || segments[5] != "authorize"
        || segments[2].is_empty()
        || segments[4].is_empty()
    {
        return (404, "application/json", b"{}".to_vec());
    }
    let bearer = request
        .authorization
        .as_deref()
        .and_then(|v| v.strip_prefix("Bearer "))
        .filter(|v| !v.is_empty() && !v.contains(' '));
    let Some(token) = bearer else {
        return (
            401,
            "application/json",
            b"{\"error\":\"invalid_token\"}".to_vec(),
        );
    };
    let now = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(value) => i64::try_from(value.as_secs()).unwrap_or(i64::MAX),
        Err(_) => return (503, "application/json", b"{}".to_vec()),
    };
    let verified = state
        .trust
        .verify_access_token(token, &state.resource, &state.scopes, now);
    match verified {
        Ok(identity) if identity.tenant == segments[2] && identity.user == segments[4] => {
            (204, "application/json", Vec::new())
        }
        _ => (
            403,
            "application/json",
            b"{\"error\":\"access_denied\"}".to_vec(),
        ),
    }
}

async fn handle(mut socket: TcpStream, state: Arc<State>) {
    let result = timeout(Duration::from_secs(4), async {
        let mut bytes = Vec::new();
        loop {
            let mut part = [0u8; 1024];
            let count = socket.read(&mut part).await.map_err(|_| ())?;
            if count == 0 { return Err(()); }
            bytes.extend_from_slice(&part[..count]);
            if bytes.len() > MAX_HEADERS { return Err(()); }
            if bytes.windows(4).any(|v| v == b"\r\n\r\n") { break; }
        }
        let parsed = parse_request(&bytes);
        let (status,ctype,body) = match parsed {
            Ok(req) => decision(&state,req),
            Err(_) => (400,"application/json",b"{}".to_vec()),
        };
        let reason = match status {
            200 => "OK", 204 => "No Content", 400 => "Bad Request",
            401 => "Unauthorized", 403 => "Forbidden",
            404 => "Not Found", _ => "Service Unavailable",
        };
        let challenge = if status == 401 {
            "WWW-Authenticate: Bearer error=\"invalid_token\"\r\n"
        } else { "" };
        let headers = format!(
            "HTTP/1.1 {status} {reason}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nPragma: no-cache\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n{challenge}\r\n",
            body.len()
        );
        socket.write_all(headers.as_bytes()).await.map_err(|_| ())?;
        socket.write_all(&body).await.map_err(|_| ())?;
        Ok::<(), ()>(())
    }).await;
    if result.is_err() {
        // Timeout closes the connection; no tokens, cookies or request bodies logged.
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let issuer = config("OCAF_OIDC_ISSUER")?;
    let resource = config("OCAF_OIDC_RESOURCE")?;
    if !resource.starts_with("https://") {
        return Err("invalid resource audience".into());
    }
    let scopes: BTreeSet<String> = config("OCAF_OIDC_REQUIRED_SCOPES")?
        .split_ascii_whitespace()
        .map(str::to_owned)
        .collect();
    if scopes.is_empty() || scopes.len() > 64 {
        return Err("invalid scopes".into());
    }
    let pem_path = config("OCAF_OIDC_PUBLIC_KEY_FILE")?;
    let pem = std::fs::read(pem_path)?;
    let trust = OidcTrust::new(
        issuer.clone(),
        config("OCAF_OIDC_CLIENT_ID")?,
        config("OCAF_OIDC_TENANT")?,
        config("OCAF_OIDC_KEY_ID")?,
        &pem,
        config("OCAF_OIDC_MFA_ACR")?,
    )?;
    let addr: SocketAddr = env::var("OCAF_OAUTH_BIND")
        .unwrap_or_else(|_| "127.0.0.1:8765".into())
        .parse()?;
    if !matches!(addr.ip(), IpAddr::V4(v) if v.is_loopback()) {
        return Err("plaintext listener MUST bind to IPv4 loopback only".into());
    }
    let state = Arc::new(State {
        trust,
        issuer,
        resource,
        scopes,
    });
    let listener = TcpListener::bind(addr).await?;
    let semaphore = Arc::new(Semaphore::new(MAX_CONNECTIONS));
    loop {
        let (socket, _) = listener.accept().await?;
        let permit = Arc::clone(&semaphore).acquire_owned().await?;
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            let _permit = permit;
            handle(socket, state).await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn denies_header_smuggling_and_unauthenticated_authorization() {
        assert!(parse_request(b"POST /omat/t/users/u/authorize HTTP/1.1\r\nHost: a\r\nTransfer-Encoding: chunked\r\n\r\n").is_err());
        assert!(parse_request(
            b"POST /omat/t/users/u/authorize HTTP/1.1\r\nHost: a\r\nContent-Length: 1\r\n\r\n"
        )
        .is_err());
        assert!(parse_request(b"POST /omat/t/users/u/authorize HTTP/1.1\r\nHost: a\r\nAuthorization: Bearer a\r\nAuthorization: Bearer b\r\n\r\n").is_err());
        assert!(
            parse_request(b"POST /omat/t/users/u/authorize?x=y HTTP/1.1\r\nHost: a\r\n\r\n")
                .is_err()
        );
        assert!(
            parse_request(b"POST /omat/t/users/u/authorize HTTP/1.1\r\nHost: a\r\n\r\n").is_ok()
        );
    }
}
