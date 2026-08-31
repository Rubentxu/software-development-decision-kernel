//! Context-read tracing (SPEC-011 §3, Phase 6).
//!
//! Records a bounded, ordered, de-duplicated set of object ids that an agent
//! or behavior execution read. Records are bookkeeping-only: they MUST NOT
//! trigger reactive behaviors or consume workflow budget. The recorder caps
//! the set and flags truncation so traces stay bounded.

use serde::{Deserialize, Serialize};

/// Default cap for the object-id set (SPEC-011 §4: caps prevent unbounded
/// trace size).
pub const DEFAULT_CONTEXT_READ_CAP: usize = 100;

/// A finished context-read trace for one execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextReadRecord {
    /// Execution (behavior/agent) identifier.
    pub execution_id: String,
    /// Event id that triggered the execution, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger_event_id: Option<String>,
    /// Ordered, de-duplicated object ids read.
    pub object_ids: Vec<String>,
    /// Exact count of unique object ids.
    pub count: usize,
    /// Whether the cap truncated the set.
    pub truncated: bool,
    /// Read categories (e.g. `graph`, `artifact`, `evidence`).
    #[serde(default)]
    pub categories: Vec<String>,
    /// Optional content hashes for the read objects.
    #[serde(default)]
    pub content_hashes: Vec<String>,
}

/// Bounded recorder for context reads.
#[derive(Debug, Clone)]
pub struct ContextReadRecorder {
    /// Ordered, de-duplicated object ids.
    object_ids: Vec<String>,
    /// Maximum number of object ids retained.
    cap: usize,
    /// Whether the cap was exceeded.
    truncated: bool,
    /// Read categories.
    categories: Vec<String>,
}

impl ContextReadRecorder {
    /// Creates a recorder with the given cap.
    pub fn new(cap: usize) -> Self {
        Self {
            object_ids: Vec::new(),
            cap,
            truncated: false,
            categories: Vec::new(),
        }
    }

    /// Creates a recorder with the default cap.
    pub fn default_cap() -> Self {
        Self::new(DEFAULT_CONTEXT_READ_CAP)
    }

    /// Records one object id. Deduplicates (keeps first occurrence) and
    /// truncates beyond the cap, setting the truncated flag.
    pub fn record(&mut self, object_id: &str) {
        if self.object_ids.contains(&object_id.to_string()) {
            return;
        }
        if self.object_ids.len() >= self.cap {
            self.truncated = true;
            return;
        }
        self.object_ids.push(object_id.to_string());
    }

    /// Records a read category (deduplicated, unbounded — categories are
    /// closed-vocabulary and small).
    pub fn add_category(&mut self, category: &str) {
        if !self.categories.contains(&category.to_string()) {
            self.categories.push(category.to_string());
        }
    }

    /// Number of unique object ids recorded.
    pub fn len(&self) -> usize {
        self.object_ids.len()
    }

    /// Whether the recorder is empty.
    pub fn is_empty(&self) -> bool {
        self.object_ids.is_empty()
    }

    /// Whether truncation happened.
    pub fn truncated(&self) -> bool {
        self.truncated
    }

    /// Finishes the trace into an immutable record.
    pub fn finish(
        &self,
        execution_id: &str,
        trigger_event_id: Option<String>,
    ) -> ContextReadRecord {
        ContextReadRecord {
            execution_id: execution_id.to_string(),
            trigger_event_id,
            object_ids: self.object_ids.clone(),
            count: self.object_ids.len(),
            truncated: self.truncated,
            categories: self.categories.clone(),
            content_hashes: Vec::new(),
        }
    }
}

impl Default for ContextReadRecorder {
    fn default() -> Self {
        Self::default_cap()
    }
}

/// Event type for context-read bookkeeping (2 segments → not projected to the
/// reactive graph; see `graph.rs` skip).
pub const CONTEXT_READ_EVENT_TYPE: &str = "context.read";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_appends_and_dedups() {
        let mut recorder = ContextReadRecorder::new(100);
        recorder.record("a");
        recorder.record("b");
        recorder.record("a");
        assert_eq!(recorder.object_ids, vec!["a", "b"]);
        assert!(!recorder.truncated);
    }

    #[test]
    fn cap_truncates_with_flag() {
        let mut recorder = ContextReadRecorder::new(2);
        recorder.record("a");
        recorder.record("b");
        recorder.record("c");
        assert_eq!(recorder.object_ids, vec!["a", "b"]);
        assert!(recorder.truncated);
        assert_eq!(recorder.len(), 2);
    }

    #[test]
    fn finish_produces_record() {
        let mut recorder = ContextReadRecorder::new(100);
        recorder.record("graph:cycle:c-1");
        recorder.record("artifact:spec-012");
        recorder.add_category("graph");
        recorder.add_category("artifact");
        let record = recorder.finish("exec-1", Some("evt-trigger".into()));
        assert_eq!(record.execution_id, "exec-1");
        assert_eq!(record.trigger_event_id.as_deref(), Some("evt-trigger"));
        assert_eq!(record.count, 2);
        assert!(!record.truncated);
        assert_eq!(record.categories, vec!["graph", "artifact"]);
    }

    #[test]
    fn empty_recorder_finishes_with_zero() {
        let recorder = ContextReadRecorder::default_cap();
        let record = recorder.finish("exec-0", None);
        assert_eq!(record.count, 0);
        assert!(record.object_ids.is_empty());
    }
}
