# HANDOFF — cycle-28 CLOSED → cycle-29 starting point

> **Cycle:** `p-52b95ef55999f9de/kernel-cycle-28-map-max-concurrency-and-error-aggregation`
> **Status:** CLOSED ✅
> **Tag:** `v1.44.0` peels to `<commit-sha>`
> **HEAD:** `<commit-sha>` on `feat/kernel-cycle-28-map-max-concurrency-and-error-aggregation`
> **Path:** A-min
> **Date closed:** 2026-08-26
> **Next cycle:** **cycle-29** — `sddk dev reconcile` (Authoritative IDE reconciliation) — *rescheduled 2026-08-26: previously planned for Map source-context isolation + cross-tick replay, moved to cycle-30.*

---

## 1. What cycle-28 shipped

Promoted `Operator::Map` from cycle-27 first-failure + sequential-only semantics to full runtime semantics:
- **`max_concurrency` enforcement** via `CountingSemaphore` (ADR-0055). Divergent from `Parallel`: Map's `map_max_concurrency_effective(0, n) = n.max(1)` (unbounded), not `apply_default_max_concurrency(0) = 16`.
- **Collect-all error aggregation**: `outputs["results"]` (successes only, iteration order) + `outputs["failures"]` (`[{index, reason}]`). Outcome: `Succeeded` if ≥1 succeeded; `Failed` with composite reason (top-10 truncated with `...`) if ALL failed.
- **Doc-oracle update**: cycle-28 items removed from deferred list; only `cross-tick replay (cycle-29)` remains deferred.

### Final 4 commits

```
<commit-4> docs(handoff): cycle-28 handoff (cycle-28)
<commit-3> docs(adr): ADR-0063 + ADR-0061 supersession marker (cycle-28)
<commit-2> feat(engine): Map max_concurrency + collect-all GREEN plumbing (cycle-28)
<commit-1> test(engine): RED tests for Map max_concurrency + collect-all (cycle-28)
```

`<commit-1>` touches only the test file; `<commit-2>` touches impl + docstring; `<commit-3>` touches both ADR files; `<commit-4>` touches only the handoff. TDD chronology validated by strict verifier.

### Files affected (5)

| File | Δ | What |
|---|---|---|
| `crates/sddk-engine/src/operator.rs` | +307/-70 | Map impl 1037-1392 + docstring 1037-1060; `map_max_concurrency_effective` helper; `build_map_composite_failure_reason` helper |
| `crates/sddk-engine/tests/map_operator_tests.rs` | +435/-15 | 18 tests total (was 13 cycle-27 + 5 new cycle-28 + 3 doc-oracle updates + 1 rewrite) |
| `docs/adr/ADR-0063-map-max-concurrency-error-aggregation.md` | NEW | Map max_concurrency + collect-all decision |
| `docs/adr/ADR-0061-map-operator-stub-scope.md` | +2/-2 | Supersession marker for cycle-28 |
| `docs/handoff/HANDOFF-2026-08-26-cycle-28-map-max-concurrency-error-aggregation.md` | NEW | This file (cycle-28 handoff to cycle-29) |

Total: ~745 insertions, ~87 deletions across 5 files.

### Spec coverage (7/7 COMPLIANT)

`{cycle-artifacts-dir}/specs/engine/REQ-Map-Max-Concurrency-Errors.md` — 4 requirements, 7 scenarios:

| Requirement | Scenarios | Status |
|---|---|---|
| REQ-Map-Max-Concurrency | 3 | ✅ |
| REQ-Map-Collect-All-Errors | 3 | ✅ |
| REQ-Map-Doc-Cycle28-OutOfScope | 1 | ✅ |

---

## 2. Debt findings (resolved / deferred to cycle-29)

`debt-report.json` at `{cycle-artifacts-dir}/debt-report.json`.

| ID | Cluster | Severity | Priority | Summary | Status |
|---|---|---|---|---|---|
| DC-MAP-001 | coupling | medium | P2 | `source.evaluate(ctx)` reuses parent `OperatorContext`; per-iteration scratch isolation via per-thread store | Partially resolved (body iterations get per-thread scratch); source still reuses parent ctx |
| DC-MAP-002 | coupling | medium | P2 | `Map` calls `dispatch()` global for both source and body | Deferred to cycle-29 |
| DE-MAP-001 | overeng | low | P3 | `inputs_override` field deliberately NOT introduced | Closed (housekeeping) |
| DE-MAP-002 | overeng | low | P3 | BTreeMap→JSON ordering: per-iteration outputs aggregated into JSON | Closed (contract test exists) |
| DE-MAP-003 | overeng | low | P3 | Doc drift risk: ADR-0061 §Consequences stale | Closed (ADR-0061 superseded marker added) |

