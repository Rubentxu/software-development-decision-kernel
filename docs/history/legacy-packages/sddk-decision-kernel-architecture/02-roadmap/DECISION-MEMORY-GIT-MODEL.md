# SDDK Decision Memory — Git-like Model

> **Status:** canonical design context for the CDD work items in H4/H6.
> **Detailed research:** `docs/evolutivo-continuidad-sesiones-delegacion-deliberacion.md`.
> **Not a source of execution order:** use `EXECUTION-SPINE.yaml`.

## 1. Purpose

Decision Memory gives SDDK a durable, traversable history of **why** work evolved the way it did.

It is intentionally Git-like:

```text
execution/event truth  ────────────────┐
planning/runtime/decision truth ───────┤
agent contributions + artifacts ──────┤
accepted knowledge ────────────────────┘
                     ↓
             Decision Memory DAG
                     ↓
       context-specific tree projections
```

The DAG is a projection/index over canonical evidence, not a competing source of runtime truth.

## 2. Mental model

```text
                             refs/heads/what-if/cache-B
                            o M7
                           /   \
refs/heads/canonical → M1--M2--M3--M6--M9 ← HEAD
                            \      /   \
                             M4--M5     M8
                         option-B      secretary/recovery
```

A new session does not ask “what did chat remember?”. It resolves `HEAD`, walks the relevant memory tree and verifies staleness against current canonical revisions.

## 3. Core objects

### DecisionMemoryCommit

Immutable content-addressed node:

```yaml
id: sha256(canonical_payload)
parents: [memory_commit_id]
tree: memory_tree_id
author: {actor_type: human|agent|system, actor_id: string}
timestamp: rfc3339
project_id: string
work_item_id: string|null
cycle_run_id: string|null
subject_revision: string|null
planning_revision: string|null
workflow_revision: string|null
event_cursor: string|null
message: string
reason: string
provenance_refs: []
```

A normal commit has one parent. A synthesis/merge may have multiple parents.

### DecisionMemoryTree

Immutable semantic snapshot of refs to typed objects:

```text
goal/
decisions/
options/
assumptions/
risks/
questions/
frontier/
delegations/
contributions/
dissent/
artifacts/
knowledge/
```

### DecisionMemoryBlob

Content-addressed typed payload or pointer set for decisions, options, evidence refs, assumptions, risks, questions, contributions, synthesis receipts, context deltas, revisit triggers and negative knowledge.

## 4. Refs and authority

Suggested namespace:

```text
refs/heads/canonical
refs/heads/session/<session-id>
refs/heads/decision/<decision-id>/<option>
refs/heads/what-if/<experiment-id>
refs/heads/rejected/<decision-id>/<option>
refs/tags/cycle/<cycle-id>
refs/tags/release/<version>
refs/tags/milestone/<work-item-id>
HEAD -> refs/heads/canonical
```

Rules:

1. refs are movable pointers; objects are immutable.
2. `canonical` is the accepted deliberative projection.
3. `what-if` and `rejected` refs are advisory and cannot mutate workflow/planning authority.
4. moving canonical HEAD requires the same policy/authority path that authorized the underlying decision.
5. every ref movement has an append-only reflog entry.

## 5. Merge semantics

A memory merge never means blind textual merge.

```yaml
DecisionMemoryMergeReceipt:
  parents: [M7, M8]
  merge_base: M3
  conflicts: []
  selected_claims: []
  rejected_claims: []
  dissent_preserved: []
  evidence_refs: []
  authority: ...
```

The merge result preserves parentage and explicit conflict resolution.

## 6. Traversal operations

Target semantics:

```text
sddk memory status
sddk memory log --graph
sddk memory show <ref|commit>
sddk memory diff <A>..<B>
sddk memory branch
sddk memory branch <name> <ref>
sddk memory merge-base <A> <B>
sddk memory ancestors <ref>
sddk memory why <decision|state>
sddk memory reflog
sddk memory fork <checkpoint> --as what-if/foo
sddk memory compare canonical what-if/foo
sddk resume explain [--at <ref|timestamp>]
sddk session diff <A> <B>
```

The exact CLI is not fixed by this document; these are semantic acceptance targets.

## 7. LLM projection

The Context Compiler selects a minimal tree view from the DAG.

### Resume projection

```text
HEAD
├─ current goal
├─ current semantic Work Item
├─ runtime frontier
├─ binding decisions
├─ active assumptions
├─ open risks
├─ open questions
├─ negative knowledge
├─ pending delegations
└─ continuation candidates
```

### Decision projection

```text
Question
├─ Option A [selected]
│  ├─ pros
│  ├─ cons
│  ├─ evidence
│  └─ outcome
├─ Option B [rejected]
│  ├─ reason
│  └─ revisit_when
└─ Option C [open]
   └─ missing evidence
```

### Delegation projection

```text
orchestrator
├─ agent A → contribution C1
├─ agent B → contribution C2
├─ agent C → contribution C3
└─ synthesis S1
   ├─ consumed: C1,C2,C3
   ├─ conflicts
   ├─ dissent
   └─ selected decision
```

## 8. Session continuity

A session checkpoint should be a ref/tag to a memory commit rather than an unrelated narrative snapshot.

`diff(previous_session, HEAD)` projects changes in runtime, planning, code subject, decisions, assumptions, risks, questions, contributions and continuation frontier.

A historical checkout is read/reconstruction only. Before using it as current context, staleness checks compare its subject/planning/workflow/knowledge revisions with current authority.

## 9. Delegation integration

Every non-trivial delegation carries a `ContextLease` with `decision_memory_head` and returns an `AgentContributionEnvelope`.

The coordinator creates an `OrchestrationSynthesisReceipt` that records consumed/omitted contributions, conflicts, dissent, evidence and information-loss checks.

Decision Memory stores refs to those immutable artifacts; it does not replace them.

## 10. Search and counterfactuals

Core runtime performs only deterministic legal-action frontier + policy + explicit candidate scoring.

Workflow Lab may later fork memory branches and test bounded search strategies:

- Pareto baseline;
- beam/best-first;
- Tree of Thoughts-like;
- Graph of Thoughts-like;
- MCTS/LATS-like.

Experimental branches cannot move canonical HEAD without normal policy/human promotion.

## 11. Invariants

- immutable object once addressed;
- deterministic canonical serialization/hash;
- parent links form an acyclic graph;
- no authoritative fact exists only in a summary;
- no branch silently becomes authoritative;
- raw contribution/evidence remains reachable from synthesis;
- unresolved material dissent is preserved;
- replay/fork does not rewrite history;
- memory deletion/compaction obeys reachability + retention policy;
- private model chain-of-thought is never required or persisted.

## 12. Acceptance direction

The CDD work is complete only when a fresh LLM session can:

1. resolve canonical HEAD;
2. render a bounded resume tree;
3. diff it with an earlier session/checkpoint;
4. traverse why/decision branches to original evidence;
5. inspect rejected alternatives/revisit triggers;
6. recover delegated contributions and synthesis receipts;
7. propose next candidates with pros/cons;
8. fork a what-if branch without mutating canonical state.
