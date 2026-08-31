# SDDK Verify Phase

## Role And Boundary

Prove that the exact cycle implementation satisfies its specifications with real, production-ready code. Verify is read-only and scoped to the changed code plus the runtime paths needed by the cycle.

Do not substitute task completion for evidence. Do not run `sddk-debt-verify`: that later phase audits broader technical debt.

## Required Inputs

- `path`: `B-direct | A-min | A-lite | A-full`
- `verify_role`: `coordinator | lens`; `lens_id` is required only for a lens invocation
- exact `base_commit` and `head_commit`, or a reproducible dirty-diff digest
- testing capabilities and project-local quality commands
- Strict TDD flag and runner, when active
- risk declarations from project standards and available cycle artifacts
- architecture intent/model manifest when proposal or design declared
  `architecture_impact: boundary|deployable`
- `cli_context` and expected CLI call budget from the shared CLI contract

Acceptance authority depends on path:

| Path | Required authority |
|---|---|
| B-direct | User request, selected skill contract, bug reproduction or jurisprudence claim, project invariants, and execution diff |
| A-min | Spec, tasks, apply evidence, and project invariants |
| A-lite | Proposal, spec, tasks, apply evidence, and project invariants |
| A-full | Proposal, spec, design, tasks, apply evidence, and project invariants |

Missing or contradictory authoritative input blocks verification; it never becomes a warning.

## Mandatory Gates

These gates run on every path. Adaptive lenses only add depth.

| Gate | Passing evidence | Failure |
|---|---|---|
| Subject identity | Base/head SHA, clean state or diff digest, CWD, timestamp | `blocked` + verdict `FAIL` |
| Behavioral compliance | Every required scenario has a passing test that reaches production logic | `FAIL` |
| Real implementation | No stub, placeholder, hardcoded value (data, path, secret, magic constant satisfying known examples), unreachable body, or production-wired mock / fake / in-memory adapter in the changed business path | `FAIL` |
| Documentation discipline | No comments in changed production paths reference issue numbers, PR IDs, task identifiers, user handles, cycle / phase pointers, or commit-history metadata; only language-standard docs (`///` in Rust, JSDoc in TS, docstrings in Python, doc comments in Go) explain the *what* and *why* | `FAIL` |
| Test strength | Assertions observe required outcomes; changed boundaries have real contract/integration evidence | `FAIL` |
| Regression and build | Fresh relevant tests and repository-required build/type/lint/regression checks pass | `FAIL`; infrastructure absence is `blocked` |
| Pre-commit discipline | Apply MUST run gates against commit's tree, not dirty working tree | `git status --porcelain` empty + explicit HEAD citation in verify report | blocking |
| Apply-Push discipline | apply-report shows no publication command; lease fields are never publication authority | `FAIL` (cycle rejected); read-only remote ops (`git rev-parse`, `git ls-remote`, `git fetch --dry-run`) do not trigger | blocking |
| Production readiness | Every readiness dimension is `PASS` or evidence-backed `N/A` | `FAIL` when applicable behavior is missing; unknown critical applicability is `blocked` |
| Design and SOLID | No concrete changed-scope violation breaks the approved design, substitutability, client contracts, dependency direction, or local changeability | `FAIL` if material; otherwise warning |
| Task completeness | Every required task, including planned hardening/refactor work, is complete | `FAIL`; only a pre-declared optional item with no required-path impact may warn |

## Validation Pipeline L0-L6

| Layer | Purpose | Output |
|---|---|---|
| L0 Subject | Pin base/head/diff, artifact hashes, CWD, and clean state | Immutable subject or `BLOCKED` |
| L1 Deterministic | Run required tests/build/type/lint/format/security checks once | Command evidence; non-zero mandatory checks cannot be downgraded |
| L2 Candidate | Generate placeholder, hardcode, fake wiring, docs, test-oracle, and design candidates | Candidate set, never verdicts |
| L3 Reachability | Trace candidates through production entry points, composition roots, and required scenarios | `yes|no|unknown` reachability with evidence |
| L4 Adjudication | Confirm defect, exemption, false positive, or insufficient evidence | Findings conforming to `contracts/verify-finding.schema.json` |
| L5 Adversarial | Run path lenses and blind judges against the same immutable subject | Independent envelopes; no lifecycle calls |
| L6 Synthesis | Reconcile without overriding deterministic failures; persist report, gates, transition, and ledger proof | Final verdict and CLI trace/budget summary |

