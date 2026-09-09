---
id: arch-spec-012-configuration
package_local_id: SPEC-012
package_source: docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-012-CONFIGURATION.md
status: proposed
supersedes_history: false
adopted_at: 2026-09-09
adoption_cycle: p-63676b11dc0ef88f/architecture-adoption-m0-supersession
---

# arch-spec-012 — CONFIGURATION

> **Mirror of package SPEC `SPEC-012`.** Repository-native identifier is `arch-spec-012-configuration` (range arch-spec-001..018). Per SUPERSESSION.md, the packages original SPEC ID is preserved in `package_local_id` and the canonical content is copied verbatim into this file.

## Crosswalk

| Surface | Identifier |
|---|---|
| Package local | `SPEC-012` |
| Repository native | `arch-spec-012-configuration` |
| Source | `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-012-CONFIGURATION.md` |
| Adoption cycle | `p-63676b11dc0ef88f/architecture-adoption-m0-supersession` |

---

# SPEC-012 — Configuration, conventions and lock semantics

## Project file

Recommended minimal `sddk.toml`:

```toml
[project]
kind = "rust-service"

[workflow]
default_target = "change"

[packs]
use = ["sdd", "uat"]

[memory]
hot_budget = 2000
context_budget = 16000

[policy]
profile = "team-default"
```

## Requirements

- deterministic precedence;
- `sddk config explain <key>` returns source chain;
- unknown keys fail or warn according to schema/version, never silently drift;
- secrets use external secret providers/environment, not lock/config files;
- `sddk.lock` optionally pins pack/workflow/policy/schema contract versions;
- scoped configuration inherits from project and can only override declared keys.


## Agent configuration

Configuration may select provider/model routing, enable/disable Skills, choose AgentProfile defaults and add project instruction sources. Resolved configuration is immutable/hashable for a Run/execution and referenced by AgentExecutionReceipt.

Secrets never enter instruction/cheat-sheet material. Provider credentials remain in secret handling boundaries.
