---
id: arch-spec-015-command-registry-and-agent-surface
package_local_id: SPEC-015
package_source: docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-015-COMMAND-REGISTRY-AND-AGENT-SURFACE.md
status: proposed
supersedes_history: false
adopted_at: 2026-09-09
adoption_cycle: p-63676b11dc0ef88f/architecture-adoption-m0-supersession
---

# arch-spec-015 — COMMAND-REGISTRY-AND-AGENT-SURFACE

> **Mirror of package SPEC `SPEC-015`.** Repository-native identifier is `arch-spec-015-command-registry-and-agent-surface` (range arch-spec-001..018). Per SUPERSESSION.md, the packages original SPEC ID is preserved in `package_local_id` and the canonical content is copied verbatim into this file.

## Crosswalk

| Surface | Identifier |
|---|---|
| Package local | `SPEC-015` |
| Repository native | `arch-spec-015-command-registry-and-agent-surface` |
| Source | `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-015-COMMAND-REGISTRY-AND-AGENT-SURFACE.md` |
| Adoption cycle | `p-63676b11dc0ef88f/architecture-adoption-m0-supersession` |

---

# SPEC-015 — Command Registry and contextual AgentCommandSurface

## Command Registry

One typed registry owns the public CLI contract. A `CommandSpec` includes:

```yaml
id: run.verify
syntax: "sddk run verify [options]"
purpose: "Execute the verification target"
stability: stable
side_effect_class: governed
required_authority: execute
outputs:
  human: text
  machine: RunReceiptV1
examples: [...]
```

Arguments/options and machine-output schemas are structured fields, not parsed from help prose.

## AgentCommandSurface

Derived from:

```text
CommandRegistry
+ target/task
+ AgentProfile relevance
+ enabled Packs
+ platform/config availability
```

It intentionally does not duplicate AuthorityEngine decisions.

Each entry contains:

- exact syntax and parameter contract;
- purpose and when-to-use guidance;
- side-effect class;
- authority requirement (descriptive);
- expected machine output;
- preconditions;
- 1–3 executable examples;
- related commands;
- stability/deprecation state.

## Cheat sheet forms

Required renderers:

```text
Markdown/text   human/LLM compact guide
JSON            provider/tool integration
```

Recommended porcelain:

```text
sddk help agent
sddk help agent --target verify
sddk help agent --target verify --format json
```

Provider adapters normally inject the relevant surface directly. Calling help is recovery/diagnostic behavior, not routine discovery.

## Example integrity

Every published example must be represented as structured example data and covered by parser/fixture tests. Examples that mutate external state run in sandbox fixtures or dry-run mode.
