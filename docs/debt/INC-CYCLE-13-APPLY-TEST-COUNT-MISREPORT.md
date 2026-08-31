---
id: INC-CYCLE-13-APPLY-TEST-COUNT-MISREPORT
title: "Apply envelope reported 636 workspace tests when actual is 1076"
status: open
severity: medium
priority: P2
fingerprint: "67e91b05600e2991"
fingerprint_aliases: ["67e91b05600e2991dd84dff035c9b21b104cee5482f258bb5c03dafec0ec682b"]
cluster_id: CL-REPORTING-DEFECT
created: 2026-08-22
created_by: sddk-verify
owner: orchestrator
---

# INC-CYCLE-13-APPLY-TEST-COUNT-MISREPORT — apply envelope test count misrepresentation

> Durable record for one debt finding across cycles. See ADR-0047 §3.2.

## Context

Cycle-13 (`kernel-cycle-13-m1-hexagonal-ports`, M1 hexagonal exit, A-min path)
recorded `apply-progress.yaml` with the following incorrect aggregates:

```yaml
gates_verified:
  - cargo_test_workspace: PASS (636 tests, 0 failures)

test_aggregate:
  total_workspace_tests: 636
```

The ground-truth at verify-time is **1076 tests** across 61 test binaries
(verified via `cargo test --workspace` and counting `test result: ok. N passed`
lines per binary). The tree is correct (1076 PASS / 0 FAIL); the apply
envelope misrepresented the count.

The 636 figure appears to be a partial aggregation — possibly a `cargo test
-p sddk-engine --tests` sum (≈70 tests), or a count of test fns in the
sddk-engine test files (≈75 including port_contracts.rs). Neither matches
636; the source of the 636 number is opaque but it is structurally smaller
than the full workspace.

## Rationale

- **Severity = medium**: the tree is correct (1076 PASS / 0 FAIL confirmed
  at verify-time), so this is a **process defect, not a correctness defect**.
  However, the misrepresentation **misinformed the orchestrator** at
  apply-time: the orchestrator received a "636 tests green" signal that
  under-reported coverage by ~440 tests (44% understatement). This creates
  a false-positive confidence in the apply phase and reduces the orchestrator's
  ability to detect partial regressions. Workaround: verify phase re-ran
  `cargo test --workspace` and produced the ground-truth count (1076).

- **Priority = P2**: not blocking release; this cycle's verification ran the
  ground-truth command and produced correct evidence. The remediation is a
  tooling/process improvement to the sddk-apply aggregator (count from
  `cargo test --workspace --format json` parsed output, or parse
  `test result: ok. N passed` lines from the workspace-wide output rather
  than per-binary aggregation).

- **Cluster = `CL-REPORTING-DEFECT`** (apply-phase reporting accuracy).

## Lifecycle

| Date | Actor | Change | Evidence |
|------|-------|--------|----------|
| 2026-08-22 | sddk-verify | created | `apply-progress.yaml` lines 30, 37-47 (incorrect `total_workspace_tests: 636`); `verify-report.md` §Issues → WARNING 2 |
| 2026-08-28 | sddk-debt-verify (cycle `p-52b95ef55999f9de/kernel-cli-agent-information-flow`) | cross-cycle correlation — same class recurrence; current cycle's `FIND-980410` correlates via `fingerprint_aliases` (15 vs 16 test-count claim in `apply-progress.md` L22, L26). Cross-cycle correlation active; underlying execution is correct (16/16 PASS); defect is reporting accuracy | `debt-report.json` sha256:`d45e510441b01e49ca827cf84cd4567ffe4b5e2c1ef47bbde2b8ea64dbb60bc2` `findings[FIND-980410].fingerprint_aliases: [67e91b05600e2991, 67e91b05600e2991dd84dff035c9b21b104cee5482f258bb5c03dafec0ec682b]` |

## References

- `~/.local/share/sddk/projects/p-52b95ef55999f9de/changes/kernel-cycle-13-m1-hexagonal-ports/apply-progress.yaml` — the apply envelope with the misreport
- `~/.local/share/sddk/projects/p-52b95ef55999f9de/cycle-artifacts/p-52b95ef55999f9de/kernel-cycle-13-m1-hexagonal-ports/verify-report.md` — verify phase output (ground-truth 1076)
- `~/.local/share/sddk/projects/p-52b95ef55999f9de/cycle-artifacts/p-52b95ef55999f9de/kernel-cycle-13-m1-hexagonal-ports/implementation-receipt.md` — implementation receipt documenting the misreport

> Filled by `sddk-archive` (cycle-13); consumed by `sddk-debt-verify` for cross-cycle correlation via fingerprint.