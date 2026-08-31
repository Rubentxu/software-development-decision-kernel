# HANDOFF — cycle-27 CLOSED → cycle-28 starting point

> **Cycle:** `p-52b95ef55999f9de/kernel-cycle-27-source-operator-evaluation`
> **Status:** CLOSED ✅
> **Tag:** `v1.43.0` peels to `8abdd5d`
> **HEAD:** `8abdd5d` on `main`
> **Path:** A-min
> **Date closed:** 2026-08-24
> **Next cycle:** **cycle-28** — `Map max_concurrency + error aggregation collect-all`

---

## 1. What cycle-27 shipped

Promoted `Operator::Map` from cycle-26 stub (`const STUB_N: usize = 3`, body called 3 times against nothing) to a real source-evaluation runtime: dispatch the source operator, extract `outputs["items"]: Array`, clone the body `Task` per iteration, merge `{item, index}` into inputs, sequential evaluate (concurrency deferred to cycle-28), first-failure aggregation (collect-all deferred to cycle-28).

### Final 2 commits (RED → GREEN, after apply-correction round 1)

```
8abdd5d feat(engine): Map source evaluation GREEN plumbing (cycle-27)
4c55f36 test(engine): RED tests for Map source plumbing (cycle-27)
```

`4c55f36` touches only the test file; `8abdd5d` touches impl + ADR-0062 + handoff. This is the TDD chronology the strict verifier required.

### Files affected (4)

| File | Δ | What |
|---|---|---|
| `crates/sddk-engine/src/operator.rs` | +130/-X | Map impl 1047-1092 + docstring 1037-1046 |
| `crates/sddk-engine/tests/map_operator_tests.rs` | +528/-X | 13 tests total (was 3 cycle-26 stubs + 9 cycle-27 + 4 doc-oracle in correction round) |
| `docs/adr/ADR-0062-map-source-plumbing.md` | NEW | Collection-key convention `outputs["items"]: Array` + Task-only body restriction + per-iteration inputs injection |
| `docs/handoff/HANDOFF-2026-08-26-cycle-27-source-operator-evaluation.md` | NEW | This file (cycle-27 handoff to cycle-28) |

Total: 908 insertions, 67 deletions across 4 files.

### Spec coverage (14/14 COMPLIANT)

`{cycle-artifacts-dir}/specs/engine/REQ-Map-Source-Evaluation.md` — 5 requirements, 14 scenarios:

| Requirement | Scenarios | Status |
|---|---|---|
| REQ-Map-Source-Evaluation | 3 | ✅ |
| REQ-Map-Inputs-Injection | 3 | ✅ |
| REQ-Map-Collection-Key-Convention | 3 | ✅ |
| REQ-Map-Body-Restriction | 2 | ✅ |
| REQ-Map-Doc-Cycle27-OutOfScope | 3 | ✅ |

---

## 2. Debt findings (5, deferred to cycle-28 + backlog)

`debt-report.json` at `{cycle-artifacts-dir}/debt-report.json` (sha256 `32b0a7ebb614bf1fb38cc3d7ad52fdb9939831e80f507a695d3a622684eff53d`).

| ID | Cluster | Severity | Priority | Summary |
|---|---|---|---|---|
| DC-MAP-001 | coupling | medium | P2 | `source.evaluate(ctx)` reuses parent `OperatorContext`; scratches `node_run.state`, `attempts`, `store` are observable by Map's body iterations — breaks scratch isolation that Parallel preserves |
| DC-MAP-002 | coupling | medium | P2 | `Map` calls `dispatch()` global for both source and body — same global coupling as Parallel/Sequence |
| DE-MAP-001 | overeng | low | P3 | `inputs_override` field on `OperatorContext` was deliberately NOT introduced (positive finding — cycle-27 limited scope to Task-only body via clone+merge) |
| DE-MAP-002 | overeng | low | P3 | BTreeMap→JSON ordering: per-iteration outputs aggregated into JSON; deterministic order is preserved but worth a contract test in cycle-28 |
| DE-MAP-003 | overeng | low | P3 | Doc drift risk: ADR-0061 §Consequences is now stale (describes cycle-26 state); cycle-28 handoff should mark ADR-0061 as superseded by ADR-0062 or roll ADR-0061 §Consequences forward |