Required-tool absence is `BLOCKED`/`insufficient_evidence`, never a fabricated
PASS. Load `skills/sddk-verify/references/multi-stack-validation.md` only for
affected stacks.

When an architecture manifest is required, consume it read-only through
`skills/sddk-c4-likec4/SKILL.md`. Verify subject/graph revision, evidence
coverage, and planned-versus-actual delta. Do not infer architecture from a
render. `planned_but_missing`, `invalid`, or missing mandatory evidence blocks;
renderer failure alone preserves the semantic verdict and uses the fallback.

## Procedure

### 1. Pin The Subject

Record base/head SHA and `git status`. The subject MUST be a clean commit tree — `git status --porcelain` MUST be empty. Evidence from another subject, cached summaries, or unidentifiable runs is invalid. If dirty, block until the working tree is cleaned and gates re-run against the verified clean HEAD.

### 2. Build The Behavioral Matrix

Map every requirement and scenario to implementation symbols, test file/name, command, and observed result. For B-direct, derive requirements only from its authority row above; do not invent a spec. Use:

- `COMPLIANT`: covering test passed and exercised production logic.
- `FAILING`: covering test ran and failed.
- `UNTESTED`: no covering executable test exists.
- `BLOCKED`: required evidence could not run or artifacts contradict.

Any required row other than `COMPLIANT` prevents PASS and PASS_WITH_WARNINGS.

### 3. Prove The Implementation Is Real

Inspect the changed production files, callers, adapters, and composition root.

1. Search the changed production diff for markers and empty primitives such as `TODO`, `FIXME`, `XXX`, `HACK`, `todo!`, `unimplemented!`, `NotImplemented`, empty/pass-only bodies, placeholder panics, and constant success responses.
2. Inspect every hit in context. Fail reachable placeholders or behavior required by the cycle; do not fail unrelated historical text outside the changed execution path.
3. Trace each scenario from entry point to the changed implementation. Fail dead, unwired, bypassed, or tests-only code.
4. Confirm mocks, stubs, fakes, in-memory adapters, and fixtures are confined to tests or an explicitly approved non-production profile. Changed external boundaries need a contract or integration test that executes the real adapter; if the real dependency cannot run locally, require its official emulator/sandbox plus a contract test and record the limitation.
5. Challenge suspicious hard-coded values or branches that satisfy only known examples. Require another scenario, negative control, RED evidence, or targeted mutation evidence.

### 3.b Prove The Documentation Discipline

The Code Quality Standards in `apply.md` §"Code Quality Standards" require that comments exist only to explain functionality. Comments whose only purpose is to point at issue trackers, task IDs, user handles, or commit history substitute meta-traceability for documentation and are a violation.

1. When the changed subject is an applicable Rust/Cargo workspace, run
   `sddk dev check --since <cycle-base-sha>` as an advisory candidate generator,
   not as the documentation gate's sole proof. Record its argv, exit code, and
   output digest.
2. The current command is Cargo-coupled, restricts its diff input to `crates/`,
   can include a shifted hunk line, has incomplete pattern coverage, does not
   retain multiline-comment state reliably, may skip unreadable files, and its
   JSON summary lacks structured findings. It therefore cannot prove a clean
   multi-stack diff. Command/tool absence or a zero-finding result is
   `insufficient_evidence`, not PASS.
3. Treat scanner hits as candidates and inspect them in context. The command is
   deliberately advisory in this baseline, so its non-zero exit is not a failed
   mandatory quality command by itself; confirmed traceability-only comments
   fail, while documented false positives are recorded with evidence.
4. Inspect every changed file with the repository's stack-aware tools and a
   source review bound to base/head/diff digest. Comments that exist purely for
   issue/task/user/commit traceability and do not document behavior are `FAIL`.
   Comments whose primary content explains behavior may attach a requirement ID.
5. For language-specific public APIs, sample contracts in the language's idiom
   (`///` in Rust, JSDoc in TS, docstrings in Python, doc comments in Go).
   Missing documentation without a deferral reason is `WARNING`, not `FAIL`.
