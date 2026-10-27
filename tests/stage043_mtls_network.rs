//! Real localhost TLS handshake regression for the standalone OMAs admission server.
use ocaf_runtime::omas_server::AgentRegistrationRequest;
use ocaf_runtime::omas_signed::SignedOperation;
use openssl::pkey::PKey;
use openssl::sha::sha256;
use openssl::sign::Signer;
use openssl::ssl::{SslConnector, SslFiletype, SslMethod, SslVerifyMode};
use openssl::x509::X509;
use rusqlite::Connection;
use std::io::{BufRead, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn mtls_rejects_anonymous_client_and_accepts_pinned_certificate() {
    let temp = tempfile::tempdir().unwrap();
    let cert = temp.path().join("cert.pem");
    let key = temp.path().join("key.pem");
    let output = Command::new("openssl")
        .args([
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-nodes",
            "-days",
            "1",
            "-subj",
            "/CN=localhost",
            "-keyout",
        ])
        .arg(&key)
        .arg("-out")
        .arg(&cert)
        .output()
        .expect("openssl executable");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let certificate = X509::from_pem(&std::fs::read(&cert).unwrap()).unwrap();
    let fingerprint = sha256(&certificate.to_der().unwrap());
    let hex = |bytes: &[u8]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let signing = PKey::generate_ed25519().unwrap();
    let public = signing.raw_public_key().unwrap();
    let agents = temp.path().join("agents.sqlite");
    let nonces = temp.path().join("nonces.sqlite");
    Connection::open(&agents).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(temp.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::set_permissions(&agents, std::fs::Permissions::from_mode(0o600)).unwrap();
    Connection::open(&nonces).unwrap();
    let reserved = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = reserved.local_addr().unwrap();
    drop(reserved);
    let child = Command::new(env!("CARGO_BIN_EXE_ocaf-omas-mtls-server"))
        .env("OCAF_OMAS_MTLS_BIND", address.to_string())
        .env("OCAF_OMAS_MTLS_CA", &cert)
        .env("OCAF_OMAS_MTLS_CERT", &cert)
        .env("OCAF_OMAS_MTLS_KEY", &key)
        .env("OCAF_OMAS_AGENT_DB", &agents)
        .env("OCAF_OMAS_NONCE_DB", &nonces)
        .env("OCAF_OMAS_PEER_CERT_SHA256", hex(&fingerprint))
        .env("OCAF_OMAS_PEER_ED25519_PUBKEY", hex(&public))
        .env("OCAF_OMAS_PEER_SENDER", "test-node")
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    struct Kill(std::process::Child);
    impl Drop for Kill {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let _guard = Kill(child);
    let mut ready = false;
    for _ in 0..80 {
        if TcpStream::connect(address).is_ok() {
            ready = true;
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
    assert!(ready, "mTLS server failed to bind");
    let mut anonymous = SslConnector::builder(SslMethod::tls()).unwrap();
    anonymous.set_verify(SslVerifyMode::NONE);
    let stream = TcpStream::connect(address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    if let Ok(mut tls) = anonymous.build().connect("localhost", stream) {
        let _ = tls.write_all(b"{}\n");
        let mut buf = [0u8; 256];
        assert_eq!(
            tls.read(&mut buf).unwrap_or(0),
            0,
            "anonymous client received application response"
        );
    }
    let mut authorized = SslConnector::builder(SslMethod::tls()).unwrap();
    authorized.set_verify(SslVerifyMode::NONE);
    authorized
        .set_certificate_file(&cert, SslFiletype::PEM)
        .unwrap();
    authorized
        .set_private_key_file(&key, SslFiletype::PEM)
        .unwrap();
    let stream = TcpStream::connect(address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let mut tls = authorized.build().connect("localhost", stream).unwrap();
    tls.write_all(b"{}\n").unwrap();
    let mut response = String::new();
    std::io::BufReader::new(&mut tls)
        .read_line(&mut response)
        .unwrap();
    assert!(
        !response.is_empty(),
        "authenticated TLS connection yielded no JSON response"
    );
    let result: serde_json::Value = serde_json::from_str(&response).unwrap();
    assert_eq!(result["ok"], false);
    assert_eq!(result["error"], "invalid envelope");

    let request = AgentRegistrationRequest {
        request_id: "mtls-e2e-001".into(),
        agent_id: "OCAF-AGENT-001".into(),
        tenant: "tenant_a".into(),
        manifest_digest: format!("sha256:{}", "a".repeat(64)),
        declared_state: "STAGED".into(),
        readiness: false,
        ostc_enabled: false,
        source_access: "disabled".into(),
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let mut operation = SignedOperation {
        tenant: request.tenant.clone(),
        namespace: format!("{}/agents", request.tenant),
        target: format!("{}/agents/{}", request.tenant, request.agent_id),
        opcode: 5,
        fuel: 1,
        issued_at: now,
        nonce: "mtls-e2e-unique-nonce".into(),
        payload: serde_json::to_vec(&request).unwrap(),
        rpc_sender: Some("test-node".into()),
        signature: Vec::new(),
    };
    operation.signature = Signer::new_without_digest(&signing)
        .unwrap()
        .sign_oneshot_to_vec(&operation.canonical_bytes())
        .unwrap();
    let envelope = serde_json::json!({"request": request, "operation": operation});
    let mut client = SslConnector::builder(SslMethod::tls()).unwrap();
    client.set_verify(SslVerifyMode::NONE);
    client
        .set_certificate_file(&cert, SslFiletype::PEM)
        .unwrap();
    client.set_private_key_file(&key, SslFiletype::PEM).unwrap();
    let connector = client.build();
    for expected in [true, false] {
        let socket = TcpStream::connect(address).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut tls = connector.connect("localhost", socket).unwrap();
        writeln!(tls, "{envelope}").unwrap();
        let mut answer = String::new();
        std::io::BufReader::new(&mut tls)
            .read_line(&mut answer)
            .unwrap();
        let result: serde_json::Value = serde_json::from_str(&answer).unwrap();
        if expected {
            assert_eq!(result["ok"], true, "{result}");
            assert_eq!(result["decision"]["admitted"], true, "{result}");
        } else {
            assert_eq!(result["ok"], false, "{result}");
        }
    }
    let connection = Connection::open(&agents).unwrap();
    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM stage043_agents", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);
}
