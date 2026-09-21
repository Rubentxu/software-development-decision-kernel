# ActiveGraph Design Transfer

## Reference basis

Design inspiration was reviewed against `yoheinakajima/activegraph-packs` main at commit:

`6639a5385518ad49f74813373c85cf96eff9adc0`

Relevant upstream concepts include an event-sourced object graph, typed objects/relations, reactive behaviors, a deliberately small core vocabulary, layered packs, optional `integrates_with` dependencies and behavior-local context.

This is inspiration, not a runtime dependency and not a wholesale architecture transplant.

## Adopt

### Event-first projection updates

Meaningful conformance facts should be reproducible from accepted events/evidence/bases. Graph state is rebuildable.

### Small shared vocabulary

Avoid turning SDDK shared kernel or SemanticGraph into a universal software ontology. Universal relations must earn their place; paradigm/domain-specific vocabulary belongs in layered schemas/lenses.

### Layered capabilities

CogniCode/Chronos and future analyzers should resemble optional integrations:

```text
requires      = only true hard dependency
integrates_with = optional enrichment
```

Absence degrades to `NOT_EVALUATED`, not failure unless the project policy explicitly requires that provider.

### Behavior-local context

Reactive verifiers declare the graph/evidence slice they need. Do not ship a whole global context blob into every behavior/LLM.

### Behavior maps

Generate graph/event behavior maps as developer UX and conformance evidence.

## Adapt, do not copy

ActiveGraph demonstrates coordination through reactive graph state without a central orchestrator. SDDK SHALL NOT remove its explicit Goal/WorkItem/Workflow/Run orchestration because those semantics are already deliberate product concepts.

Use reactive coordination for:

- knowledge refresh;
- graph projections;
- incremental verification;
- provider observations;
- context deltas;
- workbook refresh.

Use explicit Workflow/Run for delivery lifecycle and governed multi-step work.

## Reject

- graph as a second authority;
- domain packs directly deciding Governance;
- implicit behavior chains where a user-visible lifecycle/gate must be explicit;
- a universal ontology that absorbs all language/paradigm concepts.

## Lateral synthesis

SDDK can combine ActiveGraph-like reactive composition with its stronger typed decision/governance model:

```text
reactive evidence acquisition
        +
explicit typed decisions/contracts
        +
AuthorityEngine for effects
        +
receipts/provenance
```

That combination is more valuable than copying either model literally.
