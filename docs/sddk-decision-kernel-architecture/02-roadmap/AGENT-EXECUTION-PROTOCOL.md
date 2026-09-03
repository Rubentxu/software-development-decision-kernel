# SDDK Agent Execution Protocol

> **Purpose:** define the deterministic procedure an LLM/agent must follow to continue SDDK from any clean checkout without guessing roadmap intent.
> **Machine-readable plan:** [`EXECUTION-SPINE.yaml`](./EXECUTION-SPINE.yaml)

## 1. Core rule

The agent MUST NOT infer the next evolution from prose, old cycle numbers, commit chronology, or whichever design document looks newest.

The canonical execution decision is:

```text
EXECUTION-SPINE.yaml
        +
Planning Ledger (when H1 is shipped)
        +
actual execution/release evidence
        =
next semantic Work Item
```

Until H1 exists, `EXECUTION-SPINE.yaml` is the bootstrap machine-readable planning source.

## 2. Startup algorithm

Every implementation session starts with this algorithm:

1. Read `EXECUTION-SPINE.yaml`.
2. Read the current repository/release evidence relevant to the first non-terminal item.
3. Reconcile the plan only if evidence proves its status is stale.
4. Count `ACTIVE` work items.
   - exactly one: resume it;
   - more than one: fail closed and reconcile planning state;
   - zero: scan by ascending `order`.
5. Select the first non-terminal item whose `depends_on` entries are all terminal (`SHIPPED`, `ABSORBED`, `SUPERSEDED`).
6. If selected item is `BLOCKED`, stop and report the blocker. Never jump to a later item merely because it is easier.
7. If selected item is `PROPOSED`, verify that its acceptance contract is present; then promote it to `READY`.
8. Create/bind one concrete cycle/run execution instance to that semantic Work Item.
9. Execute only the selected Work Item plus explicitly admitted prerequisite fixes.
10. Validate its `exit_gate` with durable evidence.
11. Mark it `SHIPPED` or `ABSORBED`, attach evidence, close the run, and recompute the next item.

## 3. Cycle identity

A cycle number is not a feature name.

Correct:

```text
Work Item: DW-RUNTIME-003
Execution binding: cycle-72
```

Incorrect:

```text
"cycle-72" == generated workflow runtime
```

This prevents historical cycle-number collisions and lets the same semantic capability be retried, paused, superseded or completed across multiple execution attempts without changing its identity.

## 4. One canonical line

The default governance rule is **one ACTIVE Work Item on the canonical spine**.

A second concurrent item is allowed only if an ADR proves:

- there is no dependency edge between the items;
- they cannot mutate the same authoritative state or contracts;
- merge/conflict risk is bounded;
- the Planning Ledger can represent both active bindings unambiguously.

Without that ADR, the agent works serially.

## 5. What an agent may read for design

Once the next Work Item is selected, the agent may consult:

- `BACKLOG.md` for capability context;
- `ROADMAP.md` for horizon intent and exit gates;
- `EVOLUTION-CROSSWALK.md` for historical proposal disposition;
- ADRs/specs for accepted constraints;
- old evolution packs for design ideas only;
- `CHANGELOG.md`, git history and tests for shipped truth.

None of those sources may override the execution ordering silently.

If a design source contradicts the spine, the agent must either:

1. reconcile a stale status using stronger execution evidence, or
2. propose an ADR/plan-version change.

It must not improvise a different roadmap.

## 6. Work Item admission checklist

Before moving `PROPOSED -> READY`, the agent verifies:

- dependencies are terminal;
- objective is still useful against current code;
- acceptance/exit gate is testable;
- required ADR/spec decisions exist or are part of the item;
- no newer work has already `ABSORBED` the capability;
- scope is small enough for one bounded cycle, or the item is split before execution;
- UAT/evidence expectations are explicit.

## 7. Scope expansion rule

During a cycle the agent may discover new work. It must classify it:

- **required to satisfy current exit gate:** add as a child task of the current Work Item;
- **new independent capability:** create a new semantic Work Item and place it in the spine with explicit dependencies;
- **future optimization:** backlog it after the capability it optimizes;
- **invalidated assumption:** stop and propose ADR/plan revision.

The agent must not silently absorb large adjacent features into the current cycle.

## 8. Updating the spine

Changing `EXECUTION-SPINE.yaml` is a governed planning operation.

Allowed without ADR:

- status transition supported by evidence;
- adding evidence references;
- splitting an oversized not-yet-ACTIVE item while preserving dependency order;
- clarifying an exit gate without changing product intent.

Requires an ADR or explicit planning decision:

- reordering dependency-bearing items;
- skipping a horizon gate;
- deleting an admitted capability;
- introducing a second authority/control path;
- changing the GA terminal condition;
- promoting an experimental strategy to default without required evidence.

Every structural plan change increments `schema_version` or a future `plan_revision` field once H1 owns the model.

## 9. Horizon path to GA

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

Within each horizon, `EXECUTION-SPINE.yaml` gives the exact semantic cycle order.

## 10. Terminal condition

The plan is complete only when `GA-002` is terminal with evidence.

At that point the agent must not continue inventing work under this plan. Post-GA evolution starts with a new versioned execution plan derived from observed needs, compatibility commitments and retained backlog.

## 11. Minimal agent prompt contract

An orchestrator can prepend the following rule to any SDDK development agent:

> Before making implementation changes, read `docs/sddk-decision-kernel-architecture/02-roadmap/EXECUTION-SPINE.yaml` and follow `AGENT-EXECUTION-PROTOCOL.md`. Resume the unique ACTIVE semantic Work Item; otherwise select the earliest dependency-satisfied non-terminal item. Do not skip BLOCKED items, do not use cycle numbers as capability identities, and do not begin later roadmap work unless the canonical spine is updated through its governance rules. Finish the selected item's exit gate with durable evidence, update its status, and only then compute the next item.

This is deliberately short enough to embed in AgentHost/IDE instructions later while keeping the detailed governance in this document.
