# Mandatory Complete Workflow (MCW)

Source of truth for end-to-end SDDK execution. Every cycle MUST follow this numbered sequence. Skipping any step = broken pipeline.

The MCW runs in **5 phases**, each with numbered steps. Hard gates only where stated.

For the triage gate that selects which path (B-direct / A-min / A-lite / A-full) executes this MCW, see `prompts/sddk/orchestrator.md` § Triage.

---

## Phase 0 — Cycle Pre-flight

### Step 0.1 — Trunk Sync (MANDATORY)

```
git fetch origin main
git checkout main && git pull origin main
```

Hard gate: `git rev-parse HEAD == origin/main`. If `main` cannot be pulled → BLOCK.

### Step 0.2 — Previous Cycle Consolidation Check

Verify previous cycle is fully closed. **Do NOT use remote branch existence as the primary signal** — stale branches linger. Use the validated `cli_context` built under `skills/_shared/cli-usage-contract.md`, then use Git as secondary verification.

**Primary check:** run `skills/sddk-cycle-resume/SKILL.md` inline and consume its
validated `cli_context`. It is the sole owner of bootstrap/cycle reconstruction
argv. A vault `_active.md` marker or Git tag is informational and never
overrides CLI state.

**Secondary check — Git verification only (not primary):**
```bash
# Detect default branch and primary remote dynamically
PRIMARY_REMOTE=$(git remote | head -1)
[ -n "$PRIMARY_REMOTE" ] || PRIMARY_REMOTE="origin"
DEFAULT_BRANCH=$(git symbolic-ref "refs/remotes/$PRIMARY_REMOTE/HEAD" 2>/dev/null | sed "s|refs/remotes/$PRIMARY_REMOTE/||")
[ -n "$DEFAULT_BRANCH" ] || DEFAULT_BRANCH="main"

# Normalize branch output before filtering; git indents remote branch names.
UNMERGED=$(git branch -r --no-merged "$PRIMARY_REMOTE/$DEFAULT_BRANCH" \
  | sed 's/^[*[:space:]]*//' \
  | grep -E "^${PRIMARY_REMOTE}/(feat|fix|refactor|chore|perf|test|docs)/" \
  || true)
[ -z "$UNMERGED" ] || BLOCK "Unmerged cycle branches: $UNMERGED"

```

Hard gate: a supplied cycle ID is either resumed or closed **AND** there are no
matching unmerged cycle branches. On a cold start without a trusted cycle ID,
block automated start with `runtime-active-cycle-discovery-unavailable`: the
runtime cannot discover or serialize distinct cycle IDs project-wide. Recover a
trusted ID or request explicit human acceptance of that unresolved risk. A
historical tag never overrides an active lock.
Provider pull-request state is optional external context and is never queried as
an SDDK cycle gate.

If unmerged branches exist: recover the corresponding trusted cycle ID or ask
for explicit recovery; do not start a competing cycle.


### Step 0.3 — Knowledge Coverage Check (A-full only)

Resolve milestone, work items, ADRs, architecture context, ownership, and learnings from the external knowledge vault.

```bash
cat "$VAULT/milestones/_active.md" 2>/dev/null
ls "$VAULT"/milestones/M-*.md "$VAULT"/adrs/ADR-*.md 2>/dev/null
cat "$VAULT/_index.md" 2>/dev/null
```

Hard gates:
- Vault index or milestone state missing → BLOCK and repair adoption/knowledge state.
- Cycle's milestone missing from the vault → add it before proceeding.
- ADR with `superseded by ADR-NNN` where NNN doesn't exist → block.

For A-lite/A-min/B-direct: skip this step.

### Step 0.4 — Triage

Run the triage gate (C0-C3 + jurisprudence + path selection). Output: selected path, lenses, F3 tuning. Inject into next phase launch plan. See `prompts/sddk/orchestrator.md` § Triage and `prompts/sddk/decision-model.md` § Path Selection.

---

## Phase 1 — Plan

### A-full: explore → propose → spec+design(PARALLEL) → coherence(propose→spec) → tasks → coherence(spec+design→tasks) → review-budget → branch-creation

**Step 1.1 — Explore** (A-full only)

Delegate to `sddk-explore`. Output: `explore-report.md` with context quality (C0-C3) and taxonomy.

Hard gate: artifact approved.

**Step 1.2 — Propose**

