#![cfg(unix)]
// Real mTLS -> Python supervisor/ingress -> compiled Rust bridge -> OAuth/SQLite.
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use ocaf_runtime::stage044_oauth::{
    AuthenticatedSubject, AuthorizationRequest, CodeExchange, OAuthClient, OAuthService,
};
use openssl::ssl::{SslConnector, SslFiletype, SslMethod};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn run(dir: &Path, args: &[&str]) {
    let out = Command::new("openssl")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "PKI fixture creation failed");
}
fn private_file(path: &Path, data: &[u8]) {
    fs::write(path, data).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}
fn pki(dir: &Path) {
    run(
        dir,
        &[
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-nodes",
            "-keyout",
            "ca.key",
            "-out",
            "ca.crt",
            "-subj",
            "/CN=OCAF E2E CA",
            "-days",
            "1",
            "-addext",
            "basicConstraints=critical,CA:TRUE",
            "-addext",
            "keyUsage=critical,keyCertSign,cRLSign",
        ],
    );
    for name in ["server", "alice", "bob", "unknown"] {
        let key = format!("{name}.key");
        let csr = format!("{name}.csr");
        let crt = format!("{name}.crt");
        let cn = format!("/CN={name}");
        run(
            dir,
            &[
                "req", "-newkey", "rsa:2048", "-nodes", "-keyout", &key, "-out", &csr, "-subj", &cn,
            ],
        );
        fs::set_permissions(dir.join(&key), fs::Permissions::from_mode(0o600)).unwrap();
        let extensions = format!("basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\nauthorityKeyIdentifier=keyid\n{}", if name=="server" {"subjectAltName=IP:127.0.0.1\nextendedKeyUsage=serverAuth\n"} else {"extendedKeyUsage=clientAuth\n"});
        fs::write(dir.join("ext"), extensions).unwrap();
        run(
            dir,
            &[
                "x509",
                "-req",
                "-in",
                &csr,
                "-CA",
                "ca.crt",
                "-CAkey",
                "ca.key",
                "-CAcreateserial",
                "-out",
                &crt,
                "-days",
                "1",
                "-extfile",
                "ext",
            ],
        );
    }
}
fn scope(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|x| (*x).to_owned()).collect()
}
fn token(oauth: &OAuthService, origin: &str, tenant: &str, user: &str, scopes: &[&str]) -> String {
    let verifier = "v".repeat(64);
    let client_id = format!("builder-{tenant}");
    let subject = AuthenticatedSubject {
        tenant: tenant.into(),
        user: user.into(),
        authenticated_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
        assurance_level: 1,
    };
    let grant = oauth
        .authorize(
            &OAuthService::user_endpoint(tenant, user).unwrap(),
            &subject,
            true,
            &AuthorizationRequest {
                response_type: "code".into(),
                client_id: client_id.clone(),
                redirect_uri: "https://client.example/cb".into(),
                resource: origin.into(),
                scope: scope(scopes),
                state: "s".repeat(32),
                code_challenge: URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())),
                code_challenge_method: "S256".into(),
            },
        )
        .unwrap();
    oauth
        .exchange(&CodeExchange {
            client_id,
            code: grant.code,
            code_verifier: verifier,
            resource: origin.into(),
        })
        .unwrap()
        .access_token
}
fn fingerprint(path: &Path) -> String {
    let cert = openssl::x509::X509::from_pem(&fs::read(path).unwrap()).unwrap();
    Sha256::digest(cert.to_der().unwrap())
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}
struct Server(Child);
impl Server {
    fn stop(&mut self) -> bool {
        if self.0.try_wait().unwrap().is_some() {
            return false;
        }
        unsafe {
            libc::kill(self.0.id() as i32, libc::SIGTERM);
        }
        let deadline = Instant::now() + Duration::from_secs(25);
        while Instant::now() < deadline {
            if let Some(status) = self.0.try_wait().unwrap() {
                return status.success();
            }
            thread::sleep(Duration::from_millis(30));
        }
        let _ = self.0.kill();
        let _ = self.0.wait();
        false
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop();
    }
}
struct Lab {
    root: PathBuf,
    origin: String,
    port: u16,
}
impl Lab {
    fn start(&self) -> Server {
        let base = Path::new(env!("CARGO_MANIFEST_DIR"));
        let child = Command::new("python3")
            .current_dir(base)
            .args([
                "-m",
                "scripts.builder_cloud_supervisor",
                "--bridge-bin",
                env!("CARGO_BIN_EXE_ocaf-builder-cloud-bridge"),
                "--bridge-config",
            ])
            .arg(self.root.join("bridge.json"))
            .args([
                "--origin",
                &self.origin,
                "--port",
                &self.port.to_string(),
                "--cert",
            ])
            .arg(self.root.join("server.crt"))
            .arg("--key")
            .arg(self.root.join("server.key"))
            .arg("--ca")
            .arg(self.root.join("ca.crt"))
            .arg("--registry")
            .arg(self.root.join("identities.json"))
            .arg("--assets")
            .arg(base.join("apps/agents-builder-web"))
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let mut server = Server(child);
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            assert!(
                server.0.try_wait().unwrap().is_none(),
                "supervisor exited before readiness"
            );
            if TcpStream::connect(("127.0.0.1", self.port)).is_ok() {
                return server;
            }
            assert!(
                Instant::now() < deadline,
                "HTTPS listener startup timed out"
            );
            thread::sleep(Duration::from_millis(50));
        }
    }
    fn request(
        &self,
        cert: Option<&str>,
        method: &str,
        path: &str,
        token: &str,
        body: &[u8],
    ) -> Result<(u16, Vec<u8>), Box<dyn std::error::Error>> {
        let mut cfg = SslConnector::builder(SslMethod::tls_client())?;
        cfg.set_ca_file(self.root.join("ca.crt"))?;
        if let Some(name) = cert {
            cfg.set_certificate_file(self.root.join(format!("{name}.crt")), SslFiletype::PEM)?;
            cfg.set_private_key_file(self.root.join(format!("{name}.key")), SslFiletype::PEM)?;
            cfg.check_private_key()?;
        }
        let socket = TcpStream::connect_timeout(
            &format!("127.0.0.1:{}", self.port).parse()?,
            Duration::from_secs(3),
        )?;
        socket.set_read_timeout(Some(Duration::from_secs(12)))?;
        socket.set_write_timeout(Some(Duration::from_secs(12)))?;
        let mut tls = cfg.build().connect("127.0.0.1", socket)?;
        write!(tls,"{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAuthorization: Bearer {token}\r\nOrigin: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",self.port,self.origin,body.len())?;
        tls.write_all(body)?;
        let mut raw = Vec::new();
        let mut buf = [0; 4096];
        loop {
            let n = tls.read(&mut buf)?;
            if n == 0 {
                break;
            }
            raw.extend_from_slice(&buf[..n]);
            if raw.len() > 2 * 1024 * 1024 {
                return Err("response too large".into());
            }
            if let Some(i) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
                let head = std::str::from_utf8(&raw[..i])?;
                if let Some(len) = head
                    .lines()
                    .find_map(|line| line.strip_prefix("Content-Length: "))
                    .and_then(|v| v.parse::<usize>().ok())
                {
                    if raw.len() == i + 4 + len {
                        break;
                    }
                    if raw.len() > i + 4 + len {
                        return Err("overlong HTTP response".into());
                    }
                }
            }
        }
        let i = raw
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .ok_or("missing HTTP header")?;
        let header = std::str::from_utf8(&raw[..i])?;
        let status = header
            .split_whitespace()
            .nth(1)
            .ok_or("missing HTTP status")?
            .parse::<u16>()?;
        let len = header
            .lines()
            .find_map(|line| line.strip_prefix("Content-Length: "))
            .ok_or("missing length")?
            .parse::<usize>()?;
        if raw.len() != i + 4 + len {
            return Err("truncated HTTP response".into());
        }
        Ok((status, raw[i + 4..].to_vec()))
    }
}
#[test]
fn mtls_oauth_sqlite_cas_and_restart_full_stack() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    pki(&root);
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let lab = Lab {
        root: root.clone(),
        origin: format!("https://127.0.0.1:{port}"),
        port,
    };
    let oauth_db = root.join("oauth.sqlite");
    let project_db = root.join("projects.sqlite");
    for f in [&oauth_db, &project_db] {
        private_file(f, b"");
    }
    let clients: Vec<OAuthClient> = ["local", "another"]
        .into_iter()
        .map(|tenant| OAuthClient {
            client_id: format!("builder-{tenant}"),
            tenant: tenant.into(),
            redirect_uri: "https://client.example/cb".into(),
            resource: lab.origin.clone(),
            scopes: scope(&["builder:read", "builder:write"]),
            step_up_scopes: scope(&[]),
        })
        .collect();
    let registration:Vec<Value>=clients.iter().map(|c|json!({"client_id":c.client_id,"tenant":c.tenant,"redirect_uri":c.redirect_uri,"resource":c.resource,"scopes":c.scopes,"step_up_scopes":c.step_up_scopes})).collect();
    let oauth = OAuthService::open(&oauth_db, clients).unwrap();
    let alice = token(
        &oauth,
        &lab.origin,
        "local",
        "alice",
        &["builder:read", "builder:write"],
    );
    let bob = token(
        &oauth,
        &lab.origin,
        "another",
        "bob",
        &["builder:read", "builder:write"],
    );
    let reader = token(&oauth, &lab.origin, "local", "alice", &["builder:read"]);
    private_file(&root.join("bridge.json"),&serde_json::to_vec(&json!({"origin":lab.origin,"audience":lab.origin,"socket":root.join("bridge.sock"),"oauth_database":oauth_db,"builder_database":project_db,"clients":registration})).unwrap());
    let identities: BTreeMap<String, Value> = [("alice", "local"), ("bob", "another")]
        .into_iter()
        .map(|(user, tenant)| {
            (
                fingerprint(&root.join(format!("{user}.crt"))),
                json!({"tenant":tenant,"user":user}),
            )
        })
        .collect();
    private_file(
        &root.join("identities.json"),
        &serde_json::to_vec(&identities).unwrap(),
    );
    let mut server = lab.start();
    let get = |cert: &str, token: &str| {
        lab.request(Some(cert), "GET", "/v1/projects/hello", token, b"")
            .unwrap()
    };
    let send = |cert: &str, method: &str, path: &str, token: &str, value: &Value| {
        lab.request(
            Some(cert),
            method,
            path,
            token,
            &serde_json::to_vec(value).unwrap(),
        )
        .unwrap()
    };
    let project: Value = serde_json::from_str(include_str!(
        "../apps/agents-builder-web/fixtures/hello.json"
    ))
    .unwrap();
    assert_eq!(
        lab.request(Some("alice"), "GET", "/", &alice, b"")
            .unwrap()
            .0,
        200
    );
    assert_eq!(
        send("alice", "POST", "/v1/projects/preview", &alice, &project).0,
        200
    );
    assert_eq!(get("alice", &alice).0, 404);
    let save = json!({"project":project,"expected":null});
    assert_eq!(
        send("alice", "PUT", "/v1/projects/hello", &reader, &save).0,
        403
    );
    let (status, body) = send("alice", "PUT", "/v1/projects/hello", &alice, &save);
    assert_eq!(status, 200);
    let first: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(first["revision"], 1);
    assert_eq!(
        send("alice", "PUT", "/v1/projects/hello", &alice, &save).0,
        409
    );
    assert_eq!(get("bob", &alice).0, 403);
    assert_eq!(get("alice", &bob).0, 403);
    assert_eq!(get("bob", &bob).0, 404);
    assert_eq!(get("alice", "invalid").0, 403);
    assert_eq!(get("unknown", &alice).0, 403);
    assert!(lab
        .request(None, "GET", "/v1/projects/hello", &alice, b"")
        .is_err());
    let mut next = project.clone();
    next["revision"] = 2.into();
    let second = json!({"project":next,"expected":first});
    assert_eq!(
        send("alice", "PUT", "/v1/projects/hello", &alice, &second).0,
        200
    );
    let (status, body) = get("alice", &reader);
    assert_eq!(status, 200);
    assert_eq!(
        serde_json::from_slice::<Value>(&body).unwrap()["head"]["revision"],
        2
    );
    let (status, body) = lab
        .request(
            Some("alice"),
            "GET",
            "/v1/projects/hello/history",
            &reader,
            b"",
        )
        .unwrap();
    assert_eq!(status, 200);
    assert_eq!(
        serde_json::from_slice::<Vec<Value>>(&body).unwrap().len(),
        2
    );
    // Opt-in real GUI run against this ephemeral integration-test PKI and OAuth fixture.
    if let Some(binary) = std::env::var_os("OCAF_LINUX_GUI_BINARY") {
        let driver = std::env::var_os("OCAF_LINUX_GUI_DRIVER").expect("missing GUI driver");
        let status = Command::new("python3")
            .arg("scripts/verify_builder_linux_cloud_gui.py")
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .env("OCAF_LINUX_GUI_BINARY", binary)
            .env("OCAF_LINUX_GUI_DRIVER", driver)
            .env("OCAF_TEST_GUI_TOKEN", &alice)
            .env("OCAF_TEST_GUI_FOREIGN_TOKEN", &bob)
            .env("OCAF_BUILDER_CLOUD_ORIGIN", &lab.origin)
            .env("OCAF_BUILDER_CLOUD_CA", root.join("ca.crt"))
            .env("OCAF_BUILDER_CLOUD_CERT", root.join("alice.crt"))
            .env("OCAF_BUILDER_CLOUD_KEY", root.join("alice.key"))
            .status()
            .unwrap();
        assert!(status.success(), "native Linux Cloud GUI suite failed");
    }
    assert!(server.stop());
    assert!(!root.join("bridge.sock").exists());
    let mut restarted = lab.start();
    let (status, body) = get("alice", &reader);
    assert_eq!(status, 200);
    assert_eq!(
        serde_json::from_slice::<Value>(&body).unwrap()["head"]["revision"],
        2
    );
    assert!(restarted.stop());
    assert!(!root.join("bridge.sock").exists());
}
