//! Identity types for projects, workspaces, and cycles.
//!
//! Provides stable identification independent of filesystem paths or remote URL variations.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use thiserror::Error;
use uuid::Uuid;

/// Errors that can occur during identity operations.
#[derive(Debug, Error)]
pub enum IdentityError {
    /// A project identifier does not satisfy the canonical format.
    #[error("invalid project ID format: {0}")]
    InvalidProjectId(String),
    /// A workspace identifier is empty or invalid.
    #[error("invalid workspace ID format: {0}")]
    InvalidWorkspaceId(String),
    /// A cycle identifier does not satisfy the canonical format.
    #[error("invalid cycle ID format: {0}")]
    InvalidCycleId(String),
    /// A remote URL is empty or cannot be normalized.
    #[error("empty or invalid remote URL")]
    InvalidRemoteUrl,
    /// Stable project identity was requested without a scope.
    #[error("scope is required for project identity")]
    MissingScope,
    /// A monorepo scope is unsafe or cannot be normalized.
    #[error("invalid project scope: {0}")]
    InvalidScope(String),
    /// A fallback identity seed is absent or is not a UUID.
    #[error("fallback seed must be a valid UUID")]
    InvalidFallbackSeed,
}

/// A globally unique project identifier.
///
/// Derived from normalized remote URL + scope, providing stable identity
/// regardless of local checkout location or remote URL variations.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectId(String);

impl ProjectId {
    /// Creates a new ProjectId after validating the format.
    pub fn new(id: impl Into<String>) -> Result<Self, IdentityError> {
        let id = id.into();
        if id.is_empty() {
            return Err(IdentityError::InvalidProjectId("cannot be empty".into()));
        }
        if !Regex::new(r"^[a-zA-Z][a-zA-Z0-9_-]*$")
            .unwrap()
            .is_match(&id)
        {
            return Err(IdentityError::InvalidProjectId(id));
        }
        Ok(Self(id))
    }

    /// Creates a ProjectId from a hash that may start with a digit.
    /// Use this only for computed stable IDs, not for user-provided IDs.
    pub fn from_hash_prefix(id: impl Into<String>) -> Result<Self, IdentityError> {
        Self::new(id)
    }

    /// Returns the underlying string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Source material used to derive a logical project identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentitySource {
    /// The identity was derived from a canonical Git remote and scope.
    Remote,
    /// The identity was derived from a caller-supplied stable UUID and scope.
    Fallback,
}

/// Fully resolved deterministic project identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ResolvedProjectIdentity {
    /// Stable logical project identifier.
    pub project_id: ProjectId,
    /// Canonical transport-neutral remote representation, when available.
    pub remote_url: Option<String>,
    /// Canonical monorepo scope.
    pub scope: String,
    /// Material selected for identity derivation.
    pub identity_source: IdentitySource,
    /// Canonical UUID used by fallback identity, when applicable.
    pub fallback_seed: Option<String>,
}

/// Knowledge profile persisted at adoption time.
///
/// This is the single source of truth for the canonical knowledge vault path.
/// The vault path is selected at adoption time and stored here so it remains
/// stable even if the checkout is renamed or moved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct KnowledgeProfile {
    /// Stable project identifier derived from remote URL or fallback seed.
    pub project_id: ProjectId,
    /// Human-readable project name (basename of the adopted checkout root).
    pub project_name: String,
    /// Canonical knowledge vault path under `$HOME/.sddk-knowledge/`.
    pub vault_path: PathBuf,
    /// Whether optional Engram memory integration is enabled.
    pub engram_enabled: bool,
}

impl fmt::Display for ProjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<ProjectId> for String {
    fn from(id: ProjectId) -> Self {
        id.0
    }
}

/// A workspace-specific identifier.
///
/// Uniquely identifies a checkout or worktree within a project.
/// Changes if the project is checked out to a different path.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspaceId(String);