Delegate to `sddk-propose`. Output: `proposal.md`.

**Step 1.3 — Spec + Design (PARALLEL)** (A-full only)

Delegate spec + design concurrently. Both required before tasks.

**ADR Creation**: if spec/design contains architectural decisions → write ADR before tasks.

Hard gate: both spec and design approved.

**Step 1.4 — Coherence Check (propose → spec)** (A-full only)

Score ≥ 60 to proceed.

**Step 1.5 — Tasks**

Delegate to `sddk-tasks`. Output: `tasks.md` with file lists, commit messages, scope.

**Step 1.6 — Coherence Check (spec+design → tasks)** (A-full only)

Score ≥ 60.

**Step 1.7 — Review Budget Guard (advisory)**

Inspect `tasks.md` for forecast. Sizing is advisory per ADR-0070 and
`cli-usage-contract.md#matrix.sizing.advisory`. Size alone MUST NOT block,
fail, force scope cuts, or force chained PRs.

| Forecast | Action |
|----------|--------|
| Any | **Recommend** next action; **allow** cycle to proceed; **rationale** in apply artifact |

The A-full pipeline ordering is mandatory: `tasks → apply → verify → debt-verify → release → archive`
(per `cli-usage-contract.md#lifecycle.archive.complete`; no step is skippable).

**Step 1.8 — Branch Creation** (after tasks)

```
git checkout -b <type>/<description>
git push -u origin <type>/<description>
```

Hard gate: branch matches `^[a-z]+/[a-z0-9-]+$`, type in `feat|fix|chore|docs|refactor|perf|test|ci|revert`.

For A-lite/A-min/B-direct: branch creation happens before apply (inline step), same rule.

### A-lite

Phases: `propose → spec → design → apply → verify`. Coherence: 1 (apply→verify). Skip explore, tasks, coherence (propose→spec and spec+design→tasks).

### A-min

Phases: `explore → spec → apply → verify`. Coherence: 0 unless spec complexity high.

### B-direct

Load skill → execute → light verify. No SDDK phases.

---

## Phase 2 — Build

### Step 2.1 — Apply

Delegate to `sddk-apply`. Output: atomic conventional commits on branch.

Hard gate: every commit passes the declarative git checklist (type/scope/imperative/72-char/no AI attribution).

The apply phase follows `prompts/sddk/phases/apply.md`, which loads `phases/apply-strict-tdd.md` conditionally when `strict_tdd_mode: true` in the launch plan. The orchestrator sets this from project testing-capabilities (cached during sddk-init).

Within apply, the per-task inner loop (Loop Engineering L3) runs Razonar→Actuar→Observar→Evaluar with:
- `per_task_max_attempts` hard brake (default 5)
- No-progress streak detection (default 3 same signatures → BLOCK)
- Strict TDD discipline when active (RED→GREEN→TRIANGULATE→REFACTOR)
- Safety Net (pre-existing failure detection)
- Merge Protocol (no overwrite of prior apply-progress)

### Step 2.2 — Coherence Check (apply → verify) (A-full, A-lite)

Score ≥ 60.

### Step 2.3 — Verify

Delegate to `sddk-verify`. Output: `verify-report.md` with test pyramid, lens verdicts, verdict (PASS / PW / FAIL).

Hard gate: PASS or PW. If FAIL → return to Step 2.1 (correction cycle).

### Step 2.4 — Debt-Verify (v3.3 — MANDATORY on A-*, n/a on B-direct)

Run `sddk-debt-verify` unconditionally after verify returns PASS or PW on an
A-* path. B-direct disables this gate. Depth is fixed by the selected path:

| Path | Depth | Clusters |
|------|-------|----------|
| A-full | deep | architecture, smells, duplication, coupling, overeng |
| A-lite | standard | smells, duplication, coupling, overeng |
| A-min | smoke | coupling, overeng |
| B-direct | disabled | none |

`prompts/sddk/phases/debt-verify.md` is the sole authority for audit input,
finding normalization, baseline attribution, deterministic aggregation,
decision rules, and output schemas. Persist `debt-report.json` as machine
authority and derive `debt-report.md` from it.

Hard gate: `PASS` or `PASS_WITH_WARNINGS`. `FAIL` returns to the declared
remediation target on the same cycle branch, then reruns verify and debt-verify
(maximum three remediation rounds). `INCONCLUSIVE` retries the failed coverage
or requires human review; it never proceeds to release.

