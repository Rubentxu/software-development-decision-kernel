# SDDK Verify: Strict TDD Module

Load this module only when Strict TDD is active and a runner is available. It adds evidence gates; it does not replace the standard verify phase.

## Required Evidence

`apply-progress` must contain one row per behavior-bearing task:

| Task | Test | Safety Net | RED | GREEN | TRIANGULATE | REFACTOR |
|---|---|---|---|---|---|---|

Each state needs a command/result or persisted artifact tied to the task. Labels such as "done", "written first", or a final green diff are claims, not evidence.

## Gates

| Gate | PASS | FAIL / WARNING |
|---|---|---|
| Safety net | Existing behavior was run before modifying existing code | Missing is WARNING; known regression without disposition is FAIL |
| RED | The new test failed for the intended missing/defective behavior | Missing or unrelated failure is FAIL |
| GREEN | The same test passed after production implementation | Missing or different subject is FAIL |
| Triangulation | Multiple spec scenarios have distinct behavioral tests | Missing required scenario is FAIL |
| Refactor | Green suite was rerun after cleanup; production code is not merely minimal fake code | Regression or placeholder is FAIL |
| Test strength | Assertions observe outputs/state/errors and reach production logic | Vacuous or mock-only proof is FAIL |

Git history may prove test-before-code when the RED commit precedes implementation. A single final diff cannot prove chronology; require persisted RED command/output, commit, or equivalent timestamped evidence.

## Assertion Audit

Fail tests whose only oracle is one of these patterns:

- tautology: `expect(true).toBe(true)`, `assert 1 == 1`
- existence/type only: `toBeDefined`, `not.toBeNull`, `is not None`
- loop over a statically empty collection
- mock-call assertion without an observable behavior assertion
- empty/snapshot result with no contrasting setup or behavioral meaning

An otherwise weak assertion is acceptable only when companion assertions or triangulated cases prove the required outcome.

## Doubles And Pure Logic

Count mocked collaborators as a diagnostic, not an automatic score. Many doubles require a fidelity review: does the test still execute production policy, and is the changed boundary covered by a real contract/integration test? Fail when doubles replace the behavior being claimed.

When apply claims a pure-function extraction, verify no hidden I/O/global state and test it directly without mocking the function itself.

## Report Addition

Add to `verify-report.md`:

```markdown
## Strict TDD Evidence
| Task | Safety Net | RED | GREEN | Triangulation | Refactor | Verdict | Evidence |

### Assertion And Double Findings
- Vacuous assertions: {N}
- Mock-only behavioral claims: {N}
- Missing real-boundary tests: {N}
```

Strict TDD never authorizes a weaker production-readiness gate. If required TDD evidence is missing, verdict is `FAIL`; if the runner cannot execute for an infrastructure reason, return `status: blocked` with verdict `FAIL`.

## CLI Ledger Contract

This module extends `verify.md`; it has no standalone transition. The parent
phase's ledger contract covers strict-TDD evidence gates.

Transition reference (inherited from parent verify phase):
```
Transition:   phase.verify.complete.a-min
Matrix row:   lifecycle.cycle.transition.verify
Artifact:     {cycle_artifacts_dir}/verify-report.md
On failure:   blocked — runtime remains OPEN/verify; strict-TDD evidence gates unsatisfied
```

Full procedure (from `cli-usage-contract.md#matrix`):
1. `sddk cycle status --root . --scope . --cycle {cycle_id} --format json` → confirm phase is `verify`.
2. Build `{evidence_json}` with strict-TDD evidence table path/SHA-256, RED/GREEN/TRIANGULATE/REFACTOR
   results per task, and one result per assertion-quality gate. Set `{outcome}` to `passed`
   only when every behavior-bearing task has RED→GREEN evidence and no vacuous assertions.
3. `sddk cycle evaluate-gate --root . --scope . --cycle {cycle_id}
   --transition phase.verify.complete.a-min --gate tdd-evidence-complete
   --outcome {outcome} --evaluator sddk.cli --evidence {evidence_json}
   --timestamp {now} --actor sddk --format json`
4. `sddk ledger verify --root . --scope . --format json`

On failure: blocked — strict-TDD gates are mandatory. A failed CLI invocation
or ledger verification is a blocker.

- `prompts/sddk/phases/apply-strict-tdd.md`
- `prompts/sddk/phases/verify.md`
