//! Stage 0.44 OMAs/OMAt authorization boundary.
//! mTLS peer identity is resolved by stage043_mtls_admission before this call;
//! a request's user, tenant or agent fields cannot select an OAuth principal.
use crate::omat_server::{OmatPolicy, OmatRegistry, ToolCall, ToolReceipt};
use crate::stage043_omat_route::{
    dispatch_tool_call, AuthenticatedPeer, DispatchError, OmatRouteGuard,
};
use crate::stage044_oauth::{AuthorizedGrant, OAuthError, OAuthService};
use std::collections::{BTreeSet, HashMap};

#[derive(Clone, Debug)]
pub struct BoundAgent {
    pub tenant: String,
    pub agent_id: String,
    pub user: String,
}

#[derive(Clone, Debug)]
pub struct ToolAuthorization {
    pub tool_id: String,
    pub scopes: BTreeSet<String>,
    pub min_assurance: u8,
}

#[derive(Debug)]
pub enum GateError {
    Configuration,
    IdentityDenied,
    OAuth(OAuthError),
    Dispatch(DispatchError),
}

/// Operator-owned map from certificate-authenticated agent identity to a
/// single user. This mapping MUST be sourced from a verified identity registry,
/// never from model arguments or tool metadata.
pub struct OAuthOmatGate {
    agents: HashMap<(String, String), String>,
    tools: HashMap<String, ToolAuthorization>,
    resource: String,
}

impl OAuthOmatGate {
    pub fn new(
        resource: String,
        agents: Vec<BoundAgent>,
        tools: Vec<ToolAuthorization>,
    ) -> Result<Self, GateError> {
        if !resource.starts_with("https://") || agents.is_empty() || tools.is_empty() {
            return Err(GateError::Configuration);
        }
        let mut agent_map = HashMap::new();
        for a in agents {
            if !valid_id(&a.tenant)
                || !valid_id(&a.agent_id)
                || !valid_id(&a.user)
                || agent_map.insert((a.tenant, a.agent_id), a.user).is_some()
            {
                return Err(GateError::Configuration);
            }
        }
        let mut tool_map = HashMap::new();
        for tool in tools {
            if !valid_id(&tool.tool_id)
                || tool.scopes.is_empty()
                || tool.min_assurance == 0
                || tool_map.insert(tool.tool_id.clone(), tool).is_some()
            {
                return Err(GateError::Configuration);
            }
        }
        Ok(Self {
            agents: agent_map,
            tools: tool_map,
            resource,
        })
    }

    /// The caller supplies the already verified TLS peer, NOT a JSON principal.
    /// The token must match the operator-bound user, tenant and exact resource.
    pub fn authorize(
        &self,
        oauth: &OAuthService,
        token: &str,
        peer: &AuthenticatedPeer,
        call: &ToolCall,
    ) -> Result<AuthorizedGrant, GateError> {
        if call.tenant_id != peer.tenant || call.agent_id != peer.principal {
            return Err(GateError::IdentityDenied);
        }
        let user = self
            .agents
            .get(&(peer.tenant.clone(), peer.principal.clone()))
            .ok_or(GateError::IdentityDenied)?;
        let tool = self
            .tools
            .get(&call.tool_id)
            .ok_or(GateError::IdentityDenied)?;
        oauth
            .validate_access(
                token,
                &peer.tenant,
                user,
                &self.resource,
                &tool.scopes,
                tool.min_assurance,
            )
            .map_err(GateError::OAuth)
    }

    /// Authorization does not grant source access or bypass the stage 0.43
    /// executor gate. Tool execution still requires OMAt's own policy.
    pub fn dispatch(
        &self,
        oauth: &OAuthService,
        token: &str,
        route: &OmatRouteGuard,
        peer: &AuthenticatedPeer,
        topic: &str,
        call: &ToolCall,
        policy: &OmatPolicy,
        registry: &OmatRegistry,
    ) -> Result<ToolReceipt, GateError> {
        self.authorize(oauth, token, peer, call)?;
        dispatch_tool_call(route, peer, topic, call, policy, registry).map_err(GateError::Dispatch)
    }
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}
