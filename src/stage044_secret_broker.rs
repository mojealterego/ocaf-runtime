//! Stage 0.44: encrypted, tenant/user-isolated outbound secret broker.
//!
//! No caller-controlled URL, host, IP, HTTP method, redirect or credential
//! name is accepted. Every destination is pinned in operator-owned config.
//! The broker is NOT exposed as a generic proxy or LLM tool.
use crate::stage044_oauth::{private_database, AuthorizedGrant};
use openssl::rand::rand_bytes;
use openssl::ssl::{SslConnector, SslMethod, SslVerifyMode, SslVersion};
use openssl::symm::{decrypt_aead, encrypt_aead, Cipher};
use rusqlite::{params, Connection, OpenFlags, OptionalExtension, TransactionBehavior};
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;
use thiserror::Error;

const MAX_SECRET: usize = 8192;
const MAX_BODY: usize = 65_536;
const MAX_RESPONSE: u64 = 1_048_576;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BrokerError {
    #[error("invalid operator route or secret configuration")]
    Configuration,
    #[error("access denied")]
    Denied,
    #[error("encrypted secret unavailable")]
    SecretUnavailable,
    #[error("secure storage unavailable")]
    Storage,
    #[error("cryptographic operation failed")]
    Crypto,
    #[error("outbound transport rejected")]
    Transport,
    #[error("upstream response rejected")]
    Response,
}

#[derive(Clone, Debug)]
pub struct BrokerRoute {
    pub route_id: String,
    pub tool_id: String,
    pub hostname: String,
    pub pinned_ip: Ipv4Addr,
    pub port: u16,
    pub path: String,
    pub resource: String,
    pub required_scopes: BTreeSet<String>,
    pub min_assurance: u8,
}

impl BrokerRoute {
    pub(crate) fn validate(&self) -> Result<(), BrokerError> {
        let hostname = self.hostname.as_bytes();
        let safe_host = !hostname.is_empty() && hostname.len() <= 253
            && self.hostname.contains('.')
            && hostname.iter().all(|b| b.is_ascii_alphanumeric() || *b == b'-' || *b == b'.')
            && !self.hostname.starts_with('.') && !self.hostname.ends_with('.')
            && !self.hostname.contains("..") && self.hostname.parse::<IpAddr>().is_err()
            && self.hostname.split('.').all(|label| !label.is_empty() && label.len() <= 63
                && !label.starts_with('-') && !label.ends_with('-'));
        if !safe_id(&self.route_id) || !safe_id(&self.tool_id)
            || !safe_host || !public_ipv4(self.pinned_ip) || self.port != 443
            || !self.path.starts_with('/') || self.path.starts_with("//")
            || self.path.len() > 2048
            || self.path.bytes().any(|b| b <= 32 || b >= 127 || b == b'#' || b == b'\\')
            || !self.resource.starts_with("https://") || self.resource.len() > 2048
            || self.required_scopes.is_empty() || self.required_scopes.len() > 64
            || self.min_assurance == 0
        { return Err(BrokerError::Configuration); }
        Ok(())
    }
}

/// Fail-closed IPv4 public-address allowlist; deliberately rejects IPv6 until
/// the operator deploys an equivalent reviewed global-address classifier.
fn public_ipv4(ip: Ipv4Addr) -> bool {
    let [a,b,c,_] = ip.octets();
    !(a == 0 || a == 10 || a == 127 || a >= 224
        || (a == 100 && (64..=127).contains(&b))
        || (a == 169 && b == 254)
        || (a == 172 && (16..=31).contains(&b))
        || (a == 192 && (b == 168 || (b == 0 && c == 0) || (b == 0 && c == 2)))
        || (a == 198 && ((b == 18 || b == 19) || (b == 51 && c == 100)))
        || (a == 203 && b == 0 && c == 113)
        || (a == 192 && b == 88 && c == 99)
        || (a == 192 && b == 0 && c == 0)
        || (a == 192 && b == 0 && c == 2))
}
fn safe_id(s: &str) -> bool {
    !s.is_empty() && s != "." && s != ".." && s.len() <= 128 && s.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}

