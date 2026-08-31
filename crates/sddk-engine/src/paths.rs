//! Explicit XDG path resolution for project adoption.

use std::path::{Path, PathBuf};

use sddk_domain::{AdoptionStoragePaths, ProjectId, WorkspaceId};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Explicit environment values used by XDG path resolution.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct XdgEnvironment {
    /// Home directory used only for missing XDG overrides.
    pub home: Option<PathBuf>,
    /// Optional `XDG_DATA_HOME` override.
    pub data_home: Option<PathBuf>,
    /// Optional `SDDK_DATA_DIR` override (takes precedence over `XDG_DATA_HOME`
    /// for the data root; all framework state lives under it).
    pub sddk_data_dir: Option<PathBuf>,
    /// Optional `XDG_STATE_HOME` override.
    pub state_home: Option<PathBuf>,
    /// Optional `XDG_CACHE_HOME` override.
    pub cache_home: Option<PathBuf>,
}

/// Fully resolved absolute paths for one project workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AdoptionPaths {
    /// Project-scoped XDG data directory.
    pub project_data: PathBuf,
    /// Persisted knowledge profile.
    pub knowledge_profile: PathBuf,
    /// Project-shared artifact directory.
    pub artifacts: PathBuf,
    /// Project-shared cycle artifact directory.
    pub cycle_artifacts: PathBuf,
    /// Project-shared generated docs directory.
    pub generated: PathBuf,
    /// Project-shared SQLite database.
    pub ledger: PathBuf,
    /// SDDK-wide cache directory.
    pub cache: PathBuf,
    /// Workspace-specific adoption receipt.
    pub receipt: PathBuf,
}

impl AdoptionPaths {
    /// Converts paths to the receipt wire representation after UTF-8 validation.
    pub fn to_storage_paths(
        &self,
        knowledge_vault: &Path,
    ) -> Result<AdoptionStoragePaths, PathResolutionError> {
        Ok(AdoptionStoragePaths {
            vault: path_string(knowledge_vault)?,
            artifacts: path_string(&self.artifacts)?,
            cycle_artifacts: path_string(&self.cycle_artifacts)?,
            generated: path_string(&self.generated)?,
            ledger: path_string(&self.ledger)?,
            cache: path_string(&self.cache)?,
            receipt: path_string(&self.receipt)?,
        })
    }
}

/// Errors emitted while resolving XDG storage paths.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PathResolutionError {
    /// A supplied home or XDG directory is not absolute.
    #[error("{variable} must be an absolute path: {path:?}")]
    NonAbsolute {
        /// Environment variable represented by the value.
        variable: &'static str,
        /// Rejected path.
        path: PathBuf,
    },
    /// An XDG fallback was needed but no home directory was supplied.
    #[error("HOME is required when an XDG directory is not set")]
    MissingHome,
    /// A project or workspace identifier is unsafe for path construction.
    #[error("unsafe identity component: {0}")]
    UnsafeIdentity(String),
    /// A path cannot be represented in the JSON receipt.
    #[error("path is not valid UTF-8: {0:?}")]
    NonUtf8Path(PathBuf),
}

/// Resolves project and workspace storage paths without reading process state.
pub fn resolve_xdg_paths(
    environment: &XdgEnvironment,
    project_id: &str,
    workspace_id: &str,
) -> Result<AdoptionPaths, PathResolutionError> {
    ProjectId::new(project_id).map_err(|_| unsafe_identity(project_id))?;
    WorkspaceId::new(workspace_id).map_err(|_| unsafe_identity(workspace_id))?;
    validate_optional("HOME", environment.home.as_deref())?;
    validate_optional("XDG_DATA_HOME", environment.data_home.as_deref())?;
    validate_optional("SDDK_DATA_DIR", environment.sddk_data_dir.as_deref())?;
    validate_optional("XDG_STATE_HOME", environment.state_home.as_deref())?;
    validate_optional("XDG_CACHE_HOME", environment.cache_home.as_deref())?;

    let data_home = resolve_base(
        environment
            .sddk_data_dir
            .as_deref()
            .or(environment.data_home.as_deref()),
        environment.home.as_deref(),
        ".local/share",
        dirs::data_dir(),
    )?;
    let state_home = resolve_base(
        environment.state_home.as_deref(),
        environment.home.as_deref(),
        ".local/state",
        dirs::state_dir(),
    )?;
    let cache_home = resolve_base(
        environment.cache_home.as_deref(),
        environment.home.as_deref(),
        ".cache",
        dirs::cache_dir(),
    )?;
    let project_data = data_home.join("sddk/projects").join(project_id);
    let project_state = state_home.join("sddk/projects").join(project_id);
    Ok(AdoptionPaths {
        knowledge_profile: project_data.join("knowledge-profile.json"),
        project_data: project_data.clone(),
        artifacts: project_data.join("artifacts"),
        cycle_artifacts: project_data.join("cycle-artifacts"),
        generated: project_data.join("generated"),
        ledger: project_state.join("ledger.sqlite"),
        cache: cache_home.join("sddk"),
        receipt: project_data
            .join("workspaces")
            .join(workspace_id)
            .join("adoption.json"),
    })
}

