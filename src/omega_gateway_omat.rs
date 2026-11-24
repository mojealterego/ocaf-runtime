//! Gateway tool contract for the existing OCAF OMAs/OMAt transport.
//! This is an intentionally disabled staging contract, not an OSTc attestation,
//! executor, or authorization to send prompts outside the verified boundary.
use crate::omat_server::{ExecutorKind, OmatError, OmatRegistry, ToolCall, ToolManifest, ToolRisk};
use crate::stage043_omat_route::{omat_resource_id, AuthenticatedPeer};
use serde::{Deserialize, Serialize};
use serde_json::json;

pub const TOOL_ID: &str = "omega.gateway.local.chat.v1";
pub const CAPABILITY: &str = "model.local.chat";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GatewayPrompt {
    pub input: String,
    pub max_output_tokens: u32,
}

fn segment(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

impl GatewayPrompt {
    pub fn validate(&self) -> Result<(), OmatError> {
        if self.input.trim().is_empty()
            || self.input.len() > 16_384
            || !(16..=4096).contains(&self.max_output_tokens)
        {
            return Err(OmatError::InvalidArguments);
        }
        Ok(())
    }
}

/// The signed manifest registration workflow may later publish a new enabled
/// version, only after existing operator approval and verified OSTc admission.
pub fn staged_manifest(tenant: &str) -> Result<ToolManifest, OmatError> {
    if !segment(tenant) {
        return Err(OmatError::InvalidIdentifier);
    }
    Ok(ToolManifest {
        tenant_id: tenant.into(),
        tool_id: TOOL_ID.into(),
        version: "1".into(),
        description: "Bounded local-only OMEGA Gateway chat; staging, not executable".into(),
        input_schema: json!({
            "type":"object","additionalProperties":false,
            "properties":{"input":{"type":"string","minLength":1,"maxLength":16384},
                "max_output_tokens":{"type":"integer","minimum":16,"maximum":4096}},
            "required":["input","max_output_tokens"]
        }),
        output_schema: json!({
            "type":"object","additionalProperties":false,
            "properties":{"text":{"type":"string","maxLength":131072}},
            "required":["text"]
        }),
        capabilities: [CAPABILITY.to_owned()].into_iter().collect(),
        risk: ToolRisk::ReadOnly,
        executor: ExecutorKind::External,
        timeout_ms: 5000,
        max_output_bytes: 262144,
        enabled: false,
    })
}

/// Produce a local registry entry for staging. Deployment must use the existing
/// authenticated signed-manifest mechanism, not caller-supplied registry edits.
pub fn register_staged(registry: &mut OmatRegistry, tenant: &str) -> Result<String, OmatError> {
    registry.register(staged_manifest(tenant)?)
}

pub struct StagedGatewayCall {
    pub call: ToolCall,
    pub topic: String,
    pub resource: String,
}

pub fn stage_call(
    peer: &AuthenticatedPeer,
    agent: &str,
    request_id: &str,
    idempotency: &str,
    prompt: &GatewayPrompt,
) -> Result<StagedGatewayCall, OmatError> {
    if !segment(&peer.principal)
        || !segment(&peer.tenant)
        || !segment(agent)
        || !segment(request_id)
        || !segment(idempotency)
    {
        return Err(OmatError::InvalidIdentifier);
    }
    prompt.validate()?;
    let call = ToolCall {
        request_id: request_id.into(),
        tenant_id: peer.tenant.clone(),
        agent_id: agent.into(),
        tool_id: TOOL_ID.into(),
        arguments: serde_json::to_value(prompt).map_err(|_| OmatError::InvalidArguments)?,
        idempotency_key: idempotency.into(),
        ostc_enabled: false,
        source_access: "disabled".into(),
    };
    let topic = format!("ocaf/omat/{}/requests/{}", peer.tenant, request_id);
    let resource = omat_resource_id(&call);
    Ok(StagedGatewayCall { call, topic, resource })
}

/// Uses the existing HMAC + capability Zenoh request path. This cannot activate
/// a tool or attest OSTc: the server must reject this staging request today.
#[cfg(feature = "zenoh")]
pub async fn probe_staged_gateway(
    session: &zenoh::Session,
    peer: &AuthenticatedPeer,
    staged: StagedGatewayCall,
    nonce: &str,
    ticket: crate::sasos_capability::CapabilityTicket,
    transport_key: &[u8; 32],
) -> anyhow::Result<()> {
    anyhow::ensure!(staged.call.tenant_id == peer.tenant, "tenant mismatch");
    anyhow::ensure!(!staged.call.ostc_enabled && staged.call.tool_id == TOOL_ID, "not a staging probe");
    let frame = crate::stage043_omat_zenoh::sign_omas_request_with_capability(
        &peer.principal, &staged.topic, nonce, staged.call, ticket, transport_key,
    )?;
    crate::stage043_omat_zenoh::request_omat_zenoh(session, &staged.topic, frame).await
}
