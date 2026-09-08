//! Engineering Assurance: Evidence Resolvers and Deterministic Evaluators.
//!
//! Cycle: EA-ASSURANCE-002 (H7, order 410, context pack
//! `engineering-assurance`).
//!
//! Pure, deterministic adapter-fact resolver that converts a stream
//! of typed `AdapterFact` records into `Vec<AssuranceEvidence>`
//! (sorted in canonical order) suitable for feeding the
//! `AssuranceEngine` (`EA-ASSURANCE-001`).
//!
//! ## Design
//!
//! - **Closed-set `AdapterFact`.** Eight variants cover adapter
//!   outputs (cargo test, clippy, fmt, receipt, replay proof,
//!   bridge marker, custom).
//! - **Pure, deterministic resolver.** Same input ⇒ same output.
//! - **Sorted output.** Output is sorted by `EvidenceTag::id()`
//!   ascending, ties broken by `(source, collected_at)`.
//! - **Reuses `EvidenceTag`.** No duplicate taxonomy.
//! - **`#[non_exhaustive]`** on every public type.
//!
//! See:
//!
//! - `REQ-EngineeringAssuranceResolvers` (spec, accepted)
//! - `ADR-102` (architecture decision, accepted)
//! - `REQ-EngineeringAssuranceProfiles` (dependency)

use serde::{Deserialize, Serialize};

use crate::engineering_assurance::{AssuranceEvidence, EvidenceTag};

// ── AdapterFact ──────────────────────────────────────────────────────────

/// Closed-set adapter fact vocabulary.
///
/// Adapters parse their output (cargo test stdout, clippy JSONL,
/// receipt files, etc.) and emit a stream of these facts to the
/// resolver. The resolver reshapes facts into `AssuranceEvidence`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum AdapterFact {
    /// A `cargo test` summary record.
    CargoTest {
        /// Passed test count for this summary.
        passed: usize,
        /// Failed test count for this summary.
        failed: usize,
    },
    /// A single `cargo clippy` line at warning or error level.
    ClippyLine {
        /// Level (e.g. "warning", "error", "deny").
        level: String,
        /// Lint code (e.g. "unused_variables").
        code: String,
        /// Lint message.
        message: String,
    },
    /// A `cargo fmt --check` line about a file.
    FmtLine {
        /// File path being checked.
        filepath: String,
        /// Whether the file needs formatting.
        needs_format: bool,
    },
    /// A receipt file record.
    ReceiptRecord {
        /// SHA-256 hex string.
        sha256: String,
        /// Human-readable description.
        description: String,
    },
    /// A replay proof record for a cycle.
    ReplayProofRecord {
        /// Cycle id.
        cycle: String,
        /// Number of attempts in the proof.
        attempts: usize,
    },
    /// A bridge marker closure record (canonical → lab, etc.).
    BridgeMarkerClosed {
        /// Bridge name (e.g. "canonical_to_lab").
        bridge: String,
    },
    /// Custom fact (extension seam).
    Custom {
        /// Kind identifier.
        kind: String,
        /// Opaque payload.
        payload: serde_json::Value,
    },
}

impl AdapterFact {
    /// Stable identifier (used for sorting and audit).
    pub fn id(&self) -> String {
        match self {
            AdapterFact::CargoTest { passed, failed } => {
                format!("cargo_test(passed={passed},failed={failed})")
            }
            AdapterFact::ClippyLine { level, code, .. } => {
                format!("clippy_line({level}:{code})")
            }
            AdapterFact::FmtLine {
                filepath,
                needs_format,
            } => {
                format!("fmt_line({filepath},needs={needs_format})")
            }
            AdapterFact::ReceiptRecord { sha256, .. } => {
                format!("receipt_record(sha256={sha256})")
            }
            AdapterFact::ReplayProofRecord { cycle, attempts } => {
                format!("replay_proof_record(cycle={cycle},attempts={attempts})")
            }
            AdapterFact::BridgeMarkerClosed { bridge } => {
                format!("bridge_marker_closed(bridge={bridge})")
            }
            AdapterFact::Custom { kind, .. } => format!("custom({kind})"),
        }
    }
}

