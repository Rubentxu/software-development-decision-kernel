# SDDK — LLM START HERE

> **This is the canonical entry point for any LLM/agent that continues SDDK development.**
> Do not choose work from old evolution packs, cycle numbers, commit chronology, or whichever document looks newest.

## 1. The only startup path

Every development session MUST follow this order:

```text
LLM-START-HERE.md
        ↓
EXECUTION-SPINE.yaml          ← determine CURRENT / NEXT
        ↓
EXECUTION-TIMELINE.md         ← understand where CURRENT sits on the path to GA
        ↓
CYCLE-CONTEXT-MAP.yaml        ← determine exactly what context CURRENT must load
        ↓
AGENT-EXECUTION-PROTOCOL.md   ← execute/govern the cycle
        ↓
current Work Item exit_gate
        ↓
evidence + status transition
        ↓
recompute NEXT
```

The agent MUST NOT reorder those responsibilities.

## 2. Exact source responsibilities

| Question | Read this |
|---|---|
| What do I work on now? | `EXECUTION-SPINE.yaml` |
| What comes after it all the way to GA? | `EXECUTION-TIMELINE.md` |
| Which documents/code/evidence must I load for this Work Item? | `CYCLE-CONTEXT-MAP.yaml` |
| Why does this horizon exist? | `ROADMAP.md` |
| What does the capability mean? | `BACKLOG.md` |
| How do historical evolutions map to the current plan? | `EVOLUTION-CROSSWALK.md` and pack `STATUS.md` files |
| How do I execute, stop, split or complete a cycle? | `AGENT-EXECUTION-PROTOCOL.md` |
| What is already shipped? | `CHANGELOG.md`, tags/commits, current tests and runtime behavior |
| What decisions constrain implementation? | accepted ADRs/specs selected by `CYCLE-CONTEXT-MAP.yaml` |

## 3. Deterministic current-cycle selection

1. Read `EXECUTION-SPINE.yaml`.
2. If exactly one Work Item is `ACTIVE`, resume it.
3. Otherwise scan ascending `order`.
4. Select the first non-terminal Work Item whose dependencies are terminal.
5. If it is `BLOCKED`, stop and explain the blocker. **Never jump ahead.**
6. If it is `PROPOSED`, admit it only after its acceptance contract exists.
7. Bind a concrete cycle/run ID to the semantic Work Item.

A cycle number is an execution instance, not roadmap identity.

Example:

```text
semantic Work Item: DW-RUNTIME-003
execution attempt: cycle-72
```

## 4. Mandatory context-loading order for the selected cycle

After selecting the Work Item, load context in four tiers.

### Tier 0 — navigation and governance — ALWAYS

Read:

- `EXECUTION-SPINE.yaml` — selected item, dependency and `exit_gate`;
- `EXECUTION-TIMELINE.md` — predecessor/successor and horizon position;
- `CYCLE-CONTEXT-MAP.yaml` — context pack for the selected item;
- `AGENT-EXECUTION-PROTOCOL.md` — execution rules.

### Tier 1 — canonical capability context — ALWAYS

Read only the relevant sections of:

- `ROADMAP.md` for the selected horizon;
- `BACKLOG.md` for the selected capability/Work Item;
- `EVOLUTION-CROSSWALK.md` if the capability originates in an older evolution;
- the completion evidence of every direct dependency.

### Tier 2 — cycle-specific design context — REQUIRED BY MAP

Load the exact dossier/spec/ADR/code anchors defined for the Work Item family in `CYCLE-CONTEXT-MAP.yaml`.

Rules:

- accepted ADR/spec beats historical proposal prose;
- current code/tests beat stale implementation descriptions;
- `STATUS.md` must be read before using a historical evolution pack;
- do not load an entire historical pack when the context map points to a smaller subset.

### Tier 3 — exploratory context — ONLY WHEN NEEDED

Search additional code, ADRs, specs, commits or historical material only when:

- Tier 0–2 expose a contradiction;
- the acceptance gate cannot be implemented from known contracts;
- current code proves the plan stale;
- an architectural decision is missing.

Do not expand context merely because more documents exist.

## 5. Required cycle context snapshot

Before changing implementation, the agent MUST record a short context snapshot in the cycle/run artifacts containing:

```yaml
work_item: <semantic-id>
execution_binding: <cycle/run-id>
horizon: <Hn>
direct_dependencies:
  - <id + terminal evidence>
consulted:
  canonical:
    - <files>
  design:
    - <files / ADRs / specs>
  code_tests:
    - <paths>
  execution_evidence:
    - <receipts / commits / tests / release evidence>
conflicts_found: []
assumptions: []
exit_gate: <copied from EXECUTION-SPINE.yaml>
```

This prevents a later agent from having to reconstruct why a cycle made a decision.

Once H1 Planning Ledger exists, this snapshot should become a projection of ledger/run provenance rather than a parallel source of truth.

## 6. Temporal line to GA

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

This horizon list is orientation only. The exact cycle-by-cycle order is `EXECUTION-TIMELINE.md` / `EXECUTION-SPINE.yaml`.

## 7. End-of-cycle rule

A cycle is not complete because code was written.

It is complete only when:

1. the selected Work Item `exit_gate` is demonstrably satisfied;
2. tests/UAT/evidence required by the context pack pass;
3. architecture/dependency rules still hold;
4. durable evidence is attached;
5. the semantic Work Item becomes `SHIPPED`, `ABSORBED` or `SUPERSEDED`;
6. planning state is reconciled;
7. the agent recomputes the next Work Item from the canonical spine.

## 8. Final condition

The current evolution plan ends only at `GA-002` with evidence.

The agent MUST NOT invent post-GA work under this plan. Post-GA work starts from a new versioned plan.

## 9. Minimal instruction for an external LLM

The following is sufficient to bootstrap a capable agent:

> Continue SDDK by reading `docs/sddk-decision-kernel-architecture/02-roadmap/LLM-START-HERE.md`. Follow the canonical execution spine exactly. Select only the current semantic Work Item, load its mandatory context through `CYCLE-CONTEXT-MAP.yaml`, record the cycle context snapshot, satisfy the Work Item exit gate with durable evidence, update planning state, and only then compute the next Work Item. Never infer roadmap order from historical cycle numbers or old evolution documents.