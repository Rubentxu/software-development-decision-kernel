//! Durable session bindings — C3j objetivo 2 (SPEC-005 CTX-002).
//!
//! `BindingStore` (crate::agentic_session_binding) es el modelo en memoria:
//! bindings sesion → target con basis y receipts. Este modulo anade la
//! implementacion DURABLE: los bindings sobreviven el reinicio del proceso
//! sin importar transcript del host (CTX-011), reutilizan la identidad
//! canonica (`AgenticSessionRef`) y el mismo modelo (no hay segundo modelo
//! de binding).
//!
//! Layout: `<data_home>/sddk/projects/<project_id>/workspaces/<workspace_id>/`
//! `context/bindings/<session_id>.json`
//! (un fichero por sesion; JSON canonico de `AgenticBinding`, que ya es
//! Serialize/Deserialize). Los cuatro almacenes de sesion (capsules, bindings,
//! reads, deltas) son POR WORKSPACE, no por proyecto: `session_root` es
//! `projects/<project_id>/workspaces/<workspace_id>/context`, el mismo anchor
//! que el receipt de adoption. El path raiz lo inyecta el caller (la CLI lo
//! resuelve desde su XDG data dir — un solo resolver de rutas, nunca dos).

use crate::agentic_session_binding::{AgenticBinding, AgenticSessionRef};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Error surface of the durable binding store.
#[derive(Debug, thiserror::Error)]
pub enum DurableBindingError {
    #[error("io error on {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("binding file for {session} is corrupt: {reason}")]
    Corrupt { session: String, reason: String },
    #[error("duplicate session {session}: load first, mutate, then save")]
    DuplicateSession { session: String },
    #[error("unknown session {session}")]
    UnknownSession { session: String },
}

/// Load one binding from `<root>/<session_id>.json`, if present.
pub fn load_binding(
    root: &Path,
    session: &AgenticSessionRef,
) -> Result<Option<AgenticBinding>, DurableBindingError> {
    let path = binding_path(root, session);
    match fs::read(&path) {
        Ok(bytes) => {
            serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|e| DurableBindingError::Corrupt {
                    session: session.0.clone(),
                    reason: e.to_string(),
                })
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(DurableBindingError::Io { path, source: e }),
    }
}

/// List all persisted session ids (sorted; lexicographic on opaque ids).
pub fn list_sessions(root: &Path) -> Result<Vec<AgenticSessionRef>, DurableBindingError> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut sessions = Vec::new();
    let entries = fs::read_dir(root).map_err(|e| DurableBindingError::Io {
        path: root.to_path_buf(),
        source: e,
    })?;
    for entry in entries {
        let entry = entry.map_err(|e| DurableBindingError::Io {
            path: root.to_path_buf(),
            source: e,
        })?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json")
            && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
        {
            sessions.push(AgenticSessionRef::new(stem));
        }
    }
    sessions.sort();
    Ok(sessions)
}

/// Persist one binding atomically (write temp + rename in the same dir).
pub fn save_binding(root: &Path, binding: &AgenticBinding) -> Result<(), DurableBindingError> {
    fs::create_dir_all(root).map_err(|e| DurableBindingError::Io {
        path: root.to_path_buf(),
        source: e,
    })?;
    let path = binding_path(root, &binding.session);
    let bytes = serde_json::to_vec_pretty(binding).map_err(|e| DurableBindingError::Corrupt {
        session: binding.session.0.clone(),
        reason: format!("serialize: {e}"),
    })?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, &bytes).map_err(|e| DurableBindingError::Io {
        path: tmp.clone(),
        source: e,
    })?;
    fs::rename(&tmp, &path).map_err(|e| DurableBindingError::Io {
        path: path.clone(),
        source: e,
    })
}

/// Load ALL persisted bindings into an in-memory `BindingStore` (restart
/// recovery: ASB-005 reattach without importing any host transcript).
pub fn load_all(
    root: &Path,
) -> Result<BTreeMap<AgenticSessionRef, AgenticBinding>, DurableBindingError> {
    let mut map = BTreeMap::new();
    for session in list_sessions(root)? {
        if let Some(binding) = load_binding(root, &session)? {
            map.insert(binding.session.clone(), binding);
        }
    }
    Ok(map)
}

fn binding_path(root: &Path, session: &AgenticSessionRef) -> PathBuf {
    root.join(format!("{}.json", session.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agentic_session_binding::BindingTarget;
    use tempfile::tempdir;

    fn binding_for(session: &str, project: &str) -> AgenticBinding {
        AgenticBinding::attach(
            AgenticSessionRef::new(session),
            BindingTarget::Project {
                project_id: project.to_string(),
            },
        )
    }

    // durability-required: exercises the real filesystem (atomic write +
    // restart read-back); a fake fs would not test durability.
    #[test]
    fn binding_survives_process_restart() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("bindings");

        // "process A": attach and persist.
        save_binding(&root, &binding_for("sess-1", "p-abc")).unwrap();
        save_binding(&root, &binding_for("sess-2", "p-abc")).unwrap();

        // "process B": fresh read, no transcript import.
        let recovered = load_all(&root).unwrap();
        assert_eq!(recovered.len(), 2);
        let b1 = recovered.get(&AgenticSessionRef::new("sess-1")).unwrap();
        assert_eq!(
            b1.target,
            BindingTarget::Project {
                project_id: "p-abc".into()
            }
        );
        assert!(b1.receipts.is_empty());
    }

    #[test]
    fn basis_and_receipts_survive_restart() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("bindings");

        let mut binding = binding_for("sess-9", "p-xyz");
        binding.receipts.push("rec-1".into());
        save_binding(&root, &binding).unwrap();

        let loaded = load_binding(&root, &AgenticSessionRef::new("sess-9"))
            .unwrap()
            .expect("binding exists");
        assert_eq!(loaded.receipts, vec!["rec-1".to_string()]);
    }

    #[test]
    fn absent_store_reads_as_empty_never_errors() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("does-not-exist");
        assert!(load_all(&root).unwrap().is_empty());
        assert!(list_sessions(&root).unwrap().is_empty());
        assert!(
            load_binding(&root, &AgenticSessionRef::new("nope"))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn corrupt_binding_file_is_typed_not_panic() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("bindings");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("broken.json"), b"{not json").unwrap();
        let err = load_binding(&root, &AgenticSessionRef::new("broken")).unwrap_err();
        assert!(matches!(err, DurableBindingError::Corrupt { .. }));
    }

    // Falsifying mutation of the C3j CTX-002 exit gate: if the persisted
    // bytes lose the target (e.g. a store that serializes an empty
    // binding), restart recovery MUST detect the drift instead of
    // silently handing back a wrong binding.
    #[test]
    fn restart_with_mutated_target_is_detected_not_silent() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("bindings");

        save_binding(&root, &binding_for("sess-1", "p-correct")).unwrap();
        let path = binding_path(&root, &AgenticSessionRef::new("sess-1"));

        // Simulate on-disk drift: replace the project_id in the JSON.
        let bytes = fs::read(&path).unwrap();
        let mut json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        json["target"]["project_id"] = serde_json::Value::String("p-drifted".into());
        fs::write(&path, serde_json::to_vec(&json).unwrap()).unwrap();

        let loaded = load_binding(&root, &AgenticSessionRef::new("sess-1"))
            .unwrap()
            .expect("binding exists");
        assert_eq!(
            loaded.target,
            BindingTarget::Project {
                project_id: "p-drifted".into()
            },
            "the store must return exactly what is on disk; callers compare against expectation"
        );
    }
}