// ── EvidenceResolver ─────────────────────────────────────────────────────

/// Pure, deterministic evidence resolver.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct EvidenceResolver {
    source: String,
}

impl EvidenceResolver {
    /// Construct a resolver bound to a `source` label (used as
    /// `AssuranceEvidence.source`).
    pub fn new(source: String) -> Self {
        Self { source }
    }

    /// Resolver source label.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Resolve a slice of adapter facts into `AssuranceEvidence`.
    pub fn resolve(&self, facts: &[AdapterFact], collected_at: String) -> Vec<AssuranceEvidence> {
        let mut ev: Vec<AssuranceEvidence> = Vec::new();
        let mut clippy_warnings = 0_usize;
        let mut fmt_violations = 0_usize;
        let mut cargo_total_passed = 0_usize;
        let mut cargo_total_failed = 0_usize;
        let mut cargo_clean = true;
        let mut receipts: Vec<String> = Vec::new();
        let mut replay_proofs: Vec<(String, usize)> = Vec::new();
        let mut bridge_markers: Vec<String> = Vec::new();

        for f in facts {
            match f {
                AdapterFact::CargoTest { passed, failed } => {
                    cargo_total_passed += *passed;
                    cargo_total_failed += *failed;
                    if *failed > 0 {
                        cargo_clean = false;
                    }
                }
                AdapterFact::ClippyLine { .. } => {
                    clippy_warnings += 1;
                }
                AdapterFact::FmtLine { needs_format, .. } => {
                    if *needs_format {
                        fmt_violations += 1;
                    }
                }
                AdapterFact::ReceiptRecord { sha256, .. } => {
                    receipts.push(sha256.clone());
                }
                AdapterFact::ReplayProofRecord { cycle, attempts } => {
                    replay_proofs.push((cycle.clone(), *attempts));
                }
                AdapterFact::BridgeMarkerClosed { bridge } => {
                    bridge_markers.push(bridge.clone());
                }
                AdapterFact::Custom { .. } => {
                    // Custom facts are not resolved by default.
                }
            }
        }

        if cargo_clean && (cargo_total_passed + cargo_total_failed) > 0 {
            ev.push(AssuranceEvidence {
                tag: EvidenceTag::EngineTest {
                    count: cargo_total_passed + cargo_total_failed,
                    passed: cargo_total_passed,
                },
                collected_at: collected_at.clone(),
                source: self.source.clone(),
            });
        }
        if clippy_warnings == 0 {
            ev.push(AssuranceEvidence {
                tag: EvidenceTag::ClippyClean,
                collected_at: collected_at.clone(),
                source: self.source.clone(),
            });
        }
        if fmt_violations == 0 {
            ev.push(AssuranceEvidence {
                tag: EvidenceTag::FmtClean,
                collected_at: collected_at.clone(),
                source: self.source.clone(),
            });
        }
        for r in receipts {
            ev.push(AssuranceEvidence {
                tag: EvidenceTag::Receipt { sha256: r },
                collected_at: collected_at.clone(),
                source: self.source.clone(),
            });
        }
        for (cycle, attempts) in replay_proofs {
            ev.push(AssuranceEvidence {
                tag: EvidenceTag::ReplayProof {
                    cyc_id: cycle,
                    attempts,
                },
                collected_at: collected_at.clone(),
                source: self.source.clone(),
            });
        }
        for bridge in bridge_markers {
            ev.push(AssuranceEvidence {
                tag: EvidenceTag::BridgeClosed,
                collected_at: collected_at.clone(),
                source: self.source.clone(),
            });
            // bridge names are diagnostic; we only emit BridgeClosed
            // (the BridgeClosed tag is opaque), so the bridge value
            // is informational. No extra tag for bridge name alone.
            let _ = bridge;
        }

        // Deterministic ordering: by tag id, then source, then
        // collected_at.
        ev.sort_by(|a, b| {
            a.tag
                .id()
                .cmp(&b.tag.id())
                .then_with(|| a.source.cmp(&b.source))
                .then_with(|| a.collected_at.cmp(&b.collected_at))
        });
        ev
    }
}

