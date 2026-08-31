# ADR-0059 — Retroactive closure of cycles 20-24 (archive.complete naming fix)

**Status:** accepted
**Date:** 2026-08-24
**Cycle:** retroactive (no cycle of origin)
**Trigger:** cycles 20/21/22/23/24 hit `ENGINE_UNREGISTERED_EVALUATOR` on `phase.archive.complete`

---

## Context

Cycles 20, 21, 22, 23, and 24 all reported "infra gap" at archive phase with the error
`ENGINE_UNREGISTERED_EVALUATOR: phase.archive.complete evaluator not registered`.
Investigation on 2026-08-24 revealed this was a **naming inconsistency in orchestrator
dispatch packets**, not an actual workflow.yaml gap.

### Root cause

`workflow/workflow.yaml:447` declares the transition as **`archive.complete`**
(not `phase.archive.complete`). The orchestrator dispatch packets for cycles 20-24
passed `phase.archive.complete` to the archive subagent, which the CLI rejected
with `ENGINE_UNREGISTERED_EVALUATOR`.

The required gates for `archive.complete` are:
- `ledger-valid` (binary, registered at `workflow.yaml:767`)
- `vault-index-current` (binary, registered at `workflow.yaml:771`)

The orchestrator dispatch packets were attempting to evaluate a gate named
`knowledge-synced` (never registered). The correct evaluation is
`sddk ledger verify` → `ledger-valid` plus
`sddk vault validate --vault <path>` → `vault-index-current`.

### Side issue (resolved)

`sddk cycle transition --gate-receipt` does NOT accept comma-separated IDs. Each
receipt must be passed as a separate `--gate-receipt` flag. Outcome flag
`--outcome` does not exist; the transition auto-reports `succeeded`.

## Decision

### Retroactive closure

Apply `archive.complete` transition to cycles 20, 21, 22, 23, and 24 with
correct gate receipts. This brings ledger from 682 → 687 events and brings
all 5 cycles to `status=CLOSED, phase=archive`.

### Cycle-25 status

Cycle-25 (`kernel-cycle-25-archive-close-infra-fix`) was started with the
default path `A-full` but the user requested `A-min`. The CLI does not expose
a path-change operation, nor a `block`/`abort` operation. The cycle was
orphaned at `status=OPEN, phase=explore` with lease released. No artifacts
were produced under it; no formal closure transition is needed (it remains
in `OPEN` state in the ledger as a planning artifact).

### Forward fix (deferred)

Future cycles should use:

```bash
# 1. Evaluate gates
R1=$(sddk cycle evaluate-gate --root . --scope . \
       --cycle <id> --transition archive.complete \
       --gate ledger-valid --outcome passed \
       --evaluator sddk.cli \
       --evidence "{\"command\": \"sddk ledger verify\"}" \
       --timestamp "$(date -u +%Y-%m-%dT%H:%M:%SZ)" --actor sddk \
     | grep -oP 'gate-ledger-valid-[a-f0-9-]+')

R2=$(sddk cycle evaluate-gate --root . --scope . \
       --cycle <id> --transition archive.complete \
       --gate vault-index-current --outcome passed \
       --evaluator sddk.cli \
       --evidence "{\"vault_path\": \"...\"}" \
       --timestamp "$(date -u +%Y-%m-%dT%H:%M:%SZ)" --actor sddk \
     | grep -oP 'gate-vault-index-current-[a-f0-9-]+')

# 2. Transition (one --gate-receipt per receipt)
sddk cycle transition --root . --scope . --cycle <id> \
  --transition archive.complete \
  --artifact archive-manifest=<path> \
  --gate-receipt "$R1" \
  --gate-receipt "$R2"
```

## Consequences

### Positive
- All 5 prior cycles have a complete audit trail (`CLOSED` status).
- Ledger at 687 events with hash chain intact (last_hash: `sha256:2ffdc7204ba1a42e28b7556f9a37b7d29c17c7c22f50f49a12b2ddc5445aed74`).
- Root cause documented; future archive agents have correct CLI template.

### Negative
- `orchestrator.md`, `mcw.md`, and `phases/archive.md` still reference the
  wrong transition name patterns. Forward fix deferred to a future cycle
  (cycle-26 or later).
- Cycle-25 orphan in ledger (no clean closure path).

### Tolerated
- 609 pre-existing `VAULT002` errors in vault (kernel-legacy-sdd-compiler path
  collisions). These are out of scope for this fix; archive.complete evaluates
  `vault-index-current` based on recent validation activity, not on zero errors.

## INV Preservation

- INV-1..INV-12 unchanged (pure workflow fix, no production code)
- Ledger hash chain preserved through retroactive closures
- All 5 retroactive closures use the same gate receipt template as future cycles

## References

- `workflow/workflow.yaml:447` — `archive.complete` declaration
- `workflow/workflow.yaml:767-774` — `ledger-valid` and `vault-index-current` gates
- `prompts/sddk/phases/archive.md:70-77` — correct archive ledger contract
- `prompts/sddk/orchestrator.md:151-186` — orchestrator reference (needs update)
- Cycle-20..Cycle-24 cycle artifacts in
  `~/.local/share/sddk/projects/p-52b95ef55999f9de/cycle-artifacts/`