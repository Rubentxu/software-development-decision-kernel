# SPEC-028 — Reactive Behaviors

**Status:** Proposed — refined for dynamic workflows

## Purpose
Use events as the nervous system while avoiding needless LLM prompts.

## Reaction levels
- **L0 OBSERVE** — persist/project only.
- **L1 DETERMINISTIC** — runtime behavior reacts without LLM.
- **L2 COGNITIVE** — compile typed signal and invoke Supervisor/critic/human.

## Behavior families

### Runtime
retry, timeout, resume, node-ready, join, loop guard, no-progress.

### Dynamic workflow
- MapExpansionBehavior;
- ExpansionValidationBehavior;
- GraphRevisionBehavior;
- ConvergenceBehavior;
- WorkUnitDeduplicationBehavior.

### Provider/routing/context/governance
Keep the previous families: quota/health/circuit, routing, staleness/capsules, policy/approval.

## Contract

```rust
trait Behavior {
    fn subscriptions(&self) -> &[EventPattern];
    fn evaluate(&self, event: &EventEnvelope, view: &BehaviorView) -> BehaviorDecision;
}
```

Decisions: Ignore, Emit, Issue(command), CreateSignal.

## Expansion rule
Behaviors MAY create `ExpansionProposal` or graph commands, but graph mutation occurs only in Workflow Runtime after validation and event append.

## Loop safety
Invocation idempotency `(behavior_id, trigger_event_id, version)`, reaction-depth budget, graph-size budget and repeated-state/no-progress detection are mandatory.

## Prompt injection policy
Never concatenate arbitrary event messages into Supervisor prompts. Typed signal → Context Compiler → trusted rendering.