pub struct BrokerResponse {
    pub status: u16,
    pub body: Vec<u8>,
}
impl std::fmt::Debug for BrokerResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrokerResponse")
            .field("status", &self.status)
            .field("body", &"[REDACTED]")
            .finish()
    }
}

pub struct SecretBroker {
    db_path: PathBuf,
    key: [u8; 32],
    routes: HashMap<String, BrokerRoute>,
}

impl SecretBroker {
    /// Master key must be loaded from an operator-owned 0600 secret file or
    /// hardware key service. Never place it in source control or LLM context.
    pub fn open(db_path: &Path, key: [u8; 32], routes: Vec<BrokerRoute>) -> Result<Self, BrokerError> {
        if !private_database(db_path) || routes.is_empty() || routes.len() > 1024
            || key.iter().all(|b| *b == 0)
        { return Err(BrokerError::Configuration); }
        let mut registry = HashMap::new();
        for route in routes {
            route.validate()?;
            if registry.insert(route.route_id.clone(), route).is_some() { return Err(BrokerError::Configuration); }
        }
        let broker = Self { db_path: db_path.to_owned(), key, routes: registry };
        broker.connection()?.execute_batch(
            "CREATE TABLE IF NOT EXISTS oauth_broker_secrets (
                tenant TEXT NOT NULL, user_id TEXT NOT NULL, route TEXT NOT NULL,
                nonce BLOB NOT NULL, ciphertext BLOB NOT NULL, tag BLOB NOT NULL,
                PRIMARY KEY(tenant,user_id,route)
            );"
        ).map_err(|_| BrokerError::Storage)?;
        Ok(broker)
    }

    /// Load a 32-byte AES key from an operator-owned chmod 0600 hex file.
    pub fn open_from_key_file(db_path: &Path, key_file: &Path, routes: Vec<BrokerRoute>) -> Result<Self, BrokerError> {
        let key_hex = crate::ops::secrets::read_private_text(key_file, 128)
            .map_err(|_| BrokerError::Configuration)?;
        let key_hex = key_hex.trim_end_matches(['\r','\n']);
        if key_hex.len() != 64 { return Err(BrokerError::Configuration); }
        let mut key = [0u8; 32];
        for (index, chunk) in key.chunks_mut(1).enumerate() {
            chunk[0] = u8::from_str_radix(&key_hex[index*2..index*2+2],16)
                .map_err(|_| BrokerError::Configuration)?;
        }
        Self::open(db_path,key,routes)
    }

    fn connection(&self) -> Result<Connection, BrokerError> {
        let conn = Connection::open_with_flags(&self.db_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NOFOLLOW)
            .map_err(|_| BrokerError::Storage)?;
        conn.busy_timeout(Duration::from_secs(5)).map_err(|_| BrokerError::Storage)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")
            .map_err(|_| BrokerError::Storage)?;
        Ok(conn)
    }

    /// Operator-only provisioning API. MUST NOT be exposed to agent requests.
    pub fn provision_secret(&self, tenant: &str, user: &str, route: &str, secret: &[u8]) -> Result<(), BrokerError> {
        if !safe_id(tenant) || !safe_id(user) || !self.routes.contains_key(route)
            || secret.is_empty() || secret.len() > MAX_SECRET
            || secret.iter().any(|b| *b <= 32 || *b >= 127)
        { return Err(BrokerError::Configuration); }
        let mut nonce = [0u8; 12];
        rand_bytes(&mut nonce).map_err(|_| BrokerError::Crypto)?;
        let mut tag = [0u8; 16];
        let aad = binding(tenant, user, route);
        let ciphertext = encrypt_aead(Cipher::aes_256_gcm(), &self.key, Some(&nonce), &aad, secret, &mut tag)
            .map_err(|_| BrokerError::Crypto)?;
        self.connection()?.execute(
            "INSERT INTO oauth_broker_secrets(tenant,user_id,route,nonce,ciphertext,tag)
             VALUES(?1,?2,?3,?4,?5,?6)
             ON CONFLICT(tenant,user_id,route) DO UPDATE SET
               nonce=excluded.nonce,ciphertext=excluded.ciphertext,tag=excluded.tag",
            params![tenant,user,route,nonce.as_slice(),ciphertext,tag.as_slice()]
        ).map_err(|_| BrokerError::Storage)?;
        Ok(())
    }

    /// Every invocation is checked against the validated OAuth grant, exact
    /// tool identity and fixed route. The secret never enters agent arguments.
    pub fn authorize_route<'a>(
        &'a self, grant: &AuthorizedGrant, route_id: &str, tool_id: &str,
    ) -> Result<&'a BrokerRoute, BrokerError> {
        let route = self.routes.get(route_id).ok_or(BrokerError::Denied)?;
        if route.tool_id != tool_id || grant.resource() != route.resource
            || grant.assurance_level() < route.min_assurance
            || !route.required_scopes.is_subset(grant.scopes())
        { return Err(BrokerError::Denied); }
        Ok(route)
    }

    /// Outbound HTTPS POST: TLS >=1.3, verified hostname, operator-pinned
    /// public IP, no DNS lookup, no redirects, bounded response and timeouts.
    /// Only a trusted executor may invoke this method after OAuth validation.
    pub fn execute(
        &self, grant: &AuthorizedGrant, route_id: &str, tool_id: &str,
        json_body: &[u8],
    ) -> Result<BrokerResponse, BrokerError> {
        let route = self.authorize_route(grant, route_id, tool_id)?;
        if json_body.len() > MAX_BODY || serde_json::from_slice::<Value>(json_body).is_err() {
            return Err(BrokerError::Denied);
        }
        let conn = self.connection()?;
        let row: Option<(Vec<u8>,Vec<u8>,Vec<u8>)> = conn.query_row(
            "SELECT nonce,ciphertext,tag FROM oauth_broker_secrets
             WHERE tenant=?1 AND user_id=?2 AND route=?3",
            params![grant.tenant(),grant.user(),route_id],
            |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))
        ).optional().map_err(|_| BrokerError::Storage)?;
        let (nonce, ciphertext, tag) = row.ok_or(BrokerError::SecretUnavailable)?;
        if nonce.len() != 12 || tag.len() != 16 || ciphertext.len() > MAX_SECRET + 16 {
            return Err(BrokerError::Crypto);
        }
        let aad = binding(grant.tenant(),grant.user(),route_id);
        let mut secret = decrypt_aead(Cipher::aes_256_gcm(), &self.key, Some(&nonce), &aad, &ciphertext, &tag)
            .map_err(|_| BrokerError::Crypto)?;
        if secret.is_empty() || secret.len() > MAX_SECRET
            || secret.iter().any(|b| *b <= 32 || *b >= 127)
        { secret.fill(0); return Err(BrokerError::Crypto); }
        let result = https_post(route, &secret, json_body);
        secret.fill(0);
        result
    }

    /// Operator-only atomic re-encryption of every stored secret. The old key
    /// remains active until the entire SQLite transaction has committed.
    /// Multi-process rollout must be coordinated by the operator.
    pub fn rotate_master_key(&mut self, new_key: [u8; 32]) -> Result<(), BrokerError> {
        if new_key.iter().all(|b| *b == 0) || new_key == self.key {
            return Err(BrokerError::Configuration);
        }
        let mut conn = self.connection()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| BrokerError::Storage)?;
        type Entry = (String,String,String,Vec<u8>,Vec<u8>,Vec<u8>);
        let entries: Vec<Entry> = {
            let mut stmt = tx.prepare("SELECT tenant,user_id,route,nonce,ciphertext,tag FROM oauth_broker_secrets")
                .map_err(|_| BrokerError::Storage)?;
            let rows = stmt.query_map([],|r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?)))
                .map_err(|_| BrokerError::Storage)?;
            rows.collect::<Result<Vec<Entry>,_>>().map_err(|_| BrokerError::Storage)?
        };
        for (tenant,user,route,nonce,ciphertext,tag) in entries {
            if nonce.len()!=12 || tag.len()!=16 || ciphertext.len()>MAX_SECRET+16 { return Err(BrokerError::Crypto); }
            let aad = binding(&tenant,&user,&route);
            let mut plaintext = decrypt_aead(Cipher::aes_256_gcm(),&self.key,Some(&nonce),&aad,&ciphertext,&tag)
                .map_err(|_| BrokerError::Crypto)?;
            let mut next_nonce=[0u8;12];
            rand_bytes(&mut next_nonce).map_err(|_| BrokerError::Crypto)?;
            let mut next_tag=[0u8;16];
            let next_ciphertext=encrypt_aead(Cipher::aes_256_gcm(),&new_key,Some(&next_nonce),&aad,&plaintext,&mut next_tag)
                .map_err(|_| BrokerError::Crypto)?;
            plaintext.fill(0);
            tx.execute("UPDATE oauth_broker_secrets SET nonce=?1,ciphertext=?2,tag=?3 WHERE tenant=?4 AND user_id=?5 AND route=?6",
                params![next_nonce.as_slice(),next_ciphertext,next_tag.as_slice(),tenant,user,route])
                .map_err(|_| BrokerError::Storage)?;
        }
        tx.commit().map_err(|_| BrokerError::Storage)?;
        self.key.fill(0);
        self.key = new_key;
        Ok(())
    }

    pub fn revoke_secret(&self, tenant: &str, user: &str, route: &str) -> Result<(), BrokerError> {
        if !safe_id(tenant) || !safe_id(user) || !self.routes.contains_key(route) { return Err(BrokerError::Denied); }
        self.connection()?.execute("DELETE FROM oauth_broker_secrets WHERE tenant=?1 AND user_id=?2 AND route=?3",
            params![tenant,user,route]).map_err(|_| BrokerError::Storage)?;
        Ok(())
    }
}

