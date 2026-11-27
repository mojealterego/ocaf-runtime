//! Owner-controlled read-only HTTPS source acquisition for research.
//!
//! A pinned external IPv4 address and validated TLS hostname are supplied by
//! an authenticated operator, never taken from an LLM or research evidence.
//! This is NOT an OMAt executor; OSTc admission and agent delegation remain
//! fail-closed until the distinct protected tool boundary is implemented.
//! Responses are text only, bounded, uncompressed, HTTP/1.1 200 without
//! redirects or transfer encoding. SHA-256 binds the actual returned bytes.

use crate::{architect_research_evidence::SourceRecord, stage044_secret_broker::public_ipv4};
use openssl::ssl::{SslConnector, SslMethod, SslVerifyMode, SslVersion};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream},
    time::{Duration, Instant},
};

const MAX_BODY: usize = 1_048_576;
const MAX_HEADER: usize = 8_192;
const MAX_CHUNK_OVERHEAD: usize = 128 * 1024;
const MAX_CHUNKS: usize = 4096;
const MAX_UNIX: u64 = 253_402_300_799;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinnedSourceRoute {
    pub tenant: String,
    pub hostname: String,
    pub pinned_ipv4: Ipv4Addr,
    pub path: String,
    pub max_body_bytes: usize,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchedSource {
    pub source: SourceRecord,
    pub status: u16,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SourceFetchError {
    #[error("invalid or untrusted source route")]
    InvalidRoute,
    #[error("HTTPS transport or certificate validation failed")]
    Transport,
    #[error("unsupported, malformed or unauthorized HTTP response")]
    Response,
    #[error("response exceeded the permitted text size")]
    TooLarge,
}

fn valid_id(input: &str) -> bool {
    !input.is_empty()
        && input.len() <= 96
        && input
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.:-".contains(&b))
}

impl PinnedSourceRoute {
    /// A pinned IPv4 avoids DNS rebinding. No runtime hostname resolution,
    /// redirects, caller-controlled headers or unapproved authentication.
    pub fn validate(&self) -> Result<(), SourceFetchError> {
        let host = self.hostname.as_bytes();
        let lower_host = self.hostname.to_ascii_lowercase();
        let valid_hostname = host.len() <= 253
            && self.hostname.contains('.')
            && !host.is_empty()
            && host
                .iter()
                .all(|b| b.is_ascii_alphanumeric() || *b == b'-' || *b == b'.')
            && !lower_host.ends_with(".local")
            && !lower_host.ends_with(".localhost")
            && lower_host != "localhost"
            && self.hostname.parse::<IpAddr>().is_err()
            && self.hostname.split('.').all(|part| {
                !part.is_empty()
                    && part.len() <= 63
                    && !part.starts_with('-')
                    && !part.ends_with('-')
            });
        let safe_path = self.path.len() <= 2048
            && self.path.starts_with('/')
            && !self.path.starts_with("//")
            && !self
                .path
                .bytes()
                .any(|b| b <= 32 || b >= 127 || b == b'#' || b == b'\\');
        if !valid_id(&self.tenant)
            || !valid_hostname
            || !safe_path
            || !public_ipv4(self.pinned_ipv4)
            || !(1..=MAX_BODY).contains(&self.max_body_bytes)
            || !(1..=15).contains(&self.timeout_seconds)
        {
            return Err(SourceFetchError::InvalidRoute);
        }
        Ok(())
    }

    pub fn url(&self) -> Result<String, SourceFetchError> {
        self.validate()?;
        Ok(format!("https://{}{}", self.hostname, self.path))
    }
}

/// Strictly decode bounded chunked transfer framing. The final zero chunk
/// must be followed by exactly CRLF; trailers and pipelined data are rejected.
fn decode_chunked(raw: &[u8], cap: usize) -> Result<Vec<u8>, SourceFetchError> {
    let mut cursor = 0_usize;
    let mut decoded = Vec::new();
    for _ in 0..MAX_CHUNKS {
        let remaining = raw.get(cursor..).ok_or(SourceFetchError::Response)?;
        let header_end = remaining
            .windows(2)
            .position(|w| w == b"\r\n")
            .ok_or(SourceFetchError::Response)?;
        if !(1..=16).contains(&header_end) {
            return Err(SourceFetchError::Response);
        }
        let size_text = std::str::from_utf8(&remaining[..header_end])
            .map_err(|_| SourceFetchError::Response)?;
        if !size_text.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(SourceFetchError::Response);
        }
        let size = usize::from_str_radix(size_text, 16).map_err(|_| SourceFetchError::Response)?;
        cursor = cursor
            .checked_add(header_end + 2)
            .ok_or(SourceFetchError::Response)?;
        if size == 0 {
            return if raw.get(cursor..) == Some(b"\r\n".as_slice()) {
                Ok(decoded)
            } else {
                Err(SourceFetchError::Response)
            };
        }
        if size > cap.saturating_sub(decoded.len()) {
            return Err(SourceFetchError::TooLarge);
        }
        let end = cursor.checked_add(size).ok_or(SourceFetchError::Response)?;
        if raw.get(end..end + 2) != Some(b"\r\n".as_slice()) {
            return Err(SourceFetchError::Response);
        }
        decoded.extend_from_slice(raw.get(cursor..end).ok_or(SourceFetchError::Response)?);
        cursor = end + 2;
    }
    Err(SourceFetchError::Response)
}

/// Parse a fully received bounded HTTP/1.1 reply. Source digest always
/// covers the exact retrieved BODY, not an URL, header or guessed page text.
pub fn parse_response(
    raw: &[u8],
    source_url: &str,
    retrieved_unix: u64,
    max_body_bytes: usize,
) -> Result<FetchedSource, SourceFetchError> {
    if retrieved_unix == 0
        || retrieved_unix > MAX_UNIX
        || !(1..=MAX_BODY).contains(&max_body_bytes)
        || !source_url.starts_with("https://")
        || source_url.len() > 2300
    {
        return Err(SourceFetchError::InvalidRoute);
    }
    let split = raw
        .windows(4)
        .position(|s| s == b"\r\n\r\n")
        .ok_or(SourceFetchError::Response)?;
    if split > MAX_HEADER {
        return Err(SourceFetchError::TooLarge);
    }
    let head = std::str::from_utf8(&raw[..split]).map_err(|_| SourceFetchError::Response)?;
    let body = &raw[split + 4..];
    if body.len() > max_body_bytes.saturating_add(MAX_CHUNK_OVERHEAD) {
        return Err(SourceFetchError::TooLarge);
    }
    let mut lines = head.split("\r\n");
    let status = lines.next().ok_or(SourceFetchError::Response)?;
    if !status.starts_with("HTTP/1.1 200 ") {
        return Err(SourceFetchError::Response);
    }
    let mut len: Option<usize> = None;
    let mut is_chunked = false;
    let mut content_type = None;
    for line in lines {
        if line.starts_with(' ') || line.starts_with('\t') {
            return Err(SourceFetchError::Response);
        }
        let (name, value) = line.split_once(':').ok_or(SourceFetchError::Response)?;
        if name.is_empty()
            || !name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
            || value.bytes().any(|b| b < 32 && b != b'\t' || b == 127)
        {
            return Err(SourceFetchError::Response);
        }
        if name.eq_ignore_ascii_case("transfer-encoding") {
            if is_chunked || !value.trim().eq_ignore_ascii_case("chunked") {
                return Err(SourceFetchError::Response);
            }
            is_chunked = true;
        }
        if name.eq_ignore_ascii_case("content-encoding")
            && !value.trim().eq_ignore_ascii_case("identity")
        {
            return Err(SourceFetchError::Response);
        }
        if name.eq_ignore_ascii_case("content-length") {
            if len.is_some() || value.trim().is_empty() {
                return Err(SourceFetchError::Response);
            }
            len = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| SourceFetchError::Response)?,
            );
        }
        if name.eq_ignore_ascii_case("content-type") {
            if content_type.is_some() {
                return Err(SourceFetchError::Response);
            }
            content_type = Some(value.trim().to_ascii_lowercase());
        }
    }
    let decoded = if is_chunked {
        if len.is_some() {
            return Err(SourceFetchError::Response);
        }
        decode_chunked(body, max_body_bytes)?
    } else {
        let size = len.ok_or(SourceFetchError::Response)?;
        if size > max_body_bytes {
            return Err(SourceFetchError::TooLarge);
        }
        if size == 0 || size != body.len() {
            return Err(SourceFetchError::Response);
        }
        body.to_vec()
    };
    if decoded.is_empty() {
        return Err(SourceFetchError::Response);
    }
    if let Some(t) = content_type {
        let media = t.split(';').next().unwrap_or_default().trim();
        if !(media.starts_with("text/")
            || matches!(
                media,
                "application/json" | "application/xml" | "application/xhtml+xml"
            ))
        {
            return Err(SourceFetchError::Response);
        }
    }
    let hash = format!("{:x}", Sha256::digest(&decoded));
    Ok(FetchedSource {
        source: SourceRecord {
            url: source_url.to_owned(),
            retrieved_unix,
            content_sha256: hash,
        },
        status: 200,
        body: decoded,
    })
}

