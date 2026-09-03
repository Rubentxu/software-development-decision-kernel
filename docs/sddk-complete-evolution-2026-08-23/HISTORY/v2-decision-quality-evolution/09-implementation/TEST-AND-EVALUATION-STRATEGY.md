# Test & Evaluation Strategy

## Engineering Assurance

### Unit
Fingerprints, evidence freshness, obligation satisfaction, verdict matrix, profile resolution.

### Contract
Pack manifest, event schemas, capability resolution, profile schema.

### Negative
High finding without evidence; stale required evidence; Rust profile leaking into kernel; review attempting mutation.

### Replay
Same events produce same assessment projection/verdict.

## Governed Continuous Improvement

### Configuration identity
Reject causal comparison if required config refs are missing.

### Holdout isolation
Optimizer capabilities cannot read holdout paths/artifacts.

### Candidate immutability
Candidate content hash cannot change after evaluation begins.

### Fork isolation
Candidate mutation cannot contaminate baseline workspace.

### Promotion
No promotion without evaluation contract, candidate evaluation, policy decision and promotion receipt.

### Replay
Replaying promotion events does not re-apply side effects.

## Evaluator quality

Do not trust one LLM judge as sole evaluator.

Priority:

```text
deterministic oracle
→ human gold/reference
→ independent structured evaluator
→ LLM judge as supplementary signal
```

Where LLM judges matter, calibrate them periodically against human/reference cases.

## Candidate protocol

1. freeze base revision;
2. freeze evaluation contract;
3. record all harness/config refs;
4. apply candidate in isolation;
5. run development evaluation;
6. nominate candidate;
7. run hidden holdout;
8. compute hard gates;
9. compute Pareto comparison;
10. issue eligibility/rejection;
11. optional shadow rollout;
12. monitor and promote/revert.

## Search-strategy evaluation

GEPA/MCTS/population search is itself an experiment. Compare against a simple baseline such as one reflection-generated candidate. Retain complex optimizer only if quality gain justifies added cost/complexity.

## Golden task coverage

Use capability/change classes from SPEC-024: architecture, security, testing, implementation, UAT, incident, refactor, migration, plus SDDK dogfood tasks for workflow structure, skills, routing and context strategies.

## Metrics

Quality first: acceptance/invariant success, regression/security, evidence completeness, human corrections.

Efficiency second: first pass, retries, tokens/cost/latency, calls/handoffs, context read/reuse, convergence rounds.

For stochastic evaluations, define minimum sample counts and uncertainty reporting. Never promote a small apparent gain from one lucky run.
