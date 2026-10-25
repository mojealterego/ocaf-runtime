//! Stage 0.44: pinned-issuer OpenID Connect ID-token verification.
//!
//! The gateway must bind a random, one-use nonce and CSRF state to its own
//! authenticated browser session. This verifier MUST NOT accept a nonce or
//! issuer selected by an HTTP client as a trust configuration.
use crate::stage044_oauth::AuthenticatedSubject;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use openssl::hash::MessageDigest;
use openssl::memcmp;
use openssl::pkey::{PKey, Public};
use openssl::sign::Verifier;
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum OidcError {
    #[error("OIDC trust configuration rejected")]
    Configuration,
    #[error("OIDC identity token rejected")]
    InvalidToken,
    #[error("OIDC token cryptographic verification failed")]
    Signature,
}

pub struct OidcTrust {
    issuer: String,
    client_id: String,
    tenant: String,
    key_id: String,
    key: PKey<Public>,
    mfa_acr: String,
}

impl OidcTrust {
    /// Public key PEM and kid must come from an operator-pinned JWKS source.
    /// Key rotation is explicit; never honor JWT jku, x5u or embedded keys.
    pub fn new(
        issuer: String,
        client_id: String,
        tenant: String,
        key_id: String,
        rsa_public_pem: &[u8],
        mfa_acr: String,
    ) -> Result<Self, OidcError> {
        let host = issuer
            .strip_prefix("https://")
            .ok_or(OidcError::Configuration)?
            .split('/')
            .next()
            .ok_or(OidcError::Configuration)?;
        if host.is_empty()
            || host.contains('@')
            || host.contains(':')
            || issuer.contains('?')
            || issuer.contains('#')
            || issuer.ends_with('/')
            || !safe_id(&client_id)
            || !safe_id(&tenant)
            || !safe_id(&key_id)
            || mfa_acr.is_empty()
            || mfa_acr.len() > 256
        {
            return Err(OidcError::Configuration);
        }
        let key =
            PKey::public_key_from_pem(rsa_public_pem).map_err(|_| OidcError::Configuration)?;
        let rsa = key.rsa().map_err(|_| OidcError::Configuration)?;
        if rsa.size() < 256 {
            return Err(OidcError::Configuration);
        }
        Ok(Self {
            issuer,
            client_id,
            tenant,
            key_id,
            key,
            mfa_acr,
        })
    }

