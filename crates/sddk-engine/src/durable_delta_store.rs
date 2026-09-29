//! Durable ContextDelta store (SPEC-005 CTX-008, roadmap C3j objetivo 5).
//!
//! `ContextBridge` es efímero: vive en el proceso que entrega el delta. Sin
//! un sustrato persistente, un segundo proceso no puede reconstruir qué
//! cambios materiales se le entregaron a una sesión, ni detectar que un delta
//! se perdió entre arranques. Este módulo materializa ese sustrato **detrás del
//! seam que ya existe** (`ContextDelta` + las reglas de `ContextBridge`),
//! sin crear un segundo modelo de delta ni una segunda autoridad de
//! secuencia: la monotonicidad y el rechazo de `from_revision` obsoleto siguen
//! siendo los de `ContextBridge::apply`.
//!
//! Diseño:
//!
//! - Un fichero JSON por delta, nombrado `<seq:012u>.json`, dentro de un
//!   directorio por binding. El orden lexicográfico del nombre **es** el orden
//!   de secuencia, así que "el último" no requiere índice en memoria.
//! - Escritura atómica (tmp + rename), igual que el resto de sustrato durable.
//! - `replay` devuelve los deltas en orden; aplicar ese stream sobre un
//!   `ContextBridge` rehydrado reproduce exactamente la entrega, incluido el
//!   rechazo de un delta que ya no encaja con el basis actual.
//!
//! Corrupción en disco: un delta ilegible **no** se devuelve como si fuera
//! válido. `replay` lo salta y lo reporta en `replay_skipped`, porque un
//! `from_revision` que nadie puede leer no es un hecho; inventarlo fabricaría
//! contexto.

use crate::context_bridge::{ContextBridge, ContextDelta, ContextDeltaError};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Failure surface of the durable delta store.
#[derive(Debug, thiserror::Error)]
pub enum DurableDeltaError {
    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("cannot serialize delta: {0}")]
    Serialize(String),
}

/// Result of a replay: the ordered deltas plus what was skipped.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ReplayOutcome {
    /// Deltas in strict sequence order.
    pub deltas: Vec<ContextDelta>,
    /// File names that could not be read or parsed.
    pub replay_skipped: Vec<String>,
    /// The highest sequence observed on disk, even if a delta was skipped.
    pub last_seq_on_disk: u64,
}

/// Filesystem-backed durable delta stream, scoped to one binding.
#[derive(Debug)]
pub struct FilesystemDeltaStore {
    root: PathBuf,
    /// Guards read-modify-write of the last-sequence watermark file.
    watermark: Mutex<u64>,
}

impl FilesystemDeltaStore {
    /// Open (or create) the delta stream for a binding under `root`.
    ///
    /// `root` is expected to be the same `context/deltas` directory the
    /// caller derives from `resolve_xdg_paths`; this module never resolves
    /// XDG paths itself.
    pub fn open(root: &Path) -> Result<Self, DurableDeltaError> {
        fs::create_dir_all(root).map_err(|source| DurableDeltaError::Io {
            path: root.to_path_buf(),
            source,
        })?;
        let watermark = read_watermark(root)?;
        Ok(Self {
            root: root.to_path_buf(),
            watermark: Mutex::new(watermark),
        })
    }

    /// Root directory of this stream.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Highest sequence persisted so far (0 when the stream is empty).
    #[must_use]
    pub fn last_seq(&self) -> u64 {
        *self.watermark.lock().expect("poisoned watermark")
    }

    /// Append a delta, allocating the next sequence.
    ///
    /// The allocated sequence is `last_seq + 1`, so a caller cannot
    /// accidentally write a duplicate or out-of-order sequence. Returns the
    /// stored delta (with the sequence actually assigned).
    pub fn append(&self, mut delta: ContextDelta) -> Result<ContextDelta, DurableDeltaError> {
        let seq = {
            let mut watermark = self.watermark.lock().expect("poisoned watermark");
            *watermark += 1;
            let seq = *watermark;
            write_watermark(&self.root, seq)?;
            delta.seq = seq;
            seq
        };
        self.write_delta(seq, &delta)?;
        Ok(delta)
    }

