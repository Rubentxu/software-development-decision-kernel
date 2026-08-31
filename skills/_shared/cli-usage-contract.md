# SDDK Agent CLI Usage Contract

This contract is the single authority for how orchestrators, phase coordinators,
workers, and lenses invoke the SDDK CLI. It governs instructions only; it does
not add commands or change runtime behavior.

## Ownership

Use **one owner per lifecycle call**:

| Caller | Owns | Must not do |
|---|---|---|
| Orchestrator | Bootstrap, adoption/knowledge resolution, dispatch legality, state reconstruction | Evaluate phase gates on behalf of coordinators |
| Phase coordinator | Fresh state check when required, lease renewal, gate evaluation, transition, post-transition ledger verification | Repeat immutable bootstrap queries |
| Worker or lens | Task-specific tools and evidence | Invoke cycle, lock, gate, transition, or ledger lifecycle commands |

Never repeat a lifecycle call in a wrapper, skill, and phase prompt. The owner
runs it once and passes the validated result in the handoff.

## Machine Output

- Use `--format json` whenever the command supports it.
- Validate the exit code and expected JSON fields before consuming values.
- Record exact argv and an `output_digest`; do not pass free-form stdout as state.
- Text output is for humans, never an automation contract.
- Keep `--root` and `--scope` explicit on lifecycle commands.

## CLI Context

The orchestrator constructs and passes this compact snapshot:

```yaml
cli_context:
  cli_version: semver
  observed_at: RFC3339
  project: {root: absolute-path, project_id: string, adopted: bool}
  knowledge: {vault_path: absolute-path, profile_present: bool, engram_enabled: bool}
  cycle: {cycle_id: string|null, status: string|null, phase: string|null, path: string|null, updated_at: RFC3339|null}
  lease: {owner: string, fencing_token: integer, expires_at_ms: integer}|null
  cycle_artifacts_dir: absolute-path|null
  source_commands: [{argv: [], exit_code: integer, output_digest: sha256}]
```

Resolve adoption, project identity, knowledge profile, and vault once. Resolve
`cycle_artifacts_dir` once after a valid cycle ID exists; before cycle start it
is `null`. Never infer a field that the CLI did not return.

## Bootstrap

Run `skills/sddk-cycle-resume/SKILL.md` inline. It is the sole owner of exact
bootstrap argv and returns the validated `cli_context`; callers must not repeat
its commands.

`knowledge status` already returns `project_id`, `vault_path`, profile presence,
vault presence, and Engram status. Do not also call `knowledge path` unless the
caller needs only the path and did not call `knowledge status`.

When a trusted `cycle_id` is available, resume resolves cycle status and its
artifact directory. Phase coordinators refresh only mutable cycle state under
the freshness policy below.

The baseline has no global active-cycle discovery command. `cycle lock status`
requires `--cycle`, and `cycle status` already includes the lease when the cycle
ID is known. The current runtime does not serialize distinct cycle IDs
project-wide. On a cold start without a trusted cycle ID, leave cycle fields and
`cycle_artifacts_dir` null and block automated cycle start with
`runtime-active-cycle-discovery-unavailable`. Recover a trusted ID or obtain an
explicit human override that acknowledges an active conflict cannot be
excluded; never claim that `cycle start` proves project-wide serialization.
Never treat an invalid invocation as "no active cycle".

## Freshness And Lease

Immutable fields may be reused across phases. Refresh cycle status:

- after restart or compaction;
- before dispatch when another process may have mutated state;
- immediately before gate/transition after a long phase;
- when the lease may expire before the next mutation.

Compare `expires_at_ms` with the expected time to the next mutation. Renew only
when needed:

```bash
sddk cycle lock renew --root "$PROJECT_ROOT" --scope . \
  --cycle "$CYCLE_ID" --owner "$LEASE_OWNER" \
  --fencing-token "$FENCING_TOKEN" --format json
```

The renewal response replaces the prior lease snapshot and keeps the fencing
token. Never acquire or renew a lease merely to satisfy an outdated template.

## Gate Evidence

The coordinator resolves the outcome before invoking `evaluate-gate`. Evidence
must include the material observations that justify it:

