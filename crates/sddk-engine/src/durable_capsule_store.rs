//! Durable capsule store — C3j objetivo 1 (SPEC-005 CTX-001).
//!
//! Implementa el seam `CapsuleStore` (crate::cold_start) con persistencia
//! real: las capsules sobreviven el reinicio del proceso. Stop condition
//! del bundle respetada: sin base de datos canónica nueva — un fichero JSON
//! por capsule bajo `<root>/`, con nombres de fichero seguros derivados del
//! `CapsuleTarget` y el `capsule_id` canónico dentro del body.
//!
//! Semántica de las consultas (idéntica a `InMemoryCapsuleStore`):
//! - `last_capsule(workflow_run)`: la última persistida para ese run.
//! - `last_capsule_for_node(run, node)`: exacta, o la última del nodo.
//! - `resolve_ref(capsule_id)`: por id canónico `run:node:attempt`.
//! - `persist`: upsert atómico (write temp + rename).

use crate::cold_start::CapsuleStore;
use crate::context_capsule::{CapsuleTarget, ContextCapsule};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Error surface of the durable capsule store.
#[derive(Debug, thiserror::Error)]
pub enum DurableCapsuleError {
    #[error("io error on {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("capsule file {path} is corrupt: {reason}")]
    Corrupt { path: PathBuf, reason: String },
}

/// Filesystem-backed durable `CapsuleStore`.
#[derive(Debug)]
pub struct FilesystemCapsuleStore {
    root: PathBuf,
    /// In-memory index of capsule_id → file name, rebuilt lazily from the
    /// directory listing so a second process sees what the first wrote
    /// without any shared state.
    index: Mutex<std::collections::BTreeMap<String, String>>,
}

impl FilesystemCapsuleStore {
    /// Create (or open) a durable capsule store at `root`.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, DurableCapsuleError> {
        let root = root.into();
        fs::create_dir_all(&root).map_err(|e| DurableCapsuleError::Io {
            path: root.clone(),
            source: e,
        })?;
        let store = Self {
            root,
            index: Mutex::new(std::collections::BTreeMap::new()),
        };
        store.rebuild_index()?;
        Ok(store)
    }

    fn rebuild_index(&self) -> Result<(), DurableCapsuleError> {
        let mut index = self.index.lock().expect("poisoned");
        index.clear();
        if !self.root.exists() {
            return Ok(());
        }
        let entries = fs::read_dir(&self.root).map_err(|e| DurableCapsuleError::Io {
            path: self.root.clone(),
            source: e,
        })?;
        for entry in entries {
            let entry = entry.map_err(|e| DurableCapsuleError::Io {
                path: self.root.clone(),
                source: e,
            })?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json")
                && let Ok(capsule) = self.read_file(&path)
            {
                index.insert(capsule.capsule_id, file_name_for(&capsule.for_target));
            }
        }
        Ok(())
    }

    fn read_file(&self, path: &Path) -> Result<ContextCapsule, DurableCapsuleError> {
        let bytes = fs::read(path).map_err(|e| DurableCapsuleError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        serde_json::from_slice(&bytes).map_err(|e| DurableCapsuleError::Corrupt {
            path: path.to_path_buf(),
            reason: e.to_string(),
        })
    }
}

impl CapsuleStore for FilesystemCapsuleStore {
    fn last_capsule(&self, workflow_run: &str) -> Option<ContextCapsule> {
        let index = self.index.lock().expect("poisoned");
        // Deterministic: lexicographically last file for the run, matching
        // the in-memory store's "rev over BTreeMap" behavior.
        let mut candidates: Vec<&String> = index
            .values()
            .filter(|f| f.split(':').next() == Some(workflow_run))
            .collect();
        candidates.sort();
        let last = candidates.pop()?.clone();
        drop(index);
        self.read_file(&self.root.join(last)).ok()
    }

    fn last_capsule_for_node(&self, workflow_run: &str, node_run: &str) -> Option<ContextCapsule> {
        let index = self.index.lock().expect("poisoned");
        let mut candidates: Vec<&String> = index
            .values()
            .filter(|f| match parse_file_name(f) {
                Some((run, node)) => run == workflow_run && node == node_run,
                None => false,
            })
            .collect();
        candidates.sort();
        let last = candidates.pop()?.clone();
        drop(index);
        self.read_file(&self.root.join(last)).ok()
    }

    fn resolve_ref(&self, cid: &str) -> Option<ContextCapsule> {
        let file = {
            let index = self.index.lock().expect("poisoned");
            index.get(cid).cloned()?
        };
        self.read_file(&self.root.join(file)).ok()
    }

    fn persist(&self, capsule: &ContextCapsule) {
        let name = file_name_for(&capsule.for_target);
        let path = self.root.join(&name);
        let bytes = match serde_json::to_vec_pretty(capsule) {
            Ok(bytes) => bytes,
            Err(_) => return,
        };
        let tmp = path.with_extension("json.tmp");
        if fs::write(&tmp, &bytes).is_err() {
            return;
        }
        if fs::rename(&tmp, &path).is_err() {
            let _ = fs::remove_file(&tmp);
            return;
        }
        let mut index = self.index.lock().expect("poisoned");
        index.insert(capsule.capsule_id.clone(), name);
    }
}

