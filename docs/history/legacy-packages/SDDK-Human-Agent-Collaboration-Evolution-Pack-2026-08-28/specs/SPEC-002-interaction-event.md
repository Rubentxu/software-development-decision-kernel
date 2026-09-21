# SPEC-002 — InteractionEvent

## Event types
`cycle.started`, `cycle.resumed`, `phase.started`, `phase.progress`,
`finding.noteworthy`, `decision.required`, `decision.made`,
`assumption.invalidated`, `plan.reframed`, `phase.blocked`,
`phase.completed`, `cycle.completed`.

## Required fields
schema_version, event_id, timestamp, cycle_id, type, phase, attention, facts, evidence_refs.

## Rules
- event_id unique;
- facts structured;
- prose optional;
- evidence refs immutable where possible;
- attention is domain classification, not renderer whim.

## Acceptance
Schema validation + reconstruction tests + duplicate id rejection.
