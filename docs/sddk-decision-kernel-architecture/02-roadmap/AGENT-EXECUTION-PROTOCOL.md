# SDDK Agent Execution Protocol

> **Purpose:** define the deterministic procedure an LLM/agent must follow to continue SDDK from any clean checkout without guessing roadmap intent or loading irrelevant historical context.
> **Canonical entry point:** [`LLM-START-HERE.md`](./LLM-START-HERE.md)
> **Machine-readable plan:** [`EXECUTION-SPINE.yaml`](./EXECUTION-SPINE.yaml)
> **Human temporal projection:** [`EXECUTION-TIMELINE.md`](./EXECUTION-TIMELINE.md)
> **Machine-readable cycle context:** [`CYCLE-CONTEXT-MAP.yaml`](./CYCLE-CONTEXT-MAP.yaml)

## 1. Core rule

The agent MUST NOT infer the next evolution from prose, old cycle numbers, commit chronology, or whichever design document looks newest.

The canonical execution/context decision is:

```text
EXECUTION-SPINE.yaml
        +
EXECUTION-TIMELINE.md
        +
CYCLE-CONTEXT-MAP.yaml
        +
Planning Ledger (when H1 is shipped)
        +
actual execution/release evidence
        =
next semantic Work Item + required cycle context
```

Until H1 exists, `EXECUTION-SPINE.yaml` is the bootstrap machine-readable planning source. `CYCLE-CONTEXT-MAP.yaml` is the bootstrap machine-readable context-routing source.

## 2. Mandatory startup algorithm

Every implementation session starts with this exact algorithm:

1. Read `LLM-START-HERE.md`.
2. Read `EXECUTION-SPINE.yaml`.
3. Count `ACTIVE` Work Items.
   - exactly one: resume it;
   - more than one: fail closed and reconcile planning state;
   - zero: scan by ascending `order`.
4. Select the first non-terminal Work Item whose `depends_on` entries are all terminal (`SHIPPED`, `ABSORBED`, `SUPERSEDED`).
5. If the selected item is `BLOCKED`, stop and report the blocker. Never jump to a later item merely because it is easier.
6. If the selected item is `PROPOSED`, verify that its acceptance contract is present before promoting it to `READY`.
7. Locate the selected Work Item in `EXECUTION-TIMELINE.md` to establish predecessor, successor, horizon and context-pack key.
8. Resolve that context-pack key in `CYCLE-CONTEXT-MAP.yaml`.
9. Load Tier 0, Tier 1 and the selected Tier 2 pack before implementation.
10. Read current repository/release evidence relevant to the selected Work Item and all direct dependencies.
11. Reconcile the plan only if evidence proves status/context is stale.
12. Record the required cycle context snapshot.
13. Create/bind one concrete cycle/run execution instance to the semantic Work Item.
14. Execute only the selected Work Item plus explicitly admitted prerequisite fixes.
15. Validate its `exit_gate` with durable evidence.
16. Mark it `SHIPPED`, `ABSORBED` or `SUPERSEDED`, attach evidence, close the run, reconcile planning state and recompute the next item.

Skipping the context-loading steps is a protocol violation: a cycle is not ready to implement until its context snapshot exists.

## 3. Temporal identity: semantic Work Item vs concrete cycle

A cycle number is not a feature name.

Correct:

```text
Work Item: DW-RUNTIME-003
Execution binding: cycle-72
Temporal position: order 160 / H2
```

Incorrect:

```text
"cycle-72" == generated workflow runtime
```

This prevents historical cycle-number collisions and lets the same semantic capability be retried, paused, superseded or completed across multiple execution attempts without changing its identity.

`EXECUTION-TIMELINE.md` is the human chronological projection. `EXECUTION-SPINE.yaml` remains normative for exact order, status and dependencies.

## 4. One canonical line

The default governance rule is **one ACTIVE Work Item on the canonical spine**.

A second concurrent item is allowed only if an ADR proves:

- there is no dependency edge between the items;
- they cannot mutate the same authoritative state or contracts;
- merge/conflict risk is bounded;
- the Planning Ledger can represent both active bindings unambiguously;
- each item has an independent context/evidence snapshot.

