//! Pack registry lifecycle (SPEC-006 §7, ADR-006).
//!
//! Discovers declarative pack manifests under `<root>/packs/*/manifest.toml`,
//! verifies their dependencies, and persists enable/disable state in the
//! project XDG data area (ADR-0011). All operations are idempotent.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use sddk_domain::{load_pack_manifest, validate_pack_manifest};

/// Errors emitted by the pack registry.
#[derive(Debug, Error)]
pub enum PackRegistryError {
    /// The pack directory could not be scanned.
    #[error("failed to scan pack directory {path:?}: {source}")]
    Scan {
        /// Directory that failed to scan.
        path: PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },
    /// A pack manifest could not be loaded.
    #[error("failed to load pack manifest for {id}: {source}")]
    Load {
        /// Pack identifier.
        id: String,
        /// Underlying domain error.
        source: sddk_domain::PackError,
    },
    /// A pack with this identifier is not known to the registry.
    #[error("pack not found: {0}")]
    NotFound(String),
    /// A pack with this identifier is already installed.
    #[error("pack already installed: {0}")]
    AlreadyInstalled(String),
    /// The manifest failed validation.
    #[error("pack manifest invalid: {0}")]
    Invalid(String),
    /// A required dependency is not satisfied by any discovered pack.
    #[error("unsatisfied requirement for {pack}: {requirement}")]
    UnsatisfiedRequirement {
        /// Pack that declared the requirement.
        pack: String,
        /// Unmet requirement.
        requirement: String,
    },
    /// The registry state could not be read or written.
    #[error("registry state error: {0}")]
    State(String),
}

/// Registry entry for one discovered pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryEntry {
    /// Stable pack identifier.
    pub id: String,
    /// Pack version.
    pub version: String,
    /// Pack category.
    pub category: String,
    /// Whether the pack is enabled.
    pub enabled: bool,
    /// Path to the pack manifest.
    pub manifest_path: PathBuf,
}

/// Verification report for a pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VerifyReport {
    /// Pack identifier.
    pub id: String,
    /// Whether the pack verified cleanly.
    pub valid: bool,
    /// Diagnostics emitted by manifest validation.
    pub diagnostics: Vec<sddk_domain::PackDiagnostic>,
    /// Requirements that could not be satisfied.
    pub unsatisfied: Vec<String>,
}

/// Persisted registry state (enable/disable flags).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct RegistryState {
    /// Pack id → enabled flag.
    #[serde(default)]
    enabled: BTreeMap<String, bool>,
}

/// Pack registry operating on one project root.
#[derive(Debug, Clone)]
pub struct PackRegistry {
    /// Project root where `packs/` lives.
    root: PathBuf,
    /// XDG state path for enable/disable persistence.
    state_path: PathBuf,
}