6. Track confirmed pre-existing violations as incidences; never infer that they
   are absent from the scanner's limited or failed output.

### 4. Execute Fresh Evidence

Run deterministic checks before semantic judgment:

1. Scenario-focused tests.
2. Tests for the changed package/module.
3. Repository-required regression suite and build/type/lint/format checks.
4. Risk-specific checks declared by spec, design, project standards, or testing capabilities.

For each command record CWD, exact command, timestamp, exit code, subject SHA/diff digest, and log path or concise output. An LLM lens cannot reinterpret a non-zero exit as success.

### 5. Judge Test Strength

- Reject tautologies, type/existence-only assertions used alone, ghost loops, snapshots with no relevant oracle, and tests that only assert mock calls when behavior is required.
- Require a test to fail when its covered behavior is broken. Accept persisted Strict TDD RED evidence, a mutation command with tool/output recorded, or an equivalent negative control tied to the same subject.
- Treat coverage as reachability evidence, not behavioral proof.
- If doubles isolate a changed boundary, require a contract or integration test that executes the real adapter or approved emulator/sandbox. A mock-only proof is insufficient.

### 6. Judge Production Quality And SOLID

Evaluate changed code against the approved design and existing project conventions. Report concrete evidence, not generic scores:

| Principle | Verify |
|---|---|
| SRP | The change does not mix unrelated reasons to change or policy with infrastructure. |
| OCP | The required extension does not force avoidable edits across stable modules. |
| LSP | Implementations preserve the declared input, output, error, and state contract. |
| ISP | Changed clients are not forced to depend on methods or data they do not use. |
| DIP | Policy depends toward the project's intended abstraction/boundary, not a new infrastructure detail. |

SOLID is not a demand for classes, interfaces, or layers. Fail only a concrete material violation in the changed scope. Run mandatory `entropy-sdd` Protocol D when configured, but use its estimates as supporting evidence rather than the sole verdict.

Evaluate every readiness dimension: errors/recovery, state/data integrity, resource cleanup, concurrency, migrations/compatibility, security, performance, and observability/deployability. Mark `N/A` only with changed-scope evidence. Security applies to changed external input, auth, authorization, secrets, or trust boundaries; migrations apply to persisted/schema changes; concurrency applies to async/shared state; performance applies to declared hot paths/SLOs; observability applies to services or operational failure modes. Unknown applicability in security, data integrity, or migration blocks verification.

An item is optional only when an authoritative artifact marked it optional before apply and it cannot affect a required scenario or mandatory production gate. Elevate it to required when source/runtime evidence shows a regression or dependency from a required path.

### 7. Run Path Lenses

Core gates above remain mandatory.

| Path | Lenses |
|---|---|
| B-direct | `direct-acceptance` inline |
| A-min | `spec-compliance`, `test-quality` |
| A-lite | `spec-compliance`, `test-quality`, `production-readiness` |
| A-full | `spec-compliance`, `architecture-connascence`, `test-quality`, `design-coherence`, `jd-judge-a`, `jd-judge-b` |

Production readiness remains a mandatory core gate even when no dedicated lens is configured. Lens focus:

| Lens | Focus |
|---|---|
| `direct-acceptance` | B-direct authority versus final behavior and diff |
| `spec-compliance` | Requirements/scenarios versus implementation and tests |
| `test-quality` | Oracle strength, negative controls, doubles, and regressions |
| `production-readiness` | Readiness matrix and concrete SOLID effects without a design artifact |
| `architecture-connascence` | A-full design boundaries, dependencies, connascence, and entropy evidence |
| `design-coherence` | A-full design decisions versus production implementation |
| `jd-judge-a`, `jd-judge-b` | Blind adversarial deficiency search |

The coordinator runs mandatory deterministic gates once. For A-* paths it launches all configured lenses in one parallel batch: use `sddk-verify` with `verify_role: lens` and one `lens_id` for non-judge lenses, and the exact `jd-judge-a` / `jd-judge-b` agents for judges. A lens never dispatches, persists, updates the ledger, or reruns supplied commands. The coordinator waits, deduplicates, synthesizes, persists, and alone decides the verdict.

Every lens receives the same subject identity, artifact paths, changed files, commands already run, Strict TDD mode, and one focus. Synthesis deduplicates findings but cannot downgrade deterministic failures or missing mandatory evidence.