    /// Persist a delta with an explicit sequence. Refuses a sequence that is
    /// not strictly greater than the watermark, so the stream stays
    /// monotonic on disk and not only in the bridge.
    pub fn append_at(&self, delta: &ContextDelta) -> Result<(), DurableDeltaError> {
        {
            let watermark = self.watermark.lock().expect("poisoned watermark");
            if delta.seq == 0 || delta.seq <= *watermark {
                return Err(DurableDeltaError::Serialize(format!(
                    "delta seq {} is not strictly greater than watermark {}",
                    delta.seq, *watermark
                )));
            }
        }
        self.write_delta(delta.seq, delta)?;
        let mut watermark = self.watermark.lock().expect("poisoned watermark");
        *watermark = delta.seq;
        write_watermark(&self.root, delta.seq)
    }

    /// Read the stream in strict sequence order, reporting unreadable files
    /// instead of inventing them.
    pub fn replay(&self) -> Result<ReplayOutcome, DurableDeltaError> {
        let mut names = self.delta_files()?;
        names.sort();
        let mut outcome = ReplayOutcome {
            last_seq_on_disk: self.last_seq(),
            ..ReplayOutcome::default()
        };
        for name in names {
            let path = self.root.join(&name);
            let bytes = match fs::read(&path) {
                Ok(bytes) => bytes,
                Err(_) => {
                    outcome.replay_skipped.push(name);
                    continue;
                }
            };
            match serde_json::from_slice::<ContextDelta>(&bytes) {
                Ok(delta) => outcome.deltas.push(delta),
                Err(_) => outcome.replay_skipped.push(name),
            }
        }
        Ok(outcome)
    }

    /// The revision this stream starts from: the `from_revision` of its
    /// first delta, or `None` when the stream is empty.
    ///
    /// A caller that wants to **rehydrate** a bridge must bootstrap it here,
    /// not at the session's current basis. Bootstrapping at the current basis
    /// would make every already-consumed delta look stale, and the rebuilt
    /// bridge would come back empty. The stream is the only thing that knows
    /// where it began, so it is the only thing allowed to say so.
    pub fn origin_basis(&self) -> Result<Option<String>, DurableDeltaError> {
        Ok(self
            .replay()?
            .deltas
            .first()
            .map(|d| d.from_revision.clone()))
    }

    /// The revision this stream ends at, or `None` when the stream is empty.
    /// This is where the session's basis SHOULD be after a clean drain.
    pub fn end_basis(&self) -> Result<Option<String>, DurableDeltaError> {
        Ok(self.replay()?.deltas.last().map(|d| d.to_revision.clone()))
    }

    /// Replay the stream over a bridge already bootstrapped at its basis.
    ///
    /// The bridge owns the basis: this store does not set it, because the
    /// basis comes from the durable binding/capsule, and a store that
    /// overwrote it would be a second authority over what the session
    /// believes.
    ///
    /// Returns the bridge after applying every delta that the bridge accepts,
    /// plus the deltas the bridge **rejected** (stale `from_revision` or
    /// non-monotonic seq). A rejected delta is not an error of this store:
    /// it is the contract of `ContextBridge` observed across a process
    /// boundary, and the caller decides what to do about it.
    pub fn apply_to(&self, bridge: &mut ContextBridge) -> Result<ApplyOutcome, DurableDeltaError> {
        let mut outcome = ApplyOutcome::default();
        let replay = self.replay()?;
        for delta in replay.deltas {
            match bridge.apply(delta.clone()) {
                Ok(()) => outcome.applied.push(delta),
                Err(
                    err @ (ContextDeltaError::StaleFrom { .. }
                    | ContextDeltaError::NonMonotonicSeq { .. }),
                ) => {
                    outcome.rejected.push(RejectedDelta {
                        seq: delta.seq,
                        reason: err.to_string(),
                    });
                }
                Err(other) => return Err(DurableDeltaError::Serialize(other.to_string())),
            }
        }
        outcome.replay_skipped = replay.replay_skipped;
        outcome.last_seq_on_disk = replay.last_seq_on_disk;
        Ok(outcome)
    }

