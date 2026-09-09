// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// context_compiler.rs — T-05 (M3 arch-spec-006)
//
// Composes planning / run / memory / graph / vault adapters into a
// `ContextCapsuleV2` carrying provenance (ordered adapter ids) and
// `Staleness` (fact-log head diff).

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct AdapterId(pub String);

impl AdapterId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Adapter for the M3 ContextCompiler. Returns payload bytes per adapter id.
pub trait ContextAdapter {
    fn adapter_id(&self) -> AdapterId;
    fn compiled_at_log_head(&self) -> u64;
    fn payload(&self) -> Vec<u8>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Staleness {
    pub compiled_at_log_head: u64,
    pub read_at_log_head: u64,
}

impl Staleness {
    pub fn diff(&self) -> i64 {
        self.read_at_log_head as i64 - self.compiled_at_log_head as i64
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextCapsuleV2 {
    pub payload: Vec<u8>,
    pub provenance: Vec<AdapterId>,
    pub staleness: Staleness,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ContextError {
    #[error("adapter {0} produced an out-of-range log head")]
    InvalidLogHead(String),
}

/// Composes 4 adapters into a typed capsule. Unknown sources are preserved
/// (never silently dropped) by recording their payload under a stub id.
pub struct ContextCompiler {
    pub planning: Box<dyn ContextAdapter>,
    pub run: Box<dyn ContextAdapter>,
    pub memory: Box<dyn ContextAdapter>,
    pub graph: Box<dyn ContextAdapter>,
    pub vault_knowledge: Option<Box<dyn ContextAdapter>>,
    /// Optional list of "unknown" sources to record explicitly.
    pub unknown_sources: Vec<AdapterId>,
}

impl ContextCompiler {
    pub fn compile(&self, read_at_log_head: u64) -> Result<ContextCapsuleV2, ContextError> {
        let owned: Vec<AdapterSlot<'_>> = vec![
            self.adapter_slot("planning", &*self.planning),
            self.adapter_slot("run", &*self.run),
            self.adapter_slot("memory", &*self.memory),
            self.adapter_slot("graph", &*self.graph),
        ];
        // Compute parts: ids + bytes for the 4 core adapters. Optionally
        // include vault knowledge; unknown sources recorded in provenance.
        let mut parts: Vec<(&str, Vec<u8>)> = Vec::new();
        let mut provenance_set: BTreeSet<AdapterId> = BTreeSet::new();
        let mut max_compiled: u64 = 0;

        // Validate compiled_at_log_head against read_at_log_head.
        for entry in &owned {
            let h = entry.adapter.compiled_at_log_head();
            if h > read_at_log_head {
                return Err(ContextError::InvalidLogHead(entry.id.clone()));
            }
            if h > max_compiled {
                max_compiled = h;
            }
            parts.push((entry.id.as_str(), entry.adapter.payload()));
            provenance_set.insert(AdapterId::new(&entry.id));
        }

        // Owned list of vault adapter id (so the borrow of v outlives the parts push).
        let mut owned_vault_id: Option<AdapterId> = None;
        if let Some(v) = &self.vault_knowledge {
            let h = v.compiled_at_log_head();
            if h > read_at_log_head {
                return Err(ContextError::InvalidLogHead(
                    v.adapter_id().as_str().to_string(),
                ));
            }
            if h > max_compiled {
                max_compiled = h;
            }
            let v_id = v.adapter_id();
            owned_vault_id = Some(v_id.clone());
            provenance_set.insert(v_id);
        }
        if let (Some(v), Some(v_id)) = (&self.vault_knowledge, owned_vault_id.as_ref()) {
            parts.push((v_id.as_str(), v.payload()));
        }

        for u in &self.unknown_sources {
            provenance_set.insert(u.clone());
        }

        let payload = parts
            .iter()
            .flat_map(|(_, b)| b.iter().copied())
            .collect::<Vec<u8>>();
        let provenance: Vec<AdapterId> = provenance_set.iter().cloned().collect();
        let staleness = Staleness {
            compiled_at_log_head: max_compiled,
            read_at_log_head,
        };
        Ok(ContextCapsuleV2 {
            payload,
            provenance,
            staleness,
        })
    }

    /// Helper: project a borrowed adapter into a (id, payload) tuple of
    /// owned data so the compile body can satisfy the borrow checker while
    /// keeping each adapter call deterministic.
    fn adapter_slot<'a>(
        &self,
        _label: &'a str,
        adapter: &'a dyn ContextAdapter,
    ) -> AdapterSlot<'a> {
        AdapterSlot {
            id: adapter.adapter_id().as_str().to_string(),
            adapter,
        }
    }
}

struct AdapterSlot<'a> {
    id: String,
    adapter: &'a dyn ContextAdapter,
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeAdapter {
        id: AdapterId,
        head: u64,
        bytes: Vec<u8>,
    }
    impl ContextAdapter for FakeAdapter {
        fn adapter_id(&self) -> AdapterId {
            self.id.clone()
        }
        fn compiled_at_log_head(&self) -> u64 {
            self.head
        }
        fn payload(&self) -> Vec<u8> {
            self.bytes.clone()
        }
    }