    /// Validate an OAuth access token, never an OIDC ID token, for a fixed
    /// resource audience and operator-selected scopes. Used by the HTTP
    /// protected-resource gateway before OMAt's independent policy checks.
    pub fn verify_access_token(
        &self,
        jwt: &str,
        resource_audience: &str,
        required_scopes: &std::collections::BTreeSet<String>,
        now: i64,
    ) -> Result<AuthenticatedSubject, OidcError> {
        if jwt.len() > 16_384
            || !resource_audience.starts_with("https://")
            || required_scopes.is_empty()
            || required_scopes.len() > 64
        {
            return Err(OidcError::InvalidToken);
        }
        let mut parts = jwt.split('.');
        let h = parts.next().ok_or(OidcError::InvalidToken)?;
        let p = parts.next().ok_or(OidcError::InvalidToken)?;
        let s = parts.next().ok_or(OidcError::InvalidToken)?;
        if parts.next().is_some()
            || h.is_empty()
            || p.is_empty()
            || s.is_empty()
            || h.len() > 4096
            || p.len() > 8192
            || s.len() > 2048
        {
            return Err(OidcError::InvalidToken);
        }
        let decode = |v: &str| {
            URL_SAFE_NO_PAD
                .decode(v)
                .map_err(|_| OidcError::InvalidToken)
        };
        let header: Value =
            serde_json::from_slice(&decode(h)?).map_err(|_| OidcError::InvalidToken)?;
        if header.get("alg").and_then(Value::as_str) != Some("RS256")
            || header.get("kid").and_then(Value::as_str) != Some(self.key_id.as_str())
            || header.get("crit").is_some()
            || header.get("jku").is_some()
            || header.get("x5u").is_some()
            || header.get("jwk").is_some()
            || header.get("zip").is_some()
            || header.get("typ").is_some_and(|v| v.as_str() != Some("JWT"))
        {
            return Err(OidcError::InvalidToken);
        }
        let mut verifier =
            Verifier::new(MessageDigest::sha256(), &self.key).map_err(|_| OidcError::Signature)?;
        verifier
            .update(format!("{h}.{p}").as_bytes())
            .map_err(|_| OidcError::Signature)?;
        if !verifier
            .verify(&decode(s)?)
            .map_err(|_| OidcError::Signature)?
        {
            return Err(OidcError::Signature);
        }
        let claims: Value =
            serde_json::from_slice(&decode(p)?).map_err(|_| OidcError::InvalidToken)?;
        let string = |key: &str| claims.get(key).and_then(Value::as_str);
        if string("iss") != Some(self.issuer.as_str())
            || string("typ") != Some("Bearer")
            || string("azp") != Some(self.client_id.as_str())
        {
            return Err(OidcError::InvalidToken);
        }
        let aud = claims.get("aud").ok_or(OidcError::InvalidToken)?;
        let matches_resource = match aud {
            Value::String(v) => v == resource_audience,
            Value::Array(v) if v.len() <= 16 => {
                v.iter().any(|x| x.as_str() == Some(resource_audience))
            }
            _ => false,
        };
        if !matches_resource {
            return Err(OidcError::InvalidToken);
        }
        let exp = claims
            .get("exp")
            .and_then(Value::as_i64)
            .ok_or(OidcError::InvalidToken)?;
        let iat = claims
            .get("iat")
            .and_then(Value::as_i64)
            .ok_or(OidcError::InvalidToken)?;
        if exp <= now
            || exp > now.saturating_add(3600)
            || iat > now.saturating_add(60)
            || iat < now.saturating_sub(3600)
            || claims
                .get("nbf")
                .and_then(Value::as_i64)
                .is_some_and(|nbf| nbf > now.saturating_add(60))
        {
            return Err(OidcError::InvalidToken);
        }
        let sub = string("sub").ok_or(OidcError::InvalidToken)?;
        if !safe_id(sub) {
            return Err(OidcError::InvalidToken);
        }
        let scopes: std::collections::BTreeSet<&str> = string("scope")
            .ok_or(OidcError::InvalidToken)?
            .split_ascii_whitespace()
            .collect();
        if !required_scopes
            .iter()
            .all(|scope| scopes.contains(scope.as_str()))
        {
            return Err(OidcError::InvalidToken);
        }
        let auth_time = claims
            .get("auth_time")
            .and_then(Value::as_i64)
            .unwrap_or(iat);
        let fresh_mfa = claims.get("auth_time").is_some()
            && string("acr") == Some(self.mfa_acr.as_str())
            && auth_time <= now.saturating_add(60)
            && now.saturating_sub(auth_time) <= 300;
        Ok(AuthenticatedSubject {
            tenant: self.tenant.clone(),
            user: sub.to_owned(),
            authenticated_at: auth_time,
            assurance_level: if fresh_mfa { 2 } else { 1 },
        })
    }

