# Tool-Use Process Mining for Agent Interface Improvement

**Status:** Later GCI capability; not required for Agent-First v1.

## Goal

Use real Event Ledger tool trajectories to discover where the agent-facing interface leaks deterministic complexity.

## Derived trajectory

```text
Goal / normalized intent
  → ToolCall A
  → ToolCall B
  → ToolCall C
  → Result
```

## Signals

### Frequent subsequence

Example:

```text
cycle.status
→ cycle.lock.status
→ artifacts-dir
→ evaluate-gate
→ transition
```

Potential interpretation:

```text
several low-level operations represent one stable semantic goal
```

### Help-before-use

High:

```text
help → command → invalid → help → command
```

Potential interface/document/schema problem.

### Same-state repeated read

If state fingerprint is unchanged:

```text
query X
→ query X
```

may indicate missing ContextCapsule reuse or unclear output.

### Result overfetch

Large tool output with small downstream use may justify a smaller structured projection.

## Deterministic first

Sequence counts, fingerprints and basic mining are deterministic.

LLM interpretation occurs only after a typed PatternSignal.

## GCI lifecycle

```text
ToolTrajectory
→ PatternSignal
→ InterfaceImprovementProposal
→ Goal/Tool Candidate
→ Workflow Laboratory
→ parity + efficiency evaluation
→ promote/reject
```

## Prohibited shortcut

Do not automatically turn every frequent sequence into a macro.

Frequency does not prove semantic equivalence.

## Comparison

A candidate tool/goal must preserve SPEC-049 completeness.

## Future strategies

Potential research:

- schema dependency/hypergraph planning;
- process-model discovery;
- trajectory clustering by goal;
- route-specific tool pruning.

These remain replaceable strategies.
