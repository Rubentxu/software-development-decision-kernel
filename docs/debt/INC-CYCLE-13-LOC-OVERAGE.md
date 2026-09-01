---
id: INC-CYCLE-13-LOC-OVERAGE
title: "Cycle-13 port_contracts.rs LOC overage vs spec AC M13-002 budget"
status: resolved
severity: medium
priority: P2
fingerprint: "e6c644d1e8ce5f17"
fingerprint_aliases: ["e6c644d1e8ce5f17405c1f1c34627c7823b52966f7fbb54a149fa18bf9cbb101"]
cluster_id: CL-LOC-OVERAGE
created: 2026-08-22
created_by: sddk-verify
owner: orchestrator
resolved_by: p-63676b11dc0ef88f/cycle-13-debt-sweep-correction
resolved_at: 2026-09-01
---

# INC-CYCLE-13-LOC-OVERAGE — port_contracts.rs LOC overage

> Durable record for one debt finding across cycles. See ADR-0047 §3.2.

## Context

Cycle-13 (`kernel-cycle-13-m1-hexagonal-ports`, M1 hexagonal exit, A-min path)
delivered `crates/sddk-engine/tests/port_contracts.rs` at **347 LOC** vs the
spec AC M13-002 budget of **≤150 LOC** (ADR-0048 test-fixtures budget).

The overage is REAL (not misclassification): 347 LOC for 9 contract tests
covering 6 port surfaces (`Ledger`, `EventStore`, `GraphStore`, `ForkStore`,
`ProjectionStore`, `ControlPlane`) plus 2 byte-equivalence cross-checks
(`event_count`, `cycle_record`), plus shared builder helpers
(`mk_project`/`mk_workspace`/`mk_cycle`/`mk_event` ≈ 60 LOC). ~38 LOC/test
density is reasonable for contract tests with shared fixtures.

Implementation LOC: **0** (zero production `src/` change — anti-AC preserved).
Boilerplate LOC: **5** (Cargo.toml + waiver yaml). The overage is isolated
to the test-fixtures category.

The verify phase flagged this as WARNING DEBT-CYCLE-13-PORT-CONTRACTS-LOC-OVERAGE
in `verify-report.md`.

## Rationale

- **Severity = medium**: the deliverable satisfies the spec's behavioral
  requirements (≥8 distinct tests, all required port surfaces covered, 2
  byte-equiv cross-checks). The overage is structural density, not
  correctness. Workaround: extract shared builders to
  `crates/sddk-engine/tests/common/port_contracts_helpers.rs` to drop ~60 LOC
  and split large test bodies (event_store_roundtrip ≈ 47 LOC) into
  helper-built envelopes.

- **Priority = P2**: not blocking release; remediation is a refactor pass
  that does not change behavior. Aligns with the cycle-9 precedent
  (ADR-0047-inc02 §"LOC reality lesson": per-file LOC targets are deprecated,
  total-module-sum is the budget, structural debt may be reabsorbed in
  subsequent cycles).

- **Cluster = `CL-LOC-OVERAGE`** (over-engineering family, test-fixture
  density sub-cluster).

## Lifecycle

| Date | Actor | Change | Evidence |
|------|-------|--------|----------|
| 2026-08-22 | sddk-verify | created | `verify-report.md` §Issues → WARNING 1 (DEBT-CYCLE-13-PORT-CONTRACTS-LOC-OVERAGE); LOC table in §LOC Adjudication |

## References

- `crates/sddk-engine/tests/port_contracts.rs` (347 LOC) — the over-budget file
- `docs/sddk-decision-kernel-architecture/03-adrs/ADR-0048-loc-budget-policy-reformulation.md` — policy
- `docs/adr/ADR-0047-inc01-cycle-8-loc-budget-exception.md` — exception precedent
- `docs/adr/ADR-0047-inc02-cycle-9-apply-discipline-and-loc-reality.md` — LOC reality lesson
- `~/.local/share/sddk/projects/p-52b95ef55999f9de/cycle-artifacts/p-52b95ef55999f9de/kernel-cycle-13-m1-hexagonal-ports/verify-report.md` — verify phase output

## Closure Evidence

Closed by `p-63676b11dc0ef88f/cycle-13-debt-sweep-correction` (v1.66.3).

- **Resolution:** ADR-0048 supersedes per-file LOC targets with total-module-sum budgets. port_contracts.rs (244 LOC) untouched in correction commit. 9/9 byte-equivalence tests pass.
- **Archive manifest:** `archive-manifest.md` sha256 `5445cba1bcd7b268a262e5006f69b6c331ea7b7f458f15a5b417b21ad3e143fb`
- **Release tag:** [v1.66.3](https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v1.66.3)
- **Release receipt sha256:** `fe537a9920a309ecb9a980884c1b4bde0f8d0cf5104f7b2360c7ecf70930ff29`

> Filled by `sddk-archive` (cycle-13); consumed by `sddk-debt-verify` for cross-cycle correlation via fingerprint.