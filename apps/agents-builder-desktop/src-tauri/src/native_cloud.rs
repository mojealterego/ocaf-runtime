//! Linux-only, operator-configured mTLS transport. No caller-supplied URLs or keys.
use reqwest::{blocking::Client, Certificate, Identity, Method, Url};
use serde::Serialize;
use std::{
    env,
    fs::{self, OpenOptions},
    io::Read,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
    sync::Mutex,
    time::Duration,
};
use tauri::{Manager, WebviewWindow};

const LIMIT: usize = 128 * 1024;
const KEY_LIMIT: usize = 64 * 1024;

#[derive(Default)]
pub struct NativeCloud {
    active: Mutex<()>,
}

#[derive(Serialize)]
pub struct Receipt {
    status: u16,
    body: String,
}

pub(crate) fn identifier(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s != "."
        && s != ".."
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
}

fn route(method: &str, path: &str, body: Option<&str>) -> bool {
    if body.is_some_and(|b| b.len() > LIMIT || !b.starts_with('{')) {
        return false;
    }
    match (method, path.strip_prefix("/v1/projects/"), body) {
        ("GET", Some(id), None) => identifier(id),
        ("PUT", Some(id), Some(_)) => identifier(id) && id != "preview",
        ("POST", Some("preview"), Some(_)) => true,
        _ => false,
    }
}

fn origin(value: &str) -> Result<Url, &'static str> {
    let url = Url::parse(value).map_err(|_| "Invalid Cloud origin")?;
    if url.scheme() != "https"
        || url.host().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.origin().ascii_serialization() != value
        || url.port() == Some(0)
    {
        return Err("Cloud origin must be a canonical HTTPS authority");
    }
    Ok(url)
}

// Operator-controlled paths, not supplied by the WebView. Private keys must
// not be readable or replaceable by an untrusted local user.
fn credential(variable: &str, private: bool) -> Result<Vec<u8>, &'static str> {
    let name = env::var_os(variable).ok_or("Cloud credential not configured")?;
    let path = Path::new(&name);
    if !path.is_absolute() || path.canonicalize().ok().as_deref() != Some(path) {
        return Err("Cloud credential path must be canonical and absolute");
    }
    let parent = fs::metadata(path.parent().ok_or("Missing credential directory")?)
        .map_err(|_| "Invalid credential directory")?;
    let uid = unsafe { libc::geteuid() };
    if private && (parent.uid() != uid || parent.mode() & 0o077 != 0) {
        return Err("Cloud key directory must be private");
    }
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|_| "Cloud credential unavailable")?;
    let metadata = file
        .metadata()
        .map_err(|_| "Cloud credential unavailable")?;
    if !metadata.is_file()
        || metadata.len() == 0
        || metadata.len() > KEY_LIMIT as u64
        || metadata.mode() & 0o022 != 0
        || (private
            && (metadata.uid() != uid || metadata.mode() & 0o077 != 0 || metadata.nlink() != 1))
    {
        return Err("Unsafe Cloud credential file");
    }
    let mut value = Vec::new();
    file.take((KEY_LIMIT + 1) as u64)
        .read_to_end(&mut value)
        .map_err(|_| "Cloud credential unreadable")?;
    if value.len() > KEY_LIMIT || value.is_empty() {
        return Err("Invalid Cloud credential size");
    }
    Ok(value)
}