impl WorkspaceId {
    /// Creates a new WorkspaceId after validating the format.
    pub fn new(id: impl Into<String>) -> Result<Self, IdentityError> {
        let id = id.into();
        if id.is_empty()
            || !Regex::new(r"^[a-zA-Z][a-zA-Z0-9_-]*$")
                .unwrap()
                .is_match(&id)
        {
            return Err(IdentityError::InvalidWorkspaceId("cannot be empty".into()));
        }
        Ok(Self(id))
    }

    /// Returns the underlying string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WorkspaceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<WorkspaceId> for String {
    fn from(id: WorkspaceId) -> Self {
        id.0
    }
}

/// A cycle identifier within a project.
///
/// Format: {project_id}/{cycle_name}
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CycleId(String);

impl CycleId {
    /// Creates a new CycleId after validating the format.
    pub fn new(id: impl Into<String>) -> Result<Self, IdentityError> {
        let id = id.into();
        if id.is_empty() {
            return Err(IdentityError::InvalidCycleId("cannot be empty".into()));
        }
        if !Regex::new(r"^[a-zA-Z][a-zA-Z0-9_-]+/[a-z][a-z0-9_-]*$")
            .unwrap()
            .is_match(&id)
        {
            return Err(IdentityError::InvalidCycleId(id));
        }
        Ok(Self(id))
    }

    /// Creates a CycleId from project and cycle name.
    pub fn from_parts(project: &ProjectId, cycle_name: &str) -> Result<Self, IdentityError> {
        if cycle_name.is_empty() {
            return Err(IdentityError::InvalidCycleId(
                "cycle name cannot be empty".into(),
            ));
        }
        Self::new(format!("{}/{}", project, cycle_name))
    }

    /// Returns the underlying string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the project portion of the cycle ID.
    pub fn project(&self) -> &str {
        self.0.split('/').next().unwrap_or(&self.0)
    }

    /// Returns the cycle name portion (after the slash).
    pub fn cycle_name(&self) -> &str {
        self.0.split('/').nth(1).unwrap_or(&self.0)
    }
}

impl fmt::Display for CycleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<CycleId> for String {
    fn from(id: CycleId) -> Self {
        id.0
    }
}

/// Normalizes a remote URL to a canonical form.
///
/// Strips transport credentials, query/fragment suffixes, trailing `.git`, and
/// normalizes HTTPS, `ssh://`, and SCP-style remotes to one HTTPS-shaped form.
pub fn normalize_remote_url(url: &str) -> Result<String, IdentityError> {
    let url = url.trim();
    if url.is_empty() || url.chars().any(char::is_whitespace) {
        return Err(IdentityError::InvalidRemoteUrl);
    }

    let without_suffix = url
        .split(['?', '#'])
        .next()
        .ok_or(IdentityError::InvalidRemoteUrl)?
        .trim_end_matches('/');
    let (scheme, authority, path) = if let Some((scheme, rest)) = without_suffix.split_once("://") {
        if !scheme.eq_ignore_ascii_case("https") && !scheme.eq_ignore_ascii_case("ssh") {
            return Err(IdentityError::InvalidRemoteUrl);
        }
        let (authority, path) = rest
            .split_once('/')
            .ok_or(IdentityError::InvalidRemoteUrl)?;
        (scheme, authority, path)
    } else {
        let (authority, path) = without_suffix
            .split_once(':')
            .ok_or(IdentityError::InvalidRemoteUrl)?;
        if authority.contains('/') || authority.is_empty() {
            return Err(IdentityError::InvalidRemoteUrl);
        }
        ("scp", authority, path)
    };

    let authority = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    let authority = normalize_authority(authority, scheme)?;
    let path = normalize_remote_path(path)?;
    Ok(format!("https://{authority}/{path}"))
}

