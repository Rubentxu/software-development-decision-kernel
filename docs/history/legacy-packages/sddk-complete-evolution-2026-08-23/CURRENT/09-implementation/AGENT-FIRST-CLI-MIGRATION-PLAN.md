# Agent-First CLI Migration Plan

## 1. Do not start by adding commands

First instrument how agents use existing commands.

Questions:

```text
Which commands are preceded by --help?
Which fail due to arguments?
Which reads repeat on unchanged state?
Which command sequences recur?
Which outputs are actually consumed?
Which reports are required downstream?
```

## 2. Extract application services

Current CLI modules should increasingly become:

```text
parse args
→ construct request
→ invoke application service
→ render result
```

The application service is callable from AgentHost/stdio without shelling out.

## 3. First semantic read: `state`

Build DecisionSnapshot.

Expected immediate benefit:

- remove repeated project/cycle/lock/graph/evidence discovery calls;
- reduce prompt plumbing.

No behavior changes.

## 4. First semantic write path

Recommended:

```text
cycle.verified
```

Reason:

- bounded compared with full release/archive;
- contains meaningful verification/report obligations;
- tests deterministic+cognitive composition.

## 5. Preserve detailed reporting

Internally the goal may execute/reuse:

```text
verify
engineering assurance
debt verify
coherence/gates
```

Each owning operation still writes its detailed artifact.

GoalResult returns references.

## 6. Capability resolution refinement

Move from caller-supplied executable where possible:

```text
CapabilityRequest
    ↓
Capability Registry
    ↓
eligible provider/tool route
    ↓
typed run spec
```

Retain explicit-runner capability mode for debugging/advanced external tools.

## 7. Persistent process evaluation

After semantic consolidation, measure whether process-start/open-state overhead remains material.

Only then introduce `sddk serve --stdio`.

Do not use a daemon to solve a semantic problem.

## 8. Generated agent tool docs

Tool descriptor and examples derive from canonical contracts.

Prompts should refer to semantic tool IDs, not duplicate full syntax documentation.

## 9. Migration metrics

Per goal:

```text
legacy low-level calls
semantic calls
invalid calls
help calls
result tokens
latency
required reports present
receipts present
human corrections
```

## 10. Kill criterion

If a high-level goal reduces observability, report quality, recovery control or correctness, it does not replace the existing path.