None are blocking. **Cycle-28 picks up DC-MAP-001/002 (P2 medium) via concurrency control.** The P3 findings can be a quick ADR housekeeping pass.

---

## 3. Next cycle — cycle-28

**Goal:** `Map max_concurrency` + error aggregation collect-all. Closes ADR-0061/0062 limitations (2) and (4).

**Path:** A-min (same shape as cycle-27).

**Scope:**
1. **`max_concurrency` enforcement** — semaphore-gated thread pool. ADR-0055 (`docs/adr/ADR-0055-p3-closure-counting-semaphore.md`) defines the semaphore primitive. Wire `Map::evaluate` to fan out body iterations through a `Semaphore::acquire()` before each `body_op.evaluate(ctx)`. When `max_concurrency == 0`, fall back to unbounded (back-compat with cycle-26/27 semantics where the field was ignored). When `max_concurrency == 1`, preserve sequential semantics (no thread spawn).
2. **Collect-all error aggregation** — instead of returning on first `Failed`, accumulate per-iteration results into `outputs["results"]: Vec<Value>` AND collect failures into `outputs["failures"]: Vec<{index, reason}>`. Final outcome: `Succeeded` if at least one iteration succeeded, `Failed` if all failed. (Trade-off: see open question below.)
3. **Doc-oracle update** — Map docstring must now mention cycle-28 in the deferred list is REMOVED. Add `max_concurrency` enforcement as in-scope, list only `cross-tick replay (cycle-29)` as deferred.

**Open question for cycle-28 spec:**
- Empty `results` + non-empty `failures` → return `Failed` with composite reason? Or `Succeeded { results: [], failures: [...] }`?
- Recommendation: `Succeeded` if at least one body succeeded, `Failed` if all failed. Document explicitly. Confirm with user before spec.

**Pre-allocated test surface (cycle-28 spec draft, 3 new requirements):**
- REQ-Map-Max-Concurrency (3 scenarios) — `max_concurrency=1` sequential; `max_concurrency=2` parallel via semaphore; `max_concurrency=0` unbounded
- REQ-Map-Collect-All-Errors (3 scenarios) — partial failures still Succeeded with `failures`; all failures → Failed with composite; mixed
- REQ-Map-Doc-Cycle28-OutOfScope (1 scenario) — only `cross-tick replay` (cycle-29) remains deferred

Update `map_operator_tests.rs` with ~5-6 new tests, fix the cycle-27 doc-oracle tests (they check for `cycle-28` in docstring but cycle-28 will remove that mention → adjust the assertions).

---

## 4. Operational notes for the next session

### Environment (already set up)

- `sddk` CLI: `/home/rubentxu/.local/bin/sddk`, version 1.39.0.
- Working dir: `~/Proyectos/agentesIA/sddk-framework/` (NOT `~/.sddk-shared/`).
- Memory: Engram MCP (mandatory close with `mem_session_summary`).
- Vault: `~/.sddk-knowledge/sddk-framework/`.

### Cycle-management commands (proven in cycle-27)

