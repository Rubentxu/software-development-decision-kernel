# Reactive Architecture Conformance Loop

## Goal

Use events and graph state to make architecture verification incremental without replacing SDDK's explicit planning/execution lifecycle.

## Flow

```text
WorkspaceChangeSet / canonical event / provider observation
        ↓
KMT invalidation / freshness
        ↓
AffectedSoftwareUnits
        ↓
AffectedArchitecturalContracts
        ↓
VerifyPreflight
        ↓
minimal probes
        ↓
ClaimAssessment + Evidence
        ↓
SemanticGraph projection update
        ↓
Alignment delta
        ↓
useful ContextDelta / Workbook refresh
```

## Materiality

Not every file read, grep, generated timestamp or host event belongs in the Canonical Event Log.

Classify input as:

- **EPHEMERAL** — local observation/telemetry, no semantic change;
- **MATERIAL** — may alter knowledge/contracts/verification basis;
- **AUTHORITATIVE FACT** — accepted event through canonical append path.

## Behavior-local context

A verifier should request only the graph slice/evidence required by its contracts, not receive a global context blob. This reduces stale context and makes provenance tractable.

Example:

```text
DependencyBoundaryVerifier
requires:
  contract ids
  affected modules
  dependency edges
  declared ownership
```

not the whole repository narrative.

## Reactive behaviors vs orchestrated workflows

Use reactive behavior when:

- input arrival is the natural trigger;
- the transformation is bounded/idempotent;
- ordering can be defined through event horizon/basis;
- failure can be retried/replayed safely.

Use explicit Workflow/Run when:

- human-visible lifecycle matters;
- multiple steps/gates form a business transaction;
- authority/approval boundaries need explicit orchestration;
- long-running coordination has its own state.

## JCode integration

At J5:

```text
turn_done
  → coalesced material changes
  → affected contracts
  → Verify
  → ArchitectureConformanceDelta
  → ContextDelta(no_reply)
```

Normal tensions are `CONTEXTUAL`; only explicit Governance policy can escalate to blocking behavior.

## Providers

CogniCode and Chronos can emit observations that trigger re-evaluation through SDDK-owned ports. They do not directly mutate Knowledge truth or Alignment status.
