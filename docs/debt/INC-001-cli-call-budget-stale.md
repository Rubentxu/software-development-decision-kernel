---
id: INC-001-cli-call-budget-stale
title: "Golden dataset test asserted stale 6-column CLI call-budgets row"
status: resolved
severity: medium
priority: P2
fingerprint: "7c8a1f4d9e2b0a3c"
fingerprint_aliases: []
cluster_id: CL-37
created: 2026-08-24
created_by: sddk-archive (cycle-19, ADR-0060 follow-up)
owner: orchestrator
closed: 2026-08-28
closed_by: sddk-debt-verify (cycle p-52b95ef55999f9de/kernel-cli-agent-information-flow)
---

# INC-001-cli-call-budget-stale — golden dataset test lags the CLI call-budgets table

> Durable record for one debt finding across cycles. See ADR-0047 §3.2.

## Context

The `Call Budgets` table in `skills/_shared/cli-usage-contract.md` gained a
new `Inventory` column when the cycle-19 evolution introduced the `sddk cycle
inventory` primitive (ADR-0060 / `prompts/sddk/phases/verify.md` § Files
Inventory). The column records that every transitioning phase and every
report that closes the cycle (verify, release, archive) calls `sddk cycle
inventory` exactly once.

The golden dataset test
[`tests/test_golden_dataset_contract.py::GoldenDatasetContract::test_cli_mutants_and_budgets_are_covered`](../../../tests/test_golden_dataset_contract.py)
was not updated in lockstep with the table. It still asserts the pre-cycle-19
shape:

```python
self.assertIn("| Verify | 1 | 0-1 if required | 2 | 1 | 1 |", contract)
```

which corresponds to the 6-column layout `Status | Renewal | Gate evaluations
| Transition | Ledger verify` (5 numeric cells). After the cycle-19 change the
row reads:

```
| Verify | 1 | 0-1 if required | 1 | 2 | 1 | 1 |
```

i.e. the new 7-column layout adds `Inventory | Gate evaluations | Transition
| Ledger verify` with the Inventory cell holding `1` for Verify, Release and
Archive, `1` for the CAS-only row, and `0` for the Filesystem-only row.

The drift was caught by `python3 tests/test_golden_dataset_contract.py`
during the cycle-19 verification gate (round 1, commit `410cfe0`) and fixed
in the same cycle by replacing the stale literal with the new row in the
test (commit `7f16edc`).

## Rationale

- **Severity = medium** per `docs/debt/SEVERITY.md`. The test failure
  blocks the gate (one of 17 tests in the suite fails), but the underlying
  contract was correct from day one of the column addition. There is no
  data loss, no security boundary breached, and no correctness regression.
- **Priority = P2** per `docs/debt/PRIORITY.md`. The drift is the third
  instance of a class where contract changes and test fixtures drift
  because no enforcement links them. Re-bakeable in one cycle by adding a
  schema check that parses the markdown table and asserts that every
  declared row matches a snapshot in `tests/fixtures/cli-budget-snapshot.json`.
- **Cluster = `CL-37`** (test-fixture / contract drift). Precedent: none
  recorded yet under this cluster; this is the first durable observation.

## Lifecycle

| Date | Actor | Change | Evidence |
|------|-------|--------|----------|
| 2026-08-24 | sddk-debt-verify (cycle-19 round 1) | identified | `tests/test_golden_dataset_contract.py` exit code 1, `FAIL: test_cli_mutants_and_budgets_are_covered` |
| 2026-08-24 | sddk-apply (cycle-19) | remediated | commit `7f16edc` updates the assertion to the 7-column row; `python3 tests/test_golden_dataset_contract.py` → 17/17 OK |
| 2026-08-24 | sddk-archive (cycle-19) | filed INC | this file (docs/debt/INC-001-cli-call-budget-stale.md) per `prompts/sddk/phases/archive.md` § Follow-up Incidences |
| 2026-08-28 | sddk-debt-verify (cycle `p-52b95ef55999f9de/kernel-cli-agent-information-flow`) | closed — remediation commit `7f16edc` verified applied; `tests/test_golden_dataset_contract.py` line no longer matches the pre-cycle-19 6-column literal; status set to `resolved` | `debt-report.json` sha256:`d45e510441b01e49ca827cf84cd4567ffe4b5e2c1ef47bbde2b8ea64dbb60bc2`; cycle `p-52b95ef55999f9de/kernel-cli-agent-information-flow` is the active cycle that finalized closure during its debt-verify gate |

## References

- `tests/test_golden_dataset_contract.py::test_cli_mutants_and_budgets_are_covered` — failing assertion (round 1)
- `skills/_shared/cli-usage-contract.md` — `## Call Budgets` table (post-fix)
- commit `410cfe0` — first commit of the cycle-19 contract evolution
- commit `7f16edc` — fix commit (this INC's remediation)
- ADR-0060 — Evidence Contracts for the SDDK Prompt Layer
- `prompts/sddk/phases/debt-verify.md` — schema mapping cycle-7b
- `prompts/sddk/phases/archive.md` — incidence persistence contract cycle-7b

> Filled by `sddk-archive` (cycle-19); consumed by `sddk-debt-verify`
> for cross-cycle correlation via fingerprint `7c8a1f4d9e2b0a3c`.