Without that ADR, the agent works serially.

## 5. Mandatory cycle context model

The selected Work Item determines what the agent is allowed to load as implementation context.

### Tier 0 — navigation/governance — always read

- selected entry from `EXECUTION-SPINE.yaml`;
- selected entry from `EXECUTION-TIMELINE.md`;
- selected context pack from `CYCLE-CONTEXT-MAP.yaml`;
- this protocol.

### Tier 1 — canonical capability context — always read

- selected horizon section in `ROADMAP.md`;
- selected capability/Work Item in `BACKLOG.md`;
- direct dependency completion evidence;
- current code/tests for the contract boundary being changed;
- accepted ADRs/specs directly constraining the Work Item.

Read `EVOLUTION-CROSSWALK.md` whenever the capability was inherited from an older evolution.

### Tier 2 — selected design pack — mandatory

Load the `must_read`, `discover_and_read`, invariants and code anchors from the matching `CYCLE-CONTEXT-MAP.yaml` pack.

Historical pack rule:

1. read its `STATUS.md` first;
2. load only the files relevant to the current Work Item;
3. treat historical cycle numbers and original execution order as non-canonical;
4. use old material for rationale/acceptance ideas only where it does not conflict with accepted/current contracts.

### Tier 3 — exploration — on demand only

Search additional code, ADRs, specs, commits or historical docs only if:

- Tier 0–2 reveal a contradiction;
- required acceptance behavior is underspecified;
- current code proves the planned assumption stale;
- a decision is missing;
- tests expose an unmodeled prerequisite.

The existence of more documentation is not a reason to load it. Context must remain bounded and relevant.

## 6. Required cycle context snapshot

Before implementation, persist a context snapshot with the concrete cycle/run evidence.

Minimum shape:

```yaml
work_item: DW-RUNTIME-003
execution_binding: cycle-72
horizon: H2
temporal_order: 160
direct_dependencies:
  - id: DW-RUNTIME-002
    evidence: <receipt/commit/test/release refs>
consulted:
  canonical:
    - EXECUTION-SPINE.yaml
    - EXECUTION-TIMELINE.md
    - ROADMAP.md#H2
    - BACKLOG.md#DW-RUNTIME
  design:
    - <context-pack must_read files>
  adrs_specs:
    - <accepted decisions>
  code_tests:
    - <paths>
  execution_evidence:
    - <dependency/current-state evidence>
conflicts_found: []
assumptions: []
exit_gate: <exact spine exit gate>
```

The snapshot is not a second roadmap. It is evidence of **what context the executing agent actually used**.

Once H1 Planning Ledger exists, these fields should be projected from ledger/run provenance wherever possible.

## 7. Source authority and conflict resolution

No single source type answers every question.

| Question | Authority |
|---|---|
| What is next? | `EXECUTION-SPINE.yaml` / Planning Ledger after H1 |
| Where does it sit in the journey? | `EXECUTION-TIMELINE.md` |
| What context must be loaded? | `CYCLE-CONTEXT-MAP.yaml` |
| Why is the horizon ordered this way? | `ROADMAP.md` |
| What capability is intended? | `BACKLOG.md` + accepted spec/ADR |
| What historical idea does this absorb? | `EVOLUTION-CROSSWALK.md` + pack `STATUS.md` |
| What is actually released? | current behavior/tests + `CHANGELOG.md` + tags/commits |
| What happened in the predecessor? | cycle/run artifacts, receipts and ledger evidence |

If sources disagree, the agent MUST NOT silently pick the convenient answer.

Procedure:

1. establish actual released/runtime/test truth;
2. establish accepted ADR/spec intent and compatibility obligations;
3. establish canonical planning state;
4. inspect historical STATUS/crosswalk only to understand drift/origin;
5. reconcile stale planning if execution evidence is authoritative;
6. if accepted design and implementation materially conflict, stop and create the required governed decision/ADR before continuing.

## 8. What an agent may read for design

Once the next Work Item and context pack are selected, the agent may consult:

- `BACKLOG.md` for capability context;
- `ROADMAP.md` for horizon intent and exit gates;
- `EVOLUTION-CROSSWALK.md` for historical proposal disposition;
- accepted ADRs/specs for constraints;
- only the historical pack files admitted by the context map;
- `CHANGELOG.md`, git history and tests for shipped truth;
- predecessor cycle evidence to understand assumptions, debt and carry-forward.

