//! Deterministic, policy-bound model selection (Argon PDF 1.75).
//! Operator-pinned profiles are trusted inputs, never model-supplied claims.
//! A selected profile is not permission to perform an external model call.
use std::collections::BTreeSet;
use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Privacy {
    Public,
    TenantConfidential,
    LocalOnly,
}

#[derive(Clone, Debug)]
pub struct ModelProfile {
    pub id: String,
    pub allowed_tenant: Option<String>,
    pub capability: String,
    pub privacy: Privacy,
    pub requires_network: bool,
    pub verified_result: bool,
    pub cost_micro_per_1000_tokens: u64,
    pub p95_latency_ms: u64,
    pub max_context_tokens: u64,
    pub enabled: bool,
}

#[derive(Clone, Debug)]
pub struct RouteRequest {
    pub tenant: String,
    pub capability: String,
    pub tokens: u64,
    pub max_total_cost_micro: u64,
    pub max_latency_ms: u64,
    pub minimum_privacy: Privacy,
    pub network_permitted: bool,
    pub require_verified_result: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Selection {
    pub profile_id: String,
    pub estimated_cost_micro: u64,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum RoutingError {
    #[error("invalid route input or model catalog")]
    Invalid,
    #[error("resource limit exceeded")]
    Limit,
    #[error("no model meets trust, capability and budget constraints")]
    Denied,
}

fn valid(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.:-".contains(&b))
}

pub fn select(
    profiles: &[ModelProfile],
    request: &RouteRequest,
) -> Result<Selection, RoutingError> {
    if profiles.is_empty()
        || profiles.len() > 128
        || request.tokens == 0
        || request.tokens > 1_000_000
        || request.max_latency_ms == 0
    {
        return Err(RoutingError::Limit);
    }
    if !valid(&request.tenant) || !valid(&request.capability) {
        return Err(RoutingError::Invalid);
    }
    let mut ids = BTreeSet::new();
    for model in profiles {
        if !valid(&model.id)
            || !valid(&model.capability)
            || !model.allowed_tenant.as_ref().is_none_or(|t| valid(t))
            || !ids.insert(&model.id)
            || model.p95_latency_ms == 0
            || model.max_context_tokens == 0
        {
            return Err(RoutingError::Invalid);
        }
    }
    profiles
        .iter()
        .filter_map(|model| {
            if !model.enabled
                || model.capability != request.capability
                || !model
                    .allowed_tenant
                    .as_ref()
                    .is_none_or(|t| t == &request.tenant)
                || model.privacy < request.minimum_privacy
                || model.requires_network && !request.network_permitted
                || request.require_verified_result && !model.verified_result
                || model.max_context_tokens < request.tokens
                || model.p95_latency_ms > request.max_latency_ms
            {
                return None;
            }
            let cost = model
                .cost_micro_per_1000_tokens
                .checked_mul(request.tokens)?
                .checked_add(999)?
                / 1000;
            (cost <= request.max_total_cost_micro).then_some((cost, model.p95_latency_ms, &model.id))
        })
        .min()
        .map(|(estimated_cost_micro, _, profile_id)| Selection {
            profile_id: profile_id.clone(),
            estimated_cost_micro,
        })
        .ok_or(RoutingError::Denied)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn model(id: &str, privacy: Privacy, cost: u64) -> ModelProfile {
        ModelProfile {
            id: id.into(),
            allowed_tenant: None,
            capability: "chat".into(),
            privacy,
            requires_network: false,
            verified_result: true,
            cost_micro_per_1000_tokens: cost,
            p95_latency_ms: 80,
            max_context_tokens: 8192,
            enabled: true,
        }
    }
    fn request() -> RouteRequest {
        RouteRequest {
            tenant: "tenant-a".into(),
            capability: "chat".into(),
            tokens: 500,
            max_total_cost_micro: 100,
            max_latency_ms: 100,
            minimum_privacy: Privacy::TenantConfidential,
            network_permitted: false,
            require_verified_result: true,
        }
    }
    #[test]
    fn deterministic_selection_and_checked_billing() {
        let models = [
            model("remote", Privacy::Public, 2),
            model("private", Privacy::LocalOnly, 30),
            model("faster", Privacy::TenantConfidential, 40),
        ];
        assert_eq!(
            select(&models, &request()).unwrap(),
            Selection {
                profile_id: "private".into(),
                estimated_cost_micro: 15,
            }
        );
    }
    #[test]
    fn fail_closed_for_tenant_privacy_network_verification_and_overflow() {
        let mut m = model("a", Privacy::LocalOnly, 1);
        m.allowed_tenant = Some("tenant-b".into());
        assert_eq!(select(&[m.clone()], &request()), Err(RoutingError::Denied));
        m.allowed_tenant = None;
        m.requires_network = true;
        assert_eq!(select(&[m.clone()], &request()), Err(RoutingError::Denied));
        m.requires_network = false;
        m.verified_result = false;
        assert_eq!(select(&[m.clone()], &request()), Err(RoutingError::Denied));
        m.verified_result = true;
        m.cost_micro_per_1000_tokens = u64::MAX;
        assert_eq!(select(&[m], &request()), Err(RoutingError::Denied));
    }
    #[test]
    fn duplicates_and_bad_input_rejected() {
        let m = model("dup", Privacy::LocalOnly, 1);
        assert_eq!(
            select(&[m.clone(), m], &request()),
            Err(RoutingError::Invalid)
        );
        let mut r = request();
        r.tenant = "../foreign".into();
        assert_eq!(
            select(&[model("a", Privacy::LocalOnly, 1)], &r),
            Err(RoutingError::Invalid)
        );
    }
}