    fn make_compiler() -> ContextCompiler {
        ContextCompiler {
            planning: Box::new(FakeAdapter {
                id: AdapterId::new("planning"),
                head: 100,
                bytes: b"plan-bytes".to_vec(),
            }),
            run: Box::new(FakeAdapter {
                id: AdapterId::new("run"),
                head: 110,
                bytes: b"run-bytes".to_vec(),
            }),
            memory: Box::new(FakeAdapter {
                id: AdapterId::new("memory"),
                head: 95,
                bytes: b"memory-bytes".to_vec(),
            }),
            graph: Box::new(FakeAdapter {
                id: AdapterId::new("graph"),
                head: 105,
                bytes: b"graph-bytes".to_vec(),
            }),
            vault_knowledge: Some(Box::new(FakeAdapter {
                id: AdapterId::new("vault"),
                head: 100,
                bytes: b"vault-bytes".to_vec(),
            })),
            unknown_sources: vec![AdapterId::new("experimental-source")],
        }
    }

    #[test]
    fn context_compiler_returns_capsule_with_provenance_and_staleness() {
        let compiler = make_compiler();
        let capsule = compiler.compile(150).unwrap();
        assert_eq!(capsule.staleness.compiled_at_log_head, 110);
        assert_eq!(capsule.staleness.read_at_log_head, 150);
        assert_eq!(capsule.staleness.diff(), 40);
        // 4 known + 1 vault + 1 unknown = at least 5 entries.
        assert!(capsule.provenance.len() >= 5);
    }

    #[test]
    fn context_compiler_preserves_unknown_source_paths() {
        let compiler = make_compiler();
        let capsule = compiler.compile(150).unwrap();
        assert!(
            capsule
                .provenance
                .iter()
                .any(|a| a.as_str() == "experimental-source"),
            "unknown source must be preserved in provenance"
        );
    }

    #[test]
    fn context_capsule_provenance_is_ordered_deterministically() {
        let compiler = make_compiler();
        let c1 = compiler.compile(150).unwrap();
        let c2 = compiler.compile(150).unwrap();
        let keys_a: Vec<String> = c1
            .provenance
            .iter()
            .map(|a| a.as_str().to_string())
            .collect();
        let keys_b: Vec<String> = c2
            .provenance
            .iter()
            .map(|a| a.as_str().to_string())
            .collect();
        assert_eq!(keys_a, keys_b, "provenance ordering must be deterministic");
        // Lexicographic ascending.
        let mut sorted = keys_a.clone();
        sorted.sort();
        assert_eq!(keys_a, sorted);
    }

    #[test]
    fn context_compiler_invalid_log_head_returns_typed_error() {
        let mut compiler = make_compiler();
        compiler.run = Box::new(FakeAdapter {
            id: AdapterId::new("run"),
            head: 1_000_000, // > read_at_log_head
            bytes: b"x".to_vec(),
        });
        let err = compiler.compile(150).unwrap_err();
        match err {
            ContextError::InvalidLogHead(s) => {
                assert_eq!(s, "run");
            }
        }
    }

    #[test]
    fn context_capsule_payload_concatenates_adapter_bytes_in_order() {
        let compiler = make_compiler();
        let capsule = compiler.compile(150).unwrap();
        // Concatenation order = adapter insertion order: plan, run, mem, graph, vault.
        let mut expected = Vec::new();
        expected.extend_from_slice(b"plan-bytes");
        expected.extend_from_slice(b"run-bytes");
        expected.extend_from_slice(b"memory-bytes");
        expected.extend_from_slice(b"graph-bytes");
        expected.extend_from_slice(b"vault-bytes");
        assert_eq!(capsule.payload, expected);
    }
}
