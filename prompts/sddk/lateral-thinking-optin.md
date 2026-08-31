# Lateral Thinking — F1 and F4 (opt-in)

These patterns extend SDDK beyond its linear reference flow. **Not default-on.** Use only when the launch plan justifies them.

## F1: Crystallize Pattern

**Trigger**: `propose` or `design` phase where 2+ fundamentally different approaches exist (not variations of the same idea).

**What it does**: Before committing to one approach, enumerate the decision space.

**Crystallize questions** (ask before selecting):
1. What are the 2-3 fundamentally different approaches?
2. What does each optimize for?
3. What does each sacrifice?
4. Which is hardest to reverse? (needs most scrutiny)
5. Can the hard decision be deferred to a later phase?

**Output injected into proposal/design**:

```markdown
## Decision Space

| Approach | Optimizes | Sacrifices | Reversibility |
|----------|-----------|------------|---------------|
| A | X | Y | high/medium/low |
| B | ... | ... | ... |

## Crystallized Decision

Chosen: A
Reasoning: {why A over B and C}
Open Questions: {what remains uncertain}
Deferrable: {what can wait}
```

**When NOT to use**: C3 with one dominant approach; simple bug fix; time-constrained path (B-direct, A-min).

## F4: Speculative Execution

**Trigger**: `design` phase when 2+ architecturally distinct approaches are viable AND the cost of exploring them is justified.

**What it does**: Run 2-3 design hypotheses in parallel, then select based on explicit criteria.

**Flow**:

```
design starts
   ↓
crystallize decision space (F1)
   ↓
2-3 hypotheses viable?
   ↓ YES
[Design A]  [Design B]  (parallel via task)
   ↓             ↓
   └──────┬──────┘
          ↓
   compare & select
          ↓
   chosen design
```

**Comparison criteria** (must be explicit before running):

```markdown
## Speculative Comparison Criteria

- Correctness: Which solves ALL requirements?
- Simplicity: Fewer concepts/interfaces/moving parts?
- Changeability: Easier to modify when requirements change?
- Reviewability: Reviewable in 30 minutes?
- Testability: Easier to test at unit/integration level?

Weight: {criteria sorted by importance for this change}
```

**Max budget**: 2x time of a single design. If 2 candidates still competitive after budget → pick the simpler one.

**When NOT to use**: Known best approach exists (established pattern); parallel cost > benefit; C0/C1 (crystallize first).

## Orchestrator integration

Add to launch plan when lateral thinking applies:

```yaml
lateral_thinking: F1 | F4 | F1+F4 | None
lateral_config:
  crystallize_questions: [list]   # F1
  speculative_hypotheses: [A, B]  # F4
  comparison_criteria: {...}      # F4
```

**Decision tree**:

```
Phase is propose or design?
├── Multiple valid approaches exist?
│   ├── YES → Consider F1 (Crystallize)
│   └── NO → 2+ hypotheses viable?
│       ├── YES → Consider F4 (Speculative)
│       └── NO → Proceed normally
```

**Cost**: F1 = ~1 extra prompt + 1 turn per approach. F4 = 2-3x design phase cost. Use sparingly.