fn normalize_authority(authority: &str, scheme: &str) -> Result<String, IdentityError> {
    if authority.is_empty() {
        return Err(IdentityError::InvalidRemoteUrl);
    }
    let (host, port) = if authority.starts_with('[') {
        let closing = authority.find(']').ok_or(IdentityError::InvalidRemoteUrl)?;
        let host = &authority[..=closing];
        let remainder = &authority[closing + 1..];
        let port = if remainder.is_empty() {
            None
        } else {
            Some(
                remainder
                    .strip_prefix(':')
                    .ok_or(IdentityError::InvalidRemoteUrl)?,
            )
        };
        (host, port)
    } else if let Some((host, port)) = authority.rsplit_once(':') {
        if port.chars().all(|character| character.is_ascii_digit()) {
            (host, Some(port))
        } else {
            (authority, None)
        }
    } else {
        (authority, None)
    };
    if host.is_empty() || port.is_some_and(|port| port.is_empty()) {
        return Err(IdentityError::InvalidRemoteUrl);
    }
    let default_port = match scheme.to_ascii_lowercase().as_str() {
        "https" => Some("443"),
        "ssh" => Some("22"),
        _ => None,
    };
    let host = host.to_ascii_lowercase();
    match port.filter(|port| Some(*port) != default_port) {
        Some(port) => Ok(format!("{host}:{port}")),
        None => Ok(host),
    }
}

fn normalize_remote_path(path: &str) -> Result<String, IdentityError> {
    let path = path.trim_matches('/');
    let path = path
        .strip_suffix(".git")
        .unwrap_or(path)
        .trim_end_matches('/');
    if path.is_empty()
        || path
            .split('/')
            .any(|segment| segment.is_empty() || matches!(segment, "." | ".."))
    {
        return Err(IdentityError::InvalidRemoteUrl);
    }
    Ok(path.to_owned())
}

/// Normalizes and validates a required monorepo scope.
pub fn normalize_scope(scope: &str) -> Result<String, IdentityError> {
    let scope = scope.trim().replace('\\', "/");
    if scope.is_empty() {
        return Err(IdentityError::MissingScope);
    }
    if scope == "." {
        return Ok(scope);
    }
    if scope.starts_with('/') {
        return Err(IdentityError::InvalidScope(scope));
    }
    let segments = scope
        .trim_matches('/')
        .split('/')
        .filter(|segment| *segment != ".")
        .collect::<Vec<_>>();
    if segments.is_empty()
        || segments
            .iter()
            .any(|segment| segment.is_empty() || *segment == "..")
    {
        return Err(IdentityError::InvalidScope(scope));
    }
    Ok(segments.join("/"))
}

/// Resolves project identity from either a remote or a stable fallback UUID.
pub fn resolve_project_identity(
    remote_url: Option<&str>,
    scope: &str,
    fallback_seed: Option<&str>,
) -> Result<ResolvedProjectIdentity, IdentityError> {
    let scope = normalize_scope(scope)?;
    match (remote_url, fallback_seed) {
        (Some(remote), None) => {
            let remote_url = normalize_remote_url(remote)?;
            let project_id = ProjectId::new(stable_project_id(&remote_url, &scope))?;
            Ok(ResolvedProjectIdentity {
                project_id,
                remote_url: Some(remote_url),
                scope,
                identity_source: IdentitySource::Remote,
                fallback_seed: None,
            })
        }
        (None, Some(seed)) => {
            let seed = Uuid::parse_str(seed).map_err(|_| IdentityError::InvalidFallbackSeed)?;
            let fallback_seed = seed.hyphenated().to_string();
            let project_id = ProjectId::new(stable_fallback_project_id(&fallback_seed, &scope))?;
            Ok(ResolvedProjectIdentity {
                project_id,
                remote_url: None,
                scope,
                identity_source: IdentitySource::Fallback,
                fallback_seed: Some(fallback_seed),
            })
        }
        _ => Err(IdentityError::InvalidFallbackSeed),
    }
}