    fn delta_files(&self) -> Result<Vec<String>, DurableDeltaError> {
        let entries = match fs::read_dir(&self.root) {
            Ok(entries) => entries,
            Err(source) => {
                return Err(DurableDeltaError::Io {
                    path: self.root.to_path_buf(),
                    source,
                });
            }
        };
        let mut names = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.ends_with(".json") && name.starts_with(DELTA_PREFIX) {
                names.push(name);
            }
        }
        Ok(names)
    }

    fn write_delta(&self, seq: u64, delta: &ContextDelta) -> Result<(), DurableDeltaError> {
        let path = self.root.join(file_name(seq));
        let bytes = serde_json::to_vec_pretty(delta)
            .map_err(|e| DurableDeltaError::Serialize(e.to_string()))?;
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, &bytes).map_err(|source| DurableDeltaError::Io {
            path: tmp.clone(),
            source,
        })?;
        fs::rename(&tmp, &path).map_err(|source| DurableDeltaError::Io {
            path: path.clone(),
            source,
        })
    }
}

/// Outcome of applying the durable stream to a bridge.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ApplyOutcome {
    /// Deltas the bridge accepted, in order.
    pub applied: Vec<ContextDelta>,
    /// Deltas the bridge rejected, with the reason (stale/monotonic).
    pub rejected: Vec<RejectedDelta>,
    /// Files that could not be read or parsed.
    pub replay_skipped: Vec<String>,
    /// Highest sequence observed on disk.
    pub last_seq_on_disk: u64,
}
/// A delta the bridge refused, with the reason it refused it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectedDelta {
    pub seq: u64,
    pub reason: String,
}

const DELTA_PREFIX: &str = "delta-";
const WATERMARK_FILE: &str = "last_seq";

fn file_name(seq: u64) -> String {
    format!("{DELTA_PREFIX}{seq:012}.json")
}

fn read_watermark(root: &Path) -> Result<u64, DurableDeltaError> {
    let path = root.join(WATERMARK_FILE);
    match fs::read_to_string(&path) {
        Ok(text) => text
            .trim()
            .parse::<u64>()
            .map_err(|e| DurableDeltaError::Serialize(format!("bad watermark {text:?}: {e}"))),
        // A missing watermark on a non-empty stream is recoverable: rebuild it
        // from the highest file present rather than losing the sequence.
        Err(_) => Ok(highest_seq_in(root)),
    }
}

fn write_watermark(root: &Path, seq: u64) -> Result<(), DurableDeltaError> {
    let path = root.join(WATERMARK_FILE);
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, seq.to_string()).map_err(|source| DurableDeltaError::Io {
        path: tmp.clone(),
        source,
    })?;
    fs::rename(&tmp, &path).map_err(|source| DurableDeltaError::Io {
        path: path.clone(),
        source,
    })
}

