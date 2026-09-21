# SDDK Agent Execution Protocol

> **Purpose:** deterministic procedure an LLM/agent follows to continue SDDK from any clean checkout/session without guessing roadmap intent, trusting chat memory or losing delegated context.
> **Entry point:** [`LLM-START-HERE.md`](./LLM-START-HERE.md)
> **Machine plan:** [`EXECUTION-SPINE.yaml`](./EXECUTION-SPINE.yaml)
> **Timeline:** [`EXECUTION-TIMELINE.md`](./EXECUTION-TIMELINE.md)
> **Context routing:** [`CYCLE-CONTEXT-MAP.yaml`](./CYCLE-CONTEXT-MAP.yaml)
> **Decision Memory model:** [`DECISION-MEMORY-GIT-MODEL.md`](./DECISION-MEMORY-GIT-MODEL.md)

## 1. Core rule

The agent MUST NOT infer the next evolution from prose, chat/session memory, historical cycle numbers, commit chronology or the newest-looking design document.

Bootstrap decision:

```text
EXECUTION-SPINE.yaml
 + EXECUTION-TIMELINE.md
 + CYCLE-CONTEXT-MAP.yaml
 + actual execution/release evidence
 + Planning Ledger when H1 exists
 = next semantic Work Item + required context
```

Once H4 CDD exists:

```text
above authoritative state
 + canonical Decision Memory HEAD
 + staleness validation
 = bounded ResumeView + deliberative provenance
```

Decision Memory never outranks the canonical sources from which it is projected.

## 2. Mandatory startup algorithm

1. Read `LLM-START-HERE.md`.
2. Read `EXECUTION-SPINE.yaml`.
3. Count `ACTIVE` Work Items.
   - one: resume it;
   - more than one: fail closed/reconcile;
   - zero: scan ascending `order`.
4. Select first non-terminal item whose dependencies are terminal.
5. If `BLOCKED`, stop; never jump ahead.
6. If `PROPOSED`, require acceptance contract before `READY`.
7. Locate it in `EXECUTION-TIMELINE.md`.
8. Resolve context-pack key in `CYCLE-CONTEXT-MAP.yaml`.
9. Load Tier 0/1/selected Tier 2 context.
10. Reconstruct current runtime/cycle/ledger/vault state through authoritative mechanisms; do not use chat memory as state authority.
11. Once CDD is shipped, resolve Decision Memory canonical `HEAD`, validate its referenced subject/planning/workflow/knowledge revisions and build `ResumeView`.
12. Read direct dependency completion evidence and relevant released behavior/tests.
13. Reconcile stale planning/context only with stronger evidence.
14. Record cycle context snapshot.
15. Bind concrete cycle/run to semantic Work Item.
16. Execute only selected Work Item plus explicitly admitted prerequisite repairs.
17. Validate exact `exit_gate` with durable evidence.
18. Persist/reconcile planning/runtime state and, when CDD exists, contribution/synthesis/Decision Memory provenance.
19. Mark terminal and recompute NEXT.

A cycle is not implementation-ready until its context snapshot exists.

## 3. Semantic identity vs execution identity

Correct:

```text
Work Item: CDD-MEMORY-001
Execution binding: cycle-84
Temporal order: 267 / H4
```

Incorrect:

```text
cycle-84 == decision memory
```

Semantic identity survives retry/pause/supersede; execution IDs are evidence instances.

## 4. One canonical work line

Default: one `ACTIVE` semantic Work Item.

Concurrency requires an accepted decision proving:

- no dependency edge;
- no shared authoritative mutation conflict;
- bounded merge/conflict risk;
- planning can represent both bindings;
- each has independent context/evidence identity.

No proof → serial execution.

## 5. Context tiers

### Tier 0 — navigation/governance

- selected spine entry;
- selected timeline entry;
- selected context pack;
- this protocol.

### Tier 1 — canonical capability context

- relevant `ROADMAP.md` horizon;
- relevant `BACKLOG.md` capability;
- direct dependency evidence;
- current code/tests;
- accepted ADR/spec constraints;
- crosswalk/status when inherited from old proposals.

### Tier 2 — selected design pack

Use `must_read`, `discover_and_read`, invariant and code anchors from `CYCLE-CONTEXT-MAP.yaml`.