// ── Audit guard ──────────────────────────────────────────────────────────

#[allow(unused)]
const ADAPTER_FACT_VARIANT_LIST: &[AdapterFact] = &[
    AdapterFact::CargoTest {
        passed: 0,
        failed: 0,
    },
    AdapterFact::ClippyLine {
        level: String::new(),
        code: String::new(),
        message: String::new(),
    },
    AdapterFact::FmtLine {
        filepath: String::new(),
        needs_format: false,
    },
    AdapterFact::ReceiptRecord {
        sha256: String::new(),
        description: String::new(),
    },
    AdapterFact::ReplayProofRecord {
        cycle: String::new(),
        attempts: 0,
    },
    AdapterFact::BridgeMarkerClosed {
        bridge: String::new(),
    },
    AdapterFact::Custom {
        kind: String::new(),
        payload: serde_json::Value::Null,
    },
];

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn resolver() -> EvidenceResolver {
        EvidenceResolver::new("sddk-engine".to_string())
    }

    // ── S-1: Cargo test passes ⇒ single EngineTest evidence ──────────────

    #[test]
    fn s1_cargo_test_passes_single_engine() {
        let r = resolver();
        let facts = vec![
            AdapterFact::CargoTest {
                passed: 50,
                failed: 0,
            },
            AdapterFact::CargoTest {
                passed: 30,
                failed: 0,
            },
            AdapterFact::CargoTest {
                passed: 20,
                failed: 0,
            },
        ];
        let ev = r.resolve(&facts, "t".to_string());
        let et: Vec<&AssuranceEvidence> = ev
            .iter()
            .filter(|e| matches!(e.tag, EvidenceTag::EngineTest { .. }))
            .collect();
        assert_eq!(et.len(), 1);
        match &et[0].tag {
            EvidenceTag::EngineTest { count, passed } => {
                assert_eq!(*count, 100);
                assert_eq!(*passed, 100);
            }
            _ => unreachable!(),
        }
    }

    // ── S-2: Cargo test fails ⇒ no EngineTest evidence ──────────────────

    #[test]
    fn s2_cargo_test_fails_no_engine() {
        let r = resolver();
        let facts = vec![AdapterFact::CargoTest {
            passed: 50,
            failed: 1,
        }];
        let ev = r.resolve(&facts, "t".to_string());
        assert!(
            !ev.iter()
                .any(|e| matches!(e.tag, EvidenceTag::EngineTest { .. }))
        );
    }

    // ── S-3: Clippy clean when no clippy lines ──────────────────────────

    #[test]
    fn s3_clippy_clean_when_no_lines() {
        let r = resolver();
        let ev = r.resolve(&[], "t".to_string());
        assert!(ev.iter().any(|e| matches!(e.tag, EvidenceTag::ClippyClean)));
    }

    // ── S-4: Clippy warning ⇒ no ClippyClean ────────────────────────────

    #[test]
    fn s4_clippy_warning_no_clean() {
        let r = resolver();
        let facts = vec![AdapterFact::ClippyLine {
            level: "warning".to_string(),
            code: "unused_variables".to_string(),
            message: "msg".to_string(),
        }];
        let ev = r.resolve(&facts, "t".to_string());
        assert!(!ev.iter().any(|e| matches!(e.tag, EvidenceTag::ClippyClean)));
    }

    // ── S-5: Fmt clean when no fmt violations ───────────────────────────

    #[test]
    fn s5_fmt_clean_when_no_violations() {
        let r = resolver();
        let ev = r.resolve(&[], "t".to_string());
        assert!(ev.iter().any(|e| matches!(e.tag, EvidenceTag::FmtClean)));
    }

    // ── S-6: Fmt violation ⇒ no FmtClean ────────────────────────────────

    #[test]
    fn s6_fmt_violation_no_clean() {
        let r = resolver();
        let facts = vec![AdapterFact::FmtLine {
            filepath: "x.rs".to_string(),
            needs_format: true,
        }];
        let ev = r.resolve(&facts, "t".to_string());
        assert!(!ev.iter().any(|e| matches!(e.tag, EvidenceTag::FmtClean)));
    }

    // ── S-7: Receipts per receipt record ────────────────────────────────

    #[test]
    fn s7_receipts_per_record() {
        let r = resolver();
        let facts = vec![
            AdapterFact::ReceiptRecord {
                sha256: "a".to_string(),
                description: "r1".to_string(),
            },
            AdapterFact::ReceiptRecord {
                sha256: "b".to_string(),
                description: "r2".to_string(),
            },
        ];
        let ev = r.resolve(&facts, "t".to_string());
        let rs: Vec<&AssuranceEvidence> = ev
            .iter()
            .filter(|e| matches!(e.tag, EvidenceTag::Receipt { .. }))
            .collect();
        assert_eq!(rs.len(), 2);
    }

    // ── S-8: Replay proof per record ────────────────────────────────────

    #[test]
    fn s8_replay_proof_per_record() {
        let r = resolver();
        let facts = vec![AdapterFact::ReplayProofRecord {
            cycle: "C1".to_string(),
            attempts: 7,
        }];
        let ev = r.resolve(&facts, "t".to_string());
        let mut ok = false;
        for e in &ev {
            if let EvidenceTag::ReplayProof { cyc_id, attempts } = &e.tag
                && cyc_id == "C1"
                && *attempts == 7
            {
                ok = true;
                break;
            }
        }
        assert!(ok);
    }

    // ── S-9: Bridge marker closed ───────────────────────────────────────

    #[test]
    fn s9_bridge_marker_closed() {
        let r = resolver();
        let facts = vec![AdapterFact::BridgeMarkerClosed {
            bridge: "canonical_to_lab".to_string(),
        }];
        let ev = r.resolve(&facts, "t".to_string());
        assert!(
            ev.iter()
                .any(|e| matches!(e.tag, EvidenceTag::BridgeClosed))
        );
    }

    // ── S-10: Determinism ───────────────────────────────────────────────

    #[test]
    fn s10_determinism_byte_equal() {
        let r = resolver();
        let facts = vec![
            AdapterFact::CargoTest {
                passed: 100,
                failed: 0,
            },
            AdapterFact::ReceiptRecord {
                sha256: "abc".to_string(),
                description: "r".to_string(),
            },
            AdapterFact::ReplayProofRecord {
                cycle: "C1".to_string(),
                attempts: 3,
            },
            AdapterFact::BridgeMarkerClosed {
                bridge: "b".to_string(),
            },
        ];
        let a = r.resolve(&facts, "t".to_string());
        let b = r.resolve(&facts, "t".to_string());
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap()
        );
    }

    // ── Extra: AdapterFact::id stability ────────────────────────────────

    #[test]
    fn adapter_fact_ids() {
        assert_eq!(
            AdapterFact::CargoTest {
                passed: 1,
                failed: 2
            }
            .id(),
            "cargo_test(passed=1,failed=2)".to_string()
        );
        assert_eq!(
            AdapterFact::BridgeMarkerClosed {
                bridge: "b".to_string()
            }
            .id(),
            "bridge_marker_closed(bridge=b)".to_string()
        );
        assert_eq!(EvidenceTag::ClippyClean.id(), "clippy_clean".to_string());
    }

    // ── Extra: Custom facts do not produce evidence ────────────────────

    #[test]
    fn custom_facts_ignored() {
        let r = resolver();
        let facts = vec![AdapterFact::Custom {
            kind: "custom".to_string(),
            payload: serde_json::json!({}),
        }];
        let ev = r.resolve(&facts, "t".to_string());
        // Only ClippyClean + FmtClean + (no EngineTest because no
        // CargoTest fact) but no Receipt / ReplayProof / BridgeClosed.
        assert_eq!(ev.len(), 2);
    }

    // ── Extra: source propagates ────────────────────────────────────────

    #[test]
    fn source_propagates() {
        let r = EvidenceResolver::new("my-source".to_string());
        let ev = r.resolve(&[], "t".to_string());
        for e in &ev {
            assert_eq!(e.source, "my-source");
        }
    }
}
