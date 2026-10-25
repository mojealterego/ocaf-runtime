//! Stage 0.44 Vault Transit remote cryptography adapter.
//! This is a real HTTPS API adapter, NOT evidence of a deployed KMS/HSM.
//! Vault key MUST be non-exportable, derived, and provisioned by an operator.
use crate::stage044_secret_broker::{
    https_post_with_auth, BrokerAuthHeader, BrokerError, BrokerRoute,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use thiserror::Error;

const MAX_PLAINTEXT: usize = 8192;
const MAX_CIPHERTEXT: usize = 32768;

#[derive(Debug, Error)]
pub enum KmsError {
    #[error("invalid KMS operator configuration")]
    Configuration,
    #[error("KMS unavailable or operation rejected")]
    Unavailable,
    #[error("KMS cryptographic response invalid")]
    InvalidResponse,
}

pub struct VaultTransit {
    route: BrokerRoute,
    key_name: String,
    token_file: PathBuf,
}

impl VaultTransit {
    pub fn new(
        route: BrokerRoute, key_name: String, token_file: &Path,
    ) -> Result<Self, KmsError> {
        if !safe_id(&key_name) || !token_file.is_absolute()
            || route.path != format!("/v1/transit/encrypt/{key_name}")
            || route.validate().is_err()
        { return Err(KmsError::Configuration); }
        crate::ops::secrets::read_private_text(token_file, 4096)
            .map_err(|_| KmsError::Configuration)?;
        Ok(Self { route, key_name, token_file: token_file.to_owned() })
    }

    pub fn encrypt(
        &self, tenant: &str, user: &str, provider: &str, plaintext: &[u8],
    ) -> Result<String, KmsError> {
        if plaintext.is_empty() || plaintext.len() > MAX_PLAINTEXT {
            return Err(KmsError::Configuration);
        }
        let context = context(tenant,user,provider)?;
        let body = json!({"plaintext":STANDARD.encode(plaintext),"context":context});
        let response = self.call("encrypt", &body)?;
        let ciphertext = response.get("ciphertext").and_then(Value::as_str)
            .ok_or(KmsError::InvalidResponse)?;
        if !valid_ciphertext(ciphertext) { return Err(KmsError::InvalidResponse); }
        Ok(ciphertext.to_owned())
    }

    pub fn decrypt(
        &self, tenant: &str, user: &str, provider: &str, ciphertext: &str,
    ) -> Result<Vec<u8>, KmsError> {
        if !valid_ciphertext(ciphertext) { return Err(KmsError::Configuration); }
        let context = context(tenant,user,provider)?;
        let response = self.call("decrypt", &json!({"ciphertext":ciphertext,"context":context}))?;
        let encoded = response.get("plaintext").and_then(Value::as_str)
            .ok_or(KmsError::InvalidResponse)?;
        let plaintext = STANDARD.decode(encoded).map_err(|_| KmsError::InvalidResponse)?;
        if plaintext.is_empty() || plaintext.len() > MAX_PLAINTEXT {
            return Err(KmsError::InvalidResponse);
        }
        Ok(plaintext)
    }

    /// Vault rewrap rotates ciphertext to the newest key version without
    /// returning plaintext to the caller.
    pub fn rewrap(
        &self, tenant: &str, user: &str, provider: &str, ciphertext: &str,
    ) -> Result<String, KmsError> {
        if !valid_ciphertext(ciphertext) { return Err(KmsError::Configuration); }
        let context = context(tenant,user,provider)?;
        let response = self.call("rewrap", &json!({"ciphertext":ciphertext,"context":context}))?;
        let result = response.get("ciphertext").and_then(Value::as_str)
            .ok_or(KmsError::InvalidResponse)?;
        if !valid_ciphertext(result) { return Err(KmsError::InvalidResponse); }
        Ok(result.to_owned())
    }

    fn call(&self, operation: &str, body: &Value) -> Result<Value, KmsError> {
        let mut route = self.route.clone();
        route.path = match operation {
            "encrypt" | "decrypt" | "rewrap" =>
                format!("/v1/transit/{operation}/{}",self.key_name),
            _ => return Err(KmsError::Configuration),
        };
        route.validate().map_err(|_| KmsError::Configuration)?;
        let mut token = crate::ops::secrets::read_private_text(&self.token_file, 4096)
            .map_err(|_| KmsError::Unavailable)?.into_bytes();
        let data = serde_json::to_vec(body).map_err(|_| KmsError::Configuration)?;
        let response = https_post_with_auth(&route, &token, &data, BrokerAuthHeader::VaultToken);
        token.fill(0);
        let response = response.map_err(|_: BrokerError| KmsError::Unavailable)?;
        if response.status != 200 { return Err(KmsError::Unavailable); }
        let json: Value = serde_json::from_slice(&response.body)
            .map_err(|_| KmsError::InvalidResponse)?;
        json.get("data").filter(|v| v.is_object())
            .cloned().ok_or(KmsError::InvalidResponse)
    }
}

fn safe_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 128 && value != "." && value != ".."
        && value.bytes().all(|b| b.is_ascii_alphanumeric() || b"_-.".contains(&b))
}

fn context(tenant: &str, user: &str, provider: &str) -> Result<String, KmsError> {
    if !safe_id(tenant) || !safe_id(user) || !safe_id(provider) {
        return Err(KmsError::Configuration);
    }
    Ok(STANDARD.encode(format!("ocaf:044:vault:v1\0{tenant}\0{user}\0{provider}")))
}

fn valid_ciphertext(value: &str) -> bool {
    value.starts_with("vault:v") && value.len() <= MAX_CIPHERTEXT
        && value.len() > 12 && !value.bytes().any(|b| b <= 32 || b >= 127)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;
    fn route(ip: Ipv4Addr) -> BrokerRoute {
        BrokerRoute {
            route_id:"vault-kms".into(),tool_id:"kms".into(),
            hostname:"vault.example.com".into(),pinned_ip:ip,port:443,
            path:"/v1/transit/encrypt/ocaf-stage044".into(),
            resource:"https://omat.example".into(),
            required_scopes:["kms:use".into()].into_iter().collect(),
            min_assurance:1,
        }
    }
    #[test]
    fn rejects_private_kms_and_missing_operator_credentials() {
        let missing = Path::new("/does-not-exist/ocaf-vault-token");
        assert!(matches!(VaultTransit::new(route(Ipv4Addr::new(127,0,0,1)),
            "ocaf-stage044".into(),missing),Err(KmsError::Configuration)));
        assert!(matches!(VaultTransit::new(route(Ipv4Addr::new(8,8,8,8)),
            "ocaf-stage044".into(),missing),Err(KmsError::Configuration)));
        assert!(context("tenant-a","alice","provider-a").is_ok());
        assert!(context("tenant-a","alice","../metadata").is_err());
        assert!(!valid_ciphertext("http://169.254.169.254"));
    }
}
