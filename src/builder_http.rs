//! Stage 0.52: fail-closed, transport-neutral HTTP request boundary for Builder Cloud.
//!
//! A separately authenticated TLS gateway MUST supply AuthenticatedSubject.
//! This module does not trust headers for identity and does not open a socket,
//! terminate TLS, set cookies, or grant deployment permissions. The gateway
//! must enforce request framing, HTTP parser limits, TLS and connection limits.
use crate::builder_cloud::{BuilderCloudError, BuilderCloudService, READ_SCOPE, WRITE_SCOPE};
use crate::builder_core::{from_dsl, BuilderError, BuilderProject};
use crate::builder_store::{BuilderHead, BuilderStoreError, SqliteBuilderStore};
use crate::stage044_oauth::{AuthenticatedSubject, OAuthService};
use serde::Deserialize;
use serde_json::{json, Value};

const MAX_BODY_BYTES: usize = 128 * 1024;
const MAX_REQUEST_TARGET_BYTES: usize = 2048;
const MAX_HEADERS: usize = 32;
const MAX_TOTAL_HEADER_BYTES: usize = 16 * 1024;
const MAX_HEADER_VALUE_BYTES: usize = 4096;

pub struct BuilderHttpRequest<'a> {
    pub method: &'a str,
    pub path: &'a str,
    pub headers: &'a [(&'a str, &'a str)],
    pub body: &'a [u8],
}

#[derive(Debug, PartialEq, Eq)]
pub struct BuilderHttpResponse {
    pub status: u16,
    pub body: Vec<u8>,
}

pub struct BuilderHttpBoundary<'a> {
    service: BuilderCloudService<'a>,
    allowed_origin: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SaveEnvelope {
    project: BuilderProject,
    expected: Option<ExpectedHead>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedHead {
    revision: u64,
    sha256: String,
}

fn json_response(status: u16, value: Value) -> BuilderHttpResponse {
    BuilderHttpResponse {
        status,
        body: serde_json::to_vec(&value)
            .unwrap_or_else(|_| b"{\"error\":\"serialization\"}".to_vec()),
    }
}

fn reject(status: u16) -> BuilderHttpResponse {
    json_response(status, json!({ "error": format!("http_{status}") }))
}

fn valid_project_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value != "."
        && value != ".."
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
}