None of those sources may override the execution ordering silently.

If a design source contradicts the spine/context map, the agent must either:

1. reconcile a stale status/context route using stronger evidence, or
2. propose an ADR/plan/context-map version change.

It must not improvise a different roadmap.

## 9. Work Item admission checklist

Before moving `PROPOSED -> READY`, the agent verifies:

- dependencies are terminal;
- objective is still useful against current code;
- acceptance/exit gate is testable;
- required ADR/spec decisions exist or are part of the item;
- no newer work has already `ABSORBED` the capability;
- scope is small enough for one bounded cycle, or the item is split before execution;
- UAT/evidence expectations are explicit;
- `CYCLE-CONTEXT-MAP.yaml` has a matching pack and useful context route;
- the context snapshot can identify predecessor evidence and target code/tests.

## 10. Scope expansion rule

During a cycle the agent may discover new work. It must classify it:

- **required to satisfy current exit gate:** add as a child task of the current Work Item;
- **new independent capability:** create a new semantic Work Item and place it in the spine with explicit dependencies and a context-pack route;
- **future optimization:** backlog it after the capability it optimizes;
- **invalidated assumption:** stop and propose ADR/plan revision;
- **missing contextual dependency:** amend the context map without reordering product work if the dependency is informational only.

The agent must not silently absorb large adjacent features into the current cycle.

## 11. Updating the spine, timeline or context map

Changing these files is a governed planning operation.

Allowed without ADR:

- status transition supported by evidence;
- adding evidence references;
- splitting an oversized not-yet-ACTIVE item while preserving dependency order;
- clarifying an exit gate without changing product intent;
- adding a missing context source that does not alter product/dependency semantics;
- correcting a stale historical path after repository reorganization.

Requires an ADR or explicit planning decision:

- reordering dependency-bearing items;
- skipping a horizon gate;
- deleting an admitted capability;
- introducing a second authority/control path;
- changing the GA terminal condition;
- promoting an experimental strategy to default without required evidence;
- changing a context route in a way that effectively changes architecture/product scope.

Structural changes must update `EXECUTION-SPINE.yaml`, `EXECUTION-TIMELINE.md`, `CYCLE-CONTEXT-MAP.yaml`, `ROADMAP.md`/`BACKLOG.md` where relevant, and the Planning Ledger once H1 owns the model.

## 12. Horizon path to GA

The canonical path is complete, not open-ended:

```text
H0  Reconcile & Deterministic Foundations
 ↓
H1  Planning SSOT
 ↓
H2  Generated Workflow MVP
 ↓
H3  Decision Plane
 ↓
H4  AgentHost & Context Compiler
 ↓
H5  Human & Reactive Control
 ↓
H6  Runtime Completeness & Workflow Lab
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

Within each horizon, `EXECUTION-TIMELINE.md` makes the semantic cycle order visible and `EXECUTION-SPINE.yaml` makes it executable.

## 13. Terminal condition

The plan is complete only when `GA-002` is terminal with evidence.

At that point the agent must not continue inventing work under this plan. Post-GA evolution starts with a new versioned execution plan derived from observed needs, compatibility commitments and retained backlog.

## 14. Minimal agent prompt contract

An orchestrator can prepend the following rule to any SDDK development agent:

> Continue SDDK by reading `docs/sddk-decision-kernel-architecture/02-roadmap/LLM-START-HERE.md`. Determine the current semantic Work Item from `EXECUTION-SPINE.yaml`, locate it in `EXECUTION-TIMELINE.md`, load its required context through `CYCLE-CONTEXT-MAP.yaml`, and follow `AGENT-EXECUTION-PROTOCOL.md`. Record the cycle context snapshot before implementation. Do not skip blocked/earlier work, do not use cycle numbers as capability identities, and do not load historical evolution packs outside the selected context route. Finish the exact exit gate with durable evidence, update planning state, and only then compute the next Work Item.

This is deliberately short enough to embed in AgentHost/IDE instructions later while keeping detailed governance here.