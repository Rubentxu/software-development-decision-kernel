# SDDK — LLM START HERE

> **Canonical entry point for any LLM/agent that continues SDDK development.**
> Do not choose work from old evolution packs, cycle numbers, commit chronology, chat memory, or whichever document looks newest.

## 1. Startup path

Every development session follows this order:

```text
LLM-START-HERE.md
        ↓
EXECUTION-SPINE.yaml          ← determine CURRENT / NEXT
        ↓
EXECUTION-TIMELINE.md         ← locate CURRENT on the path to GA
        ↓
CYCLE-CONTEXT-MAP.yaml        ← resolve exact context pack
        ↓
AGENT-EXECUTION-PROTOCOL.md   ← execution/governance rules
        ↓
authoritative state reconstruction
        ↓
Decision Memory HEAD / ResumeView  ← once H4 CDD is shipped
        ↓
current Work Item exit_gate
        ↓
evidence + planning/runtime/memory reconciliation
        ↓
recompute NEXT
```

The responsibilities above must not be collapsed into “read all docs and decide”.

## 2. Which source answers which question?

| Question | Read/query |
|---|---|
| What do I work on now? | `EXECUTION-SPINE.yaml` / Planning Ledger after H1 |
| What comes next through GA? | `EXECUTION-TIMELINE.md` |
| What context must I load? | `CYCLE-CONTEXT-MAP.yaml` |
| How do I execute/stop/split/complete? | `AGENT-EXECUTION-PROTOCOL.md` |
| Why does this horizon exist? | `ROADMAP.md` |
| What does the capability mean? | `BACKLOG.md` |
| What historical evolution did it absorb? | `EVOLUTION-CROSSWALK.md` + pack `STATUS.md` |
| What is actually shipped? | current behavior/tests + `CHANGELOG.md` + tags/commits |
| What decisions constrain implementation? | accepted ADRs/specs selected by context map |
| Where did the previous session leave runtime state? | authoritative CLI/ledger/cycle artifacts; `sddk-cycle-resume` semantics |
| Why did we reach the current decision state? | Decision Memory `HEAD`/history after H4 CDD |
| What changed since a previous session? | Decision Memory/session diff after H4 CDD |
| What did delegated agents actually contribute? | contribution artifacts/envelopes + synthesis receipts after H4 CDD |

## 3. Current-cycle selection

1. Read `EXECUTION-SPINE.yaml`.
2. If exactly one Work Item is `ACTIVE`, resume it.
3. If more than one is `ACTIVE`, fail closed and reconcile planning.
4. If none is `ACTIVE`, scan by ascending `order`.
5. Select the first non-terminal item whose dependencies are terminal.
6. If selected item is `BLOCKED`, stop. Never jump ahead.
7. If `PROPOSED`, admit only when its acceptance contract exists.
8. Bind a concrete cycle/run only when execution starts.

Example:

```text
semantic Work Item: CDD-MEMORY-001
execution attempt: cycle-84
```

The Work Item is identity. `cycle-84` is merely one execution instance.

## 4. Continuity model: before and after H4 CDD

### 4.1 Bootstrap/current model before CDD is shipped

A fresh session reconstructs **authoritative state** from runtime/CLI/ledger/artifacts. It must not trust prior chat memory.

The existing `sddk-cycle-resume` pattern provides:

- project/adoption/knowledge state;
- trusted cycle state when cycle identity is known;
- lease/fencing;
- cycle artifacts dir;
- recent ledger events;
- vault validation.

Then load relevant durable knowledge/artifacts under the source hierarchy.

Engram/session summaries may improve recall, but are advisory and never substitute missing authoritative evidence.

### 4.2 Target model after `CDD-CONTINUE-001`

A fresh session performs both:

```text
A. authoritative reconstruction
   runtime + Planning Ledger + artifacts + vault

B. deliberative reconstruction
   Decision Memory canonical HEAD
       ↓
   staleness validation
       ↓
   bounded ResumeView
```

The ResumeView should expose:

```text
HEAD
├─ current goal
├─ current semantic Work Item
├─ runtime frontier
├─ binding decisions
├─ active assumptions
├─ open risks/questions
├─ negative knowledge
├─ pending delegations
└─ continuation candidates
```

A previous day/session is resolved through a ref/tag/checkpoint and compared with current `HEAD`, not by replaying a narrative summary.

Relevant target semantics:

```text
memory log --graph
memory show <ref>
memory diff <previous-session>..<HEAD>
memory why <decision>
resume explain [--at <ref|timestamp>]
session diff <A> <B>
```

See `DECISION-MEMORY-GIT-MODEL.md`.

## 5. Mandatory context loading

### Tier 0 — navigation/governance — ALWAYS

- selected `EXECUTION-SPINE.yaml` entry;
- selected `EXECUTION-TIMELINE.md` entry;
- matching `CYCLE-CONTEXT-MAP.yaml` pack;
- `AGENT-EXECUTION-PROTOCOL.md`.