/// Computes a stable project identifier from a normalized remote URL and scope.
///
/// The scope is typically the owner/organization or a unique context identifier.
/// This ensures that forks or multiple remotes don't collide.
/// Returns a ProjectId-compatible string prefixed with "p-" so it always starts
/// with a letter and is valid for use as a ProjectId.
pub fn stable_project_id(normalized_remote: &str, scope: &str) -> String {
    let hex = framed_hash("sddk.project.remote.v1", &[normalized_remote, scope]);
    format!("p-{}", &hex[..16])
}

/// Computes a stable project identifier from a fallback UUID and scope.
pub fn stable_fallback_project_id(fallback_seed: &str, scope: &str) -> String {
    let hex = framed_hash("sddk.project.fallback.v1", &[fallback_seed, scope]);
    format!("p-{}", &hex[..16])
}

/// Computes a stable workspace identifier from project ID and canonical filesystem path.
pub fn stable_workspace_id(project: &ProjectId, canonical_path: &str) -> String {
    let hex = framed_hash("sddk.workspace.v1", &[project.as_str(), canonical_path]);
    format!("w-{}", &hex[..24])
}

fn framed_hash(domain: &str, parts: &[&str]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(domain.len().to_be_bytes());
    hasher.update(domain.as_bytes());
    for part in parts {
        hasher.update(part.len().to_be_bytes());
        hasher.update(part.as_bytes());
    }
    let hash = hasher.finalize();
    format!("{hash:x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_id_valid() {
        let id = ProjectId::new("my-project").unwrap();
        assert_eq!(id.as_str(), "my-project");
        assert_eq!(id.to_string(), "my-project");
    }

    #[test]
    fn test_project_id_invalid_starts_with_number() {
        let result = ProjectId::new("123-project");
        assert!(result.is_err());
    }

    #[test]
    fn test_project_id_from_hash() {
        // stable_project_id returns p-* prefix so it always starts with letter
        let stable = stable_project_id("https://github.com/owner/repo", "owner");
        assert!(stable.starts_with("p-"));
        // Should be usable as ProjectId
        let id = ProjectId::from_hash_prefix(&stable).unwrap();
        assert!(id.as_str().starts_with("p-"));
    }

    #[test]
    fn test_cycle_id_parts() {
        let id = CycleId::new("my-project/add-oauth").unwrap();
        assert_eq!(id.project(), "my-project");
        assert_eq!(id.cycle_name(), "add-oauth");
    }

    #[test]
    fn test_cycle_id_from_parts() {
        let project = ProjectId::new("my-project").unwrap();
        let id = CycleId::from_parts(&project, "add-oauth").unwrap();
        assert_eq!(id.as_str(), "my-project/add-oauth");
    }

    #[test]
    fn test_normalize_remote_url_https() {
        let url = "https://github.com/owner/repo.git";
        let normalized = normalize_remote_url(url).unwrap();
        assert_eq!(normalized, "https://github.com/owner/repo");
    }

    #[test]
    fn test_normalize_remote_url_ssh() {
        let url = "git@github.com:owner/repo.git";
        let normalized = normalize_remote_url(url).unwrap();
        assert_eq!(normalized, "https://github.com/owner/repo");
    }

    #[test]
    fn common_remote_forms_are_equivalent() {
        let forms = [
            "https://GitHub.COM/owner/repo.git/",
            "https://github.com:443/owner/repo",
            "ssh://git@github.com/owner/repo.git",
            "ssh://git@github.com:22/owner/repo",
            "git@github.com:owner/repo.git",
        ];
        let normalized = forms.map(normalize_remote_url).map(Result::unwrap);
        assert!(normalized.iter().all(|remote| remote == &normalized[0]));
        assert_eq!(normalized[0], "https://github.com/owner/repo");
    }

    #[test]
    fn rejects_unsupported_or_unsafe_remote_forms() {
        for remote in [
            "http://github.com/owner/repo",
            "file:///tmp/repo",
            "git@github.com:owner/../repo",
            "https://github.com/owner repo",
        ] {
            assert!(normalize_remote_url(remote).is_err(), "accepted {remote}");
        }
    }

    #[test]
    fn test_normalize_remote_url_with_fragment() {
        let url = "https://github.com/owner/repo#main";
        let normalized = normalize_remote_url(url).unwrap();
        assert_eq!(normalized, "https://github.com/owner/repo");
    }

    #[test]
    fn test_normalize_remote_url_no_git_suffix() {
        let url = "https://github.com/owner/repo";
        let normalized = normalize_remote_url(url).unwrap();
        assert_eq!(normalized, "https://github.com/owner/repo");
    }

    #[test]
    fn test_stable_project_id_https_vs_ssh_equivalent() {
        // HTTPS and SSH forms of the same repo should produce the same stable ID
        let https = normalize_remote_url("https://github.com/owner/repo.git").unwrap();
        let ssh = normalize_remote_url("git@github.com:owner/repo.git").unwrap();
        assert_eq!(https, ssh);

        let id_https = stable_project_id(&https, "owner");
        let id_ssh = stable_project_id(&ssh, "owner");
        assert_eq!(id_https, id_ssh);
    }

    #[test]
    fn test_stable_project_id_different_scopes() {
        let remote = "https://github.com/owner/repo";
        let id_owner = stable_project_id(remote, "owner");
        let id_other = stable_project_id(remote, "other");
        assert_ne!(id_owner, id_other);
    }

    #[test]
    fn test_stable_project_id_different_remotes_same_scope() {
        // Same scope but different repos should produce different IDs
        let id_repo1 = stable_project_id("https://github.com/owner/repo1", "owner");
        let id_repo2 = stable_project_id("https://github.com/owner/repo2", "owner");
        assert_ne!(id_repo1, id_repo2);
    }

    #[test]
    fn test_stable_project_id_deterministic() {
        let remote = "https://github.com/owner/repo";
        let id1 = stable_project_id(remote, "owner");
        let id2 = stable_project_id(remote, "owner");
        assert_eq!(id1, id2);
    }

    #[test]
    fn project_hash_frames_remote_and_scope() {
        assert_ne!(stable_project_id("ab", "c"), stable_project_id("a", "bc"));
    }

    #[test]
    fn fallback_identity_requires_and_canonicalizes_uuid_seed() {
        let identity = resolve_project_identity(
            None,
            "crates/./engine/",
            Some("A0B1C2D3-E4F5-4678-9ABC-DEF012345678"),
        )
        .unwrap();
        assert_eq!(identity.identity_source, IdentitySource::Fallback);
        assert_eq!(identity.scope, "crates/engine");
        assert_eq!(
            identity.fallback_seed.as_deref(),
            Some("a0b1c2d3-e4f5-4678-9abc-def012345678")
        );
        assert!(resolve_project_identity(None, ".", Some("not-a-uuid")).is_err());
    }

    #[test]
    fn test_stable_workspace_id() {
        let project = ProjectId::new("test-project").unwrap();
        let ws1 = stable_workspace_id(&project, "/home/user/project");
        let ws2 = stable_workspace_id(&project, "/home/user/project");
        assert_eq!(ws1, ws2);
    }

    #[test]
    fn test_stable_workspace_id_different_paths() {
        let project = ProjectId::new("test-project").unwrap();
        let ws1 = stable_workspace_id(&project, "/home/user/project");
        let ws2 = stable_workspace_id(&project, "/home/user/other-project");
        assert_ne!(ws1, ws2);
    }

    #[test]
    fn workspace_hash_frames_project_and_path() {
        let first = ProjectId::new("ab").unwrap();
        let second = ProjectId::new("a").unwrap();
        assert_ne!(
            stable_workspace_id(&first, "c"),
            stable_workspace_id(&second, "bc")
        );
    }
}
