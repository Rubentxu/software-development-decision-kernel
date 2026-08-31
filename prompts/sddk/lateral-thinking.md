# Lateral Thinking — F3 Self-Improving Tuner (default-on)

F1 (Crystallize) and F4 (Speculative) are opt-in. See `lateral-thinking-optin.md`.

## F3: Self-Improving Pipeline Tuner

**Trigger**: After every completed SDDK cycle, the orchestrator analyzes telemetry and aggregate metrics to tune the next cycle.

**What it does**: Reads metrics, coherence scores, phase outcomes, and jurisprudence. Produces a tuning recommendation block that influences the next cycle's triage.

## Self-tuning signals

| Signal | Source | Recommended action |
|--------|--------|-------------------|
| `first_pass_success_rate > 0.85` over 7d | aggregate metrics | Allow A-min path for C1 (skip coherence at apply→verify) |
| `coherence_score` consistently > 85 at one transition | telemetry | Drop that coherence check from default path |
| `coherence_score` drops at spec→tasks consistently | telemetry | Require deeper spec (more scenarios) before tasks |
| `correction_cycles > 2` on same task type | telemetry | Add intermediate verification inside apply for that task type |
| Same escalation trigger fires 3x | Engram memory | Promote to ADR (resolve permanently) |
| `top_bottleneck_phase == apply` for 3 cycles | aggregate | Lower apply parallelism, raise test coverage bar |
| Path distribution skewed to A-full (>70%) | aggregate | Investigate context quality misclassification |
| `teleological_coherence_pct < 70` (Level E) | metrics-schema | Spec is wrong or scope drift — escalate, don't auto-tune |
| `adaptive_recovery_hours > 24` (Level E) | metrics-schema | Blocked states accumulating — investigate apply/verify loop |
| `reflective_efficiency_ratio < 1` (Level E) | metrics-schema | Coherence checks are overhead — drop them for that path |
| `loop.no_progress` events > 2 per cycle | telemetry | Per-task attempt limit too low OR spec ambiguous; raise limit OR replan |
| `cost_L3 > 60% of cost_L5` | metrics-schema | Reduce per_task_max_attempts from 5 to 3 |

## Tuning output (injected into next cycle's launch plan)

```markdown
## Pipeline Tuning (from prior cycles)

- Recommended skip: {phase} ({reason})
- Recommended deepen: {phase} ({reason})
- Recommended lens: {lens} ({reason})
- Recommended path bias: {B-direct|A-min|A-lite|A-full} for context_quality {C?}
- Circuit threshold: {N} ({reason})
```

## Implementation

1. After `sddk-archive` (or end of any path), orchestrator reads `metrics/aggregate` from Engram.
2. Applies the signals table.
3. Writes tuning block to `{cycle-artifacts-dir}/{next_change}/tuning.md` (XDG, never inside the repo — ADR-0011).
4. Next cycle's triage reads `tuning.md` and adjusts path/effort before launch.

## Cost

One `mem_search` + one `mem_get_observation` + one write. Negligible (<2s, <100 tokens). Always worth it.

## When to disable F3

Only when the project is brand new (no cycles yet) or when aggregate metrics are <3 cycles old (insufficient sample).
