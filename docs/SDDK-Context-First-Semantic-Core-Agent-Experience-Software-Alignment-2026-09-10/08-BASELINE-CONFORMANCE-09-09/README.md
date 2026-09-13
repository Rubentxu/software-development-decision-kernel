# Baseline Conformance 09/09 — Semantic Core + Agent Experience

**Purpose:** close the 2026-09-09 `Semantic Core + Agent Experience Consolidation` contract to **100% demonstrable conformance** before the context-first R0–R11 evolution proceeds.

This directory is not a new bounded context and does not introduce product semantics. It is a **temporary conformance programme** over the existing baseline.

## Normative baseline

The contract under verification is:

`SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09`

and specifically its:

- ADR-001..ADR-017;
- SPEC-001..SPEC-018;
- M0..M9 roadmap;
- UAT-01..UAT-22;
- migration/deprecation policy;
- architecture and agent-experience ratchets.

The 2026-09-10 context-first package evolves that baseline. It MUST NOT be used to excuse an unmet 09/09 acceptance criterion. Conversely, a requirement introduced only on 10/09 is not retroactively considered a 09/09 defect.

## Definition of 100% conformance

The baseline is conformant only when all of the following are true:

1. every SPEC-001..018 has an implementation owner, code evidence and at least one executable verification;
2. UAT-01..22 are mapped to real tests/fixtures/commands and all pass against a clean installed artifact;
3. M9 removals are complete: no deprecated production authority/write path remains reachable;
4. one canonical authority exists for events, evidence, lifecycle runtime truth, revision primitives and governed side effects;
5. compatibility code that remains is read-only, time-bounded and explicitly justified;
6. architecture drift rules are blocking, not advisory;
7. documentation status agrees with implementation status;
8. a `09-09-CONFORMANCE-RECEIPT.md` is produced with no unresolved MUST-level finding.

Passing `cargo test --workspace` alone is necessary but not sufficient.

## Known audit findings to close

The 2026-09-13 audit identified these concrete closeout items:

- `events_v1` and legacy `ledger_events` still coexist;
- `PlanningEvidenceKind` migration is explicitly deferred;
- runtime-specific `CycleStatus` variants remain (`UatWaiting`, `ApprovalPending`, `Recovering`, review of `Remediating`);
- `AuthorityEngine` exists but legacy authority paths remain during additive migration;
- Decision Memory still contains Git-like history primitives that overlap the common revision substrate;
- CommandRegistry/CommandSpec is protected by parity tests but remains hand-curated relative to the public command enum;
- repository-native architecture-spec status metadata can still say `proposed` while the roadmap declares delivery;
- the 22 UATs have not yet been demonstrated through one explicit UAT→test→result matrix.

Two items audited as aligned and therefore protected from unnecessary refactoring:

- `SemanticGraph` is the canonical rebuildable graph projection;
- `ActiveGraph` may remain only as a derived view/compatibility projection, never as authority.

## Execution order

Read and implement in this order:

1. `specs/SPEC-CONF-001-BASELINE-CONFORMANCE-CONTRACT.md`
2. `roadmap/CONFORMANCE-CLOSEOUT-ROADMAP.md`
3. `reference/09-09-SPEC-CROSSWALK.md`
4. targeted closeout specs 002..008
5. `uat/UAT-09-09-CONFORMANCE-MASTER.md`
6. `uat/CONFORMANCE-FITNESS-RATCHETS.md`
7. `reference/CONFORMANCE-RECEIPT-TEMPLATE.md`

The closeout roadmap is a prerequisite for `03-ROADMAP/ROADMAP.md` R0.