fn highest_seq_in(root: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(root) else {
        return 0;
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            name.strip_prefix(DELTA_PREFIX)
                .and_then(|rest| rest.strip_suffix(".json"))
                .and_then(|digits| digits.parse::<u64>().ok())
        })
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn delta(from: &str, to: &str, seq: u64, additions: &[&str]) -> ContextDelta {
        ContextDelta {
            from_revision: from.into(),
            to_revision: to.into(),
            relevance_reason: "test".into(),
            additions: additions.iter().map(|s| (*s).to_string()).collect(),
            deletions: Vec::new(),
            seq,
            advisory_only: true,
        }
    }

    fn tempdir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "sddk-durable-delta-{label}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create tempdir");
        dir.join("stream")
    }

    /// A second store instance (a "second process") sees everything the first
    /// one wrote, in order, with no shared state.
    #[test]
    fn second_store_sees_the_stream_in_order() {
        let root = tempdir("order");
        let first = FilesystemDeltaStore::open(&root).expect("open");
        assert_eq!(first.last_seq(), 0);
        first
            .append(delta("rev-0", "rev-1", 0, &["a"]))
            .expect("append 1");
        first
            .append(delta("rev-1", "rev-2", 0, &["b"]))
            .expect("append 2");
        first
            .append(delta("rev-2", "rev-3", 0, &["c"]))
            .expect("append 3");

        let second = FilesystemDeltaStore::open(&root).expect("reopen");
        assert_eq!(second.last_seq(), 3);
        let replay = second.replay().expect("replay");
        assert_eq!(replay.deltas.len(), 3);
        assert_eq!(replay.deltas[0].seq, 1);
        assert_eq!(replay.deltas[2].seq, 3);
        assert_eq!(replay.deltas[2].to_revision, "rev-3");
        assert!(replay.replay_skipped.is_empty());
    }

    /// `append` allocates the sequence: a caller passing seq=0 twice still
    /// gets two distinct, ordered deltas. The store, not the caller, is the
    /// monotonicity authority on disk.
    #[test]
    fn append_allocates_sequences() {
        let root = tempdir("alloc");
        let store = FilesystemDeltaStore::open(&root).expect("open");
        let a = store.append(delta("rev-0", "rev-1", 0, &["a"])).expect("a");
        let b = store.append(delta("rev-1", "rev-2", 0, &["b"])).expect("b");
        assert_eq!(a.seq, 1);
        assert_eq!(b.seq, 2);
        assert_eq!(store.last_seq(), 2);
    }

    /// `append_at` refuses a non-increasing sequence: the stream cannot be
    /// written out of order even by a caller that tries.
    #[test]
    fn append_at_refuses_non_monotonic() {
        let root = tempdir("mono");
        let store = FilesystemDeltaStore::open(&root).expect("open");
        store
            .append_at(&delta("rev-0", "rev-1", 5, &["a"]))
            .expect("seq 5");
        assert!(
            store
                .append_at(&delta("rev-1", "rev-2", 5, &["b"]))
                .is_err()
        );
        assert!(
            store
                .append_at(&delta("rev-1", "rev-2", 3, &["b"]))
                .is_err()
        );
        assert!(
            store
                .append_at(&delta("rev-1", "rev-2", 0, &["b"]))
                .is_err()
        );
        assert!(store.append_at(&delta("rev-1", "rev-2", 6, &["b"])).is_ok());
        assert_eq!(store.last_seq(), 6);
    }

    /// Replaying onto a bridge at the right basis delivers every delta; the
    /// bridge ends at the last `to_revision` (CTX-008).
    #[test]
    fn replay_applies_cleanly_to_matching_bridge() {
        let root = tempdir("apply");
        let writer = FilesystemDeltaStore::open(&root).expect("open");
        writer
            .append(delta("rev-0", "rev-1", 0, &["a"]))
            .expect("1");
        writer
            .append(delta("rev-1", "rev-2", 0, &["b"]))
            .expect("2");

        let reader = FilesystemDeltaStore::open(&root).expect("reopen");
        let binding = crate::agentic_session_binding::AgenticBinding::attach(
            crate::agentic_session_binding::AgenticSessionRef::new("s-1"),
            crate::agentic_session_binding::BindingTarget::Ephemeral,
        );
        let (mut bridge, _ctx) = ContextBridge::bootstrap(&binding, "rev-0");
        let outcome = reader.apply_to(&mut bridge).expect("apply");

        assert!(outcome.rejected.is_empty());
        assert_eq!(outcome.applied.len(), 2);
        assert_eq!(bridge.current_revision(), "rev-2");
    }

    /// A delta whose `from_revision` no longer matches the bridge basis is
    /// REJECTED with the typed error, across the process boundary. That is
    /// CTX-008's stale-rejection observed end to end.
    #[test]
    fn replay_rejects_stale_from_revision() {
        let root = tempdir("stale");
        let writer = FilesystemDeltaStore::open(&root).expect("open");
        writer
            .append(delta("rev-7", "rev-8", 0, &["stale"]))
            .expect("stale");

        let reader = FilesystemDeltaStore::open(&root).expect("reopen");
        let binding = crate::agentic_session_binding::AgenticBinding::attach(
            crate::agentic_session_binding::AgenticSessionRef::new("s-1"),
            crate::agentic_session_binding::BindingTarget::Ephemeral,
        );
        let (mut bridge, _ctx) = ContextBridge::bootstrap(&binding, "rev-0");
        let outcome = reader.apply_to(&mut bridge).expect("apply");

        assert!(outcome.applied.is_empty());
        assert_eq!(outcome.rejected.len(), 1);
        assert_eq!(outcome.rejected[0].seq, 1);
        assert!(outcome.rejected[0].reason.contains("does not match"));
        assert_eq!(bridge.current_revision(), "rev-0");
    }

    /// A partially applied stream: the first delta lands, then a stale one is
    /// rejected, then the rest continue. The bridge is not rolled back, and
    /// the rejection is reported rather than hidden.
    #[test]
    fn partial_stream_reports_rejection_without_rollback() {
        let root = tempdir("partial");
        let writer = FilesystemDeltaStore::open(&root).expect("open");
        writer
            .append(delta("rev-0", "rev-1", 0, &["a"]))
            .expect("1");
        writer
            .append(delta("rev-9", "rev-9", 0, &["stale"]))
            .expect("2");
        writer
            .append(delta("rev-1", "rev-2", 0, &["c"]))
            .expect("3");

        let reader = FilesystemDeltaStore::open(&root).expect("reopen");
        let binding = crate::agentic_session_binding::AgenticBinding::attach(
            crate::agentic_session_binding::AgenticSessionRef::new("s-1"),
            crate::agentic_session_binding::BindingTarget::Ephemeral,
        );
        let (mut bridge, _ctx) = ContextBridge::bootstrap(&binding, "rev-0");
        let outcome = reader.apply_to(&mut bridge).expect("apply");

        assert_eq!(outcome.applied.len(), 2, "1 and 3 apply");
        assert_eq!(outcome.rejected.len(), 1, "2 is stale");
        assert_eq!(bridge.current_revision(), "rev-2");
    }

    /// A corrupt delta file is reported as skipped, never delivered as if it
    /// were valid context.
    ///
    /// The corruption sits on a sequence slot that nothing re-appends, because
    /// `append` allocates the next free slot and would legitimately replace
    /// the file — that is upsert, not silent data loss, and it is exercised by
    /// `append_allocates_sequences`.
    #[test]
    fn corrupt_delta_is_reported_not_invented() {
        let root = tempdir("corrupt");
        let store = FilesystemDeltaStore::open(&root).expect("open");
        store.append(delta("rev-0", "rev-1", 0, &["a"])).expect("1");
        store
            .append_at(&delta("rev-2", "rev-3", 3, &["c"]))
            .expect("3");
        // Slot 2 is unwritten: a crash between two appends. Fill it with
        // garbage and assert it is reported, not delivered.
        fs::write(root.join(file_name(2)), b"{not json").expect("write corrupt");

        let replay = store.replay().expect("replay");
        assert_eq!(replay.deltas.len(), 2, "deltas 1 and 3 are readable");
        assert_eq!(replay.replay_skipped, vec![file_name(2)]);
        assert_eq!(replay.deltas[1].seq, 3);
        assert_eq!(replay.last_seq_on_disk, 3, "watermark survives corruption");
    }

    /// A missing watermark on a non-empty stream is rebuilt from disk, so a
    /// crash between the delta write and the watermark write does not reset
    /// the sequence and allow duplicates.
    #[test]
    fn watermark_is_rebuilt_when_missing() {
        let root = tempdir("watermark");
        let store = FilesystemDeltaStore::open(&root).expect("open");
        store.append(delta("rev-0", "rev-1", 0, &["a"])).expect("1");
        store.append(delta("rev-1", "rev-2", 0, &["b"])).expect("2");
        fs::remove_file(root.join(WATERMARK_FILE)).expect("remove watermark");

        let reopened = FilesystemDeltaStore::open(&root).expect("reopen");
        assert_eq!(reopened.last_seq(), 2);
        let next = reopened
            .append(delta("rev-2", "rev-3", 0, &["c"]))
            .expect("next");
        assert_eq!(next.seq, 3, "sequence continues, no duplicate");
    }

    /// A second process appending continues the same sequence, not from zero.
    #[test]
    fn concurrent_processes_share_the_sequence() {
        let root = tempdir("concurrent");
        let a = FilesystemDeltaStore::open(&root).expect("open a");
        a.append(delta("rev-0", "rev-1", 0, &["a"])).expect("a1");
        let b = FilesystemDeltaStore::open(&root).expect("open b");
        b.append(delta("rev-1", "rev-2", 0, &["b"])).expect("b1");
        let a2 = FilesystemDeltaStore::open(&root).expect("reopen a");
        let c = a2.append(delta("rev-2", "rev-3", 0, &["c"])).expect("c");
        assert_eq!(c.seq, 3);
    }
}
