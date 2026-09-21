# SDDK Architecture Conformance + Graph Intelligence Evolution

**Status:** `READY_FOR_IMPLEMENTATION` once merged/adopted.  
**Date:** 2026-09-14  
**Integrates with:** A2→A8, J5, CogniCode and Chronos.  
**Does not replace:** Semantic Core, Knowledge, Verification, Software Alignment, Governance or Agent Experience.

## Purpose

Turn the architecture-conformance method already used to close A0/A1 into a native SDDK capability:

```text
accepted intent / ADR / spec
        ↓
typed ArchitecturalContract
        ↓
Knowledge + SemanticGraph projection
        ↓
Verify / DebVerify
        ↓
Evidence + ConformanceReceipt
        ↓
Software Alignment interpretation
        ↓
Workbooks / Agent advisory context
        ↓
Governance only when an explicit policy consumes the result
```

The capability SHALL answer with evidence, not opinions:

- what the software **is**;
- what accepted decisions say it **should be**;
- which claims are verified, contradicted, stale or unknown;
- where authority, ownership, compatibility or dependency drift exists;
- why a finding exists and which evidence supports it;
- which known regression should become a fitness ratchet.

## Architectural fit

No new `architecture_audit` bounded context is introduced. Responsibilities remain:

- **Decision**: accepted intent and decisions;
- **Knowledge**: observed/declared/inferred architectural knowledge;
- **SemanticGraph**: rebuildable graph projection and cross-tree overlay;
- **Verification**: claims, probes, Verify/DebVerify and receipts;
- **Software Alignment**: paradigm/lens-aware interpretation and suggestions;
- **Governance**: explicit gates only; never Alignment opinion as authority;
- **Agent Experience**: context deltas and human/agent UX;
- **Extension**: CogniCode/Chronos adapters and optional analyzers.

## Documents

1. `01-VISION-AND-ARCHITECTURE.md`
2. `02-ARCHITECTURE-GRAPH-MODEL.md`
3. `03-ARCHITECTURAL-CONTRACTS-AND-CLAIMS.md`
4. `04-PARADIGM-LENSES.md`
5. `05-ADT-FP-OO-DSL-MODELING.md`
6. `06-REACTIVE-CONFORMANCE-LOOP.md`
7. `07-MUTATION-COUNTERFACTUAL-PROOF-CARRYING.md`
8. `08-ACTIVEGRAPH-DESIGN-TRANSFER.md`
9. `09-ROADMAP.md`
10. `10-UAT.md`
11. `11-FITNESS-RECEIPTS.md`
12. `12-CLI-AGENT-UX.md`
13. `13-PROPOSAL-DISPOSITION.md`
14. `14-IMPLEMENTATION-PROMPT.md`

## Core principle

**Architecture is a set of typed, evidence-backed claims over a changing software graph — not a score and not an LLM opinion.**
