---
id: INC-CYCLE-14-SEVERITY-SPEC-DRIFT
title: "JournalProjection severity policy diverges from SPEC-027 categories (pack/runtime collapsed into default)"
status: resolved
severity: low
priority: P3
fingerprint: "2060c4f2b969e014"
fingerprint_aliases: []
cluster_id: CL-COUPLING
created: 2026-08-22
created_by: sddk-debt-verify
owner: orchestrator
resolved_by: p-63676b11dc0ef88f/cycle-50-housekeeping-p3
resolved_at: 2026-09-01
---

# INC-CYCLE-14-SEVERITY-SPEC-DRIFT — severity_for_event_type ↔ SPEC-027 drift

> Durable record for one debt finding across cycles. See ADR-0047 §3.2.

## Context

Cycle-14 (`p-52b95ef55999f9de/kernel-cycle-14-m2-event-foundation`,
A-min path) added `JournalProjection` in commit `edfc5fe` with a
locked severity policy table at
`crates/sddk-domain/src/projections.rs:458-486`.

The in-source policy is 7 rows:

```text
workflow.*        → Medium
execution.*       → Low      (incl. attempt.*, tool.*)
routing.*         → High     (incl. provider.*)
context.*         → Low
governance.*      → High     (incl. proposal.*, policy.*, approval.*, capability.*, receipt.*)
evidence.*        → Medium   (incl. uat.*)
human.*           → Critical
```

The driving spec, `SPEC-027-EVENT-TAXONOMY.md`
(`docs/sddk-decision-kernel-architecture/04-specs/`), lists **8 categories**:

```text
Workflow       → SPEC-027 §Categories
Execution      → SPEC-027 §Categories
Routing/provider → SPEC-027 §Categories
Context        → SPEC-027 §Categories
Governance     → SPEC-027 §Categories
Evidence/UAT   → SPEC-027 §Categories
Human          → SPEC-027 §Categories
Pack/runtime   → SPEC-027 §Categories  ← excluded from the 7-row table
```

Drift:

- **Pack/runtime** (`pack.*`, `behavior.*`, `supervisor.signal.*`,
  `supervisor.decision.*`) is in SPEC-027 as its own 8th category but
  is **omitted from the journal severity policy**. It falls through to
  the default `Severity::Medium`.
- Drift is **documented inline** at the module comment
  (`projections.rs:451-452`: "Pack/runtime events ... are excluded
  from the journal as they are internal runtime events").

The drift is therefore a deliberate engineering decision, not a silent
omission. Risk: future editors adding a SPEC-027 category row in the
doc may not realize the code consolidates two columns (Evidence and
UAT) into a single branch and silently drops the 8th category. This is a
**one-way drift** unless the in-source comment is mirrored into
SPEC-027 itself or a follow-up ADR records the consolidation decision.

## Rationale

- **Severity = low**: the journal policy is currently self-consistent
  and the drift is documented in the module. The regression surface is
  bounded: future SPEC-027 amendments may add categories without the
  code responding, but the default `Medium` is conservative enough
  that no event is silently misclassified as Critical.

- **Priority = P3**: opportunistic; the recommended remediation is one
  of:

  1. Add a `///` cross-reference at the top of `severity_for_event_type`
     pointing to `SPEC-027-EVENT-TAXONOMY.md` and noting the 7→8
     consolidation (one paragraph).
  2. Add a unit test asserting that all 8 SPEC-027 categories resolve
     to a defined severity (with pack/runtime explicitly accepting
     the default) — this would surface any future SPEC-027 amendment
     as a failing test.

  Either option is ≤30 LOC, no behavioural change.

- **Cluster = `CL-COUPLING`** (cross-source coupling risk between code
  and spec). The launch packet restricts the debt-verify scope to
  `coupling + overeng` clusters for A-min smoke; this is the first
  finding under `CL-COUPLING` for cycle-14.

## Lifecycle

| Date | Actor | Change | Evidence |
|------|-------|--------|----------|
| 2026-08-22 | sddk-debt-verify | created | 7-row policy at `projections.rs:458-486`; SPEC-027 lists 8 categories including pack/runtime |

## Closure Evidence

Closed by `p-63676b11dc0ef88f/cycle-50-housekeeping-p3` (v1.66.4).

- **Resolution:** Extended `severity_for_event_type` rustdoc with `# Severity policy cross-reference` section referencing SPEC-027, naming the 7-row vs 8-category consolidation (evidence.* + uat.* merged), and naming the pack/runtime exclusion by design. 7-branch table body unchanged; locked test `journal_projection_severity_table_locked` still asserts 7 rows and passes.
- **Closing commit:** `b5f7a4a` — docs(rustdoc): extend severity_for_event_type rustdoc with SPEC-027 cross-reference (cycle-50 commit #5)
- **Release tag:** [v1.66.4](https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v1.66.4)

## References

- `crates/sddk-domain/src/projections.rs:435-452` — module comment listing the 7-row mapping + the pack/runtime exclusion note
- `crates/sddk-domain/src/projections.rs:454-486` — `severity_for_event_type` implementation
- `crates/sddk-domain/src/projections.rs:915-952` — `journal_projection_severity_table_locked` test (7 rows asserted)
- `docs/sddk-decision-kernel-architecture/04-specs/SPEC-027-EVENT-TAXONOMY.md` — 8-category enumeration
- `docs/sddk-decision-kernel-architecture/04-specs/README.md` — SPEC-027 catalog entry
