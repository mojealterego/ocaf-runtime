//! Standalone OMAs agent-server process.
//!
//! `--health` emits a machine-readable health snapshot and exits.  Without a
//! flag the process reads registration JSON lines from stdin and emits one
//! admission decision per line.  OMAt is deliberately not part of this binary.
use ocaf_runtime::omas_server::{
    AdmissionDecision, AgentRegistrationRequest, OmasServer, OmasServerConfig,
};
use std::io::{self, BufRead};

fn main() {
    let server = match OmasServer::new(OmasServerConfig::default()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("OMAs configuration rejected: {e}");
            std::process::exit(2);
        }
    };
    if std::env::args().any(|arg| arg == "--health") {
        println!(
            "{}",
            serde_json::to_string(&server.health()).expect("health serialization")
        );
        return;
    }
    for line in io::stdin().lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(e) => {
                eprintln!("stdin: {e}");
                break;
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<AgentRegistrationRequest>(&line) {
            Ok(req) => server.admit_registration(req),
            Err(error) => AdmissionDecision {
                request_id: String::new(),
                agent_id: String::new(),
                admitted: false,
                lifecycle: "DRAFT".into(),
                reason: format!("invalid_json:{error}"),
            },
        };
        println!(
            "{}",
            serde_json::to_string(&response).expect("decision serialization")
        );
    }
}
