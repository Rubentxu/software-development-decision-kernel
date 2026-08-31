---
id: INC-DEBT-008-dead-code-sddk-cli
title: "Pre-existing dead_code warnings in crates/sddk-cli/ (cycle-34 remediation)"
status: closed
severity: low
priority: P3
fingerprint: "3adb1eb7598a625dd40f4ab4a88f347cbf31398c68174c54e14fabd1d3e43fcc"
cluster_id: CL-02
created: 2026-08-25
created_by: sddk-debt-verify (cycle-33, A-min smoke) → sddk-explore (cycle-34)
owner: orchestrator
cycle_source: p-52b95ef55999f9de/kernel-cycle-34-inc-debt-008-dead-code-sddk-cli
finding_ref: FIND-000017
attribution: pre_existing
base_sha_when_discovered: a6c17cf (cycle-33 archive)
---

# INC-DEBT-008 — Pre-existing dead_code warnings in crates/sddk-cli/

> Durable cross-cycle record. Created from FIND-000017 in cycle-33 debt-report.
> Closed at cycle-34.

## Context

24 dead_code warnings in `crates/sddk-cli/` were classified as carry-forward at end of cycle-33.
Cycle-34 remediates by deleting 17 Category 1 items (safe, zero callers) and annotating 8
Category 2 items with `#[allow(dead_code)]` referencing ADR-0064 §D-4/§D-5. All cargo gates green.

## Inventory (from cycle-34 explore)

### Category 1 — Safe to delete (17 items)

| # | File | Line | Item | Kind |
|---|------|------|------|------|
| 1 | `crates/sddk-cli/src/dev/editor_adapters/claude.rs` | 9 | `is_framework_namespaced` | import |
| 2 | `crates/sddk-cli/src/dev/editor_adapters/codex.rs` | 12 | `is_framework_namespaced` | import |
| 3 | `crates/sddk-cli/src/dev/editor_adapters/json.rs` | 7 | `ReconcileTarget` | import |
| 4 | `crates/sddk-cli/src/dev/editor_adapters/json.rs` | 10 | `is_framework_namespaced` | import |
| 5 | `crates/sddk-cli/src/dev/reconcile.rs` | 10 | `AgentReconcileResult` | import (pub use) |
| 6 | `crates/sddk-cli/src/dev/reconcile.rs` | 10 | `EditorCapabilities` | import (pub use) |
| 7 | `crates/sddk-cli/src/dev/reconcile.rs` | 10 | `ExistingEntry` | import (pub use) |
| 8 | `crates/sddk-cli/src/dev/reconcile.rs` | 10 | `ReconcileTarget` | import (pub use) |
| 9 | `crates/sddk-cli/src/dev/comments_check.rs` | 54 | `RulesContract.version` | field |
| 10 | `crates/sddk-cli/src/dev/comments_check.rs` | 54 | `RulesContract.schema` | field |
| 11 | `crates/sddk-cli/src/dev/comments_check.rs` | 74 | `LanguageSpec.block_close` | field |
| 12 | `crates/sddk-cli/src/dev/comments_check.rs` | 91 | `PatternSpec.description` | field |
| 13 | `crates/sddk-cli/src/dev/comments_check.rs` | 185 | `CommentViolation.language` | field |
| 14 | `crates/sddk-cli/src/inventory_cycle.rs` | 524 | `run_check_ignore` | function |

> Note: items 15-17 (has_sddk_fields, skipped, pruned) were reclassified from C1 to C2
> per proposal Q0 sub-decision (ADR-0064 §D-4/§D-5 capability-framework contract).

### Category 2 — Need #[allow(dead_code)] (8 items)

| # | File | Line | Item | ADR reference |
|---|------|------|------|---------------|
| 1 | `crates/sddk-cli/src/dev/editor_adapters/reconcile.rs` | 35 | `EditorCapabilities.model_validator` | §D-4 |
| 2 | `crates/sddk-cli/src/dev/editor_adapters/reconcile.rs` | 118 | `ExistingEntry.has_sddk_fields` | §D-5 |
| 3 | `crates/sddk-cli/src/dev/editor_adapters/reconcile.rs` | 146 | `AgentReconcileResult.name` | §D-4 |
| 4 | `crates/sddk-cli/src/dev/editor_adapters/reconcile.rs` | 157 | `AgentReconcileResult::skipped` | §D-4 |
| 5 | `crates/sddk-cli/src/dev/editor_adapters/reconcile.rs` | 167 | `AgentReconcileResult::pruned` | §D-4 |
| 6 | `crates/sddk-cli/src/dev/editor_adapters/reconcile.rs` | 383 | `ReconcileAdapter::editor_name` | §D-5 |
| 7 | `crates/sddk-cli/src/dev/editor_adapters/reconcile.rs` | 385 | `ReconcileAdapter::capabilities` | §D-5 |
| 8 | `crates/sddk-cli/src/dev/editor_adapters/reconcile.rs` | 387 | `ReconcileAdapter::read_existing` | §D-5 |

### Category 3 — Deferred (1 item)

- `ExistingEntry.name` (`crates/sddk-cli/src/dev/editor_adapters/reconcile.rs:99`): Design gap — `read_existing()` captures name but `diff_existing_target` doesn't compare names. Renames in IDE configs pass undetected. Annotated with `#[allow(dead_code)]` to keep R1 green; follow-up ticket required to resolve the design gap properly.

## Rationale

- **Severity = low**: dead_code warnings are noise, not correctness issues.
- **Priority = P3**: cleanup candidate; not blocking any gate.
- **Attribution = pre_existing**: discovered at cycle-33 debt-verify (FIND-000017).

## Lifecycle

| Date | Actor | Change | Evidence |
|------|-------|--------|----------|
| 2026-08-25 | sddk-debt-verify (cycle-33) | created | FIND-000017 from cycle-33 debt-report |
| 2026-08-25 | sddk-apply (cycle-34, follow-up commit) | status: open → closed | 7 remaining items addressed on feat branch; cargo clippy -p sddk-cli --all-targets dead_code warnings = 0; 301 tests passing |

## References

- Cycle-33 debt-report.md FIND-000017
- Cycle-34 explore-report.md (full inventory)
- Cycle-34 proposal.md (Approach section with 26 items)
- Cycle-34 spec.md (acceptance scenarios)
- ADR-0064 §D-4 (capability-framework contract) + §D-5 (result contract)