impl PackRegistry {
    /// Creates a registry rooted at `root` with state persisted at `state_path`.
    pub fn new(root: impl Into<PathBuf>, state_path: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            state_path: state_path.into(),
        }
    }

    /// Discovers all packs under `<root>/packs/*/manifest.toml`.
    pub fn discover(&self) -> Result<Vec<RegistryEntry>, PackRegistryError> {
        let packs_dir = self.root.join("packs");
        let mut entries = Vec::new();
        if packs_dir.is_dir() {
            let read_dir = fs::read_dir(&packs_dir).map_err(|source| PackRegistryError::Scan {
                path: packs_dir.clone(),
                source,
            })?;
            for dir_entry in read_dir {
                let dir_entry = dir_entry.map_err(|source| PackRegistryError::Scan {
                    path: packs_dir.clone(),
                    source,
                })?;
                let dir_path = dir_entry.path();
                if !dir_path.is_dir() {
                    continue;
                }
                let manifest_path = dir_path.join("manifest.toml");
                if !manifest_path.is_file() {
                    continue;
                }
                let manifest = load_pack_manifest(&manifest_path).map_err(|source| {
                    PackRegistryError::Load {
                        id: dir_path
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_default(),
                        source,
                    }
                })?;
                entries.push(RegistryEntry {
                    id: manifest.pack.id.clone(),
                    version: manifest.pack.version.clone(),
                    category: format!("{:?}", manifest.pack.category).to_ascii_lowercase(),
                    enabled: self
                        .load_state()
                        .enabled
                        .get(&manifest.pack.id)
                        .copied()
                        .unwrap_or(true),
                    manifest_path,
                });
            }
        }
        entries.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(entries)
    }

    /// Finds a registry entry by pack id.
    pub fn find(&self, id: &str) -> Result<RegistryEntry, PackRegistryError> {
        self.discover()?
            .into_iter()
            .find(|entry| entry.id == id)
            .ok_or_else(|| PackRegistryError::NotFound(id.to_string()))
    }

    /// Verifies a pack manifest and its dependency satisfaction.
    pub fn verify(&self, id: &str) -> Result<VerifyReport, PackRegistryError> {
        let entry = self.find(id)?;
        let manifest =
            load_pack_manifest(&entry.manifest_path).map_err(|source| PackRegistryError::Load {
                id: id.to_string(),
                source,
            })?;
        let diagnostics = validate_pack_manifest(&manifest);
        let available: Vec<String> = self
            .discover()?
            .into_iter()
            .filter(|candidate| candidate.id != id)
            .flat_map(|candidate| {
                load_pack_manifest(&candidate.manifest_path)
                    .ok()
                    .and_then(|manifest| {
                        manifest
                            .provides
                            .as_ref()
                            .map(|provides| provides.capabilities.clone())
                    })
                    .unwrap_or_default()
            })
            .collect();
        let mut unsatisfied = Vec::new();
        for requirement in &manifest.dependencies.requires {
            if !available.iter().any(|capability| capability == requirement) {
                unsatisfied.push(requirement.clone());
            }
        }
        Ok(VerifyReport {
            id: id.to_string(),
            valid: diagnostics.is_empty() && unsatisfied.is_empty(),
            diagnostics,
            unsatisfied,
        })
    }

    /// Enables a pack. Idempotent: enabling an enabled pack is a no-op.
    pub fn enable(&self, id: &str) -> Result<(), PackRegistryError> {
        self.find(id)?;
        let mut state = self.load_state();
        state.enabled.insert(id.to_string(), true);
        self.save_state(&state)
    }

    /// Disables a pack. Idempotent: disabling a disabled pack is a no-op.
    pub fn disable(&self, id: &str) -> Result<(), PackRegistryError> {
        self.find(id)?;
        let mut state = self.load_state();
        state.enabled.insert(id.to_string(), false);
        self.save_state(&state)
    }

    /// Installs a pack from a local source directory, verifying its manifest
    /// before copying. Rejects duplicate ids.
    pub fn install(&self, source: &Path) -> Result<RegistryEntry, PackRegistryError> {
        let manifest_path = source.join("manifest.toml");
        if !manifest_path.is_file() {
            return Err(PackRegistryError::Invalid(format!(
                "no manifest.toml at {}",
                source.display()
            )));
        }
        let manifest =
            load_pack_manifest(&manifest_path).map_err(|source| PackRegistryError::Load {
                id: "install-source".to_string(),
                source,
            })?;
        let diagnostics = validate_pack_manifest(&manifest);
        if !diagnostics.is_empty() {
            return Err(PackRegistryError::Invalid(
                diagnostics
                    .iter()
                    .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.message))
                    .collect::<Vec<_>>()
                    .join("; "),
            ));
        }
        if self.find(&manifest.pack.id).is_ok() {
            return Err(PackRegistryError::AlreadyInstalled(
                manifest.pack.id.clone(),
            ));
        }
        let target = self.root.join("packs").join(&manifest.pack.id);
        fs::create_dir_all(&target)
            .map_err(|source| PackRegistryError::State(source.to_string()))?;
        copy_dir(source, &target).map_err(|source| PackRegistryError::State(source.to_string()))?;
        Ok(RegistryEntry {
            id: manifest.pack.id,
            version: manifest.pack.version,
            category: format!("{:?}", manifest.pack.category).to_ascii_lowercase(),
            enabled: true,
            manifest_path: target.join("manifest.toml"),
        })
    }

    fn load_state(&self) -> RegistryState {
        match fs::read_to_string(&self.state_path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => RegistryState::default(),
        }
    }

    fn save_state(&self, state: &RegistryState) -> Result<(), PackRegistryError> {
        if let Some(parent) = self.state_path.parent()
            && !parent.exists()
        {
            fs::create_dir_all(parent)
                .map_err(|source| PackRegistryError::State(source.to_string()))?;
        }
        let content = serde_json::to_string_pretty(state)
            .map_err(|source| PackRegistryError::State(source.to_string()))?;
        fs::write(&self.state_path, content)
            .map_err(|source| PackRegistryError::State(source.to_string()))
    }
}