```bash
# Start cycle (creates the cycle, acquires lease)
sddk cycle start \
  --root ~/Proyectos/agentesIA/sddk-framework --scope . \
  --name "kernel-cycle-28-map-max-concurrency-and-error-aggregation" \
  --path a-min --lease-owner orchestrator --format json

# Lease acquire (re-acquire between phases since lease auto-releases)
sddk cycle lock acquire --root . --scope . --cycle <cycle_id> --owner orchestrator --format json

# Status probe
sddk cycle status --root . --scope . --cycle <cycle_id> --format json

# Transition with gate receipts
sddk cycle transition \
  --root . --scope . --cycle <cycle_id> \
  --transition <transition_id> \
  --artifact "<kind>=<path>" \
  --gate-receipt <receipt_id> \
  --gate-receipt <receipt_id> \
  --lease-owner orchestrator --fencing-token 1 --format json

# Gate evaluation
sddk cycle evaluate-gate \
  --root . --scope . --cycle <cycle_id> \
  --transition <transition_id> \
  --gate <gate_name> \
  --outcome passed \
  --evaluator sddk.cli \
  --evidence '<json>' --timestamp "$(date -u +%FT%TZ)" --actor sddk --format json

# Final ledger verify
sddk ledger verify --root . --scope . --format json
```

### Phases authoritative (A-min runtime graph)

`workflow/workflow.yaml` is authoritative. A-min phase sequence:
`explore → specify → build → verify → debt-verify → release → archive`

Note: **NO separate tasks phase**. `phase.specify.complete.a-min` transitions directly to `phase: build`. The `sddk-a-min.yaml` projection in `prompts/sddk/workflows/` lists tasks as a step but the runtime graph bypasses it. Trust the runtime graph.

### `phase.verify.complete.a-min` requires 4 gate receipts bundled in ONE transition

- `tests-pass`
- `policy-compliant`
- `debt-severity-assigned`
- `debt-priority-assigned`

Use `--gate-receipt <id>` (singular, repeatable). NOT `--requirement`.

### TDD chronology

Verifier enforces RED-then-GREEN commits. For a localized change like cycle-27, the cleanest pattern is:

```bash
git checkout -b feat/kernel-cycle-NN-<slug>
# (do work in any commit order, e.g., 1-2 commits)

# When ready to verify, rewrite history:
git reset --soft <base_sha>
git add <test-files-only>
git commit -m "test(<scope>): RED tests for <feature> (cycle-NN)"
git add <impl-and-docs>
git commit -m "feat(<scope>): <feature> GREEN plumbing (cycle-NN)"
```

### Strict-TDD gotchas

- **Pre-existing clippy::manual_ok_err** in `crates/sddk-cli/src/inventory_cycle.rs:240` is tolerated (also exists at base). Do NOT modify inventory_cycle.rs to fix it — that would scope-drift.
- For doc-oracle tests, parse the file as a string and regex-match. Example:
  ```rust
  let doc = include_str!("../src/operator.rs");
  assert!(doc.contains("max_concurrency"), "doc must mention max_concurrency");
  assert!(doc.contains("cycle-28"), "doc must point to cycle-28 for next steps");
  ```
- 13 RED tests in `map_operator_tests.rs` already exist for cycle-27; cycle-28's new tests will land in the same file. Update cycle-27's 3 doc-oracle tests when cycle-28 removes `cycle-28` from the deferred list.

### Conventional commits (AGENTS.md §2.1)

- Spanish: `feat(engine): …`, `test(engine): …`, `docs(adr): …`, `docs(handoff): …`, `fix(infra): …`, `chore(release): …`
- NO `Co-Authored-By`. NO AI attribution.
- Single concern per commit (if code + docs, one commit with rationale in body).
- Branch regex: `^[a-z]+/[a-z0-9-]+$`, type in `feat|fix|chore|docs|refactor|perf|test|ci|revert`.
- Trunk sync (mcw §Step 0.1 + 4.1): `git fetch origin main && git pull --ff-only origin main` BEFORE starting any new cycle.

### Semver (mcw §Step 3.2)

| Change type | Bump |
|---|---|
| Breaking public API/contract | major |
| New feature (non-breaking) | minor |
| Bug fix, chore, docs, refactor | patch |