    /// Verified identity, never user-supplied tenant or unverified claims.
    /// now is supplied by a trusted wall clock; test callers may inject it.
    pub fn verify_id_token(
        &self,
        jwt: &str,
        expected_nonce: &str,
        now: i64,
    ) -> Result<AuthenticatedSubject, OidcError> {
        if jwt.len() > 16_384
            || expected_nonce.len() < 32
            || expected_nonce.len() > 256
            || !expected_nonce.is_ascii()
            || expected_nonce.bytes().any(|b| b <= 32 || b >= 127)
        {
            return Err(OidcError::InvalidToken);
        }
        let mut parts = jwt.split('.');
        let h = parts.next().ok_or(OidcError::InvalidToken)?;
        let p = parts.next().ok_or(OidcError::InvalidToken)?;
        let s = parts.next().ok_or(OidcError::InvalidToken)?;
        if parts.next().is_some()
            || h.is_empty()
            || p.is_empty()
            || s.is_empty()
            || h.len() > 4096
            || p.len() > 8192
            || s.len() > 2048
        {
            return Err(OidcError::InvalidToken);
        }
        let decode = |part: &str| {
            URL_SAFE_NO_PAD
                .decode(part)
                .map_err(|_| OidcError::InvalidToken)
        };
        let header: Value =
            serde_json::from_slice(&decode(h)?).map_err(|_| OidcError::InvalidToken)?;
        if header.get("alg").and_then(Value::as_str) != Some("RS256")
            || header.get("kid").and_then(Value::as_str) != Some(self.key_id.as_str())
            || header.get("crit").is_some()
            || header.get("jku").is_some()
            || header.get("x5u").is_some()
            || header.get("jwk").is_some()
            || header.get("zip").is_some()
            || header.get("typ").is_some_and(|v| v.as_str() != Some("JWT"))
        {
            return Err(OidcError::InvalidToken);
        }
        let signature = decode(s)?;
        let mut verifier =
            Verifier::new(MessageDigest::sha256(), &self.key).map_err(|_| OidcError::Signature)?;
        verifier
            .update(format!("{h}.{p}").as_bytes())
            .map_err(|_| OidcError::Signature)?;
        if !verifier
            .verify(&signature)
            .map_err(|_| OidcError::Signature)?
        {
            return Err(OidcError::Signature);
        }
        let claims: Value =
            serde_json::from_slice(&decode(p)?).map_err(|_| OidcError::InvalidToken)?;
        let claim = |name: &str| claims.get(name).and_then(Value::as_str);
        if claim("iss") != Some(self.issuer.as_str())
            || claim("nonce").is_none_or(|v| !memcmp::eq(v.as_bytes(), expected_nonce.as_bytes()))
        {
            return Err(OidcError::InvalidToken);
        }
        let aud = claims.get("aud").ok_or(OidcError::InvalidToken)?;
        let audience_count = match aud {
            Value::String(v) if v == &self.client_id => 1,
            Value::Array(a)
                if !a.is_empty()
                    && a.len() <= 16
                    && a.iter()
                        .any(|v| v.as_str() == Some(self.client_id.as_str())) =>
            {
                a.len()
            }
            _ => return Err(OidcError::InvalidToken),
        };
        if claim("azp").is_some_and(|v| v != self.client_id)
            || (audience_count > 1 && claim("azp") != Some(self.client_id.as_str()))
        {
            return Err(OidcError::InvalidToken);
        }
        let exp = claims
            .get("exp")
            .and_then(Value::as_i64)
            .ok_or(OidcError::InvalidToken)?;
        let iat = claims
            .get("iat")
            .and_then(Value::as_i64)
            .ok_or(OidcError::InvalidToken)?;
        let auth_time = claims
            .get("auth_time")
            .and_then(Value::as_i64)
            .ok_or(OidcError::InvalidToken)?;
        if exp <= now
            || exp > now.saturating_add(3600)
            || iat > now.saturating_add(60)
            || iat < now.saturating_sub(3600)
            || auth_time > now.saturating_add(60)
            || auth_time < now.saturating_sub(3600)
            || claims
                .get("nbf")
                .and_then(Value::as_i64)
                .is_some_and(|nbf| nbf > now.saturating_add(60))
        {
            return Err(OidcError::InvalidToken);
        }
        let user = claim("sub").ok_or(OidcError::InvalidToken)?;
        if !safe_id(user) {
            return Err(OidcError::InvalidToken);
        }
        let assurance_level = if claim("acr") == Some(self.mfa_acr.as_str())
            && now.saturating_sub(auth_time) <= 300
        {
            2
        } else {
            1
        };
        Ok(AuthenticatedSubject {
            tenant: self.tenant.clone(),
            user: user.to_owned(),
            authenticated_at: auth_time,
            assurance_level,
        })
    }
}

