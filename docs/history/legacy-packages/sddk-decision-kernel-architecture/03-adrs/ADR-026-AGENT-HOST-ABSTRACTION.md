# ADR-026-AGENT-HOST-ABSTRACTION — Abstract agentic IDEs behind AgentHost event/control ports

**Status:** Accepted


## Context
OpenCode, Codex, Claude Code and future agentic hosts expose different APIs, events and control semantics.

## Decision
Define a bidirectional abstraction:
- host events -> canonical SDDK events;
- SDDK commands/context -> host controls.

Adapters advertise capabilities rather than pretending every host supports the same operations.

## Consequences
OpenCode can be the first adapter without making its API part of the domain model. Unsupported operations can degrade to restart/resume rather than fake hot-switch semantics.