A verify lens returns only findings conforming to
`prompts/sddk/contracts/verify-finding.schema.json`:

```yaml
lens_id: string
status: pass | findings | blocked
findings:
  - finding_id: sha256(rule_id + canonical subject + canonical location)
    rule_id: string
    subject: {base: sha|null, head: sha|null, diff_digest: sha256|null}
    location: {path: string, start_line: int, end_line: int, symbol: string|null}
    classification: blocking_defect | warning | suggestion | false_positive | insufficient_evidence
    severity: critical | high | medium | low
    confidence: high | medium | low
    production_reachable: yes | no | unknown
    evidence: [{kind: command|source|test|trace|artifact, observation: string, command: string|null, exit_code: int|null, output_digest: sha256|null}]
    exemption: {authority: string, reason: string, expires_at: string|null} | null
    owner_phase: apply | verify | debt-verify | replan | human
evidence_gaps: []
```

### 7.5. Apply-Push Discipline Gate

Scan `{cycle-artifacts-dir}/apply-report.md` (or equivalent apply evidence) for any forbidden-commands invocations:

1. **Detect forbidden commands** (cycle-17 expansion): Search for the following patterns:
   - `git push origin`, `git push --force`, `git push --tags`, `git push <branch>`
   - `git tag <name>` (any tag creation)
   - `gh release create` (any release creation)
   - `cargo publish` (any crate publish)
   - `gh pr create` (any PR creation)
2. **Verdict**:
   - **PASS**: No forbidden command found.
   - **FAIL**: Any forbidden command found. Lease fields authorize cycle
     mutations, NEVER Git publication.
3. **Read-only ops do not trigger**: `git rev-parse origin/main`, `git ls-remote`, `git fetch --dry-run`, `git tag --list` are permitted and do not cause failure.

4. **Deterministic drift check** (cycle-17 hardening): If `apply-progress.md` is present in the
   cycle artifacts directory, read the `pre_apply_origin_main_sha` header field, then
   recompute `post=$(git rev-parse origin/main)`:
   ```bash
   PRE_SHA=$(awk -F': *' '/^pre_apply_origin_main_sha:/ {print $2}' "$CYCLE_ARTIFACTS_DIR/apply-progress.md")
   POST_SHA=$(git rev-parse origin/main)
   if [ -z "$PRE_SHA" ]; then
       verdict=FAIL; reason="pre_apply_origin_main_sha not recorded in apply-progress.md"
   elif [ "$PRE_SHA" != "$POST_SHA" ]; then
       verdict=FAIL; reason="origin/main drifted: pre=$PRE_SHA post=$POST_SHA"
   else
       verdict=PASS
   fi
   ```
   - **FAIL**: `origin/main` changed between apply start and verify start (apply-push
     recurrence or external push). The typed finding is `apply-push-discipline-drift`.
   - **PASS**: `pre == post` — no drift detected.

### 8. Decide

| Verdict | Exact condition |
|---|---|
| `PASS` | All mandatory gates pass with fresh evidence; no blocking finding remains. |
| `PASS_WITH_WARNINGS` | All mandatory gates pass; only optional, explicitly non-blocking improvements remain. |
| `FAIL` | Any mandatory gate fails, is untested, or cannot be proven. |
| `INCONCLUSIVE` | Verification evidence is complete enough to expose a runtime-contract blocker, but the current workflow cannot accept the required transition receipts. |

Use envelope `status: blocked` with verdict `INCONCLUSIVE` when runtime authority
cannot complete the lifecycle operation. Use verdict `FAIL` for proven product
or evidence failures and `status: partial` for a recoverable code/spec failure.
Warnings never compensate for failure.

## Report Contract

Persist the schema-conforming finding array as
`{cycle-artifacts-dir}/verify-findings.json`, then derive
`{cycle-artifacts-dir}/verify-report.md` from the same findings with:

