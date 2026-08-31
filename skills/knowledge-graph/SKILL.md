---
name: knowledge-graph
description: >
  Protocol for reading and writing the SDDK knowledge graph vault. Used by all SDDK phase agents
  to create, update, and query nodes (milestones, ADRs, requirements, cycles, incidences, terms).
  The vault is outside each project repo and is resolved only through the SDDK CLI.
  Follows OKF + Obsidian Properties conventions. Wikilinks [[like-this]] create the graph.
license: MIT
metadata:
  author: gentleman-programming
  version: "3.0"
  okf_version: "0.2"
---

# Knowledge Graph Protocol

You have access to the SDDK knowledge graph vault as the **single source of truth** for project knowledge. This skill defines HOW to read, write, and query nodes. Follow this protocol exactly.

## Vault Location

Resolve the vault path using:

```bash
sddk knowledge path --root .
```

This prints the canonical vault for the stable `project_id`. Also run
`sddk knowledge status --root . --scope . --format json` to obtain the profile
and optional Engram setting.

**CRITICAL**: never derive the vault or project identity from a checkout name,
repository basename, or hard-coded home path. A rename or worktree must resolve
to the same project identity through the CLI.

The vault is outside the adopted workspace. Existing product documentation in
the workspace is read-only evidence, not knowledge authority. `sddk adopt
apply` initializes the configured vault from the runtime bundle without
planting files in the workspace.

## Repository Knowledge Import

After adoption, ingest repository-owned evidence through the governed pipeline:

```bash
sddk knowledge scan --root . --scope . --format json
sddk knowledge import --root . --scope . --plan <plan-id>
sddk knowledge verify --root . --scope .
```

`scan` reads the checkout without modifying it, classifies known ADRs, specs,
terms, incidences, roadmaps, manifests, baselines, rule catalogs, and root
context documents, then writes a reviewable plan to the external vault. The
plan records source path, Git commit, line range, SHA-256, owner, relation,
links, existing entry, proposed disposition, and quarantine reason.

`import --plan` is the explicit promotion boundary. Versioned sources with an
owner and unambiguous relation become trusted versions in the append-only
registry. Ambiguous, unowned, unversioned, changed, or contradictory sources
enter `needs_review`; contradictions create open registry incidences. Content
is stored by hash under `ingestion/objects/`, never as an untracked copy tree.

`verify` compares registered provenance with the current checkout and reports
current, changed, missing, and newly untracked evidence. Re-scan and review a
new plan before promoting a changed source.

For a reviewed compatible change to an existing entry, pass its plan
`entry_id` explicitly: `sddk knowledge import --plan <plan-id> --approve
<entry-id>`. Approval cannot promote unversioned, unowned, new ambiguous, or
contradictory sources.

`sddk rules check` resolves only the governed capability registry. It never
discovers arbitrary checkout files. If no complete trusted/current
catalog+baseline capability exists, the
architecture gate returns `not_applicable` with a traceable receipt; local
files alone never activate the gate.

## Node Types

| Type | Directory | Naming | Created by |
|------|-----------|--------|------------|
| `milestone` | `milestones/` | `M-NNN-{slug}.md` | Orchestrator |
| `active_lock` | `milestones/` | `_active.md` | Orchestrator + Release |
| `adr` | `adrs/` | `ADR-NNN-{slug}.md` | sddk-spec / sddk-design |
| `requirement` | `specs/{domain}/` | `REQ-{Slug}.md` | sddk-spec |
| `cycle` | `cycles/` | `CYC-{date}-{slug}.md` | sddk-archive |
| `incidence` | `incs/` | `INC-NNN-{slug}.md` | sddk-archive |
| `term` | `terms/` | `TERM-{Slug}.md` | sddk-explore / sddk-spec |

## Properties Convention (OKF + Obsidian)

All properties use **`snake_case`** (Obsidian Dataview compatibility). Property types:

| Type | YAML format | Example |
|------|-------------|---------|
| Text | `key: "value"` | `title: "Use JWT for Auth"` |
| Number | `key: 42` | `pr: 42` |
| Date | `key: 2026-08-03` | `created: 2026-08-03` |
| Checkbox | `key: true` | `verified: true` |
| List | `key: ["a", "b"]` | `affects_domains: ["[[auth]]"]` |
| Wikilink | `key: "[[node]]"` | `decision_authority: "[[ADR-003]]"` |
| Wikilink list | `key: ["[[n1]]", "[[n2]]"]` | `linked_adrs: ["[[ADR-003]]"]` |
| Null/empty | `key:` (no value) | `completed:` |