Historical packs:

1. read `STATUS.md` first;
2. load only current relevant files;
3. original cycle order is non-canonical;
4. old prose cannot override accepted/current evidence.

### Tier 3 — exploration

Only when contradiction, underspecification, stale plan, missing architectural decision or tests reveal unmodelled prerequisite.

## 6. Cycle context snapshot

Minimum:

```yaml
work_item: CDD-MEMORY-001
execution_binding: cycle-84
horizon: H4
temporal_order: 267
direct_dependencies:
  - id: CDD-HANDOFF-002
    evidence: []
consulted:
  canonical: []
  design: []
  adrs_specs: []
  code_tests: []
  execution_evidence: []
decision_memory_head: null
context_revision: null
contribution_synthesis_refs: []
conflicts_found: []
assumptions: []
exit_gate: <exact gate>
```

Unsupported future fields remain null before the corresponding capability ships.

## 7. Authority/conflict resolution

| Question | Primary authority |
|---|---|
| What is next? | spine / Planning Ledger after H1 |
| Where is it in GA journey? | timeline |
| What context must load? | context map |
| What is intended? | backlog + accepted spec/ADR |
| What is released? | executable behavior/tests + changelog/tags/commits |
| What happened in a run? | runtime/cycle artifacts/receipts/ledger |
| Why was a decision made after CDD? | Decision Memory path to source evidence/contributions |
| What did chat remember? | advisory only |

Conflict procedure:

1. establish current executable/runtime truth;
2. establish accepted design/compatibility obligations;
3. establish planning state;
4. use Decision Memory only to reconstruct provenance/history, not override sources;
5. reconcile stale planning if evidence is authoritative;
6. if accepted design and implementation materially conflict, stop for governed decision/ADR.

## 8. Session continuity

### Before CDD

Use existing artifact-first/runtime reconstruction (`sddk-cycle-resume` semantics), durable vault knowledge and optional Engram/session summaries.

Session summaries can suggest where to look. They cannot prove runtime/planning state.

### After CDD

Resolve:

```text
canonical Decision Memory HEAD
   + current canonical revisions
        ↓
staleness-aware ResumeView
```

A historical session is addressed via ref/tag/checkpoint, then diffed against current `HEAD`.

Expected semantics:

```text
memory log --graph
memory show <ref>
memory diff <A>..<B>
memory merge-base <A> <B>
memory why <decision>
memory reflog
resume explain [--at <ref|timestamp>]
session diff <A> <B>
```

Historical checkout is read/reconstruction. It does not restore old authority blindly.

## 9. Role topology after `CDD-ROLE-001`

Every governed role has `AgentRoleContract`.

Required validation:

- orchestrator owns top-level sequencing/synthesis;
- coordinator owns only declared bounded fan-out/join;
- leaf never dispatches;
- evaluator/judge evaluates immutable subject and does not silently mutate it;
- advisor/Secretary proposal authority is distinct from mutation authority;
- allowed workers/tools/read/write/mutation scopes are explicit;
- one synthesis owner per join;
- no cycles of delegation authority.

Role violation blocks rather than becoming a warning.

## 10. Delegation after `CDD-HANDOFF-001`

Every non-trivial governed delegation carries:

```text
DelegationRequest
 + AgentRoleContract
 + immutable ContextLease
```

`ContextLease` pins at least objective, context revision, source subject, planning/workflow revision and Decision Memory `HEAD` when available.

Worker returns `AgentContributionEnvelope`, including:

- coverage satisfied/missing;
- findings;
- proposals/alternatives/rejected options;
- pros/cons;
- assumptions/uncertainty;
- risks/open questions;
- evidence/artifact refs;
- context delta;
- recommendation/confidence;
- metrics.

The full artifact remains recoverable. The envelope is an index, not lossy replacement.

## 11. Coordinator/orchestrator synthesis after `CDD-HANDOFF-002`

Synthesis emits `OrchestrationSynthesisReceipt`:

```yaml
consumed_contributions: []
omitted_contributions: []
conflicts: []
dissent: []
resolved_by: []
selected_option: null
alternative_options: []
evidence_refs: []
compression_refs: []
information_loss_checks: []
next_candidates: []
```

Hard information-loss guard: compression/synthesis cannot silently remove:

- blockers;
- critical/high risks;
- mandatory evidence gaps;
- unresolved material dissent;
- rejected options carrying `revisit_when`;
- authority decisions;
- open questions affecting acceptance.

If these disappear between contribution and synthesis, synthesis is invalid.

## 12. Decision Memory after `CDD-MEMORY-001/002`

Decision Memory adopts Git-like semantics without storing state inside `.git`.

### Immutable object model

```text
DecisionMemoryBlob
DecisionMemoryTree
DecisionMemoryCommit(parents[])
```

### Refs

```text
refs/heads/canonical
refs/heads/session/<id>
refs/heads/decision/<decision>/<option>
refs/heads/what-if/<experiment>
refs/heads/rejected/<decision>/<option>
refs/tags/cycle/<cycle>
refs/tags/release/<version>
HEAD -> refs/heads/canonical
```

### Hard invariants

- object hash derives from deterministic canonical payload;
- addressed object is immutable;
- graph is acyclic;
- ref moves append reflog;
- merge/synthesis with multiple parents requires explicit merge receipt;
- raw source evidence remains reachable;
- advisory branch never acquires runtime authority implicitly;
- private chain-of-thought is not persisted or required.

A deletion/compaction policy may use reachability + retention, but cannot remove audit-required evidence.

## 13. Continuation frontier after `CDD-CONTINUE-001`

The orchestrator receives a bounded `ResumeView` and one or more `ContinuationCandidate` values.

Each candidate records:

- action + kind;
- prerequisites;
- pros/cons;
- risk;
- reversibility;
- evidence/confidence/uncertainty;
- expected value/cost;
- what it blocks/unlocks;
- whether human authority is required.

The user may see only the best 1–3 options, but Decision Memory preserves admitted alternatives and pruning rationale.

## 14. Human/Secretary integration

Human and Secretary paths must consume CDD rather than invent parallel contracts.

Secretary:

- L0 deterministic reactions first;
- L1 bounded proposal through Contribution/Candidate;
- cognitive replan only after deterministic paths fail;
- cannot directly move canonical memory `HEAD` or runtime state;
- accepted proposal goes through policy/authority and produces receipts/provenance.

Human decisions likewise become immutable decision evidence linked into Decision Memory after authorization.

## 15. Decision-tree/graph search governance

### Core baseline

Core Decision Plane remains deterministic:

1. enumerate legal actions;
2. policy filter;
3. create typed candidates;
4. score explicit risk/reversibility/evidence/uncertainty/cost/unlock dimensions;
5. calculate Pareto frontier;
6. escalate material ambiguity/risk.

### H6 bounded search

`LAB-DECISION-001` may fork Decision Memory branches with:

- Pareto baseline;
- bounded beam/best-first;
- depth/node/token/time budgets;
- environment feedback;
- pruning receipts;
- counterfactual comparison;
- no canonical mutation.

### Experimental strategies

`LAB-DECISION-002` may compare ToT/GoT/MCTS/LATS-like strategies.

They cannot become default without reproducible evidence of quality benefit, bounded cost/stability, rollback and preserved policy/HITL. Deterministic baseline/fallback remains.

## 16. End-of-cycle completion

A cycle is terminal only when:

1. exact spine exit gate is satisfied;
2. required tests/UAT/evidence pass;
3. architecture/dependency rules hold;
4. durable evidence is attached;
5. Work Item is `SHIPPED`, `ABSORBED` or `SUPERSEDED`;
6. planning is reconciled;
7. when available, delegation/synthesis/Decision Memory provenance is complete;
8. NEXT is recomputed.

## 17. Anti-patterns

- “I remember from yesterday...” as authority.
- full transcript as required recovery mechanism.
- summary without link to raw artifact/evidence.
- coordinator hiding worker dissent.
- worker using stale context revision silently.
- leaf dispatching agents.
- Secretary as a second orchestrator.
- treating `what-if` branch as accepted because it scored higher.
- MCTS/ToT as default before Workflow Lab proof.
- storing private chain-of-thought as product memory.
- Decision Memory becoming a second Planning/Event Ledger.

## 18. Terminal condition

This execution protocol terminates under the current plan at `GA-002`. Post-GA evolution starts under a new versioned plan.