```markdown
# Verification Report: {change-name}

## Subject
| Base | Head | Dirty diff digest | CWD | Verified at |

## Files Inventory
Source: `{cycle-artifacts-dir}/inventory.json` (`sddk.inventory/v1`).

| Bucket | Added | Modified | Deleted | Renamed |
|---|---:|---:|---:|---:|
| prompts/ | {n} | {n} | {n} | {n} |
| agents/ | {n} | {n} | {n} | {n} |
| skills/ | {n} | {n} | {n} | {n} |
| assets/ | {n} | {n} | {n} | {n} |
| tools/ | {n} | {n} | {n} | {n} |
| docs/ | {n} | {n} | {n} | {n} |
| tests/ | {n} | {n} | {n} | {n} |
| untagged_project/<segment> | {n} | {n} | {n} | {n} |

Top 25 paths sorted by status / bucket:
| Status | Bucket | Path | Renamed from | SHA-256 |

Full inventory: `{cycle-artifacts-dir}/inventory.json`. The artifact embeds
the project's `.gitignore` matches inside the `ignored_by_project` array; no
sidecar file is produced.

If `summary.unavailable_reason` is non-null, render one row instead of the
buckets:

| Reason | Behavior |
|---|---|
| git-not-initialized | `inventory-unavailable: git-not-initialized` — block verify; obtain a trust shell with `git init` and re-run. |
| git-context-missing | `inventory-unavailable: git-context-missing` — degrade with a note; verify does not block, but the next cycle phase (`sddk-apply`) must commit before transitioning. |
| io-error | `inventory-unavailable: io-error` — retry once; if still failing, treat as `git-context-missing`. |
| invalid_rev | `inventory-unavailable: invalid_rev` — block verify until the ledger's HEAD/CAS snapshot is corrected. |

## Summary
| Verdict | Mode | Path | Required scenarios | Commands passed | Critical | Warnings |

## Behavioral Compliance
| Requirement / Scenario | Production Path | Test | Status | Evidence |

## Production Readiness
| Gate | Status: PASS/FAIL/BLOCKED/N/A | Evidence | Findings / N/A reason |

## Code Quality

| Standard | Status: PASS/FAIL/WARNING | Evidence | Findings |
|----------|--------------------------|----------|----------|
| Business code reality (no stub / mock / hardcoded satisfier in `src/` / `lib/` / `bin/`) | {status} | {grep + diff references} | {list of hits} |
| Documentation discipline (no issue / task / user / cycle refs in comments) | {status} | {grep + diff references} | {list of hits} |

## SOLID And Design
| Principle / Decision | Status | Concrete evidence | Impact |

## Architecture Delta
| Stable ID / Relation | Planned | Actual | Status | Evidence |

## Commands
| Command | Exit | Subject | Evidence |

## Issues
### CRITICAL
### WARNING
### SUGGESTION

## Lens Summary
| Lens | Findings | Evidence gaps |

## Verdict
**PASS | PASS_WITH_WARNINGS | FAIL | INCONCLUSIVE**
{reason tied to mandatory gates}
```

Return:

```yaml
status: success | partial | blocked
executive_summary: 1-3 sentences
artifacts: ["{cycle-artifacts-dir}/verify-findings.json", "{cycle-artifacts-dir}/verify-report.md"]
verdict: PASS | PASS_WITH_WARNINGS | FAIL | INCONCLUSIVE
subject: {base: sha, head: sha, diff_digest: sha256|null}
findings: []  # exact objects from verify-findings.json
mandatory_gates:
  subject_identity: PASS|FAIL|BLOCKED|N/A
  behavioral_compliance: PASS|FAIL|BLOCKED|N/A
  real_implementation: PASS|FAIL|BLOCKED|N/A
  documentation_discipline: PASS|FAIL|BLOCKED|N/A
  test_strength: PASS|FAIL|BLOCKED|N/A
  regression_and_build: PASS|FAIL|BLOCKED|N/A
  production_readiness: PASS|FAIL|BLOCKED|N/A
  design_and_solid: PASS|FAIL|BLOCKED|N/A
  task_completeness: PASS|FAIL|BLOCKED|N/A
issues_by_severity: {critical: N, warning: N, suggestion: N}
unverified: []
next_recommended: sddk-debt-verify | sddk-apply correction cycle | resolve runtime receipt ordering | resolve blocker
risks: []
context_quality: C0|C1|C2|C3
lenses_used: []
skill_resolution: paths-injected | fallback-registry | fallback-path | none
architecture_validation:
  required: bool
  manifest_ref: string | null
  semantic_status: valid | insufficient_evidence | invalid | not_applicable
  render_status: rendered | unavailable | failed | not_applicable
cli_trace_summary:
  expected: {lens_lifecycle: 0, gate_evaluations: 2, transitions: 0|1, ledger_verifies: 1}
  actual: {status_queries: int, renewals: int, lens_lifecycle: int, gate_evaluations: int, transitions: int, ledger_verifies: int}
  exceptions: []
```