### Step 2.5 — Coherence Check (debt-verify → release) (A-full only)

Score ≥ 60. Runs after debt-verify so the release handoff includes functional
and debt evidence.

---

---

## Phase 3 — Release And Archive

### Step 3.1 — Local Verify And Push Main (MANDATORY)

```
git fetch origin main --tags
git checkout main && git pull --ff-only origin main
test -z "$(git status --porcelain)"
git push origin main
SHA="$(git rev-parse HEAD)"
git fetch origin main
test "$SHA" = "$(git rev-parse origin/main)"
```

Hard gate: the verified local HEAD SHA equals `origin/main`. This direct Git
postcondition, not a PR or CI/CD result, is the merge receipt authority.

### Step 3.2 — Create Or Verify Semver Tag (MANDATORY)

Compute the bump from local cycle commits and verified cycle metadata:

| Change type | Bump |
|-------------|------|
| Breaking public API/contract | `major` |
| New feature (non-breaking) | `minor` |
| Bug fix, chore, docs, refactor | `patch` |

Create or reuse one annotated tag at `SHA`, push it, and prove the remote tag
peels to `SHA`. A tag that points elsewhere blocks; a retry never creates a
replacement version. See `prompts/sddk/phases/release.md` for the executable
idempotent sequence.

Hard gate: exactly one selected annotated semver tag exists remotely and peels
to the verified main SHA.

### Step 3.3 — Local Receipts And Bookkeeping (MANDATORY)

Persist `merge-receipt` from the direct-main SHA postcondition and
`release-receipt` from the annotated remote tag, then persist
`release-report.md` and apply `release.complete`.

`no-pending-effects` means no required local Git action remains. It explicitly
excludes CI/CD, Actions, hosted releases, assets, signing, and distribution.
Those optional consumers may run after the tag and are never awaited.

### Step 3.4 — Archive (MANDATORY after release)

Delegate to `sddk-archive` only after `release-report.md` reports success.
Archive syncs delta specs, finalizes the external knowledge graph, generates the
closing HTML, persists `archive-report.md`, and creates an `archive-manifest`
that references the `release-receipt`. A successful `release.complete` normally
auto-releases the phase lease, so archive rebuilds CLI state and omits lease
flags when no live lease exists.

Hard gate: archive report and manifest persisted, vault and ledger valid, and
`archive.complete` returns runtime status `CLOSED`.

---

## Phase 4 — Trunk Sync + F3 + Reset

### Step 4.1 — Sync Local Main

```
git checkout main && git pull origin main
```

Hard gate: HEAD == origin/main.

### Step 4.2 — F3 Self-Tuning

1. Read `metrics/aggregate` from Engram.
2. Apply self-tuning signals table (see `prompts/sddk/lateral-thinking.md`).
3. Write tuning block to `sddk/next-tuning.md`.
4. Append cycle metrics to `$SDDK_DATA_DIR/projects/{project_id}/metrics/{cycle_id}.jsonl`.
5. Mirror as Engram observation with `topic_key: cycle-metrics/{cycle_id}`.
6. Update `metrics/aggregate` rolling 7d/30d.

This replaces the old `.sddk-last-cycle-complete` marker file. Runtime status
`CLOSED` plus an archive manifest linked to the release receipt proves cycle
closure; a tag alone proves release, not archive.

### Step 4.3 — Save Jurisprudence (conditional)

If cycle had `verify_verdict=PASS` + `first_pass_success=true` + reusable decision (ADR, lens, atajo):

```
mem_save(
  topic_key: jurisprudence/{category},
  title: "{goal_pattern} — {path_that_worked}",
  type: jurisprudence,
  content: {jurisprudence schema per decision-model.md}
)
```

### Step 4.4 — Print Result Contract + Next-Cycle Ready

```
✓ Cycle {goal_pattern} closed
  Path: {path} (C{x}, jurisprudence: {n} hits)
  Verdict: {verdict} {first_pass_badge}
  Lead time: {h}h  |  Cost: ${usd}  |  Tokens: {n}
  Spec coverage: {passing}/{total} scenarios ({pct}%)
  main @ {tag} ({sha})
  Bottleneck: {phase} ({reason})
  Saved as jurisprudence: {topic_key} {if reusable}

  vs rolling {window}:
    - first_pass_success_rate: {value} ({delta})
    - median_lead_time: {value}h ({delta})
    - top_bottleneck_phase: {phase} ({you_too|new})

Ready for next cycle.
```