/// Recursively copies a directory tree.
fn copy_dir(source: &Path, target: &Path) -> std::io::Result<()> {
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let entry_path = entry.path();
        let target_path = target.join(entry.file_name());
        if entry_path.is_dir() {
            fs::create_dir_all(&target_path)?;
            copy_dir(&entry_path, &target_path)?;
        } else {
            fs::copy(&entry_path, &target_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_manifest(dir: &Path, id: &str, extra: &str) {
        fs::create_dir_all(dir.join(id)).unwrap();
        let content = format!(
            r#"
[pack]
id = "{id}"
version = "0.1.0"
schema_version = 2
compatibility = ">=1.85"
risk = "low"
consequence = "creates"
category = "domain"

[dependencies]
requires = ["sddk-core"]

[[commands]]
name = "{id}"
surface = ["{id}"]

[fixtures]
paths = ["fixtures/plan.yaml"]
{extra}
"#
        );
        fs::write(dir.join(id).join("manifest.toml"), content).unwrap();
    }

    fn temp_root(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("sddk-pack-registry-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn discover_finds_packs() {
        let root = temp_root("discover");
        write_manifest(&root.join("packs"), "sddk-pack-uat", "");
        let state = root.join("state.json");
        let registry = PackRegistry::new(&root, &state);
        let entries = registry.discover().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "sddk-pack-uat");
        assert_eq!(entries[0].category, "domain");
        assert!(entries[0].enabled);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn enable_is_idempotent() {
        let root = temp_root("enable");
        write_manifest(&root.join("packs"), "sddk-pack-uat", "");
        let state = root.join("state.json");
        let registry = PackRegistry::new(&root, &state);
        registry.disable("sddk-pack-uat").unwrap();
        assert!(!registry.find("sddk-pack-uat").unwrap().enabled);
        registry.enable("sddk-pack-uat").unwrap();
        registry.enable("sddk-pack-uat").unwrap(); // idempotent
        assert!(registry.find("sddk-pack-uat").unwrap().enabled);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn disable_then_enable_roundtrips() {
        let root = temp_root("roundtrip");
        write_manifest(&root.join("packs"), "sddk-pack-uat", "");
        let state = root.join("state.json");
        let registry = PackRegistry::new(&root, &state);
        registry.disable("sddk-pack-uat").unwrap();
        registry.enable("sddk-pack-uat").unwrap();
        assert!(registry.find("sddk-pack-uat").unwrap().enabled);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn verify_reports_unsatisfied_requirement() {
        let root = temp_root("verify");
        write_manifest(&root.join("packs"), "sddk-pack-uat", "");
        let state = root.join("state.json");
        let registry = PackRegistry::new(&root, &state);
        let report = registry.verify("sddk-pack-uat").unwrap();
        assert!(!report.valid);
        assert!(report.unsatisfied.contains(&"sddk-core".to_string()));
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn verify_passes_with_provider() {
        let root = temp_root("verify-ok");
        write_manifest(&root.join("packs"), "sddk-core", "");
        // provider: sddk-core provides sddk-core capability
        let content = r#"
[pack]
id = "sddk-core"
version = "0.1.0"
schema_version = 2
compatibility = ">=1.85"
risk = "low"
consequence = "creates"
category = "core"

[provides]
capabilities = ["sddk-core"]

[[commands]]
name = "sddk-core"
surface = ["core"]

[fixtures]
paths = ["fixtures/plan.yaml"]
"#;
        fs::create_dir_all(root.join("packs/sddk-core")).unwrap();
        fs::write(root.join("packs/sddk-core/manifest.toml"), content).unwrap();
        write_manifest(&root.join("packs"), "sddk-pack-uat", "");
        let state = root.join("state.json");
        let registry = PackRegistry::new(&root, &state);
        let report = registry.verify("sddk-pack-uat").unwrap();
        assert!(report.valid, "expected valid, got {:?}", report);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn install_rejects_duplicate() {
        let root = temp_root("install-dup");
        write_manifest(&root.join("packs"), "sddk-pack-uat", "");
        let state = root.join("state.json");
        let registry = PackRegistry::new(&root, &state);
        let source = root.join("src");
        write_manifest(&source, "sddk-pack-uat", "");
        let error = registry.install(&source.join("sddk-pack-uat")).unwrap_err();
        assert!(matches!(error, PackRegistryError::AlreadyInstalled(_)));
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn install_copies_and_registers() {
        let root = temp_root("install-ok");
        let state = root.join("state.json");
        let registry = PackRegistry::new(&root, &state);
        let source = root.join("src");
        write_manifest(&source, "sddk-pack-cognicode", "");
        let entry = registry
            .install(&source.join("sddk-pack-cognicode"))
            .unwrap();
        assert_eq!(entry.id, "sddk-pack-cognicode");
        assert!(registry.find("sddk-pack-cognicode").is_ok());
        fs::remove_dir_all(&root).ok();
    }
}