**Built-in properties** (Obsidian): `aliases` (list), `tags` (list).

## Read Rules

### Rule 1: Read from vault using `sddk knowledge path`

```bash
# CORRECT — resolve path first
VAULT=$(sddk knowledge path --root .)
cat "$VAULT/adrs/ADR-003-jwt-auth.md"

# Never reconstruct this path from the checkout directory name.
```

### Rule 2: Use grep for queries

```bash
VAULT=$(sddk knowledge path --root .)

# All accepted ADRs
grep -l "status: accepted" "$VAULT/adrs"/*.md

# All requirements in auth domain
ls "$VAULT/specs/auth/REQ-*.md"

# What ADRs affect auth?
grep -l "affects_domains:.*auth" "$VAULT/adrs"/*.md

# Is a cycle active?
cat "$VAULT/milestones/_active.md"
```

### Rule 3: Follow wikilinks for navigation

When you see `[[ADR-003-jwt-auth]]` in a node, open that file to continue the trace. Wikilinks are the graph edges.

## Write Rules

### Rule 1: Read the template first

Before creating any node, read the corresponding template from the SDDK framework:

```bash
# Templates are installed into the resolved vault by adoption.
VAULT=$(sddk knowledge path --root . --scope .)
cat "$VAULT/templates/{type}.md"
```

Fill in the placeholders. Do not invent properties not in the template.

### Rule 2: Create with complete properties

Every node MUST have at minimum: `type`, `title`, `slug`, `status`, `created`, `stale_after`. Domain-specific properties are MANDATORY (see templates).

### Rule 3: Use wikilinks for ALL cross-references

```yaml
# CORRECT
decision_authority: "[[ADR-003-jwt-auth]]"
affects_requirements: ["[[REQ-Session-Expiration]]", "[[REQ-Token-Refresh]]"]

# WRONG — plain text breaks the graph
decision_authority: "ADR-003"
affects_requirements: ["Session Expiration", "Token Refresh"]
```

### Rule 4: Changelog is append-only

Every node that evolves (ADR, Requirement, Milestone) has a `## Changelog` section. Add entries at the END, never edit existing ones:

```markdown
## Changelog (bi-temporal)

- 2026-08-03T10:00 | created | status=proposed | valid_from=2026-08-03 | valid_to=∞
- 2026-08-03T15:00 | status: proposed→accepted | cycle=[[CYC-007]] | valid_from=2026-08-03 | valid_to=∞
```

### Rule 5: Log to _log.md after every write

```bash
VAULT=$(sddk knowledge path --root .)
echo "- $(date -Iseconds) | {action} | {what} | [[{node}]]" >> "$VAULT/_log.md"
```

The `_log.md` file is append-only and lives in the vault (not in the project repo).

### Rule 6: Staleness

Every node has `stale_after`. When you update a node, push `stale_after` forward:
- Milestone: +90 days from update
- ADR: +365 days from update
- Requirement: +90 days from update
- Incidence: +90 days from discovery

## Serialization Authority

`milestones/_active.md` is an informational knowledge projection, not a lock.
Never acquire or release cycle authority by reading or overwriting it. Consume
the validated CLI cycle/lease state under
`skills/_shared/cli-usage-contract.md`. Because the current runtime cannot
discover or serialize distinct cycle IDs project-wide, cold-start automation
without a trusted cycle ID fails closed rather than treating this file as a
substitute lock.

## Vault Initialization

If `sddk knowledge status --root . --scope . --format json` reports that the
profile or vault is absent, run `sddk adopt apply --root . --scope .`. Do not
copy templates manually and do not create fallback state in the workspace.

## Compact Rules

- Resolve the vault with `sddk knowledge path --root . --scope .`
- Resolve the profile with `sddk knowledge status --root . --scope . --format json`
- Never derive `project_id` or vault paths from directory names
- Properties use `snake_case`; values use wikilinks `[[]]`
- Every node has `type`, `title`, `slug`, `status`, `created`, `stale_after`
- Changelog is append-only (bi-temporal)
- Log every write to `$VAULT/_log.md`
- `$VAULT/milestones/_active.md` is informational and never serialization authority
- The vault is separate from the adopted workspace
- Read templates from `$VAULT/templates/` before creating nodes
- Adoption creates XDG operational state and the vault, never workspace files
- Ingest repository evidence with scan → review → import → verify; never copy it manually