```json
{
  "subject": {"base": "<sha>", "head": "<sha>", "diff_digest": "<sha256>"},
  "artifact": {"path": "<absolute-path>", "sha256": "<sha256>"},
  "checks": [{"id": "<check>", "result": "passed|failed", "command": "<argv|null>", "exit_code": 0, "output_digest": "<sha256|null>"}]
}
```

Boolean-only evidence such as `{"checked": true}` is forbidden. A signed gate
receipt proves receipt integrity; it does not prove that the evidence was
semantically sufficient.

Use `--format json` for `evaluate-gate`, parse `receipt_id`, then pass that exact
ID to the path-specific transition. Use `--format json` for transition and
assert its outcome, status, and phase.

Invoke lifecycle commands through a direct process API with an argv array,
never through shell interpolation. Serialize evidence canonically and pass it
as the single argv element immediately after `--evidence`. Inline command
examples are argv notation only; placeholders are not shell fragments.

## Ledger Boundaries

Run one `sddk ledger verify --format json` after a gate/transition sequence. It
verifies ledger integrity, not CAS, filesystem content, report quality, or an
Engram write. Do not use it as evidence for a non-ledger operation.

Archive is the deliberate exception: verify before evaluating `ledger-valid`,
then verify again after `archive.complete` to cover the closing append.

## Error Policy

Authoritative errors remain visible. Classify them as:

- `not_found`
- `invalid_invocation`
- `corrupt_state`
- `permission_denied`
- `tool_unavailable`

Do not use `2>/dev/null || echo ...`, empty JSON, or guessed defaults to turn an
error into absence. Only a documented `not_found` result may represent absence.
Any other CLI failure blocks the lifecycle action and is returned with exact
argv, exit code, stderr digest, and recovery action.

## Call Budgets

Every coordinator returns expected and actual call counts grouped by owner and
command class. A count above budget requires a freshness, retry, or archive
exception with evidence; workers and lenses always have a lifecycle budget of
zero.

| Phase shape | Status | Renewal | Inventory | Gate evaluations | Transition | Ledger verify |
|---|---:|---:|---:|---:|---:|---:|
| Standard transitioning phase | 0-1 | 0-1 if `expires_at_ms` requires it | 1 | Declared gate count | 1 on resolved outcome | 1 after transition |
| Verify | 1 | 0-1 if required | 1 | 2 | 1 | 1 |
| Release | 1 | 0-1 if required | 1 | 2 | 1 | 1 |
| Archive | 1 | 0-1 if required | 1 | 2 | 1 | 2 (pre-gate and closing append) |
| CAS-only propose/debt report | 0 | 0 | 1 | 0 | 0 | 0 |
| Filesystem-only coherence | 0 | 0 | 0 | 0 | 0 | 0 |

## Files Inventory Lifecycle

The cycle-scoped files inventory is mandatory for every transitioning phase and
for every report that closes the cycle (verify, debt-verify, release, archive).
It is computed by the cycle coordinator exactly once per phase, immediately
after `sddk cycle status`, and persisted under
`{cycle_artifacts_dir}/inventory.json` along with its SHA-256 envelope.

Authority:

| Command | Owner | Purpose | Inputs | Outputs |
|---|---|---|---|---|
| `sddk cycle inventory` | Cycle coordinator (verify, release, archive) | Compute the cycle-scoped files inventory | `--root`, `--scope`, `--cycle {cycle_id}` | `inventory.json` (UTF-8 JSON matching `inventory.schema.json`) and `inventory.json.sha256` next to it; stdout envelope `{contract_version, schema, path, bytes, sha256}` |

Bucket precedence is closed and stable:

| Source prefix | Bucket |
|---|---|
| `prompts/`, `agents/`, `skills/`, `assets/`, `tools/`, `docs/`, `tests/` | `prompts/`, `agents/`, `skills/`, `assets/`, `tools/`, `docs/`, `tests/` |
| anything else | `untagged_project/<first-segment>` |

The project's own `.gitignore` is the authority for ignored paths. The framework
never writes additional ignore files inside the adopted project; only git,
`git check-ignore -v`, and the working tree state are consulted.

Availability rules:

| Condition | Behavior | Phase gate |
|---|---|---|
| `.git/` exists in `--root` | Compute diff `stage-and-working-tree vs HEAD` with `--find-renames=50%` | Pass |
| `.git/` exists but no commits | Persist `inventory.json` with `summary.unavailable_reason=git-context-missing` | Pass with degrade note |
| `.git/` missing | Persist `inventory.json` with `summary.unavailable_reason=git-not-initialized` | Block verify/archive/release, surface `inventory-unavailable` |
| Diff collection fails | Persist `inventory.json` with `summary.unavailable_reason=io-error`, return same envelope, retry once | Pass with degrade note on retry success |
| Provided `--git-rev` invalid | Persist `inventory.json` with `summary.unavailable_reason=invalid_rev` | Block verify/archive/release until corrected |

Workers and lenses never call `sddk cycle inventory`; the coordinator passes the
resolved `inventory.json` to them via the cycle packet when required.


Bootstrap adoption/knowledge calls belong to the orchestrator and are not
charged again to a phase. Record duplicate calls, invalid invocations, hidden
errors, stale-lease rejections, and any lifecycle call by a worker/lens as
contract violations.

---

## Instruction Contract Matrix

The matrix is the single canonical authority for what each SDDK CLI command does,
who may invoke it, what inputs it requires, what it produces, what side effects
it causes, whether it is idempotent, and what the next handoff is. All phase
prompts reference matrix rows by anchor; the recipe never appears in prompts twice.

The matrix host is `skills/_shared/cli-usage-contract.md#matrix`.

### Matrix Schema (8 mandatory columns, exact order)

Every row has these columns in this exact order. Optional metadata MAY appear
after column 8, never before.

| # | Column | Type | Description |
|---|--------|------|-------------|
| 1 | `intent` | string | Unique identifier for the operation |
| 2 | `owner_role` | enum | Who may invoke this: `orchestrator`, `phase coordinator`, `worker`, `lens` |
| 3 | `command` | string | CLI argv invocation (example: sddk cycle status) |
| 4 | `required_inputs` | list[str] | Flags and positionals; flags use `<PLACEHOLDER>` notation |
| 5 | `expected_output` | string | JSON keys or prose envelope description |
| 6 | `side_effects` | list[str] | Typed mutations: `ledger_append`, `cas_write`, `subject_advance`, `none` |
| 7 | `idempotence` | enum | `true` \| `false` \| `conditional-on-<arg>` |
| 8 | `next_handoff` | string or list[str] | Next consumer and which row/anchor they cite |

Optional metadata keys (never required, never shift column order):
`freshness_binding`, `evidence_binding`, `failure_classification`,
`shadow_target_row`, `dry_run_invariant`, `separation_invariant`.

### Matrix Rows