fn validate_optional(
    variable: &'static str,
    value: Option<&Path>,
) -> Result<(), PathResolutionError> {
    if let Some(path) = value
        && !path.is_absolute()
    {
        return Err(PathResolutionError::NonAbsolute {
            variable,
            path: path.to_path_buf(),
        });
    }
    Ok(())
}

/// Resolution order for a base directory:
/// 1. Explicit override (XDG_* or SDDK_DATA_DIR).
/// 2. `HOME` fallback for the given subdirectory (Unix convention).
/// 3. Platform dir via the `dirs` crate (macOS `~/Library/...`, Windows
///    `%APPDATA%`/`%LOCALAPPDATA%`) — required where `HOME` does not exist.
fn resolve_base(
    override_path: Option<&Path>,
    home: Option<&Path>,
    fallback: &str,
    platform_dir: Option<PathBuf>,
) -> Result<PathBuf, PathResolutionError> {
    override_path
        .map(Path::to_path_buf)
        .or_else(|| home.map(|home| home.join(fallback)))
        .or(platform_dir)
        .ok_or(PathResolutionError::MissingHome)
}

fn path_string(path: &Path) -> Result<String, PathResolutionError> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| PathResolutionError::NonUtf8Path(path.to_path_buf()))
}

fn unsafe_identity(identity: &str) -> PathResolutionError {
    PathResolutionError::UnsafeIdentity(identity.to_owned())
}

/// Resolve the path of a project's UAT config (`uat.toml`) under the
/// XDG data root (ADR-0011 compliant — no files written into the project repo).
/// Returns a path even if the file does not exist yet; callers should create
/// the parent dir on first save.
pub fn uat_config_path(
    environment: &XdgEnvironment,
    project_id: &str,
) -> Result<PathBuf, PathResolutionError> {
    ProjectId::new(project_id).map_err(|_| unsafe_identity(project_id))?;
    let paths = resolve_xdg_paths(environment, project_id, "default")?;
    Ok(paths.project_data.join("uat.toml"))
}

/// Resolve the XDG base directory for a project's UAT state.
pub fn uat_storage_root(
    environment: &XdgEnvironment,
    project_id: &str,
) -> Result<PathBuf, PathResolutionError> {
    ProjectId::new(project_id).map_err(|_| unsafe_identity(project_id))?;
    let paths = resolve_xdg_paths(environment, project_id, "default")?;
    Ok(paths.project_data.join("uat"))
}

/// Resolves the XDG profile path for one stable project identity.
pub fn knowledge_profile_path(
    environment: &XdgEnvironment,
    project_id: &str,
) -> Result<PathBuf, PathResolutionError> {
    Ok(resolve_xdg_paths(environment, project_id, "default")?.knowledge_profile)
}

/// Resolves the external canonical knowledge vault for a project name.
pub fn knowledge_vault_path(
    environment: &XdgEnvironment,
    project_id: &str,
    project_name: &str,
) -> Result<PathBuf, PathResolutionError> {
    ProjectId::new(project_id).map_err(|_| unsafe_identity(project_id))?;
    if project_name.is_empty()
        || project_name == "."
        || project_name == ".."
        || project_name.contains('/')
        || project_name.contains('\\')
    {
        return Err(unsafe_identity(project_name));
    }
    validate_optional("HOME", environment.home.as_deref())?;
    let home = environment
        .home
        .clone()
        .or_else(dirs::home_dir)
        .ok_or(PathResolutionError::MissingHome)?;
    let root = home.join(".sddk-knowledge");
    let legacy = root.join(project_name);
    Ok(if legacy.exists() {
        legacy
    } else {
        root.join(project_id)
    })
}

/// Resolve the manifest path for a project's UAT state.
pub fn uat_manifest_path(
    environment: &XdgEnvironment,
    project_id: &str,
) -> Result<PathBuf, PathResolutionError> {
    Ok(uat_storage_root(environment, project_id)?.join("manifest.yaml"))
}

