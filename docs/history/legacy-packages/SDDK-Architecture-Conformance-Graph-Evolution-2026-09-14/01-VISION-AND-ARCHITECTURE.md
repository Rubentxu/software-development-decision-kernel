# Vision and Architecture Fit

## Problem

Today SDDK can preserve decisions, events, evidence and projections, but architecture conformance still requires a human/agent to manually connect:

`ADR/spec → concept → implementation → call path → test → UAT → receipt`.

The A0/A1 closeout showed that this method finds defects that compilation and green test suites miss: duplicate authorities, overly coarse policy semantics, stale compatibility, projection/authority confusion and missing negative evidence.

## Goal

Make this reasoning a first-class SDDK workflow while keeping the existing architecture clean.

```text
ProjectIntent + DecisionMemory + Specs
                │
                ▼
       ArchitecturalContracts
                │
        ┌───────┴────────┐
        ▼                ▼
Knowledge/KMT      SemanticGraph overlay
        │                │
        └───────┬────────┘
                ▼
        VerificationPlan
                │
      deterministic probes
       optional providers
                │
                ▼
        ClaimAssessments
                │
        ┌───────┴────────┐
        ▼                ▼
Alignment          ConformanceReceipt
(advisory)               │
        │                 ▼
        ▼             Workbooks
ContextCapsule
advisory_context
```

## State-class discipline

Every artifact SHALL remain one of the four accepted state classes:

- **FACT** — accepted event/fact that happened;
- **OBJECT** — versioned semantic object/decision/contract;
- **PROJECTION** — rebuildable graph, workbook, assessment view;
- **EPHEMERAL** — transient verification plan, candidate, local context.

`ArchitectureGraph`, authority maps, ownership maps and workbooks are PROJECTIONs. They are never canonical authority.

## Base vs enhanced

Base mode SHALL be useful with no external provider:

- repository structure and manifest analysis;
- dependency rules;
- canonical SDDK events/decisions/specs;
- deterministic fitness tests;
- declared contracts and negative fixtures.

CogniCode adds static code intelligence. Chronos adds runtime intelligence. Provider absence produces `NOT_EVALUATED`/`EvidenceGap`, never synthetic PASS.

## Non-goals

- no universal architecture score;
- no universal ontology of software;
- no LLM as mandatory judge;
- no replacement of compiler/type system/tests;
- no second semantic graph;
- no automatic enforcement from Alignment;
- no requirement that all code follow OO or FP.

## Lateral opportunity

Treat architecture conformance as a continuously evolving **immune system**:

```text
finding → accepted decision → verified repair → fitness ratchet → future regression detected early
```

The value is not only finding debt once; it is making solved architectural failures difficult to reintroduce.
