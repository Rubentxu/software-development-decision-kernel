# Real-Model Trials Runbook (cycle-19 / ADR-0060 follow-up)

This document is the operational runbook for running real evaluator and judge
adapters against the held-out corpus in `golden-dataset/cases/`. It is the
gap between the contract surface (already shipped in cycle-19) and the
statistical efficacy goal declared by ADR-0060 Consequences §Negative
("Statistical efficacy remains unproven until real evaluator/judge trials run").

This runbook is intentionally a runbook, not an execution log: the trials
need a model provider that is **not** available in this repository's CI
sandbox. The harness itself is validated contractually by
`./golden-dataset/runner/run-golden.sh --validate-only` (14 cases green) and
by `python3 tests/test_golden_dataset_contract.py`.

## What the harness gives you

- **Held-out corpus.** 14 cases in `golden-dataset/cases/` covering limpio/debt
  (01, 06, 11, 16, 21), verify multi-stack (22-25), contract mutants (26-29)
  and communication (30). The labels (`expected-verdict.yaml`) are read only
  by `grade_results.py` after both roles have finished; neither role can read
  the host checkout because `bwrap` hides it.
- **Role isolation.** Two distinct identities: `evaluator` (the system under
  test) and `judge` (adversarial critic). They receive the same input
  bundle (spec + implementation) but the judge never sees labels.
- **Snapshot durability.** Each trial is run against an immutable, per-trial
  snapshot of the case + bundle. Repeated trials over the same model must
  produce the same labels for the same input.
- **Provenance.** Every run records `identity`, `model`, `provider`,
  `invocation_id`, the argv, and the digests of bundle, evaluation, trace,
  labels, stdout, stderr, exit code, and per-trial grade.
- **Deterministic grading.** `grade_results.py` produces Precision, Recall,
  F1, false-blockers, critical-escapes, evaluator/judge disagreement, and
  `pass^k`. Targets declared in `golden-dataset/README.md`: `precision > 0.8`,
  `recall > 0.7`.

## What the runbook does not provide

- **Adapter implementations.** The harness expects `--evaluator-cmd` and
  `--judge-cmd` as placeholder shells. Adapters are repository-external
  by design — they encapsulate model credentials, network egress, and
  provider-specific output schemas. The cycle-19 PR deliberately does not
  vendor adapters because (a) credentials must not live in the framework
  repo, and (b) provider choice is a deployment decision.
- **Trial runs.** The cycle-19 verification gate executed `--validate-only`
  only. Real `--trials N` runs require a model account. CI never runs them.

## Steps for the next cycle that owns statistical efficacy

1. **Choose providers.** Select one evaluator provider and one judge provider
   with at least the following properties: deterministic output for the same
   input (or near-deterministic within `top_p <= 0.1, temperature = 0`),
   per-call cost under USD 0.05 for the corpus, and a CLI/HTTP API that can
   be wrapped in a single shell command. Candidates: an OpenAI-compatible
   endpoint, Anthropic via `claude` CLI, or a local llama.cpp endpoint.
2. **Write two adapters.** Each adapter must:
   - Accept the four placeholder paths the harness substitutes:
     `{input}` (spec+implementation bundle), `{output}` (verdict+findings
     JSON), `{trace}` (tool calls + reasoning), `{provenance}` (identity +
     model + provider + invocation_id).
   - Write the **persisted `debt-report.json` shape** defined by
     `docs/debt/debt-report.schema.json` (cycle-7b), not the working Common
     Finding shape. The mapping table in
     `prompts/sddk/phases/debt-verify.md` § Schema Mapping is the
     authoritative translation; adapters for the **judge** role emit only
     the persisted shape.
   - For the **judge** role specifically, ignore the
     `expected-verdict.yaml` file: the harness's `bwrap` mount prevents it
     from being readable, but the adapter must not assume labels exist.
   - Set `provenance.identity` to one of `verify-under-test` /
     `adversarial-judge`; `model` to the canonical model identifier;
     `provider` to the vendor slug.
