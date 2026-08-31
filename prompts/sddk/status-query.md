# Status Query — How to Reconstruct Current Project State

The orchestrator can answer "what's the current state?" at any time by querying the **knowledge graph vault** + git. The vault is the primary source; git corroborates.

## The 2 Sources

| Source | What it tells you | How to query |
|--------|-------------------|--------------|
| **Knowledge graph vault** (`$VAULT_PATH` via `sddk knowledge path`) | All knowledge: milestones, ADRs, requirements, cycles, incidences, terms — with wikilinks, status, and bi-temporal changelogs | `grep`, `ls`, open `_index.md` for Dataview MOC |
| **Git** (project repo) | What branches exist, what's merged, what tags are on main | `git branch`, `git tag`, `git log` |

## Query: "Is there an active cycle?"

```bash
# Vault lock check (authoritative)
grep "Status:" "$VAULT_PATH/milestones/_active.md"
# "LOCKED" → cycle in progress; "AVAILABLE" → no active cycle

# Git cross-check
git branch -a | grep -E "^.*(feat|fix|chore|refactor)/"
# Unmerged branches suggest in-progress or abandoned cycle
```

## Query: "What happened in the last cycle?"

```bash
# Open the most recent cycle manifest (traceability hub)
ls -t "$VAULT_PATH"/cycles/CYC-*.md | head -1
# Read it — it links to all artifacts, ADRs, requirements, and incidences

# Check the last tag on main
git tag --points-at main | tail -1
```

## Query: "What ADRs are challenged?"

```bash
grep -l "status: challenged" "$VAULT_PATH"/adrs/*.md
# Each has an Implementation Log explaining what went wrong
```

## Query: "What requirements exist in auth?"

```bash
ls "$VAULT_PATH"/specs/auth/REQ-*.md
# Each requirement links to its decision authority ADR and test path
```

## Query: "What incidences are open?"

```bash
grep -l "status: open" "$VAULT_PATH"/incs/*.md
```

## Inconsistency Detection

| Vault says | Git shows | Diagnosis |
|---|---|---|
| `_active.md` LOCKED | no matching branch | **Stale lock** — session crashed. Resume via `/sddk-release` or mark blocked. |
| AVAILABLE | unmerged feature branch | **Orphan branch** — from past cycle or manual work. |
| milestone `completed` + tag | tag missing on main | **Broken release** — re-run `/sddk-release`. |
| ADR `proposed` (old) | — | **Stuck ADR** — cycle never released. Check if blocked/abandoned. |
| ADR `challenged` | — | **Needs attention** — should trigger superseding ADR. |
| requirement `stale_after` < today | — | **Stale requirement** — may not reflect current code. Flag for review. |
