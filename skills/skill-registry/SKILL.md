---
name: skill-registry
description: "Trigger: update skills, skill registry, actualizar skills, after skill changes. Index available skills by trigger and path."
license: MIT
metadata:
  author: gentleman-programming
  version: "1.0"
---

## Activation Contract

Use this skill after installing, removing, creating, moving, or renaming skills, or when a delegator needs a fresh skill index.

## Hard Rules

- The registry is an index, not a compiler or summary. `SKILL.md` remains the source of truth.
- Do not generate or inject compact rules by default; preserve author intent by passing exact skill paths to subagents.
- Write the cache to `$SDDK_DATA_DIR/projects/{project_id}/skill-registry.md`.
- Save the registry to Engram as `topic_key: skill-registry` only when `sddk
  knowledge status` enables Engram, with `capture_prompt: false`.
- Skip `_shared` and `skill-registry`; deduplicate by skill name, preferring project-level skills over user-level skills.
- Never create or modify files in an adopted workspace.

## Decision Gates

| Situation | Action |
| --- | --- |
| Same skill exists globally and in project | Keep the project-level skill |
| Same skill exists in multiple global locations | Keep the first source in scan order |
| No skills found | Write an empty XDG registry so agents stop searching blindly |
| Agent will delegate work | Select matching registry rows and pass their `SKILL.md` paths |

## Execution Steps

1. Scan all known user and project skill directories for `*/SKILL.md`.
2. Read frontmatter only as needed to extract `name` and `description` trigger text.
3. Render `$SDDK_DATA_DIR/projects/{project_id}/skill-registry.md` with scanned
   sources, registry contract, skill name, trigger/description, scope, and
   exact path.
4. Mirror to Engram only when the resolved profile enables it.
5. Return the registry path, skill count, cache status, and whether Engram was updated.

## Output Contract

Return:
- Project ID and XDG registry path.
- Number of indexed skills.
- Whether the cache was hit or regenerated.
- Any skipped or duplicate skills when relevant.

## References

- `docs/skill-style-guide.md` — how skills should be authored before indexing.
- `skills/_shared/skill-resolver.md` — how delegators use the index.