3. **Smoke-test one case.** Pick case `01-clean-pass`. Run:
   ```bash
   ./golden-dataset/runner/run-golden.sh \
     --evaluator-cmd 'your-eval-adapter {input} {output} {trace} {provenance}' \
     --judge-cmd    'your-judge-adapter {input} {output} {trace} {provenance}' \
     --evaluator-id verify-under-test \
     --judge-id     adversarial-judge \
     --evaluator-model MODEL_A --judge-model MODEL_B \
     --pass-env PROVIDER_API_KEY \
     --read-only-path /opt/your-adapter \
     --network-policy external-model \
     --trials 1 --results-dir /tmp/golden-smoke
   python3 golden-dataset/runner/grade_results.py /tmp/golden-smoke
   ```
   Confirm `verdict=PASS` (the case is intentionally clean) and that the
   JSON envelope contains `provenance.identity`, `provenance.model`, and a
   64-hex `bundle_sha256`. If any field is missing, the adapter fails the
   contract — fix the adapter, not the harness.
4. **Run the full corpus.** Same command with `--trials 5` (the README's
   default for first-pass) and no positional `cases` argument.
5. **Inspect results.** Open `golden-dataset/results/<timestamp>/report.md`.
   Look at:
   - Per-rule TP/FP/FN, especially on cases 11 (circular-import-fail),
     16 (subtle-feature-envy-pw) and 21 (adversarial-hidden-mutation).
   - `evaluator_judge_disagreement` rows. Any disagreement is a contract
     signal: the prompts need to disambiguate.
   - `critical_escapes`. Any non-zero entry blocks this run from being
     accepted as the cycle-19 baseline.
6. **File follow-up incidences.** For each finding emitted by the evaluator
   that the judge flagged as a contract ambiguity, file an
   `INC-NNN-{slug}.md` per `docs/debt/INCIDENCE-TEMPLATE.md` with severity
   and priority from the canonical taxonomies. The schema mapping in
   `prompts/sddk/phases/debt-verify.md` decides the persisted shape.
7. **Set the cycle-20 baseline.** Copy the per-rule results into
   `golden-dataset/results/cycle-19-baseline/` (git-ignored by design —
   `golden-dataset/results/` is in `.gitignore`). The next cycle compares
   against this snapshot before declaring pruning safe.

## Why this is not in the cycle-19 PR

- The cycle-19 PR is **prompt-layer contract work**. Adapters and
  provider credentials are out of scope per `docs/responsibility-separation/SPEC.md`.
- Real trials need a network egress policy and a budget decision that
  belongs to the maintainer, not the framework repo.
- The harness contract is already enforced by `--validate-only` and by the
  test suites. Adding empty adapter stubs would either (a) include fake
  credentials or (b) introduce a no-op adapter that produces no signal
  beyond what `--validate-only` already proves.

## Evidence the cycle-19 contracts are correct

- `python3 golden-dataset/runner/run_golden.py --validate-only` → 14 cases
  validated.
- `python3 tests/test_golden_dataset_contract.py` → 17 tests green.
- `bash tests/test_workflow_contract.sh` → 458 tests green.
- `bash tests/test_inventory_contract.sh` → 5 scenarios green.
- `cargo test --workspace --locked` → 73 suites green.

## References

- `golden-dataset/README.md` — harness usage
- `golden-dataset/runner/run_golden.py --help` — full flag reference
- `prompts/sddk/phases/debt-verify.md` § Schema Mapping — persisted shape
- `docs/debt/debt-report.schema.json` — canonical JSON schema
- `docs/debt/SEVERITY.md` / `docs/debt/PRIORITY.md` — taxonomies
- ADR-0060 § Consequences — declared gap
- `prompts/sddk/phases/archive.md` § Follow-up Incidences — incidence flow