fn request(
    method: &str,
    path: &str,
    token: &str,
    body: Option<&str>,
) -> Result<Receipt, &'static str> {
    if !route(method, path, body)
        || token.is_empty()
        || token.len() > 4096
        || !token.bytes().all(|b| (33..=126).contains(&b))
    {
        return Err("Cloud request denied");
    }
    let configured =
        env::var("OCAF_BUILDER_CLOUD_ORIGIN").map_err(|_| "Cloud gateway not configured")?;
    let base = origin(&configured)?;
    let ca = Certificate::from_pem(&credential("OCAF_BUILDER_CLOUD_CA", false)?)
        .map_err(|_| "Invalid Cloud CA")?;
    let cert = credential("OCAF_BUILDER_CLOUD_CERT", false)?;
    let mut key = credential("OCAF_BUILDER_CLOUD_KEY", true)?;
    let identity = Identity::from_pkcs8_pem(&cert, &key);
    key.fill(0);
    let identity = identity.map_err(|_| "Invalid Cloud client identity")?;
    let client = Client::builder()
        .use_native_tls()
        .tls_built_in_root_certs(false)
        .add_root_certificate(ca)
        .identity(identity)
        .https_only(true)
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .min_tls_version(reqwest::tls::Version::TLS_1_2)
        .connect_timeout(Duration::from_secs(3))
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|_| "Cloud TLS configuration failed")?;
    let url = base.join(path).map_err(|_| "Cloud path denied")?;
    let mut req = client
        .request(
            Method::from_bytes(method.as_bytes()).map_err(|_| "Method denied")?,
            url,
        )
        .bearer_auth(token)
        .header("Origin", configured)
        .header("Accept", "application/json")
        .header("Cache-Control", "no-store");
    if let Some(body) = body {
        req = req
            .header("Content-Type", "application/json")
            .body(body.to_owned());
    }
    let response = req
        .send()
        .map_err(|_| "Cloud TLS/transport failure; reopen project before retrying")?;
    let status = response.status();
    if status.is_redirection() {
        return Err("Cloud redirect denied");
    }
    if response.content_length().is_some_and(|n| n > LIMIT as u64) {
        return Err("Cloud response limit");
    }
    if !status.is_success() {
        return Ok(Receipt {
            status: status.as_u16(),
            body: String::new(),
        });
    }
    let json = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| {
            v.split(';')
                .next()
                .is_some_and(|t| t.trim().eq_ignore_ascii_case("application/json"))
        });
    if !json {
        return Err("Cloud response type denied");
    }
    let mut bytes = Vec::new();
    response
        .take((LIMIT + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| "Cloud response incomplete")?;
    if bytes.len() > LIMIT {
        return Err("Cloud response limit");
    }
    Ok(Receipt {
        status: status.as_u16(),
        body: String::from_utf8(bytes).map_err(|_| "Cloud response must be UTF-8")?,
    })
}

pub(crate) fn require_caller(window: &WebviewWindow) -> Result<(), String> {
    let url = window
        .url()
        .map_err(|_| "Cloud caller denied".to_string())?;
    if window.label() != "main" || url.scheme() != "tauri" || url.host_str() != Some("localhost") {
        return Err("Cloud caller denied".into());
    }
    Ok(())
}

#[tauri::command]
pub async fn native_cloud_request(
    window: WebviewWindow,
    method: String,
    path: String,
    token: String,
    body: Option<String>,
) -> Result<Receipt, String> {
    require_caller(&window)?;
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let cloud = app.state::<NativeCloud>();
        let _lock = cloud
            .active
            .try_lock()
            .map_err(|_| "Cloud request in progress".to_string())?;
        request(&method, &path, &token, body.as_deref()).map_err(str::to_owned)
    })
    .await
    .map_err(|_| "Cloud worker failed".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_routes_only() {
        for path in [
            "/v1/projects/../x",
            "/v1/projects/%2e%2e",
            "//evil.test",
            "/v1/projects/a?x=1",
            "/v1/projects/a/history",
        ] {
            assert!(!route("GET", path, None));
        }
        assert!(route("GET", "/v1/projects/a", None));
        assert!(route("PUT", "/v1/projects/a", Some("{}")));
        assert!(route("POST", "/v1/projects/preview", Some("{}")));
        assert!(!route("DELETE", "/v1/projects/a", None));
        assert!(!route("POST", "/v1/projects/a", Some("{}")));
        assert!(!route("PUT", "/v1/projects/preview", Some("{}")));
        assert!(!route("GET", "/v1/projects/a", Some("{}")));
        assert!(!route(
            "PUT",
            "/v1/projects/a",
            Some(&"x".repeat(LIMIT + 1))
        ));
    }
    #[test]
    fn canonical_https_authorities_only() {
        for value in [
            "http://127.0.0.1:80",
            "https://example.org/",
            "https://x@y",
            "https://example.org?x=1",
            "https://example.org:0",
            "https://example.org:443",
        ] {
            assert!(origin(value).is_err(), "{value}");
        }
        assert!(origin("https://127.0.0.1:8443").is_ok());
        assert!(origin("https://builder.example.org").is_ok());
    }
}