/// File name for a target: `<workflow>:<node>:<attempt>.json`. The
/// components are opaque ids; `:` never appears in them by construction
/// (capsule_id format is `run:node:attempt`), so round-tripping is safe.
fn file_name_for(target: &CapsuleTarget) -> String {
    format!(
        "{}:{}:{}.json",
        target.workflow_run, target.node_run, target.attempt
    )
}

fn parse_file_name(name: &str) -> Option<(String, String)> {
    let stem = name.strip_suffix(".json")?;
    let mut parts = stem.splitn(3, ':');
    let run = parts.next()?.to_string();
    let node = parts.next()?.to_string();
    Some((run, node))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context_capsule::{
        CapsuleBudget, CapsuleDecisions, RecoveryState, Scope, StalenessReport,
    };

    fn capsule_for(run: &str, node: &str, attempt: &str) -> ContextCapsule {
        let target = CapsuleTarget {
            workflow_run: run.into(),
            node_run: node.into(),
            attempt: attempt.into(),
        };
        ContextCapsule {
            capsule_id: format!("{run}:{node}:{attempt}"),
            for_target: target,
            objective: format!("objective {run}/{node}"),
            definition_of_done: vec!["done".into()],
            scope: Scope::default(),
            constraints: vec![],
            decisions: CapsuleDecisions {
                accepted: vec![],
                rejected: vec![],
            },
            assumptions: vec![],
            open_questions: vec![],
            artifacts: crate::context_capsule::ArtifactBundle {
                must_read: vec![],
                relevant: vec![],
                fetch_on_demand: vec![],
            },
            negative_knowledge: vec![],
            changes_since_parent: vec![],
            evidence_required: vec![],
            tools_allowed: vec![],
            budget: CapsuleBudget {
                max_tokens: 0,
                actual_tokens: 0,
            },
            return_contract: "ok".into(),
            recovery: RecoveryState {
                previous_attempt: None,
                previous_capsule: None,
            },
            provenance: vec![],
            staleness: StalenessReport {
                stale_refs: vec![],
                as_of_ms: 0,
            },
        }
    }

    // durability-required: exercises the real filesystem; a fake would not
    // prove restart durability (CTX-001 exit gate).
    #[test]
    fn capsule_survives_process_restart() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("capsules");

        // "process A"
        {
            let store = FilesystemCapsuleStore::open(&root).unwrap();
            store.persist(&capsule_for("run-1", "node-a", "att-1"));
        }

        // "process B": brand new store instance over the same root.
        let store = FilesystemCapsuleStore::open(&root).unwrap();
        let got = store.last_capsule("run-1").expect("capsule durable");
        assert_eq!(got.capsule_id, "run-1:node-a:att-1");
        assert_eq!(got.objective, "objective run-1/node-a");
        assert_eq!(
            store
                .resolve_ref("run-1:node-a:att-1")
                .expect("ref resolvable after restart")
                .capsule_id,
            "run-1:node-a:att-1"
        );
    }

    #[test]
    fn last_capsule_prefers_latest_file_for_run_and_node_lookup_works() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("capsules");
        let store = FilesystemCapsuleStore::open(&root).unwrap();
        store.persist(&capsule_for("run-1", "node-a", "att-1"));
        store.persist(&capsule_for("run-1", "node-a", "att-2"));
        store.persist(&capsule_for("run-1", "node-b", "att-1"));
        store.persist(&capsule_for("run-2", "node-a", "att-1"));

        assert_eq!(
            store.last_capsule("run-1").unwrap().capsule_id,
            "run-1:node-b:att-1"
        );
        assert_eq!(
            store.last_capsule("run-2").unwrap().capsule_id,
            "run-2:node-a:att-1"
        );
        let node = store
            .last_capsule_for_node("run-1", "node-a")
            .expect("node capsule");
        assert_eq!(
            node.capsule_id, "run-1:node-a:att-2",
            "latest attempt of the node"
        );
        assert!(store.last_capsule_for_node("run-2", "node-zz").is_none());
    }

    #[test]
    fn persist_is_upsert_not_duplicate() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("capsules");
        let store = FilesystemCapsuleStore::open(&root).unwrap();
        let mut c = capsule_for("run-1", "node-a", "att-1");
        store.persist(&c);
        c.objective = "updated".into();
        store.persist(&c);

        let files: Vec<_> = fs::read_dir(&root)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"))
            .collect();
        assert_eq!(files.len(), 1, "upsert must not accumulate files");
        assert_eq!(
            store.resolve_ref("run-1:node-a:att-1").unwrap().objective,
            "updated"
        );
    }

    // Falsifying mutation of CTX-001: drift on disk must never surface as a
    // silently WRONG capsule; the store either returns the persisted bytes
    // faithfully (parseable) or nothing — callers re-derive expectations.
    #[test]
    fn corrupt_store_returns_none_not_garbage() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("capsules");
        {
            let store = FilesystemCapsuleStore::open(&root).unwrap();
            store.persist(&capsule_for("run-1", "node-a", "att-1"));
        }
        // Corrupt the file on disk (simulated crash mid-write).
        let path = root.join("run-1:node-a:att-1.json");
        fs::write(&path, b"{broken").unwrap();

        let store = FilesystemCapsuleStore::open(&root).unwrap();
        assert!(
            store.last_capsule("run-1").is_none(),
            "corrupt capsule must not be handed out"
        );
        assert!(store.resolve_ref("run-1:node-a:att-1").is_none());
    }
}
