# DRAFT-ADR-F — Complexity budget as trend metric

> **Status**: DRAFT (not accepted). Awaiting cycle-54+ adoption.
> **Date**: 2026-08-31
> **Author**: deep-research-orchestrator (read-only research)
> **Cycle binding**: none yet; candidate cycle-54 (depends on ADR-B)
> **Amends**: none (additive)
> **Supersedes**: none
> **Authority target**: `crates/sddk-cli/src/metrics.rs`

---

## Pre-decision: context, decisions, alternatives, compatibility, authority, migration

### Context

The framework exposes per-cycle metrics (Levels A-E + L1-L6 costs in
`crates/sddk-cli/src/metrics.rs`), but no published policy budgets a
per-cycle gate cost. The 5 Wave-1 budget gates
(`tests-pass`, `policy-compliant`, `debt-severity-assigned`,
`debt-priority-assigned`, `bounded-execution`) have no observable
complexity ceiling.

The system trap is **"Seeking the Wrong Goal"**: more gates = more
safety (false). Without a budget, drift toward "more gates" is
unmeasured.

### Decision (proposed)

Introduce a `gate_complexity_budget` metric:

```rust
pub struct GateComplexityBudget {
    pub cycle_id: CycleId,
    pub gate_count: u32,           // total gates evaluated
    pub avg_gate_eval_ms: u64,     // mean wall time per gate
    pub recovery_action_count: u32, // how many auto-recoveries fired
    pub trend: ComplexityTrend,    // Stable | Increasing3x | Decreasing3x
}
```

The metric is **reported**, not enforced. The orchestrator emits
`complexity.trend.detected` when `trend != Stable` for 3 consecutive
cycles; this event is informational and does NOT block.

### Alternatives considered

| # | Alternative | Why rejected |
|---|---|---|
| F1 | Hard cap on gate count per cycle | Breaks legitimate work; no human can predict the cap |
| F2 | Threshold on per-gate wall time | Too sensitive to hardware variance |
| F3 | LLM-evaluated "is this gate worth it?" | Shifting the Burden; non-deterministic |
| F4 | Trend-based blocking (not just reporting) | Violates "recover forward"; turns budget into a wall |

### Compatibility with current ledger

- **New metric type**, no existing type renamed.
- **New event**: `complexity.trend.detected` (informational).
- **No ledger digest change** (events are additive).
- **Metrics file format unchanged** (`debt-report.json` gains an optional
  `gate_complexity_budget` field; backward compatible).

### Authority limits

- The budget is a **metric, not a rule** (per lateral-thinking L7).
- The trend detector fires after **3 consecutive cycles** with the same
  trend direction. One outlier is not a trend.
- Human override: any cycle may carry an `override_complexity_trend`
  receipt (with owner + justification + caducity, per ADR-0047).

### Migration path

1. **Phase 1 (this research)**: ADR-F drafted; lateral-thinking L7
   documented.
2. **Phase 2 (cycle-54 candidate, A-min)**: implement `GateComplexityBudget`
   metric + trend detector. RED test first.
3. **Phase 3 (cycle-55+)**: integrate with `sddk run` facade verb (Wave
   4 completion) — budget is shown in `sddk status`.

### Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Trend detector false positives | medium | low | 3 consecutive cycles; human override |
| Budget metric adds overhead | low | low | Metric is opt-in via flag; off by default |
| LLM game the metric | low | medium | Metric is computed from ledger, not LLM input |

---

## Post-decision: formal sections (for adoption)

### Status

(pending acceptance)

### Date

(pending acceptance)

### Consequences

(pending)

### Implementation notes

- New struct: `GateComplexityBudget` in `crates/sddk-cli/src/metrics.rs`.
- New event: `complexity.trend.detected`.
- Modified: `debt-report.schema.json` (optional field).

### Compatibility / migration

See Phase 1–3 above.

### Revisit trigger

Revisit when:

- `complexity.trend.detected` becomes a routine event (sign of chronic
  drift).
- A new gate class emerges (combined with ADR-B).

### Implementation trace

- **cycle-54** (target): implements complexity budget. Refer to
  `research/cycle-supersede-replan/evidence-cards/ec-css-006-gate-cost.yml`
  and `research/cycle-supersede-replan/lateral-thinking-proposals.md`
  (L7).

---

## References

- `crates/sddk-cli/src/metrics.rs` (Levels A-E + L1-L6)
- `docs/sddk-decision-kernel-architecture/02-roadmap/ROADMAP.md` §Wave 1.5 (5 budget gates)
- `docs/debt/debt-report.schema.json` (report schema)
- `docs/adr/ADR-0047-durable-debt-remediation.md` (override discipline)
- `research/cycle-supersede-replan/lateral-thinking-proposals.md` (L7 — "metric, not rule")