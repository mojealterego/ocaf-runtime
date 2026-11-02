//! Stage 0.52: authenticated, side-effect-free Cloud Builder application boundary.
//!
//! This is a reusable service layer, not an HTTP listener or a deployed Cloud
//! product. The subject MUST come from an authenticated ingress session. Every
//! operation revalidates the opaque OAuth access token against the exact tenant,
//! user, audience and scope; the request body cannot select an identity.
//! Publishing and agent execution are intentionally not exposed.
use crate::builder_core::{compile, BuilderError, BuilderProject, CompiledPlan};
use crate::builder_store::{BuilderHead, BuilderStoreError, SqliteBuilderStore};
use crate::stage044_oauth::{AuthenticatedSubject, OAuthError, OAuthService};
use std::collections::BTreeSet;
use thiserror::Error;

const READ_SCOPE: &str = "builder:read";
const WRITE_SCOPE: &str = "builder:write";

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BuilderCloudError {
    #[error("invalid builder cloud audience")]
    Configuration,
    #[error("builder cloud OAuth authorization rejected: {0}")]
    OAuth(#[from] OAuthError),
    #[error("builder cloud project storage rejected: {0}")]
    Store(#[from] BuilderStoreError),
    #[error("builder cloud graph rejected: {0}")]
    Builder(#[from] BuilderError),
}

pub struct BuilderCloudService<'a> {
    oauth: &'a OAuthService,
    store: &'a SqliteBuilderStore,
    audience: String,
}

impl<'a> BuilderCloudService<'a> {
    pub fn new(
        oauth: &'a OAuthService,
        store: &'a SqliteBuilderStore,
        audience: &str,
    ) -> Result<Self, BuilderCloudError> {
        if !audience.starts_with("https://")
            || audience.len() > 2048
            || audience
                .bytes()
                .any(|byte| byte <= 32 || byte == b'\\' || byte == b'#')
            || audience[8..]
                .split(['/', '?'])
                .next()
                .unwrap_or("")
                .is_empty()
        {
            return Err(BuilderCloudError::Configuration);
        }
        Ok(Self {
            oauth,
            store,
            audience: audience.to_owned(),
        })
    }

    fn authorize(
        &self,
        token: &str,
        trusted_subject: &AuthenticatedSubject,
        scope: &str,
    ) -> Result<(), BuilderCloudError> {
        let required: BTreeSet<String> = [scope.to_owned()].into_iter().collect();
        self.oauth.validate_access(
            token,
            &trusted_subject.tenant,
            &trusted_subject.user,
            &self.audience,
            &required,
            1,
        )?;
        Ok(())
    }

    /// Validate an inert DAG without invoking a model, tool or sandbox.
    pub fn preview(
        &self,
        token: &str,
        trusted_subject: &AuthenticatedSubject,
        project: &BuilderProject,
    ) -> Result<CompiledPlan, BuilderCloudError> {
        self.authorize(token, trusted_subject, READ_SCOPE)?;
        Ok(compile(project, &trusted_subject.tenant)?)
    }

    /// Append a project revision under OAuth write scope and SQLite CAS.
    pub fn save(
        &self,
        token: &str,
        trusted_subject: &AuthenticatedSubject,
        project: &BuilderProject,
        expected: Option<&BuilderHead>,
    ) -> Result<BuilderHead, BuilderCloudError> {
        self.authorize(token, trusted_subject, WRITE_SCOPE)?;
        Ok(self
            .store
            .save(project, &trusted_subject.tenant, expected)?)
    }

    /// Read a fully verified project chain with optional externally pinned head.
    pub fn load(
        &self,
        token: &str,
        trusted_subject: &AuthenticatedSubject,
        project_id: &str,
        expected: Option<&BuilderHead>,
    ) -> Result<Option<(BuilderProject, BuilderHead)>, BuilderCloudError> {
        self.authorize(token, trusted_subject, READ_SCOPE)?;
        Ok(self
            .store
            .load(&trusted_subject.tenant, project_id, expected)?)
    }

    /// Return revision heads only; no unscoped tenant listing is available.
    pub fn history(
        &self,
        token: &str,
        trusted_subject: &AuthenticatedSubject,
        project_id: &str,
    ) -> Result<Vec<BuilderHead>, BuilderCloudError> {
        self.authorize(token, trusted_subject, READ_SCOPE)?;
        Ok(self.store.history(&trusted_subject.tenant, project_id)?)
    }
}
