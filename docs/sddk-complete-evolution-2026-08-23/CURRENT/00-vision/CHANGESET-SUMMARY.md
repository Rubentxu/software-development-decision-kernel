# Changeset Summary — Consolidated Decision Quality Evolution

## Previous retained pillars

### Engineering Assurance
Retained and refined.

### Governed Continuous Improvement
Retained and refined.

## New third pillar

### Agent-First Deterministic Interface

Problem:

```text
LLM currently learns/programs too much low-level CLI procedure.
```

Decision:

```text
agent says WHAT outcome is needed;
Rust resolves HOW to reach it deterministically.
```

New documents:

- ADR-045
- SPEC-047
- SPEC-048
- SPEC-049
- Agent-First architecture
- migration plan
- tool-use process-mining plan

## Critical refinement from compatibility review

A simpler call surface cannot reduce observable workflow value.

The following remain mandatory when applicable:

```text
verify report
debt report
engineering assurance
gate receipts
capability receipts
release report/receipt
archive report/manifest
metrics/analytics events
knowledge updates
human approval records
postcondition proofs
```

The new surface aggregates and indexes these artifacts rather than replacing them.

## Imported external patterns

### Netstack3
- hide understood complexity behind explicit contracts;
- keep important core paths traceable;
- validate at boundaries and preserve guarantees;
- use types/invariants/evidence proportionally.

### Maven
- ask for lifecycle outcome rather than manually listing all prior mechanical steps.

### Gradle
- dependency graph;
- declared inputs/outputs;
- work avoidance / up-to-date semantics.

### Controller/reconciliation systems
- actual state vs desired state;
- converge through verified idempotent operations.

### Agent/tool research
- small semantically distinct tool surface;
- machine-readable tool contracts;
- tool examples;
- measure redundant/failed tool trajectories;
- optional future schema-graph planning.

## Explicitly not adopted

- generic autoresearch product;
- scientific-theory ontology;
- unbounded self-modification;
- one mega-command that hides all intent;
- removing low-level expert controls;
- dropping detailed reporting for convenience.