/// Real pinned-IP, certificate-authenticated HTTPS GET. Not callable by
/// untrusted agents: the host must enforce the explicit operator route.
pub fn fetch_https(
    route: &PinnedSourceRoute,
    trusted_tenant: &str,
    retrieved_unix: u64,
) -> Result<FetchedSource, SourceFetchError> {
    route.validate()?;
    if route.tenant != trusted_tenant || retrieved_unix == 0 || retrieved_unix > MAX_UNIX {
        return Err(SourceFetchError::InvalidRoute);
    }
    let address = SocketAddr::new(IpAddr::V4(route.pinned_ipv4), 443);
    let timeout = Duration::from_secs(route.timeout_seconds);
    let start = Instant::now();
    let tcp =
        TcpStream::connect_timeout(&address, timeout).map_err(|_| SourceFetchError::Transport)?;
    tcp.set_read_timeout(Some(timeout))
        .map_err(|_| SourceFetchError::Transport)?;
    tcp.set_write_timeout(Some(timeout))
        .map_err(|_| SourceFetchError::Transport)?;
    let mut builder =
        SslConnector::builder(SslMethod::tls()).map_err(|_| SourceFetchError::Transport)?;
    builder.set_verify(SslVerifyMode::PEER);
    builder
        .set_min_proto_version(Some(SslVersion::TLS1_3))
        .map_err(|_| SourceFetchError::Transport)?;
    builder
        .set_default_verify_paths()
        .map_err(|_| SourceFetchError::Transport)?;
    let mut tls = builder
        .build()
        .connect(&route.hostname, tcp)
        .map_err(|_| SourceFetchError::Transport)?;
    if tls.ssl().verify_result() != openssl::x509::X509VerifyResult::OK {
        return Err(SourceFetchError::Transport);
    }
    let request=format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nAccept: text/*, application/json, application/xml\r\nAccept-Encoding: identity\r\nConnection: close\r\nUser-Agent: OCAF-Research-Source/1\r\n\r\n",
        route.path,route.hostname,
    );
    tls.write_all(request.as_bytes())
        .map_err(|_| SourceFetchError::Transport)?;
    tls.flush().map_err(|_| SourceFetchError::Transport)?;
    let mut response = Vec::new();
    let hard_cap = MAX_HEADER + 4 + route.max_body_bytes + MAX_CHUNK_OVERHEAD;
    let mut expected = None;
    loop {
        if start.elapsed() >= timeout {
            return Err(SourceFetchError::Transport);
        }
        let mut chunk = [0u8; 8192];
        let n = tls
            .read(&mut chunk)
            .map_err(|_| SourceFetchError::Transport)?;
        if n == 0 {
            break;
        }
        if response.len().saturating_add(n) > hard_cap {
            return Err(SourceFetchError::TooLarge);
        }
        response.extend_from_slice(&chunk[..n]);
        if expected.is_none() {
            if let Some(split) = response.windows(4).position(|w| w == b"\r\n\r\n") {
                if split > MAX_HEADER {
                    return Err(SourceFetchError::TooLarge);
                }
                let header = std::str::from_utf8(&response[..split])
                    .map_err(|_| SourceFetchError::Response)?;
                let mut content_length = None;
                for line in header.split("\r\n").skip(1) {
                    let (name, value) = line.split_once(':').ok_or(SourceFetchError::Response)?;
                    if name.eq_ignore_ascii_case("content-length") {
                        if content_length.is_some() {
                            return Err(SourceFetchError::Response);
                        }
                        content_length = Some(
                            value
                                .trim()
                                .parse::<usize>()
                                .map_err(|_| SourceFetchError::Response)?,
                        );
                    }
                }
                if let Some(len) = content_length {
                    if len > route.max_body_bytes {
                        return Err(SourceFetchError::TooLarge);
                    }
                    expected = Some(split + 4 + len);
                }
            } else if response.len() > MAX_HEADER {
                return Err(SourceFetchError::TooLarge);
            }
        }
        if let Some(expected_len) = expected {
            if response.len() >= expected_len {
                break;
            }
        }
    }
    parse_response(
        &response,
        &route.url()?,
        retrieved_unix,
        route.max_body_bytes,
    )
}