fn unique_header<'a>(headers: &'a [(&str, &'a str)], name: &str) -> Result<Option<&'a str>, ()> {
    let mut found = None;
    for (key, value) in headers {
        if key.eq_ignore_ascii_case(name) {
            if found.is_some() {
                return Err(());
            }
            found = Some(*value);
        }
    }
    Ok(found)
}

fn error_status(error: &BuilderCloudError) -> u16 {
    match error {
        BuilderCloudError::Configuration => 503,
        BuilderCloudError::OAuth(_) => 403,
        BuilderCloudError::Store(BuilderStoreError::Denied) => 403,
        BuilderCloudError::Store(BuilderStoreError::Invalid) => 400,
        BuilderCloudError::Store(BuilderStoreError::Conflict) => 409,
        BuilderCloudError::Store(BuilderStoreError::Integrity) => 409,
        BuilderCloudError::Store(BuilderStoreError::Limit) => 429,
        BuilderCloudError::Store(BuilderStoreError::Database) => 503,
        BuilderCloudError::Store(BuilderStoreError::Builder(_)) => 422,
        BuilderCloudError::Builder(_) => 422,
    }
}

impl<'a> BuilderHttpBoundary<'a> {
    pub fn new(
        oauth: &'a OAuthService,
        store: &'a SqliteBuilderStore,
        audience: &str,
        allowed_origin: &str,
    ) -> Result<Self, BuilderCloudError> {
        // A configured Origin must be a single HTTPS authority, not merely a
        // string beginning with "https://". Reject ambiguous hosts and ports.
        let authority = allowed_origin
            .strip_prefix("https://")
            .ok_or(BuilderCloudError::Configuration)?;
        if authority.is_empty()
            || authority
                .bytes()
                .any(|b| b <= 32 || b >= 127 || matches!(b, b'/' | b'?' | b'#' | b'@' | b'\\'))
        {
            return Err(BuilderCloudError::Configuration);
        }
        let (host, port) = if let Some(bracketed) = authority.strip_prefix('[') {
            let (address, suffix) = bracketed
                .split_once(']')
                .ok_or(BuilderCloudError::Configuration)?;
            address
                .parse::<std::net::Ipv6Addr>()
                .map_err(|_| BuilderCloudError::Configuration)?;
            let port = if suffix.is_empty() {
                None
            } else {
                Some(
                    suffix
                        .strip_prefix(':')
                        .ok_or(BuilderCloudError::Configuration)?,
                )
            };
            ("[ipv6]", port)
        } else {
            match authority.split_once(':') {
                Some((host, port)) => (host, Some(port)),
                None => (authority, None),
            }
        };
        if host != "[ipv6]"
            && (host.len() > 253
                || host.split('.').any(|label| {
                    label.is_empty()
                        || label.len() > 63
                        || label.starts_with('-')
                        || label.ends_with('-')
                        || !label.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
                }))
        {
            return Err(BuilderCloudError::Configuration);
        }
        if let Some(port) = port {
            if port.is_empty()
                || !port.bytes().all(|b| b.is_ascii_digit())
                || port.parse::<u16>().ok().filter(|value| *value != 0).is_none()
            {
                return Err(BuilderCloudError::Configuration);
            }
        }
        Ok(Self {
            service: BuilderCloudService::new(oauth, store, audience)?,
            allowed_origin: allowed_origin.to_owned(),
        })
    }

    /// Handle a parsed request only after a trusted TLS gateway has authenticated
    /// the subject. Never construct subject from Origin, Authorization, or JSON.
    pub fn handle(
        &self,
        request: BuilderHttpRequest<'_>,
        trusted_subject: &AuthenticatedSubject,
    ) -> BuilderHttpResponse {
        // Defense in depth: a parsed request target must be bounded even when
        // the upstream HTTP gateway is responsible for parser limits.
        if request.path.len() > MAX_REQUEST_TARGET_BYTES {
            return reject(414);
        }
        if request.body.len() > MAX_BODY_BYTES {
            return reject(413);
        }
        if request.headers.len() > MAX_HEADERS
            || request.headers.iter().any(|(name, value)| {
                name.is_empty()
                    || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
                    || value.len() > MAX_HEADER_VALUE_BYTES
                    || value.bytes().any(|b| !(32..=126).contains(&b))
            })
        {
            return reject(400);
        }
        // Bound aggregate header storage independently of the per-field cap.
        // Saturating arithmetic keeps the admission decision fail-closed.
        let header_bytes = request.headers.iter().fold(0_usize, |total, (name, value)| {
            total.saturating_add(name.len()).saturating_add(value.len())
        });
        if header_bytes > MAX_TOTAL_HEADER_BYTES {
            return reject(431);
        }
        // The gateway owns HTTP framing. Reject transfer codings here rather than
        // accepting a potentially inconsistent representation of the body.
        if request
            .headers
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case("Transfer-Encoding"))
        {
            return reject(400);
        }
        // Content-Length is a singleton. Never accept conflicting, non-decimal,
        // or inaccurate lengths from a downstream HTTP parser or proxy.
        match unique_header(request.headers, "Content-Length") {
            Err(()) => return reject(400),
            Ok(Some(value)) => {
                if value.is_empty()
                    || !value.bytes().all(|byte| byte.is_ascii_digit())
                    || value.parse::<usize>().ok() != Some(request.body.len())
                {
                    return reject(400);
                }
            }
            Ok(None) => {}
        }
        // Duplicate singleton headers are malformed, even when their values agree.
        // Check before routing so malformed Content-Type cannot be mistaken for 415.
        for singleton in ["Origin", "Authorization", "Content-Type"] {
            if unique_header(request.headers, singleton).is_err() {
                return reject(400);
            }
        }
        if request
            .headers
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case("cookie"))
        {
            return reject(403);
        }
        // Browsers omit Origin on same-origin GET. Bearer tokens and the trusted
        // gateway subject remain mandatory; mutating requests require exact Origin.
        // Any supplied cross-origin value is rejected even for GET.
        match unique_header(request.headers, "Origin") {
            Ok(Some(value)) if value == self.allowed_origin => {}
            Ok(None) if request.method == "GET" => {}
            Ok(None | Some(_)) => return reject(403),
            Err(()) => return reject(400),
        }
        let authorization = match unique_header(request.headers, "Authorization") {
            Ok(Some(value)) => value,
            Ok(None) => return reject(403),
            Err(()) => return reject(400),
        };
        let token = match authorization.strip_prefix("Bearer ") {
            Some(value)
                if !value.is_empty()
                    && value.len() <= MAX_HEADER_VALUE_BYTES
                    && value.bytes().all(|b| b.is_ascii_graphic()) =>
            {
                value
            }
            _ => return reject(403),
        };

        if request.path == "/v1/projects/preview" {
            if request.method != "POST" {
                return reject(405);
            }
            if !Self::json_content_type(request.headers) {
                return reject(415);
            }
            let source = match std::str::from_utf8(request.body) {
                Ok(source) => source,
                Err(_) => return reject(400),
            };
            if let Err(error) = self.service.authorize(token, trusted_subject, READ_SCOPE) {
                return reject(error_status(&error));
            }
            let project = match from_dsl(source, &trusted_subject.tenant) {
                Ok(project) => project,
                Err(BuilderError::Denied) => return reject(403),
                Err(_) => return reject(422),
            };
            return match self.service.preview(token, trusted_subject, &project) {
                Ok(plan) => json_response(
                    200,
                    json!({
                        "ordered_nodes": plan.ordered_nodes,
                        "requested_tools": plan.requested_tools,
                        "max_cost_minor": plan.max_cost_minor,
                        "project_sha256": plan.project_sha256
                    }),
                ),
                Err(error) => reject(error_status(&error)),
            };
        }

        let Some(rest) = request.path.strip_prefix("/v1/projects/") else {
            return reject(404);
        };
        let (project_id, history) = match rest.strip_suffix("/history") {
            Some(id) => (id, true),
            None => (rest, false),
        };
        if !valid_project_id(project_id) {
            return reject(404);
        }
        if history {
            if request.method != "GET" {
                return reject(405);
            }
            if !request.body.is_empty() {
                return reject(400);
            }
            return match self.service.history(token, trusted_subject, project_id) {
                Ok(heads) => json_response(
                    200,
                    json!(heads
                        .iter()
                        .map(|head| json!({"revision": head.revision, "sha256": head.sha256}))
                        .collect::<Vec<_>>()),
                ),
                Err(error) => reject(error_status(&error)),
            };
        }
        match request.method {
            "GET" => {
                if !request.body.is_empty() {
                    return reject(400);
                }
                match self.service.load(token, trusted_subject, project_id, None) {
                    Ok(Some((project, head))) => json_response(
                        200,
                        json!({
                            "project": project,
                            "head": {"revision": head.revision, "sha256": head.sha256}
                        }),
                    ),
                    Ok(None) => reject(404),
                    Err(error) => reject(error_status(&error)),
                }
            }
            "PUT" => {
                if let Err(error) = self.service.authorize(token, trusted_subject, WRITE_SCOPE) {
                    return reject(error_status(&error));
                }
                if !Self::json_content_type(request.headers) {
                    return reject(415);
                }
                let envelope: SaveEnvelope = match serde_json::from_slice(request.body) {
                    Ok(value) => value,
                    Err(_) => return reject(400),
                };
                if envelope.project.project_id != project_id {
                    return reject(400);
                }
                let expected = match envelope.expected {
                    Some(head) => {
                        if head.revision == 0
                            || head.sha256.len() != 64
                            || !head.sha256.bytes().all(|b| b.is_ascii_hexdigit())
                        {
                            return reject(400);
                        }
                        Some(BuilderHead {
                            revision: head.revision,
                            sha256: head.sha256,
                        })
                    }
                    None => None,
                };
                match self.service.save(
                    token,
                    trusted_subject,
                    &envelope.project,
                    expected.as_ref(),
                ) {
                    Ok(head) => json_response(
                        200,
                        json!({"revision": head.revision, "sha256": head.sha256}),
                    ),
                    Err(error) => reject(error_status(&error)),
                }
            }
            _ => reject(405),
        }
    }

    fn json_content_type(headers: &[(&str, &str)]) -> bool {
        matches!(
            unique_header(headers, "Content-Type"),
            Ok(Some("application/json"))
        )
    }
}