impl Drop for SecretBroker {
    fn drop(&mut self) { self.key.fill(0); }
}

fn binding(tenant: &str, user: &str, route: &str) -> Vec<u8> {
    format!("ocaf:044:v1\0{tenant}\0{user}\0{route}").into_bytes()
}

pub(crate) enum BrokerAuthHeader { Bearer, VaultToken }

pub(crate) fn https_post(route: &BrokerRoute, secret: &[u8], json_body: &[u8]) -> Result<BrokerResponse, BrokerError> {
    https_post_with_auth(route, secret, json_body, BrokerAuthHeader::Bearer)
}

pub(crate) fn https_post_with_auth(
    route: &BrokerRoute, secret: &[u8], json_body: &[u8], header: BrokerAuthHeader,
) -> Result<BrokerResponse, BrokerError> {
    let addr = SocketAddr::new(IpAddr::V4(route.pinned_ip), route.port);
    let socket = TcpStream::connect_timeout(&addr, Duration::from_secs(5)).map_err(|_| BrokerError::Transport)?;
    socket.set_read_timeout(Some(Duration::from_secs(5))).map_err(|_| BrokerError::Transport)?;
    socket.set_write_timeout(Some(Duration::from_secs(5))).map_err(|_| BrokerError::Transport)?;
    let mut builder = SslConnector::builder(SslMethod::tls()).map_err(|_| BrokerError::Transport)?;
    builder.set_verify(SslVerifyMode::PEER);
    builder.set_min_proto_version(Some(SslVersion::TLS1_3)).map_err(|_| BrokerError::Transport)?;
    builder.set_default_verify_paths().map_err(|_| BrokerError::Transport)?;
    let connector = builder.build();
    let mut tls = connector.connect(&route.hostname, socket).map_err(|_| BrokerError::Transport)?;
    if tls.ssl().verify_result() != openssl::x509::X509VerifyResult::OK {
        return Err(BrokerError::Transport);
    }
    let header_name = match header {
        BrokerAuthHeader::Bearer => "Authorization: Bearer ",
        BrokerAuthHeader::VaultToken => "X-Vault-Token: ",
    };
    let prefix = format!("POST {} HTTP/1.1\r\nHost: {}\r\n{}", route.path, route.hostname, header_name);
    tls.write_all(prefix.as_bytes()).map_err(|_| BrokerError::Transport)?;
    tls.write_all(secret).map_err(|_| BrokerError::Transport)?;
    tls.write_all(format!("\r\nContent-Type: application/json\r\nAccept: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",json_body.len()).as_bytes())
        .map_err(|_| BrokerError::Transport)?;
    tls.write_all(json_body).map_err(|_| BrokerError::Transport)?;
    tls.flush().map_err(|_| BrokerError::Transport)?;
    let mut data = Vec::new();
    let mut target_len: Option<usize> = None;
    loop {
        let mut chunk = [0u8; 8192];
        let n = tls.read(&mut chunk).map_err(|_| BrokerError::Transport)?;
        if n == 0 { break; }
        data.extend_from_slice(&chunk[..n]);
        if data.len() as u64 > MAX_RESPONSE { return Err(BrokerError::Response); }
        if target_len.is_none() {
            if let Some(split) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                if split > 8192 { return Err(BrokerError::Response); }
                let header = std::str::from_utf8(&data[..split]).map_err(|_| BrokerError::Response)?;
                let mut content_length: Option<usize> = None;
                for line in header.split("\r\n").skip(1) {
                    let (name,value) = line.split_once(':').ok_or(BrokerError::Response)?;
                    if name.eq_ignore_ascii_case("transfer-encoding") { return Err(BrokerError::Response); }
                    if name.eq_ignore_ascii_case("content-length") {
                        if content_length.is_some() { return Err(BrokerError::Response); }
                        content_length = Some(value.trim().parse().map_err(|_| BrokerError::Response)?);
                    }
                }
                let content_length = content_length.ok_or(BrokerError::Response)?;
                target_len = Some(split + 4 + content_length);
                if target_len.unwrap_or(usize::MAX) as u64 > MAX_RESPONSE {
                    return Err(BrokerError::Response);
                }
            } else if data.len() > 8192 { return Err(BrokerError::Response); }
        }
        if let Some(expected) = target_len {
            if data.len() == expected { break; }
            if data.len() > expected { return Err(BrokerError::Response); }
        }
    }
    if data.len() != target_len.ok_or(BrokerError::Response)? {
        return Err(BrokerError::Response);
    }
    let split = data.windows(4).position(|w| w == b"\r\n\r\n").ok_or(BrokerError::Response)?;
    if split > 8192 { return Err(BrokerError::Response); }
    let head = std::str::from_utf8(&data[..split]).map_err(|_| BrokerError::Response)?;
    let mut lines = head.split("\r\n");
    let status_line = lines.next().ok_or(BrokerError::Response)?;
    let mut status_parts = status_line.split(' ');
    if status_parts.next() != Some("HTTP/1.1") { return Err(BrokerError::Response); }
    let status: u16 = status_parts.next().ok_or(BrokerError::Response)?.parse().map_err(|_| BrokerError::Response)?;
    if !(200..300).contains(&status) { return Err(BrokerError::Response); }
    let mut length: Option<usize> = None;
    for line in lines {
        let (name,value) = line.split_once(':').ok_or(BrokerError::Response)?;
        if name.eq_ignore_ascii_case("transfer-encoding") { return Err(BrokerError::Response); }
        if name.eq_ignore_ascii_case("content-length") {
            if length.is_some() { return Err(BrokerError::Response); }
            length = Some(value.trim().parse().map_err(|_| BrokerError::Response)?);
        }
    }
    let body = data[split + 4..].to_vec();
    if length != Some(body.len()) || body.windows(secret.len()).any(|w| w == secret) { return Err(BrokerError::Response); }
    Ok(BrokerResponse {status,body})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    fn route(ip: Ipv4Addr) -> BrokerRoute {
        BrokerRoute { route_id:"provider-a".into(), tool_id:"tool-a".into(),
            hostname:"api.example.com".into(),pinned_ip:ip,port:443,path:"/v1/execute".into(),
            resource:"https://omat.example".into(),required_scopes:["tool:read".into()].into_iter().collect(),min_assurance:1 }
    }
    #[test]
    fn rejects_ssrf_addresses_and_untrusted_routes() {
        for ip in [Ipv4Addr::new(127,0,0,1),Ipv4Addr::new(169,254,169,254),
                   Ipv4Addr::new(10,0,0,1),Ipv4Addr::new(192,168,1,1),
                   Ipv4Addr::new(172,16,0,1),Ipv4Addr::new(100,100,1,1),
                   Ipv4Addr::new(203,0,113,10),Ipv4Addr::new(198,51,100,1)] {
            assert_eq!(route(ip).validate(),Err(BrokerError::Configuration));
        }
        assert!(route(Ipv4Addr::new(8,8,8,8)).validate().is_ok());
        let mut poisoned=route(Ipv4Addr::new(8,8,8,8));poisoned.path="//169.254.169.254/metadata".into();
        assert_eq!(poisoned.validate(),Err(BrokerError::Configuration));
        poisoned.path="/safe\r\nHost: evil".into();
        assert_eq!(poisoned.validate(),Err(BrokerError::Configuration));
    }
    #[test]
    fn encryption_binds_tenant_user_and_route() {
        let dir=tempfile::tempdir().unwrap();let db=dir.path().join("vault.sqlite");Connection::open(&db).unwrap();
        #[cfg(unix)]
        std::fs::set_permissions(&db, std::fs::Permissions::from_mode(0o600)).unwrap();
        let mut broker=SecretBroker::open(&db,[42u8;32],vec![route(Ipv4Addr::new(8,8,8,8))]).unwrap();
        broker.provision_secret("tenant-a","alice","provider-a",b"token-alpha").unwrap();
        let conn=broker.connection().unwrap();
        let (nonce,cipher,tag):(Vec<u8>,Vec<u8>,Vec<u8>)=conn.query_row(
            "SELECT nonce,ciphertext,tag FROM oauth_broker_secrets",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();
        assert!(!cipher.windows(11).any(|w|w==b"token-alpha"));
        assert!(decrypt_aead(Cipher::aes_256_gcm(),&[42u8;32],Some(&nonce),
            &binding("tenant-b","alice","provider-a"),&cipher,&tag).is_err());
        assert!(decrypt_aead(Cipher::aes_256_gcm(),&[42u8;32],Some(&nonce),
            &binding("tenant-a","alice","provider-a"),&cipher,&tag).is_ok());
        broker.rotate_master_key([43u8;32]).unwrap();
        let (nonce2,cipher2,tag2):(Vec<u8>,Vec<u8>,Vec<u8>)=conn.query_row(
            "SELECT nonce,ciphertext,tag FROM oauth_broker_secrets",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();
        assert!(decrypt_aead(Cipher::aes_256_gcm(),&[42u8;32],Some(&nonce2),
            &binding("tenant-a","alice","provider-a"),&cipher2,&tag2).is_err());
        assert_eq!(decrypt_aead(Cipher::aes_256_gcm(),&[43u8;32],Some(&nonce2),
            &binding("tenant-a","alice","provider-a"),&cipher2,&tag2).unwrap(),b"token-alpha");
        broker.revoke_secret("tenant-a","alice","provider-a").unwrap();
        assert_eq!(conn.query_row("SELECT count(*) FROM oauth_broker_secrets",[],|r|r.get::<_,i64>(0)).unwrap(),0);
    }
}