**Cycle-29 picks up DC-MAP-002 (P2 medium) as source-context isolation.** DC-MAP-001 is partially resolved but full isolation requires source-context work.

---

## 3. Next cycle — cycle-29

**Goal:** `sddk dev reconcile` — Authoritative IDE reconciliation. Spec: [SPEC-RECONCILE-001](../reconciliation-spec.md). ADR: [ADR-0064](../adr/ADR-0064-sddk-authored-reconciliation.md). Roadmap: [ROADMAP §Cycle-29 candidate](../sddk-decision-kernel-architecture/02-roadmap/ROADMAP.md).

**Path:** A-min.

**Scope:**
1. **`EditorCapabilities` + `ReconcileAdapter` trait** — abstractions for per-IDE capabilities and read-modify-write semantics.
2. **JSON adapters (opencode, zcode)** — read existing, mutate 5 sddk keys in place, preserve `extras`.
3. **Claude adapter** — YAML frontmatter parser (preserving unknown keys), read-modify-write `.md` files.
4. **Codex adapter** — `toml::from_str` + re-serialize with `atomic_write`, preserving unknown keys (incl. `model_reasoning_*`).
5. **`sddk dev reconcile` command** — dry-run by default, `--apply`, `--check`, `--format json`, exit codes.
6. **Pruning** — sddk-namespace-only; preserve user agents.

### Rescheduled from cycle-29 to cycle-30

The original plan for cycle-29 was `Map source-context isolation + cross-tick replay`. Rescheduled because:
- `sddk dev reconcile` has higher user-visible impact (drift detection, `sddk dev doctor` alignment).
- Map replay can absorb into cycle-30 without breaking change.

**Cycle-30 scope (preview):**
1. **Source-context isolation (DC-MAP-002)** — `source.evaluate(ctx)` should use a fresh child `OperatorContext` instead of the parent. Per-iteration body contexts already have per-thread scratch stores. The source context is the remaining gap.
2. **Cross-tick replay** — support resumption of Map operators that return `Pending` (currently propagated immediately). Requires checkpoint state management across workflow ticks.

**Pre-allocated test surface for cycle-29:**
- JSON: dry-run no-write; apply mutates 5 keys, preserves `extras`; user agent untouched; idempotent.
- Claude/Codex: read existing, update known keys, preserve unknown keys; dry-run no-write.
- Prune: bundle-only, user agents preserved.
- `--check` exit codes (0 / 1).
- Regression: `link_e2e_tests`, `models_cmd_tests`, `agent_models_tests` all green.

---

## 4. Operational notes for the next session

### Environment (already set up)

- `sddk` CLI: `/home/rubentxu/.local/bin/sddk`, version TBD.
- Working dir: `~/Proyectos/agentesIA/sddk-framework/` (NOT `~/.sddk-shared/`).
- Memory: Engram MCP (mandatory close with `mem_session_summary`).
- Vault: `~/.sddk-knowledge/sddk-framework/`.

### Cycle-management commands (proven in cycle-28)

```bash
# Build + test
cargo build --release -p sddk-cli && cargo test --workspace

# Commit (strict TDD chronology)
git add crates/sddk-engine/tests/  # RED tests first
git commit -m "test(engine): RED tests for ..."
git add crates/sddk-engine/src/    # GREEN impl second
git commit -m "feat(engine): ..."
git add docs/adr/                 # ADR third
git commit -m "docs(adr): ..."
git add docs/handoff/              # handoff last
git commit -m "docs(handoff): cycle-N handoff (cycle-N)"

# Verify clean tree before next commit
git status --porcelain  # should be clean except assets/agent-models.yaml
```

### Key implementation decisions in cycle-28

1. **Map diverges from Parallel on `max_concurrency=0`**: `map_max_concurrency_effective(0, n) = n.max(1)` (unbounded), vs `apply_default_max_concurrency(0) = 16` (cap). This is intentional per ADR-0063 §D-1.

2. **Empty collection edge case**: When `items` is empty (`len=0`), both `results=[]` and `failures=[]`. Aggregate returns `Succeeded { results: [], failures: [] }` (vacuous truth).

3. **Composite reason truncation**: Top-10 failures with `...` elision. Full list in `outputs["failures"]` (no truncation in array).

4. **Pending propagation**: If any body iteration returns `Pending`, Map propagates `Pending` immediately (early return). Does NOT aggregate Pending as a failure.

5. **`ChildPanicked` recording**: Thread panics are recorded as `Failed { reason: "child i panicked" }` in `failures`, consistent with `Parallel` behavior.
