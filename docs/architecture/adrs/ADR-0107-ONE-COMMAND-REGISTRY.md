---
id: ADR-0107-ONE-COMMAND-REGISTRY
package_local_id: ADR-014
package_source: docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/ADR-014-ONE-COMMAND-REGISTRY.md
status: accepted
supersedes_history: false
adopted_at: 2026-09-09
adoption_cycle: p-63676b11dc0ef88f/architecture-adoption-m0-supersession
accepted_at: 2026-09-12
accepted_by_cycle: "p-63676b11dc0ef88f/adr-promotion-batch-2"

implementation_evidence:
  - "crates/sddk-cli/src/command_spec.rs:254 — pub struct CommandSpec"
  - "AX-S1 drift guard test (clap_surface_and_command_specs_are_in_sync)"


superseded_by: []
related_adrs:
  - "ADR-0001-ADR-PROMOTION-PROCESS"
  - "ADR-0096-SDLC-LIFECYCLE-SEMANTICS"
stale_after: 2027-09-12

---

# ADR-0107 — ONE-COMMAND-REGISTRY

> **Mirror of package ADR `ADR-014`.** Repository-native numbering is `ADR-0107-ONE-COMMAND-REGISTRY` (range ADR-0094..0110). Per SUPERSESSION.md, the package's original ADR ID is preserved in `package_local_id` and the canonical content is copied verbatim into this file.

## Crosswalk

| Surface | Identifier |
|---|---|
| Package local | `ADR-014` |
| Repository native | `ADR-0107-ONE-COMMAND-REGISTRY` |
| Source | `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/ADR-014-ONE-COMMAND-REGISTRY.md` |
| Adoption cycle | `p-63676b11dc0ef88f/architecture-adoption-m0-supersession` |

---

# ADR-014 — One Command Registry generates human and agent CLI contracts

**Status:** Proposed

## Decision

CLI syntax/purpose/options/output/effect metadata/examples are defined once in a typed `CommandRegistry` substrate. The following are generated views:

- human help;
- shell completion metadata;
- machine-readable command schema;
- documentation tables;
- contextual AgentCommandSurface / cheat sheets.

No manually maintained agent CLI cheat sheet is authoritative.

## Rationale

Agents need reliable syntax and examples but should not spend routine calls probing `--help`. A single registry removes documentation drift while preserving progressive disclosure.

## Required metadata

Each agent-visible command declares command id, syntax, purpose, arguments/options, output contracts, side-effect class, required authority class, preconditions, related targets/tasks, stability and tested examples.

## Consequences

Examples become executable contract tests. A CLI breaking change must fail registry compatibility/golden tests before stale prompts reach agents.