/// Resolve the path of one evidence payload by content hash.
pub fn uat_evidence_path(
    environment: &XdgEnvironment,
    project_id: &str,
    sha256_ref: &str,
    ext: &str,
) -> Result<PathBuf, PathResolutionError> {
    ProjectId::new(project_id).map_err(|_| unsafe_identity(project_id))?;
    let bare = sha256_ref.strip_prefix("sha256:").unwrap_or(sha256_ref);
    if bare.len() < 2 {
        return Err(PathResolutionError::UnsafeIdentity(sha256_ref.to_string()));
    }
    let (prefix, rest) = bare.split_at(2);
    if ext.contains('/') || ext.contains('\\') || ext.contains("..") {
        return Err(PathResolutionError::UnsafeIdentity(ext.to_string()));
    }
    let root = uat_storage_root(environment, project_id)?;
    Ok(root
        .join("evidence")
        .join(prefix)
        .join(format!("{rest}.{ext}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_explicit_xdg_overrides() {
        let environment = XdgEnvironment {
            home: None,
            data_home: Some("/xdg/data".into()),
            state_home: Some("/xdg/state".into()),
            cache_home: Some("/xdg/cache".into()),
            ..XdgEnvironment::default()
        };
        let paths = resolve_xdg_paths(&environment, "p-project", "w-workspace").unwrap();
        assert_eq!(
            paths.project_data,
            Path::new("/xdg/data/sddk/projects/p-project")
        );
        assert_eq!(
            paths.ledger,
            Path::new("/xdg/state/sddk/projects/p-project/ledger.sqlite")
        );
        assert_eq!(paths.cache, Path::new("/xdg/cache/sddk"));
        assert_eq!(
            paths.receipt,
            Path::new("/xdg/data/sddk/projects/p-project/workspaces/w-workspace/adoption.json")
        );
    }

    #[test]
    fn sddk_data_dir_overrides_data_home() {
        let environment = XdgEnvironment {
            home: None,
            data_home: Some("/xdg/data".into()),
            sddk_data_dir: Some("/sddk-root".into()),
            state_home: Some("/xdg/state".into()),
            cache_home: Some("/xdg/cache".into()),
        };
        let paths = resolve_xdg_paths(&environment, "p-project", "w-workspace").unwrap();
        assert_eq!(
            paths.project_data,
            Path::new("/sddk-root/sddk/projects/p-project")
        );
        assert_eq!(
            paths.receipt,
            Path::new("/sddk-root/sddk/projects/p-project/workspaces/w-workspace/adoption.json")
        );
    }

    #[test]
    fn falls_back_to_home_for_each_missing_override() {
        let environment = XdgEnvironment {
            home: Some("/home/tester".into()),
            ..XdgEnvironment::default()
        };
        let paths = resolve_xdg_paths(&environment, "p-project", "w-workspace").unwrap();
        assert_eq!(
            paths.artifacts,
            Path::new("/home/tester/.local/share/sddk/projects/p-project/artifacts")
        );
        assert_eq!(
            paths.ledger,
            Path::new("/home/tester/.local/state/sddk/projects/p-project/ledger.sqlite")
        );
        assert_eq!(paths.cache, Path::new("/home/tester/.cache/sddk"));
    }

    #[test]
    fn falls_back_to_platform_dirs_without_home() {
        // Simulates macOS/Windows where HOME may not exist: resolution must
        // fall back to `dirs` platform directories instead of failing.
        let environment = XdgEnvironment::default();
        let paths = resolve_xdg_paths(&environment, "p-project", "w-workspace").unwrap();
        assert!(paths.project_data.is_absolute());
        assert!(paths.artifacts.is_absolute());
        assert!(paths.ledger.is_absolute());
        assert!(paths.cache.is_absolute());
        assert!(paths.project_data.ends_with("sddk/projects/p-project"));
        assert!(
            paths
                .ledger
                .ends_with("sddk/projects/p-project/ledger.sqlite")
        );
        assert!(paths.cache.ends_with("sddk"));
    }

    #[test]
    fn rejects_relative_and_unsafe_inputs() {
        let relative = XdgEnvironment {
            home: Some("relative".into()),
            ..XdgEnvironment::default()
        };
        assert!(matches!(
            resolve_xdg_paths(&relative, "p-project", "w-workspace"),
            Err(PathResolutionError::NonAbsolute {
                variable: "HOME",
                ..
            })
        ));
        let absolute = XdgEnvironment {
            home: Some("/home/tester".into()),
            ..XdgEnvironment::default()
        };
        assert!(matches!(
            resolve_xdg_paths(&absolute, "../escape", "w-workspace"),
            Err(PathResolutionError::UnsafeIdentity(_))
        ));
    }
}