```yaml
# ── lifecycle.cycle.status ──────────────────────────────────────────────
- intent: lifecycle.cycle.status
  owner_role: phase coordinator
  command: sddk cycle status
  required_inputs: ["--root", "--scope", "--cycle <CYCLE>", "--format json"]
  expected_output: "{cycle_id, status, phase, path, updated_at, artifacts, lease}"
  side_effects: []
  idempotence: true
  next_handoff: ["lifecycle.cycle.transition.<id>", "lifecycle.cycle.lock.renew"]
  freshness_binding: "lease.expires_at_ms; refresh <60s before next mutation if <5min"
  evidence_binding: "subject.sha256 + inventory.sha256"
  failure_classification: [stale_lease, corrupt_state]

# ── lifecycle.cycle.transition.<id> (generic) ────────────────────────────
# Used as a template; specific intents below (design, explore, spec, tasks)
- intent: lifecycle.cycle.transition.design
  owner_role: phase coordinator
  command: sddk cycle transition
  required_inputs:
    - --root
    - --scope
    - --cycle <CYCLE>
    - --transition <TRANSITION>
    - --artifact <KEY>=<PATH>
    - --gate-receipt <ID>
    - --lease-owner <OWNER>
    - --fencing-token <N>
  expected_output: "{transition_id, phase, status, artifact_path, receipt_id}"
  side_effects: [ledger_append, snapshot_update]
  idempotence: false
  next_handoff: ["lifecycle.ledger.verify", "lifecycle.cycle.status consumer"]
  freshness_binding: "subject_sha; lease.expires_at_ms"
  evidence_binding:
    subject: "cycle_artifacts/cas/<CYCLE>/subject.json"
    artifact: "<cycle_artifacts_dir>/<PHASE>.md sha256"
    checks: ["evaluate-gate receipt", "hashing integrity"]
  failure_classification: [stale_lease, missing_artifact, gate_not_passed]

- intent: lifecycle.cycle.transition.explore
  owner_role: phase coordinator
  command: sddk cycle transition
  required_inputs:
    - --root
    - --scope
    - --cycle <CYCLE>
    - --transition phase.explore.complete
    - --artifact exploration-report=<PATH>
    - --gate-receipt <ID>
    - --lease-owner <OWNER>
    - --fencing-token <N>
  expected_output: "{transition_id, phase, status, artifact_path, receipt_id}"
  side_effects: [ledger_append, snapshot_update]
  idempotence: false
  next_handoff: ["lifecycle.ledger.verify", "lifecycle.cycle.status consumer"]
  freshness_binding: "subject_sha; lease.expires_at_ms"
  failure_classification: [stale_lease, missing_artifact, gate_not_passed]

- intent: lifecycle.cycle.transition.spec
  owner_role: phase coordinator
  command: sddk cycle transition
  required_inputs:
    - --root
    - --scope
    - --cycle <CYCLE>
    - --transition phase.specify.complete
    - --artifact specification=<PATH>
    - --gate-receipt <ID>
    - --lease-owner <OWNER>
    - --fencing-token <N>
  expected_output: "{transition_id, phase, status, artifact_path, receipt_id}"
  side_effects: [ledger_append, snapshot_update]
  idempotence: false
  next_handoff: ["lifecycle.ledger.verify", "lifecycle.cycle.status consumer"]
  freshness_binding: "subject_sha; lease.expires_at_ms"
  failure_classification: [stale_lease, missing_artifact, gate_not_passed]

- intent: lifecycle.cycle.transition.tasks
  owner_role: phase coordinator
  command: sddk cycle transition
  required_inputs:
    - --root
    - --scope
    - --cycle <CYCLE>
    - --transition phase.plan.complete
    - --artifact implementation-plan=<PATH>
    - --gate-receipt <ID>
    - --lease-owner <OWNER>
    - --fencing-token <N>
  expected_output: "{transition_id, phase, status, artifact_path, receipt_id}"
  side_effects: [ledger_append, snapshot_update]
  idempotence: false
  next_handoff: ["lifecycle.ledger.verify", "lifecycle.cycle.status consumer"]
  freshness_binding: "subject_sha; lease.expires_at_ms"
  failure_classification: [stale_lease, missing_artifact, gate_not_passed]

- intent: lifecycle.cycle.transition.apply
  owner_role: phase coordinator
  command: sddk cycle transition
  required_inputs:
    - --root
    - --scope
    - --cycle <CYCLE>
    - --transition phase.build.complete
    - --artifact implementation-receipt=<PATH>
    - --gate-receipt <ID>
    - --lease-owner <OWNER>
    - --fencing-token <N>
  expected_output: "{transition_id, phase, status, artifact_path, receipt_id}"
  side_effects: [ledger_append, snapshot_update]
  idempotence: false
  next_handoff: ["lifecycle.ledger.verify", "lifecycle.cycle.status consumer"]
  freshness_binding: "subject_sha; lease.expires_at_ms"
  failure_classification: [stale_lease, missing_artifact, gate_not_passed]

- intent: lifecycle.cycle.transition.verify
  owner_role: phase coordinator
  command: sddk cycle transition
  required_inputs:
    - --root
    - --scope
    - --cycle <CYCLE>
    - --transition phase.verify.complete.a-min
    - --artifact verification-report=<PATH>
    - --gate-receipt <ID>
    - --lease-owner <OWNER>
    - --fencing-token <N>
  expected_output: "{transition_id, phase, status, artifact_path, receipt_id}"
  side_effects: [ledger_append, snapshot_update]
  idempotence: false
  next_handoff: ["lifecycle.ledger.verify", "lifecycle.cycle.status consumer"]
  freshness_binding: "subject_sha; lease.expires_at_ms"
  failure_classification: [stale_lease, missing_artifact, gate_not_passed]

- intent: lifecycle.cycle.transition.release
  owner_role: phase coordinator
  command: sddk cycle transition
  required_inputs:
    - --root
    - --scope
    - --cycle <CYCLE>
    - --transition release.complete
    - --artifact merge-receipt=<PATH>
    - --artifact release-receipt=<PATH>
    - --gate-receipt <ID>
    - --lease-owner <OWNER>
    - --fencing-token <N>
  expected_output: "{transition_id, phase, status, artifact_path, receipt_id}"
  side_effects: [ledger_append, snapshot_update]
  idempotence: false
  next_handoff: ["lifecycle.ledger.verify", "lifecycle.cycle.status consumer"]
  freshness_binding: "subject_sha; lease.expires_at_ms"
  failure_classification: [stale_lease, missing_artifact, gate_not_passed]

- intent: lifecycle.cycle.transition.archive
  owner_role: phase coordinator
  command: sddk cycle transition
  required_inputs:
    - --root
    - --scope
    - --cycle <CYCLE>
    - --transition archive.complete
    - --artifact archive-manifest=<PATH>
    - --gate-receipt <ID>
    - --lease-owner <OWNER>
    - --fencing-token <N>
  expected_output: "{transition_id, phase, status, artifact_path, receipt_id}"
  side_effects: [ledger_append, snapshot_update]
  idempotence: false
  next_handoff: ["lifecycle.ledger.verify", "lifecycle.cycle.status consumer"]
  freshness_binding: "subject_sha; lease.expires_at_ms"
  failure_classification: [stale_lease, missing_artifact, gate_not_passed]

# ── lifecycle.help.first-class ──────────────────────────────────────────
- intent: lifecycle.help.first-class
  owner_role: orchestrator
  command: sddk --help
  required_inputs: []
  expected_output: "First-class commands: status, plan, run, ship, recover"
  side_effects: []
  idempotence: true
  next_handoff: ["(terminal)"]

# ── facade.status ───────────────────────────────────────────────────────
- intent: facade.status
  owner_role: orchestrator
  command: sddk status
  required_inputs: ["--root", "--scope"]
  expected_output: "{cli_version, project, knowledge, cycle, lease}"
  side_effects: []
  idempotence: true
  next_handoff: ["lifecycle.cycle.status"]
  shadow_target_row: lifecycle.cycle.status

# ── facade.plan ─────────────────────────────────────────────────────────
- intent: facade.plan
  owner_role: orchestrator
  command: sddk plan
  required_inputs: ["--name <NAME>", "--path <PATH>", "--branch <BRANCH>", "--format <FORMAT>"]
  expected_output: "{plan_id, artifacts, recommendations}"
  side_effects: []
  idempotence: true
  next_handoff: ["lifecycle.plan.start.legacy-direct"]
  shadow_target_row: lifecycle.plan.start.legacy-direct

# ── facade.run ──────────────────────────────────────────────────────────
- intent: facade.run
  owner_role: orchestrator
  command: sddk run
  required_inputs: ["--goal <GOAL>", "--path <PATH>"]
  expected_output: "{run_id, status, artifacts}"
  side_effects: [subject_advance]
  idempotence: false
  next_handoff: ["lifecycle.run.complete"]
  shadow_target_row: lifecycle.run

# ── facade.ship ─────────────────────────────────────────────────────────
- intent: facade.ship
  owner_role: orchestrator
  command: sddk ship
  required_inputs: ["--tag <TAG>", "--cycle <CYCLE>?optional"]
  expected_output: "{release_plan, version_lockstep_passed, dry_run}"
  side_effects: []
  idempotence: true
  next_handoff: ["lifecycle.release.complete"]
  shadow_target_row: lifecycle.release.plan
  dry_run_invariant: "delegates to release plan --dry-run; no facade --dry-run flag"
  failure_classification: [lockstep_refused]

# ── facade.recover ──────────────────────────────────────────────────────
- intent: facade.recover
  owner_role: orchestrator
  command: sddk recover
  required_inputs: ["--cycle <CYCLE>"]
  expected_output: "{digest, event_count, invariant_preserved}"
  side_effects: []
  idempotence: true
  next_handoff: ["lifecycle.cycle.status"]
  shadow_target_row: lifecycle.cycle.rebuild
  dry_run_invariant: "digest and event_count preserved in both paths"

# ── lifecycle.release.plan ───────────────────────────────────────────────
- intent: lifecycle.release.plan
  owner_role: phase coordinator
  command: sddk release plan
  required_inputs: ["--root", "--scope", "--cycle <CYCLE>", "--tag <TAG>"]
  expected_output: "{release_plan, version_lockstep, dry_run}"
  side_effects: []
  idempotence: true
  next_handoff: ["lifecycle.release.apply"]
  freshness_binding: "subject_sha; tag_version"
  failure_classification: [lockstep_refused, stale_subject]

# ── lifecycle.release.apply ─────────────────────────────────────────────
- intent: lifecycle.release.apply
  owner_role: phase coordinator
  command: sddk release apply
  required_inputs: ["--root", "--scope", "--cycle <CYCLE>", "--tag <TAG>"]
  expected_output: "{release_receipt, sha256, manifest_sha256}"
  side_effects: [ledger_append, cas_write]
  idempotence: false
  next_handoff: ["lifecycle.release.complete"]
  freshness_binding: "subject_sha; tag_version; release_plan"
  failure_classification: [lockstep_refused, gate_not_passed]

# ── lifecycle.archive.complete ───────────────────────────────────────────
- intent: lifecycle.archive.complete
  owner_role: phase coordinator
  command: sddk archive complete
  required_inputs:
    - --root
    - --scope
    - --cycle <CYCLE>
    - --release-receipt <ID>
    - --durable-knowledge-nodes <LIST>
  expected_output: "{manifest_id, sha256, nodes[]}"
  side_effects: [cas_write, vault_write]
  idempotence: false
  next_handoff: ["cycle closes"]
  freshness_binding: "release_receipt_id"

# ── lifecycle.plan.start.legacy-direct ─────────────────────────────────
- intent: lifecycle.plan.start.legacy-direct
  owner_role: orchestrator
  command: sddk cycle start
  required_inputs: ["--root", "--scope", "--name <NAME>"]
  expected_output: "{cycle_id, status, phase, path}"
  side_effects: [ledger_append, cas_write]
  idempotence: false
  next_handoff: ["lifecycle.cycle.status"]
  shadow_target_row: facade.plan

# ── matrix.sizing.advisory ─────────────────────────────────────────────
- intent: matrix.sizing.advisory
  owner_role: phase coordinator
  command: "(no CLI; advisory projection lives in the apply/tasks phase artifact)"
  required_inputs: []
  expected_output: "{metric, forecast, budget, recommendation, rationale}"
  side_effects: ["prose/field append in tasks or apply artifact only"]
  idempotence: true
  next_handoff: ["verify reads the projection from the artifact, not from a runtime event"]
  separation_invariant: >
    this row's expected_output and side_effects MUST be a prose/field surface only;
    no runtime event variant is claimed; no runtime receipt type is claimed;
    size alone performs no evaluate-gate / transition.

# ── matrix.safety-brake ────────────────────────────────────────────────
- intent: matrix.safety-brake
  owner_role: phase coordinator
  command: "(implicit brake; consumes the row in Decision Model row 10 and YAML gates)"
  required_inputs: []
  expected_output: "typed block-class verdict (string)"
  side_effects: ["ledger_append: typed refusal entry on detection"]
  idempotence: conditional-on-failure-class
  next_handoff: ["remediation path, NOT the apply phase"]
  failure_classification:
    - test_failure
    - spec_failure
    - invariant_violation
    - wrong_subject
    - wrong_hash
    - invalid_evidence
    - corrupt_evidence
    - no_progress_streak
    - retry_exhausted
    - critical_introduced_debt
    - permission_blocker
    - release_archive_completion_guard
  separation_invariant: >
    may not import matrix.sizing.advisory surface; braked class names are
    disjoint from advisory projection keys.
```
