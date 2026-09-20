// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// context_compiler/storage_adapter.rs — AIW-S3
//
// First durable `ContextAdapter` family: snapshots from a backing
// store into the `ContextCompiler` pipeline. The adapters cache
// `(head, payload)` at construction time so `&self`-only `ContextAdapter`
// callers can re-derive the capsule without holding a `Storage` handle.
//
// Layer separation:
//   - `sddk-engine` defines the contract (`StorageSnapshot`) and the
//     concrete adapters (`StorageLedgerHeadAdapter`,
//     `StorageProjectAdapter`).
//   - A `Storage`-backed caller (e.g. tests, future CLI tool) supplies
//     the data via `StorageSnapshot` builders and never couples engine
//     to storage implementation at the dep-graph level.
//
// Slice: `p-63676b11dc0ef88f/aiw-s3-handoff-durable`

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use sddk_domain::LedgerEvent;

use super::{AdapterId, ContextAdapter};

/// A typed snapshot of one backing-store source. The snapshot carries
/// the pre-computed payload bytes and the fact-log head at the moment
/// of snapshotting. The compiler does not require a live backing store
/// at compile time, which is the durability invariant H01 demands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageSnapshot {
    /// Logical adapter id (e.g. `storage.ledger_head`).
    pub adapter_id: String,
    /// Highest ledger sequence observed when the snapshot was taken.
    pub log_head: u64,
    /// Snapshot payload bytes (already content-hashed for ADR-0047).
    pub payload: Vec<u8>,
}

impl StorageSnapshot {
    /// Build a snapshot from a slice of `LedgerEvent`. The payload is
    /// the canonical JSON of the events sorted by `sequence`; the
    /// `log_head` is the maximum `sequence` (or 0 if the slice is
    /// empty). This is the canonical form for AIW-S3 H01.
    pub fn from_ledger_events(adapter_id: impl Into<String>, events: &[LedgerEvent]) -> Self {
        let mut sorted: Vec<&LedgerEvent> = events.iter().collect();
        sorted.sort_by_key(|e| e.sequence);
        let payload = build_payload_from_events(&sorted);
        let log_head = sorted
            .last()
            .map(|e| u64::try_from(e.sequence).unwrap_or(u64::MAX))
            .unwrap_or(0);
        Self {
            adapter_id: adapter_id.into(),
            log_head,
            payload,
        }
    }

    /// Build a snapshot from arbitrary structured bytes (use when the
    /// caller has already serialized the source). The `log_head` is
    /// caller-supplied: ledger-aware sources pass the ledger head,
    /// others pass 0.
    pub fn from_bytes(adapter_id: impl Into<String>, log_head: u64, payload: Vec<u8>) -> Self {
        Self {
            adapter_id: adapter_id.into(),
            log_head,
            payload,
        }
    }
}

/// Compute the canonical JSON payload for a list of `LedgerEvent`s.
///
/// The output is a single line of UTF-8 JSON: a JSON array of canonical
/// event objects, sorted by `sequence`. Hash this to confirm
/// byte-identical reproduction (H01 durability invariant).
fn build_payload_from_events(events: &[&LedgerEvent]) -> Vec<u8> {
    serde_json::to_vec(events).unwrap_or_else(|_| b"[]".to_vec())
}

/// Adapter that exposes a ledger-head snapshot to the compiler.
#[derive(Debug, Clone)]
pub struct StorageLedgerHeadAdapter {
    adapter_id: AdapterId,
    log_head: u64,
    payload: Vec<u8>,
}

impl StorageLedgerHeadAdapter {
    /// Wrap a pre-built snapshot as an adapter.
    pub fn from_snapshot(snapshot: StorageSnapshot) -> Self {
        Self {
            adapter_id: AdapterId::new(snapshot.adapter_id),
            log_head: snapshot.log_head,
            payload: snapshot.payload,
        }
    }

    /// Convenience: build the adapter directly from a `LedgerEvent` slice.
    pub fn from_events(events: &[LedgerEvent]) -> Self {
        Self::from_snapshot(StorageSnapshot::from_ledger_events(
            "storage.ledger_head",
            events,
        ))
    }

    /// Read-only access to the cached `log_head` (for diagnostics).
    pub fn log_head(&self) -> u64 {
        self.log_head
    }

    /// SHA-256 of the cached payload (content-addressable digest).
    pub fn payload_sha256(&self) -> String {
        let mut h = Sha256::new();
        h.update(&self.payload);
        format!("{:x}", h.finalize())
    }
}

impl ContextAdapter for StorageLedgerHeadAdapter {
    fn adapter_id(&self) -> AdapterId {
        self.adapter_id.clone()
    }
    fn compiled_at_log_head(&self) -> u64 {
        self.log_head
    }
    fn payload(&self) -> Vec<u8> {
        self.payload.clone()
    }
}

/// Adapter that exposes a project-record snapshot to the compiler.
///
/// The payload is `sha256(project_id || "|" || created_at)` plus the
/// `created_at` length as a compact `u32` (LE). The motivation is to
/// keep the adapter's payload small and content-addressable without
/// requiring `ProjectRecord` to gain a public `Serialize` impl (which
/// would be a public-contract change — see SCOPE §3 C2).
#[derive(Debug, Clone)]
pub struct StorageProjectAdapter {
    adapter_id: AdapterId,
    payload: Vec<u8>,
    project_id_hash: String,
}

