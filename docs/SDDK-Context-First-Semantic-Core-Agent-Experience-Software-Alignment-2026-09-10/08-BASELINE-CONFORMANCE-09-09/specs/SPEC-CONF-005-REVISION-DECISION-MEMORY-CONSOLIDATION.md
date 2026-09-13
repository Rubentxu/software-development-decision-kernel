# SPEC-CONF-005 — Common Revision Substrate / Decision Memory Consolidation

## Baseline contract

Closes ADR-004, SPEC-003, SPEC-004 and M4/M9 requirements that Decision Memory does not invent a second generic history system.

## Problem

The baseline introduced common `Revision<T>`, object identity, refs and CAS semantics. The audited Decision Memory still carries Git-like primitives such as blobs/trees/commits/parents/refs/reflog. Some may be legitimate domain semantics; generic revision mechanics must not be duplicated.

## Decision rule

For every Decision Memory primitive classify it as:

- **SHARED PRIMITIVE** — must reuse common Revision/Object/Ref/CAS;
- **DECISION SEMANTIC** — may remain local because it represents decision-specific meaning;
- **COMPATIBILITY FORMAT** — decode/translate only;
- **REDUNDANT** — remove.

No ambiguous fifth category is allowed.

## Required end-state

Decision Memory owns decisions, assumptions, dissent, semantic diff, branch naming conventions and decision-specific audit semantics. The common revision substrate owns generic content identity, parentage, refs/CAS and shared revision mechanics.

## Requirements

- deterministic OID rules have one implementation;
- ref compare-and-swap has one implementation;
- common parent/ancestry helpers are shared unless Decision Memory proves distinct semantics;
- Decision Memory semantic diff remains domain-specific;
- reflog may remain Decision-owned only if it adds decision audit semantics beyond generic ref history;
- `ForkRecord` keeps domain semantics while reusing shared identities/revision primitives where applicable;
- persisted legacy formats have deterministic adapters and fresh-process recovery.

## ADR escape hatch

If consolidation would make the shared substrate Git-specific or leak Decision semantics into shared code, retain the specialized primitive and update SPEC-003 with a narrow ADR that explains the semantic distinction. “Avoid refactor” is not a valid justification.

## Acceptance

- deterministic OID fixture passes across processes;
- CAS race fixture has exactly one winner;
- Decision Memory recovery works from canonical project state;
- branch/diverge/merge-base and semantic diff UAT pass;
- duplicate algorithm/static scan has no unexplained generic revision implementation.
