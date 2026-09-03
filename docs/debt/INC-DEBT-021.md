---
id: INC-DEBT-021
title: "Pre-existing clippy baseline tolerated by TEST-SELECT-001 acceptance"
slug: "INC-DEBT-021-preexisting-clippy-baseline-gate-classification-and-test-select-deps"
status: open
severity: low
priority: P2
fingerprint: "f3a7b2e9c5d1f8a4b6e3c9d2a5f7b1e8c4d6a9b3e5f1c8d2a4b7e6f9c3d1a5b8"
fingerprint_aliases: []
cluster_id: CL-CB-21
created: 2026-09-03
created_by: sddk-debt-verify
owner: unassigned
cycle_origin: "p-63676b11dc0ef88f/test-select-001-impact-propagation"
---

# INC-DEBT-021 — Pre-existing clippy baseline tolerated by TEST-SELECT-001 acceptance

> Durable record for one debt finding across cycles. See ADR-0047 §3.2.
> Created by `sddk-debt-verify` during TEST-SELECT-001 A-min debt gate.
> Verdict of the originating cycle: **PASS** (1 introduced LOW finding + 0 introduced HIGH/CRITICAL/MEDIUM; pre-existing baseline items do not block the gate per cycle-7b Decision Contract).

## Context

The cycle `p-63676b11dc0ef88f/test-select-001-impact-propagation` (TEST-SELECT-001) ran the `sddk-debt-verify` post-verify gate at A-min / SMOKE depth (coupling + overengineering). The delta introduces exactly one new file (`crates/sddk-domain/src/test_select.rs`, 1619 lines) plus a one-line export in `lib.rs` and a one-line variant restoration (`RiskPolicyEscalation`) in `test_model.rs`.

The repository baseline at base commit `166d1a3809464b5c4d694474e597283bd22edccb` already contained:

| Pre-existing item | Location | Introduced by |
|---|---|---|
| `clippy::for_kv_map` warning | `crates/sddk-domain/tests/gate_classification.rs:167` | commit `ad28b0f` (`feat(uat): leg A gate classification — ADR-0080`, 2026-08-31) |
| 10 other pre-existing clippy warnings | various | various pre-base commits |
| `for_kv_map` style pattern at line 167: `for (_gate_name, classification) in &classifications { if let Some(ref waiver) = … }` — clippy warns the `for_kv_map` shape is suboptimal for inspection-only iteration | as above | as above |

The spec acceptance criterion for TEST-SELECT-001 explicitly tolerated this baseline error (`sólo el error pre-existente gate_classification.rs:167 permitido, fuera del delta`), and the verify report recorded it as `classification: false_positive, exemption: spec REQ-7 + acceptance criteria, attribution: pre_existing`. The debt gate baseline-attributes it here so the durable record survives across cycles and the same fingerprint is not re-introduced in subsequent debt reports.

## Rationale

| Atributo | Valor | Justificación |
|----------|-------|---------------|
| severity | low | clippy style lint on a test file (not production); no behavioral impact; the iteration at line 167 only performs pattern-matching assertions, no observable side-effects |
| priority | P2 | tolerated by spec acceptance for the TEST-SELECT-001 cycle; cycle-7b rule says pre-existing items do not block the gate; resolution can ship alongside adjacent clippy fixes in the next tooling cycle |
| cluster_id | CL-CB-21 | clippy-baseline cluster (newly assigned; tracks all pre-existing clippy warnings on `sddk-domain` test targets) |
| attribution | pre_existing | introduced by `ad28b0f` (2026-08-31), **before** base `166d1a3` |
| owner | unassigned | next tooling cycle that performs clippy-cleanup pass |

**Impact**: zero on TEST-SELECT-001 PASS verdict. The `cargo clippy -p sddk-domain --all-targets -- -D clippy::all` command exits 101 with **only this pre-existing error** and the delta introduces **zero new clippy errors** — confirmed by the verify report (see `verify-report.md` § Commands). Resolution: trivial refactor of the test iteration to satisfy clippy (e.g. iterate over `classifications.values()` instead of `&classifications` when `key` is unused).

## Workaround

None required. The lint is in a test target, not in production. CI gate is configured (`-D clippy::all`) so any new clippy violation introduced by a future delta would still be caught.

## Fix Direction

Two options:

1. **Option A — local fix in next cycle**: rewrite the iteration at `crates/sddk-domain/tests/gate_classification.rs:167` to use the key-less form (clippy-suggested). ~3-line change in a test file. Closes this INC.

2. **Option B — selective allow-list**: add `#[allow(clippy::for_kv_map)]` on the function with a comment explaining the explicit key intent. Defers resolution; keeps the original style. Not recommended.

**Recommendation**: Option A. It is a 3-line cleanup and resolves the lint at the source.

## Lifecycle

| Date | Actor | Change | Evidence |
|------|-------|--------|----------|
| 2026-09-03 | sddk-debt-verify | created | FIND-baseline-for_kv_map from cycle TEST-SELECT-001 `debt-report.json` (attribution: pre_existing) |
| 2026-09-03 | sddk-debt-verify | status: open | not yet fixed; carried forward from baseline `ad28b0f` |

## References

- `crates/sddk-domain/tests/gate_classification.rs:167` — `clippy::for_kv_map` warning
- `crates/sddk-domain/tests/gate_classification.rs:1-30` — full imports / context
- commit `ad28b0f` — `feat(uat): leg A gate classification — ADR-0080` (2026-08-31) — origin of `for_kv_map` pattern
- `cycle-artifacts/.../verify-report.md` § Commands — `cargo clippy -p sddk-domain --all-targets -- -D clippy::all` exit 101 with 1 unique error on `gate_classification.rs:167` (pre-existing); delta introduces 0 new clippy errors
- `cycle-artifacts/.../verify-findings.json` finding `sha256:clippy-clean-for-delta` — `classification: false_positive, exemption: spec REQ-7 + acceptance criteria`
- `cycle-artifacts/.../specification.md` § Acceptance — `sólo el error pre-existente gate_classification.rs:167 permitido, fuera del delta`
- `docs/debt/INC-DEBT-019.md` — precedent: pre-existing time-coupling attribution, fingerprint cross-cycle correlation pattern

## Follow-up (schema gap)

A separate observation from the TEST-SELECT-001 debt gate concerns the `cycle_id` pattern in `docs/debt/debt-report.schema.json`:

> `"cycle_id": { "type": "string", "pattern": "^p-[a-f0-9]{16}/kernel-cycle-[0-9]+[a-z]?-[a-z0-9-]+$" }`

This pattern excludes product cycles such as `p-63676b11dc0ef88f/test-select-001-impact-propagation`. Suggested fix in the next archive cycle:

```diff
- "pattern": "^p-[a-f0-9]{16}/kernel-cycle-[0-9]+[a-z]?-[a-z0-9-]+$"
+ "pattern": "^p-[a-f0-9]{16}/[a-z0-9][a-z0-9-]*$"
```

This widens admission to both kernel cycles (`kernel-cycle-N-…`) and product cycles (`test-select-001-…`, `uat-…`, etc.) while preserving the project-id prefix. Tracked as `cycle-7b-schema-cycle-id-pattern` in the originating debt-report `follow_up` array.