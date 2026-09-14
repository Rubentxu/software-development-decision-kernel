# Implementation Prompt — Architecture Conformance Track

Use this only after the roadmap prerequisite for the target AC milestone is green.

## Agent instruction

Implement the Architecture Conformance track without creating a competing architecture subsystem.

Read:

1. repository `AGENTS.md`;
2. production-readiness mini-roadmap;
3. this package README + `09-ROADMAP.md`;
4. arch-spec-032..041;
5. ADR-0112..0116;
6. relevant Knowledge/Verification/Alignment specs.

Rules:

- use the single SemanticGraphProjection;
- use Decision Memory/revision substrate for accepted contracts;
- Evidence status must distinguish OBSERVED/INFERRED/VERIFIED/UNKNOWN;
- missing provider/evidence is never PASS;
- Alignment remains advisory;
- Governance consumes only explicit policy/contracts, never a raw Alignment opinion;
- provider/host SDK types terminate at adapters;
- no universal quality score;
- paradigms are selectable lenses;
- prefer ADTs and typed IRs for closed domain states;
- DSL syntax compiles to canonical typed models; syntax does not own execution;
- Base must function without CogniCode/Chronos/LLM;
- advanced counterfactual/proof-carrying features must not delay AC1..AC8.

For every slice:

```text
contract → graph/evidence basis → minimal implementation → deterministic probe → negative fixture → receipt/crosswalk
```

Before marking PASS attempt to falsify the implementation with a negative or mutation fixture when the contract is critical.

Stop at each AC milestone with a named receipt and exact commit. Do not silently promote speculative findings to facts.