---

## Abort Patterns

| Scenario | Action |
|----------|--------|
| spec fails | Block design. Fix spec first. |
| design fails | Block tasks. Fix design first. |
| apply fails | Rollback to last checkpoint. Re-apply from pending. |
| verify fails | Fix in apply, re-verify. Do not skip. |
| coherence < 60 | BLOCK. Resolve contradiction. |
| artifact registry unreachable | Block. Use last-known state, mark `unverified`. |
| Local main SHA differs from origin/main | BLOCK. Investigate the remote state. |
| Tag push fails | BLOCK. Investigate permissions. |
| Debt-verify is INCONCLUSIVE | BLOCK. Retry failed coverage or request human review. |
| HTML report fails (when required) | BLOCK. Re-generate via sddk-archive. |
| Per-task attempts > CIRCUIT_PER_TASK_MAX_ATTEMPTS | BLOCK. Escalate to user (loop engineering freno duro). |

Abort commit format (mid-cycle abandon):
```
chore(abort): abandoning <change> — <reason>

Reason: <what went wrong>
Last checkpoint: <task-id>
```

---

## Anti-Patterns (FORBIDDEN)

| Anti-pattern | Consequence | Enforcement |
|--------------|-------------|-------------|
| Treating a PR as the only route to main | Adds an external dependency | Local Git postcondition is authoritative |
| Force-pushing to main | Destroys history | Checklist blocks |
| Rebasing feature branches | Loses review history | Checklist blocks |
| Starting new cycle without closing previous | Two cycles open | Step 0.2 gate |
| Waiting for CI/CD or Actions | External distribution can stall the cycle | Explicitly excluded from release gates |
| Skipping semver tag | Lost milestone | Step 3.2 gate |
| Skipping trunk sync | Working on stale main | Step 0.1 + 4.1 gates |
| Co-Authored-By in commit | AI attribution leaked | Checklist blocks |
| Running full SDDK for C3 fix | Waste | Use B-direct via triage |
| Coherence check on B-direct | Theater | Skipped by path |
| HTML report for patch tag | Overhead | Skipped by path |

---

## Quick Reference — MCW Step Index

| Phase | Step | Action | Hard gate |
|-------|------|--------|-----------|
| 0 | 0.1 | Trunk sync | HEAD == origin/main |
| 0 | 0.2 | Previous cycle closed | No unmerged branches/PRs |
| 0 | 0.3 | Knowledge coverage (A-full) | No critical gaps |
| 0 | 0.4 | Triage | Path decided |
| 1 | 1.1 | Explore (A-full) | explore-report approved |
| 1 | 1.2 | Propose | proposal approved |
| 1 | 1.3 | Spec+Design parallel (A-full) | Both approved |
| 1 | 1.4 | Coherence propose→spec (A-full) | ≥ 60 |
| 1 | 1.5 | Tasks | tasks approved |
| 1 | 1.6 | Coherence spec+design→tasks (A-full) | ≥ 60 |
| 1 | 1.7 | Review budget | advisory; recommend + allow + rationale |
| 1 | 1.8 | Branch creation | Name matches regex |
| 2 | 2.1 | Apply | Commits pass declarative git checklist |
| 2 | 2.2 | Coherence apply→verify (A-full, A-lite) | ≥ 60 |
| 2 | 2.3 | Verify | PASS or PW |
| 2 | 2.4 | **Debt-verify (MANDATORY on A-*; disabled on B-direct; depth derived from path)** | PASS or PW; INCONCLUSIVE blocks |
| 2 | 2.5 | Coherence debt-verify→release (A-full) | ≥ 60 |
| 3 | 3.1 | Local verify + direct main push | HEAD == origin/main |
| 3 | 3.2 | Annotated semver tag | Remote tag peels to main SHA |
| 3 | 3.3 | Local receipts + release transition | Receipts persisted; runtime enters RELEASED/archive |
| 3 | 3.4 | Durable archive closure | Archive manifest references release receipt; runtime CLOSED |
| 4 | 4.1 | Sync main | HEAD == origin/main |
| 4 | 4.2 | F3 tuning + metrics | Tuning written |
| 4 | 4.3 | Jurisprudence (conditional) | Observation saved |
| 4 | 4.4 | Result contract | User notified |
