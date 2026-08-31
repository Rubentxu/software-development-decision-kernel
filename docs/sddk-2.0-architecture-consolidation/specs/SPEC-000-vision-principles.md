# SPEC-000 — Vision, Positioning and Architectural Principles

**Status:** Proposed  
**Target:** SDDK 2.0 architecture-consolidation cycle

## 1. Problem statement

SDDK has grown into a capable local-first engineering runtime with workflow, capabilities, receipts, knowledge, release, UAT, agents and distribution. The risk is no longer lack of functionality; it is **semantic and structural sprawl**: large modules, overlapping sources of truth, a CLI that knows too much, adapters leaking into application logic and an expanding set of domains competing for the same core.

The architecture must preserve SDDK's strongest differentiator: **deterministic authority around probabilistic agents**.

## 2. Product positioning

SDDK SHOULD position itself as:

> A local-first operating system for evidence-governed agentic software engineering.

SDDK SHOULD sit below and around agent clients such as OpenCode, Claude Code, Codex or similar tools. It SHOULD NOT try to become another conversational coding assistant. Its value is persistent deterministic governance, evidence, replayability, portable packs and auditable software-engineering state.

## 3. Architectural principles

### P1 — Deterministic authority
Only deterministic kernel paths may authorize governed side effects. An agent response is never authority by itself.

### P2 — Ledger-first history
Operational truth is recorded as immutable domain events. Mutable databases, graph stores and HTML reports are projections or indexes.

### P3 — Graph as world model
The graph models what the system knows and how facts relate. It is a projection of the ledger and can be rebuilt.

### P4 — Workflow != world model
The workflow state machine models control flow and gates. Reactive graph behaviors model observations, patterns and proposed reactions. Neither replaces the other.

### P5 — Proposal before governed effect
Agents and graph behaviors may emit proposals. Policy and capability layers authorize and execute effects.

### P6 — Humans are actors, not UI exceptions
Human review, UAT, approvals and acceptance are domain events with identity, evidence and receipts.

### P7 — Small core, rich packs
The core vocabulary and runtime remain deliberately small. UAT, architecture, research, testing, documentation and Cognicode belong in packs/bounded contexts.

### P8 — Hexagonal boundaries are compile-time boundaries
Ports/adapters are not a diagram convention. Cargo dependencies and architecture lint MUST enforce them.

### P9 — Local-first and zero-intrusion
Operational state SHOULD live under XDG/user state paths and SHOULD NOT contaminate the target Git repository unless the user explicitly asks to write project artifacts.

### P10 — Replay before trust
Important automated decisions SHOULD be reproducible from events, content hashes, deterministic fixtures or explicitly recorded nondeterminism.

### P11 — Evidence over confidence theater
Claims, findings and verdicts SHOULD point to evidence. Confidence without provenance is insufficient for release authority.

### P12 — Deferred ideas need revisit triggers
Future ideas MUST have a reason for deferral and a concrete condition that reopens the decision.

## 4. Non-goals

SDDK 2.0 consolidation is not intended to:

- replace Git as source control;
- replace a production graph database;
- move all local validation into cloud CI;
- make graph behaviors capable of arbitrary direct side effects;
- invent a universal ontology for all software-engineering concepts;
- add a new top-level product domain during the consolidation cycle.

## 5. Success criteria

The consolidation succeeds when:

1. `sddk-app`/application ports separate use cases from concrete persistence.
2. `engine -> storage` direct coupling is removed.
3. CLI is primarily composition and presentation, not business logic.
4. the event envelope is versioned and used by new cross-domain flows.
5. UAT is extracted behind a bounded-context/pack boundary without losing behavior.
6. pack dependencies are explicit and validated.
7. knowledge graph can be rebuilt from ledger events.
8. graph behaviors cannot bypass capability policy.
9. signed local gate receipts can be independently verified.
10. architecture and complexity regressions are measurable.