Cycle-27 was minor (1.42.5 → 1.43.0) because it added observable runtime behavior (Map went from stub to real). Cycle-28 will likely also be minor unless it changes Task body's interface.

---

## 5. Cycle-27 lessons learned (avoid in cycle-28)

1. **Apply-correction round 1 was needed** because the first apply collapsed RED+GREEN into a single commit (TDD chronology violation). For cycle-28: write RED commit first, then GREEN commit. Use `git reset --soft` if you forget.
2. **Doc-oracle tests were missing** for 3 scenarios in round 1. For cycle-28: write ALL doc-oracle tests in the RED commit (they'll RED-fail until the docstring is updated in the GREEN commit).
3. **Lease auto-releases** between phase agent runs. The orchestrator (me) re-acquires between each dispatch. Don't expect the lease to persist.
4. **`--gate-receipt` is the correct flag** for multiple gates in one transition. `--requirement` is for textual non-artifact requirements, not gate IDs.
5. **`sddk cycle transition phase.build.complete`** will fail with `ENGINE_SOURCE_STATE_MISMATCH` if the cycle is already past `build` (e.g., in `verify`). The apply agent shouldn't try to re-transition after a correction round — just keep working on the same branch and let the orchestrator handle the final transition.

---

## 6. Files to inspect first when resuming

```bash
# 1. Current state
cd ~/Proyectos/agentesIA/sddk-framework
git status --short          # MUST be empty
git log --oneline -5        # last 5 commits (head should be 8abdd5d)
git tag --points-at HEAD    # should show v1.43.0

# 2. Map impl + tests
sed -n '1037,1092p' crates/sddk-engine/src/operator.rs
sed -n '1,80p' crates/sddk-engine/tests/map_operator_tests.rs   # 13 tests

# 3. ADRs
cat docs/adr/ADR-0061-map-operator-stub-scope.md  # cycle-26 stub limitations
cat docs/adr/ADR-0062-map-source-plumbing.md      # cycle-27 decisions

# 4. Spec
cat .sddk-knowledge/sddk-framework/specs/engine/REQ-Map-Source-Evaluation.md

# 5. Debt report (cycle-28 should pick up P2 findings)
cat /home/rubentxu/.local/share/sddk/projects/p-52b95ef55999f9de/cycle-artifacts/p-52b95ef55999f9de/kernel-cycle-27-source-operator-evaluation/debt-report.md

# 6. Recovery cheat sheet
git reset --hard eba8534 && git branch -D feat/kernel-cycle-27-source-operator-evaluation
```

---

## 7. Recovery cheat sheet (rollback cycle-27 if needed)

```bash
cd ~/Proyectos/agentesIA/sddk-framework
git reset --hard eba8534                   # back to pre-cycle-27
git branch -D feat/kernel-cycle-27-source-operator-evaluation
git tag -d v1.43.0                         # local
git push origin :refs/tags/v1.43.0         # remote (CAREFUL: this rewrites remote history)
```

NOTE: only do this if cycle-27 needs to be reverted before cycle-28 ships. The tag v1.43.0 is now on `origin/main` — reverting means force-pushing or a follow-up cycle.

---

## 8. Quick reference for the next session's first 3 commands

```bash
# 1. Confirm trunk is clean and at v1.43.0
cd ~/Proyectos/agentesIA/sddk-framework
git status --short && git rev-parse HEAD && git tag --points-at HEAD
# Expected: empty status, HEAD=8abdd5d, tag=v1.43.0

# 2. Confirm no active cycle (or resume the active one)
sddk cycle status --root . --scope . --cycle p-52b95ef55999f9de/kernel-cycle-27-source-operator-evaluation --format json | jq -r '.status'
# Expected: CLOSED

# 3. Start cycle-28 (only after confirming above)
sddk cycle start \
  --root . --scope . \
  --name "kernel-cycle-28-map-max-concurrency-and-error-aggregation" \
  --path a-min --lease-owner orchestrator --format json
```
