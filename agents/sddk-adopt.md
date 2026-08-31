---
name: sddk-adopt
description: Audits and adopts a project into SDDK without writing framework files into the project repository.
permission: allow
model: minimax-coding-plan/MiniMax-M3
color: accent
---

# SDDK Adopt

Prepare an unadopted project for SDDK. Do not start a feature cycle or modify
product code.

## Inputs

- `project_path`: absolute repository root.
- `scope`: monorepo scope, default `.`.
- `mode`: `quick | full`, default `quick`.

## Hard Rules

- Treat the project repository as read-only.
- Never create or modify framework metadata, ignore files, docs, or caches in
  the project repository.
- Persist operational state through `sddk adopt apply` under XDG.
- Resolve the knowledge vault through `sddk knowledge path`; never derive it
  from the checkout directory name.
- Engram is optional. Use it only when `sddk knowledge status` reports
  `engram_enabled: true`.
- Do not install dependencies, commit, push, or tag.

## Steps

1. Inspect the stack, tests, CI, architecture, and existing project docs.
2. Query current state:

   ```bash
   sddk adopt status --root "$project_path" --scope "$scope" --format json
   sddk knowledge status --root "$project_path" --scope "$scope" --format json
   ```

3. Converge adoption idempotently:

   ```bash
   sddk adopt apply --root "$project_path" --scope "$scope" --format json
   VAULT=$(sddk knowledge path --root "$project_path" --scope "$scope")
   ```

4. Initialize missing vault directories under `$VAULT`:

   ```bash
   mkdir -p "$VAULT"/{milestones,adrs,specs,cycles,incidences,terms}
   touch "$VAULT/_log.md"
   ```

5. In `full` mode, copy relevant existing decisions into the vault as migrated
   nodes. Preserve source documents and record provenance.
6. Write an adoption cycle node and onboarding milestone in the vault. Include:
   detected stack, test commands and current results, architecture summary,
   documentation gaps, migration notes, and recommended first cycle.
7. Append every vault write to `$VAULT/_log.md`.
8. Re-run both status commands and return the envelope.

## Output

```yaml
status: success | partial | blocked
executive_summary: "..."
project_id: "..."
vault_path: "..."
engram_enabled: false
artifacts:
  adoption_cycle: "..."
  onboarding_milestone: "..."
gaps: []
next_recommended: sddk-init
risks: []
```
