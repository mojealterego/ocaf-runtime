//! OMAS admission: tenant-scoped key expressions and bounded requests.
pub const MAX_FRAME_BYTES: usize = 1_048_576;
pub const MAX_PAYLOAD_BYTES: usize = 786_432;
pub const MAX_FUEL: u64 = 50_000_000;

pub fn safe_key(key: &str) -> bool {
    !key.is_empty() && key.len() <= 256 && key.split('/').all(|segment| {
        !segment.is_empty() && segment != "." && segment != ".."
            && segment.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'.')
    })
}
pub fn namespace_contains(namespace: &str, target: &str) -> bool {
    safe_key(namespace) && safe_key(target) &&
        (namespace == target || target.strip_prefix(namespace).is_some_and(|rest| rest.starts_with('/')))
}
pub fn admit(tenant: &str, namespace: &str, target: &str, frame: usize, payload: usize, fuel: u64) -> bool {
    safe_key(tenant) && !tenant.contains('/') &&
        namespace_contains(tenant, target) && namespace_contains(namespace, target) &&
        frame <= MAX_FRAME_BYTES && payload <= MAX_PAYLOAD_BYTES &&
        payload <= frame && fuel > 0 && fuel <= MAX_FUEL
}

/// Enforce OMAS resource bounds on legacy RPC execution payloads without
/// changing the existing RPC wire protocol, authentication, ACL or fencing.
/// Reject serialization errors and oversized values before replay admission.
pub fn admit_rpc_execution_payload(args: &serde_json::Value) -> bool {
    serde_json::to_vec(args).is_ok_and(|encoded| {
        encoded.len() <= MAX_PAYLOAD_BYTES
            && encoded.len() <= crate::rpc::MAX_FRAME_BYTES
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rpc_execution_payload_is_bounded() {
        assert!(admit_rpc_execution_payload(&serde_json::json!({"ok": true})));
        assert!(!admit_rpc_execution_payload(&serde_json::json!({"data": "x".repeat(70_000)})));
    }

    #[test]
    fn reject_cross_tenant_and_prefix_confusion() {
        assert!(admit("tenant_a", "tenant_a/tools", "tenant_a/tools/run", 100, 50, 1000));
        assert!(!admit("tenant_a", "tenant_a/tools", "tenant_ab/tools/run", 100, 50, 1000));
        assert!(!namespace_contains("tenant_a/tools", "tenant_a/toolsmith"));
    }
    #[test]
    fn reject_wildcards_and_unbounded_requests() {
        assert!(!safe_key("tenant_a/**"));
        assert!(!safe_key("tenant_a//tools"));
        assert!(!safe_key("tenant_a/../tools"));
        assert!(!admit("tenant_a", "tenant_a/tools", "tenant_a/tools/run", 100, 101, 1000));
        assert!(!admit("tenant_a", "tenant_a/tools", "tenant_a/tools/run", 100, 50, 0));
    }
}
