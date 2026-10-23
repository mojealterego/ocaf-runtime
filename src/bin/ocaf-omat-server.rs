//! OMAt standalone health endpoint. Tool execution remains fail-closed.

fn main() {
    println!(
        "{{\"service\":\"OMAt\",\"protocol\":\"omat.v1\",\"ready\":false,\"ostc_enabled\":false,\"source_access\":\"disabled\",\"executor\":\"fail_closed\"}}"
    );
}
