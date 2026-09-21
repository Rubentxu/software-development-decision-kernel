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