On B-direct, follow its workflow transition. On A-* paths in the current
baseline, return `blocked`/`INCONCLUSIVE` after persisting and verifying the
real gate receipts: the transition also requires debt receipts that can only be
produced by the later debt capability. Do not fabricate them or claim
`REMEDIATING/verify`. Product `FAIL` still returns to correction, but the
runtime state remains unchanged until receipt ordering converges.

## Ledger Contract (Coordinator Only)

Transition reference:
```
Transition:   phase.verify.complete.a-min
Matrix row:   lifecycle.cycle.transition.verify
Artifact:     {cycle_artifacts_dir}/verify-report.md
On failure:   blocked — runtime remains OPEN/verify; do not retry from cache
```

Full procedure (from `cli-usage-contract.md#matrix`):

Inspect `sddk cycle status --root . --scope . --cycle {cycle_id} --format
json`. Require matching cycle/path, `status=OPEN`, and `phase=verify`, then
execute `sddk cycle inventory --root . --scope . --cycle {cycle_id} --format
json`. The command persists `{cycle-artifacts-dir}/inventory.json` together
with its `.sha256` sidecar; the artifact embeds the project's `.gitignore`
matches in the `ignored_by_project` array.
The verify coordinator must report on the value persisted here; the runtime
blocks verify when `summary.unavailable_reason` is `git-not-initialized` or
`invalid_rev`, and `inventory-unavailable` is rendered into the `## Files
Inventory` block. Use the runtime capability matrix:

| Path | Runtime action |
|---|---|
| A-full | `phase.verify.complete` is unavailable: it also requires `debt-severity-assigned` and `debt-priority-assigned` before debt-verify runs |
| A-min | `phase.verify.complete.a-min` has the same receipt-ordering blocker |
| A-lite | `phase.verify.complete.a-lite` has the same receipt-ordering blocker |
| B-direct | `phase.verify.complete.b-direct` is executable with verify receipts |

1. Require `git rev-parse HEAD == head_commit`; recompute report/log hashes.
2. Build separate material evidence documents for `tests-pass` and
   `policy-compliant`, containing base/head/diff digest, result, exact commands,
   exit codes, output digests, findings/report paths and SHA-256. Derive each
   `{outcome}` from those observations and evaluate:
    `sddk cycle evaluate-gate --root . --scope . --cycle {cycle_id} --transition {transition} --gate {tests-pass|policy-compliant} --outcome {outcome} --evaluator sddk.cli --evidence {evidence_json_arg} --timestamp {now} --actor sddk --format json`
3. For A-* paths, stop after gate evaluation and ledger verification with
   blocker `runtime-receipt-ordering-unavailable`. Do not invoke transition.
4. For B-direct only, transition with `verification-report` and both receipts:
   `sddk cycle transition --root . --scope . --cycle {cycle_id} --transition phase.verify.complete.b-direct --artifact verification-report={path} --gate-receipt {tests_receipt_id} --gate-receipt {policy_receipt_id} {lease_flags_if_present} --format json`
   Append
   lease owner/token only when current cycle status contains a lease; otherwise
   omit both flags.
5. A passing B-direct verdict requires transition `outcome=succeeded`. A failure or
   blocked verification requires `outcome=failed`, `status=REMEDIATING`, and
   `phase=verify`.
6. Run `sddk ledger verify --root . --scope . --format json` before returning.

Gate evaluation is required for all paths. Transition is required only when the
runtime requirements are satisfiable (B-direct in this baseline). A CLI error
blocks the phase. Renew an expiring live lease before gate evaluation.

## References

- `skills/sddk-verify/SKILL.md`
- `prompts/sddk/phases/strict-tdd-verify.md`
- `skills/_shared/sddk-phase-common.md`
- `docs/research/sddk-verify-agent-practices.md`
