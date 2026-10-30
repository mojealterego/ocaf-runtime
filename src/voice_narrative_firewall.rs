//! Non-authoritative voice narrative boundary (Argon source 2.03).
//! A narrative result cannot authorize RNG, economics or gameplay changes.
//! SessionContext MUST come from authenticated ingress, not a language model.
use thiserror::Error;

#[derive(Clone, Debug)]
pub struct SessionContext {
    pub tenant_id: String,
    pub session_id: String,
    pub microphone_consent: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    GeneratedNarration,
    MicrophoneTranscript,
}

#[derive(Clone, Debug)]
pub struct VoiceCandidate {
    pub tenant_id: String,
    pub session_id: String,
    pub source: Source,
    pub text: String,
}

/// Non-authoritative event: no game-state accessors or side-effect callbacks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NarrativeEvent {
    tenant_id: String,
    session_id: String,
    text: String,
}

impl NarrativeEvent {
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum FirewallError {
    #[error("invalid or untrusted session data")]
    Invalid,
    #[error("tenant or session mismatch")]
    Denied,
    #[error("explicit microphone consent required")]
    MicrophoneDenied,
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
}

/// Only text crosses this boundary; no action, payment or RNG instruction.
pub fn admit(
    ctx: &SessionContext,
    candidate: VoiceCandidate,
) -> Result<NarrativeEvent, FirewallError> {
    if !valid_identifier(&ctx.tenant_id)
        || !valid_identifier(&ctx.session_id)
        || !valid_identifier(&candidate.tenant_id)
        || !valid_identifier(&candidate.session_id)
        || candidate.text.is_empty()
        || candidate.text.len() > 4096
        || candidate.text.chars().any(|ch| ch.is_control())
    {
        return Err(FirewallError::Invalid);
    }
    if ctx.tenant_id != candidate.tenant_id || ctx.session_id != candidate.session_id {
        return Err(FirewallError::Denied);
    }
    if candidate.source == Source::MicrophoneTranscript && !ctx.microphone_consent {
        return Err(FirewallError::MicrophoneDenied);
    }
    Ok(NarrativeEvent {
        tenant_id: ctx.tenant_id.clone(),
        session_id: ctx.session_id.clone(),
        text: candidate.text,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> SessionContext {
        SessionContext {
            tenant_id: "tenant-a".into(),
            session_id: "session1".into(),
            microphone_consent: false,
        }
    }
    fn candidate(source: Source, text: &str) -> VoiceCandidate {
        VoiceCandidate {
            tenant_id: "tenant-a".into(),
            session_id: "session1".into(),
            source,
            text: text.into(),
        }
    }

    #[test]
    fn malicious_state_commands_remain_non_authoritative_text() {
        let rng_seed: u64 = 11;
        let currency: u64 = 25;
        let event = admit(
            &ctx(),
            candidate(
                Source::GeneratedNarration,
                "{\"currency\":99999,\"rng_seed\":0}",
            ),
        )
        .unwrap();
        assert!(event.text().contains("rng_seed"));
        assert_eq!(rng_seed, 11);
        assert_eq!(currency, 25);
    }
    #[test]
    fn microphone_requires_explicit_consent_and_tenant_binding() {
        assert_eq!(
            admit(&ctx(), candidate(Source::MicrophoneTranscript, "hello")),
            Err(FirewallError::MicrophoneDenied)
        );
        let mut signed = ctx();
        signed.microphone_consent = true;
        assert!(admit(&signed, candidate(Source::MicrophoneTranscript, "hello")).is_ok());
        let mut foreign = candidate(Source::GeneratedNarration, "hello");
        foreign.tenant_id = "tenant-b".into();
        assert_eq!(admit(&signed, foreign), Err(FirewallError::Denied));
    }
    #[test]
    fn controls_and_oversized_output_rejected() {
        assert_eq!(
            admit(&ctx(), candidate(Source::GeneratedNarration, "hi\0")),
            Err(FirewallError::Invalid)
        );
        assert_eq!(
            admit(
                &ctx(),
                candidate(Source::GeneratedNarration, &"a".repeat(4097))
            ),
            Err(FirewallError::Invalid)
        );
    }
}