### Tier 1 — canonical capability context — ALWAYS

Read only relevant sections of:

- `ROADMAP.md` selected horizon;
- `BACKLOG.md` selected capability;
- direct dependency completion evidence;
- current code/tests for the changed boundary;
- accepted ADRs/specs directly constraining it.

Use `EVOLUTION-CROSSWALK.md` when the capability originates in older proposals.

### Tier 2 — cycle-specific design context — REQUIRED BY MAP

Load exact `must_read`, `discover_and_read`, invariants and code anchors from the selected context pack.

Rules:

- accepted ADR/spec beats historical proposal prose;
- current code/tests beat stale implementation descriptions;
- read a pack's `STATUS.md` before the historical pack itself;
- do not load whole packs when the context map selects a subset.

### Tier 3 — exploration — ONLY WHEN NEEDED

Expand context only when:

- Tier 0–2 expose a contradiction;
- acceptance cannot be implemented from known contracts;
- current code proves the plan stale;
- an architectural decision is missing;
- tests reveal an unmodelled prerequisite.

More documents are not automatically better context.

## 6. Cycle context snapshot

Before implementation record:

```yaml
work_item: <semantic-id>
execution_binding: <cycle/run-id>
horizon: <Hn>
temporal_order: <order>
direct_dependencies:
  - id: <semantic-id>
    evidence: <refs>
consulted:
  canonical: []
  design: []
  adrs_specs: []
  code_tests: []
  execution_evidence: []
decision_memory_head: <ref/hash|null>
context_revision: <hash|null>
contribution_synthesis_refs: []
conflicts_found: []
assumptions: []
exit_gate: <exact spine gate>
```

Before H1/H4, fields not yet supported are `null`/empty. Once Planning Ledger/CDD exist they should become projections from durable provenance rather than manually maintained parallel truth.

## 7. Delegation rule

### Current baseline

Use artifact-first phase handoffs and the return envelopes defined by the SDDK phase contracts. Filesystem/vault artifacts outrank summaries.

### Target after CDD

Every governed non-trivial delegation becomes:

```text
AgentRoleContract
      +
DelegationRequest
      +
immutable ContextLease
      ↓
AgentContributionEnvelope
      ↓
OrchestrationSynthesisReceipt
      ↓
Decision Memory commit/ref update if the synthesis changes deliberative state
```

The orchestrator must not discard material dissent, blockers, mandatory evidence or rejected alternatives with revisit conditions.

The raw worker artifact remains reachable even when the orchestrator receives a compressed semantic envelope.

## 8. Decision branches are not authority branches

Decision Memory can contain:

```text
refs/heads/canonical
refs/heads/decision/<id>/<option>
refs/heads/what-if/<experiment>
refs/heads/rejected/<decision>/<option>
```

Only the governed canonical path may authorize real workflow/planning mutation. A newer, longer or higher-scoring `what-if` branch has no authority by itself.

## 9. Temporal path to GA

```text
H0  Reconcile & Deterministic Foundations
 ↓
H1  Planning SSOT
 ↓
H2  Generated Workflow MVP
 ↓
H3  Decision Plane
 ↓
H4  AgentHost, Context Compiler & Decision Memory
 ↓
H5  Human & Reactive Control
 ↓
H6  Runtime Completeness, Decision Search & Workflow Lab
 ↓
H7  Engineering Assurance & UAT
 ↓
H8  Adaptive SDD
 ↓
H9  Active Graph & Cockpit
 ↓
H10 Governed Continuous Improvement
 ↓
H11 Multi-pack Proof
 ↓
H12 Supply Chain, Production Hardening & GA
```

Exact Work Item order is always the spine/timeline, not this orientation diagram.

## 10. End-of-cycle rule

A cycle is complete only when:

1. the selected Work Item `exit_gate` is demonstrably satisfied;
2. required tests/UAT/evidence pass;
3. architecture/dependency rules hold;
4. durable evidence is attached;
5. Work Item is terminal (`SHIPPED`, `ABSORBED`, `SUPERSEDED`);
6. planning state is reconciled;
7. when CDD exists, contribution/synthesis/Decision Memory provenance is reconciled;
8. NEXT is recomputed from canonical planning.

## 11. Final condition

The current plan terminates at `GA-002` with evidence. Post-GA work requires a new versioned plan.

## 12. Minimal instruction for an external LLM

> Continue SDDK by reading `docs/sddk-decision-kernel-architecture/02-roadmap/LLM-START-HERE.md`. Select only the current semantic Work Item from the canonical execution spine, load its context pack, reconstruct authoritative runtime/planning state, and—when CDD Decision Memory is available—resolve canonical HEAD and its staleness-aware ResumeView. Execute only that Work Item, preserve rich delegation/evidence provenance, satisfy its exit gate, reconcile planning/memory state, and only then compute NEXT. Never infer current truth from chat memory, old cycle numbers or historical evolution prose.