impl StorageProjectAdapter {
    /// Build an adapter from a `(project_id, created_at)` pair.
    ///
    /// The `project_id` is hashed (sha256, hex) and recorded as
    /// `project_id_hash` so the snapshot can be cross-checked at the
    /// boundary; the payload bytes encode `(created_at_len: u32 LE,
    /// created_at: bytes, project_id_hash: 32 bytes hex)`.
    pub fn new(project_id: &str, created_at: &str) -> Self {
        let project_id_hash = sha256_hex(project_id.as_bytes());
        let created_len: u32 = created_at.len().try_into().unwrap_or(u32::MAX);
        let mut payload = Vec::with_capacity(4 + created_at.len() + project_id_hash.len());
        payload.extend_from_slice(&created_len.to_le_bytes());
        payload.extend_from_slice(created_at.as_bytes());
        payload.extend_from_slice(project_id_hash.as_bytes());
        Self {
            adapter_id: AdapterId::new("storage.project"),
            payload,
            project_id_hash,
        }
    }

    /// Read-only accessor for the project hash (caller diagnostics).
    pub fn project_id_hash(&self) -> &str {
        &self.project_id_hash
    }
}

impl ContextAdapter for StorageProjectAdapter {
    fn adapter_id(&self) -> AdapterId {
        self.adapter_id.clone()
    }
    fn compiled_at_log_head(&self) -> u64 {
        // Project records are not ledger-aware; report 0 so they
        // never pin the capsule to a stale ledger head.
        0
    }
    fn payload(&self) -> Vec<u8> {
        self.payload.clone()
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context_compiler::ContextCompiler;

    #[test]
    fn ledger_head_adapter_reports_id_and_head() {
        let adapter = StorageLedgerHeadAdapter::from_events(&[]);
        assert_eq!(adapter.adapter_id().as_str(), "storage.ledger_head");
        assert_eq!(adapter.log_head(), 0);
        assert_eq!(adapter.compiled_at_log_head(), 0);
    }

    #[test]
    fn ledger_head_adapter_payload_hashes_deterministically() {
        let a = StorageLedgerHeadAdapter::from_events(&[]);
        let b = StorageLedgerHeadAdapter::from_events(&[]);
        assert_eq!(a.payload_sha256(), b.payload_sha256());
    }

    #[test]
    fn project_adapter_reports_zero_head_for_non_ledger_source() {
        let adapter = StorageProjectAdapter::new("project-1", "2026-01-01T00:00:00Z");
        assert_eq!(adapter.adapter_id().as_str(), "storage.project");
        assert_eq!(adapter.compiled_at_log_head(), 0);
        assert_eq!(adapter.project_id_hash().len(), 64);
    }

    #[test]
    fn project_adapter_payload_is_stable_for_same_inputs() {
        let a = StorageProjectAdapter::new("project-1", "2026-01-01T00:00:00Z");
        let b = StorageProjectAdapter::new("project-1", "2026-01-01T00:00:00Z");
        assert_eq!(a.payload(), b.payload());
    }

    #[test]
    fn compiler_with_storage_adapters_produces_capsule() {
        let compiler = ContextCompiler {
            planning: Box::new(StorageLedgerHeadAdapter::from_events(&[])),
            run: Box::new(StorageProjectAdapter::new("p", "2026-01-01T00:00:00Z")),
            memory: Box::new(StorageProjectAdapter::new("p", "2026-01-01T00:00:00Z")),
            graph: Box::new(StorageLedgerHeadAdapter::from_events(&[])),
            vault_knowledge: None,
            unknown_sources: vec![],
        };
        let capsule = compiler.compile(0).expect("compile");
        assert_eq!(capsule.staleness.compiled_at_log_head, 0);
        assert_eq!(capsule.staleness.read_at_log_head, 0);
        assert_eq!(capsule.staleness.diff(), 0);
        // The compiler deduplicates adapter ids in a BTreeSet, so the
        // two unique ids (ledger_head + project) each appear once.
        assert_eq!(capsule.provenance.len(), 2);
        assert!(
            capsule
                .provenance
                .iter()
                .any(|id| id.as_str() == "storage.ledger_head"),
            "storage.ledger_head missing from provenance"
        );
        assert!(
            capsule
                .provenance
                .iter()
                .any(|id| id.as_str() == "storage.project"),
            "storage.project missing from provenance"
        );
    }

    #[test]
    fn compiler_with_dedup_of_payload_is_stable_across_two_snapshots() {
        // Same data twice → byte-identical payload → capsule payload
        // is identical (H01 stability invariant).
        let compiler_a = ContextCompiler {
            planning: Box::new(StorageLedgerHeadAdapter::from_events(&[])),
            run: Box::new(StorageProjectAdapter::new("p", "2026-01-01T00:00:00Z")),
            memory: Box::new(StorageProjectAdapter::new("p", "2026-01-01T00:00:00Z")),
            graph: Box::new(StorageLedgerHeadAdapter::from_events(&[])),
            vault_knowledge: None,
            unknown_sources: vec![],
        };
        let compiler_b = ContextCompiler {
            planning: Box::new(StorageLedgerHeadAdapter::from_events(&[])),
            run: Box::new(StorageProjectAdapter::new("p", "2026-01-01T00:00:00Z")),
            memory: Box::new(StorageProjectAdapter::new("p", "2026-01-01T00:00:00Z")),
            graph: Box::new(StorageLedgerHeadAdapter::from_events(&[])),
            vault_knowledge: None,
            unknown_sources: vec![],
        };
        let cap_a = compiler_a.compile(0).expect("compile a");
        let cap_b = compiler_b.compile(0).expect("compile b");
        assert_eq!(cap_a.payload, cap_b.payload);
        assert_eq!(cap_a.staleness, cap_b.staleness);
        assert_eq!(cap_a.provenance.len(), cap_b.provenance.len());
    }
}