fn safe_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value != "."
        && value != ".."
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-.".contains(&b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use openssl::rsa::Rsa;
    use openssl::sign::Signer;
    use serde_json::json;
    const NONCE: &str = "0123456789abcdef0123456789abcdef";

    fn signed_token(
        key: &PKey<openssl::pkey::Private>,
        payload: Value,
        kid: &str,
        alg: &str,
    ) -> String {
        let h = URL_SAFE_NO_PAD
            .encode(serde_json::to_vec(&json!({"alg":alg,"kid":kid,"typ":"JWT"})).unwrap());
        let p = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&payload).unwrap());
        let body = format!("{h}.{p}");
        let mut signer = Signer::new(MessageDigest::sha256(), key).unwrap();
        signer.update(body.as_bytes()).unwrap();
        format!(
            "{body}.{}",
            URL_SAFE_NO_PAD.encode(signer.sign_to_vec().unwrap())
        )
    }
    fn setup() -> (OidcTrust, PKey<openssl::pkey::Private>) {
        let key = PKey::from_rsa(Rsa::generate(2048).unwrap()).unwrap();
        let pubkey = key.public_key_to_pem().unwrap();
        let trust = OidcTrust::new(
            "https://idp.example/realms/ocaf".into(),
            "ocaf-client".into(),
            "tenant-a".into(),
            "kid-1".into(),
            &pubkey,
            "urn:ocaf:mfa".into(),
        )
        .unwrap();
        (trust, key)
    }
    fn claims() -> Value {
        json!({"iss":"https://idp.example/realms/ocaf","aud":"ocaf-client",
            "sub":"alice","nonce":NONCE,"iat":1000,"auth_time":1000,"exp":1300,"acr":"urn:ocaf:mfa"})
    }
    #[test]
    fn signed_oidc_identity_and_fresh_mfa_are_accepted() {
        let (trust, key) = setup();
        let jwt = signed_token(&key, claims(), "kid-1", "RS256");
        let id = trust.verify_id_token(&jwt, NONCE, 1010).unwrap();
        assert_eq!(
            (id.tenant.as_str(), id.user.as_str(), id.assurance_level),
            ("tenant-a", "alice", 2)
        );
        let mut long_lived = claims();
        long_lived["exp"] = json!(1400);
        let late_jwt = signed_token(&key, long_lived, "kid-1", "RS256");
        assert_eq!(
            trust
                .verify_id_token(&late_jwt, NONCE, 1301)
                .unwrap()
                .assurance_level,
            1
        );
    }
    #[test]
    fn rejects_nonce_audience_signature_kid_and_expiry() {
        let (trust, key) = setup();
        let jwt = signed_token(&key, claims(), "kid-1", "RS256");
        assert!(trust
            .verify_id_token(&jwt, "ffffffffffffffffffffffffffffffff", 1010)
            .is_err());
        assert!(trust.verify_id_token(&jwt, NONCE, 1300).is_err());
        let mut c = claims();
        c["aud"] = json!("wrong-client");
        assert!(trust
            .verify_id_token(&signed_token(&key, c, "kid-1", "RS256"), NONCE, 1010)
            .is_err());
        assert!(trust
            .verify_id_token(&signed_token(&key, claims(), "kid-2", "RS256"), NONCE, 1010)
            .is_err());
        assert!(trust
            .verify_id_token(&signed_token(&key, claims(), "kid-1", "none"), NONCE, 1010)
            .is_err());
        let other = PKey::from_rsa(Rsa::generate(2048).unwrap()).unwrap();
        assert!(trust
            .verify_id_token(
                &signed_token(&other, claims(), "kid-1", "RS256"),
                NONCE,
                1010
            )
            .is_err());
    }
    #[test]
    fn rejects_multi_audience_without_authorized_party() {
        let (trust, key) = setup();
        let mut c = claims();
        c["aud"] = json!(["ocaf-client", "other"]);
        assert!(trust
            .verify_id_token(
                &signed_token(&key, c.clone(), "kid-1", "RS256"),
                NONCE,
                1010
            )
            .is_err());
        c["azp"] = json!("ocaf-client");
        assert!(trust
            .verify_id_token(&signed_token(&key, c, "kid-1", "RS256"), NONCE, 1010)
            .is_ok());
    }
    #[test]
    fn access_token_requires_bearer_type_resource_and_scope() {
        let (trust, key) = setup();
        let claims = json!({"iss":"https://idp.example/realms/ocaf",
            "aud":["https://omat.example","account"],"azp":"ocaf-client",
            "typ":"Bearer","sub":"alice","scope":"tool:read tool:write",
            "iat":1000,"auth_time":1000,"exp":1300,"acr":"urn:ocaf:mfa"});
        let token = signed_token(&key, claims.clone(), "kid-1", "RS256");
        let read: std::collections::BTreeSet<String> =
            ["tool:read".to_owned()].into_iter().collect();
        assert_eq!(
            trust
                .verify_access_token(&token, "https://omat.example", &read, 1010)
                .unwrap()
                .assurance_level,
            2
        );
        assert!(trust
            .verify_access_token(&token, "https://evil.example", &read, 1010)
            .is_err());
        assert!(trust
            .verify_access_token(
                &token,
                "https://omat.example",
                &["admin".to_owned()].into_iter().collect(),
                1010
            )
            .is_err());
        let mut wrong = claims;
        wrong["typ"] = json!("ID");
        assert!(trust
            .verify_access_token(
                &signed_token(&key, wrong, "kid-1", "RS256"),
                "https://omat.example",
                &read,
                1010
            )
            .is_err());
    }
}
