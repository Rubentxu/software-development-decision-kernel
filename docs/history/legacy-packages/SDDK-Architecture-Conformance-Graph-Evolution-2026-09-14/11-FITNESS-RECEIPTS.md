# Fitness, Receipts and Conformance Vectors

## Fitness classes

### Dependency fitness

Examples:

```text
no domain → provider SDK
no domain → host SDK
no Knowledge → provider SDK
no Alignment → Governance implementation
no Alignment → InstructionCompiler
no Workbook → canonical write
no host-specific types in generic Agentic API
```

### Authority fitness

```text
one CanonicalEventLog append authority
one side-effect admission authority
one SemanticGraphProjection authority as projection definition
no projection writes canonical truth
```

### Compatibility fitness

Every compatibility path has:

- owner;
- consumer;
- replacement;
- read/write status;
- removal condition;
- milestone/review trigger;
- parity/negative fixture.

### Mutation fitness

Critical rules SHOULD prove they fail after a deliberate sandbox mutation.

## Conformance vector

Do not compress architecture into a meaningless score.

```text
ArchitectureConformanceVector {
  authority: VERIFIED,
  ownership: VERIFIED,
  dependencies: VERIFIED,
  compatibility: PARTIAL,
  negative_paths: VERIFIED,
  paradigm_alignment: UNKNOWN,
  runtime_evidence: NOT_EVALUATED,
}
```

A user may define policy over vector dimensions, but the kernel does not invent a universal weighted score.

## Receipt

`ArchitectureConformanceReceipt` SHOULD include:

```text
receipt_id
project/revision basis
contract_set_digest
knowledge_basis
semantic_graph_digest
verification_plan_digest
claim_results[]
evidence_refs[]
provider_basis[]
unknowns[]
contradictions[]
compatibility_delta
fitness_results[]
mutation_results[]
created_at
```

## Proof levels

Suggested evidence strength labels:

1. `DECLARED` — intent only;
2. `OBSERVED` — deterministic observation;
3. `VERIFIED` — observation satisfies/falsifies a contract through a reproducible probe;
4. `MUTATION_VERIFIED` — guard also proved against injected violation;
5. `RUNTIME_CORROBORATED` — runtime evidence corroborates static claim.

These are not quality ranks; they describe evidence basis.

## Release gate

A5 may require the Base architecture receipt with all mandatory contracts `VERIFIED` or explicitly waived by a governed, owner/expiry-bearing Decision. Optional provider-dependent contracts may remain `NOT_EVALUATED` in Base.
