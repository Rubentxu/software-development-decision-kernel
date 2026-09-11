# AX-S3 — Contextual command-surface token budget

**Status:** COMPLETED 2026-09-11 (v1.168.16)
**Question (SPIKES.md):** measure the global CLI contract vs task-specific surfaces for common `change`, `verify`, `ship` and `recover` agents. Select minimal-surface heuristics that preserve task success while reducing irrelevant commands.
**Harness:** `crates/sddk-cli/src/spike_axs3.rs` (deterministic, pinned by 4 tests).

## Measurements (live 49-command spec table, text rendering, ~4 chars/token)

| Flow | Depth-0 commands | Depth-0 tokens | Depth-1 added | Depth-1 tokens | Global tokens |
|---|---|---|---|---|---|
| change | `change` | 40 | *(none)* | 40 | 1073 |
| verify | `verify` | 18 | *(none)* | 18 | 1073 |
| ship | `ship` | 8 | *(none)* | 8 | 1073 |
| recover | `recover` | 18 | *(none)* | 18 | 1073 |

(Global = 49 commands ≈ 8.4 KB rendered ≈ 1073 tokens estimated.)

## Findings

1. **Task surfaces already reduce budget by ~96–99%.** The existing
   `command_matches_target` heuristic (exact name, first word, substring,
   `related` edges) collapses the 49-command contract to 1 command per
   flow. The "global vs task-specific" question is answered: task-specific
   surfaces are essentially free compared to shipping the full contract
   (~2.1% of global tokens at worst).

2. **The `related` graph is the real frontier, not token cost.** Depth-1
   expansion added **zero** commands for all four flows: the facade specs
   (`change`, `verify`, `ship`, `recover`) carry no `related` edges to the
   commands they delegate to (`cycle status`, `capability run`,
   `release plan`, `cycle rebuild`, …). A task agent that needs recovery
   context (e.g. `verify` failing → `ledger verify`) cannot discover it
   through the surface; the heuristic's substring pass is what currently
   papers over this.

3. **Recommendation (adopted as direction, not yet implemented):** keep
   the minimal-surface heuristic (depth-0 + substring). Enriching the
   `related` edges on the four facades is the cheap, high-leverage next
   step when a real workflow needs cross-command discovery — one line per
   edge, no heuristic changes, and the drift guard from AX-S1 keeps the
   table honest.

## Revisit triggers

- Adding an agent profile that requires multi-command workflows from the
  cheat sheet alone.
- The spec table growing past ~100 commands (global cost starts to matter
  even for interactive use).
