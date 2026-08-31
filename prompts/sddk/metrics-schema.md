# SDDK Metrics Schema v1

Per-cycle metrics stored as `sddk/{cycle_id}/metrics.json` (filesystem) and mirrored as Engram observation with `topic_key: cycle-metrics/{cycle_id}`. Aggregates stored as `metrics/aggregate` (rolling 7d/30d).

## Why measure

Operational excellence requires comparable history. Without metrics, every triage is a guess; with metrics, jurisprudence becomes evidence-based.

## Level A — Process (per cycle)

| Field | Type | Source |
|-------|------|--------|
| `cycle_id` | string | YYYY-MM-DD-{goal-slug} |
| `goal_pattern` | string | normalized goal (no specifics) |
| `context_quality_start` | C0\|C1\|C2\|C3 | triage gate |
| `path_taken` | B-direct\|A-min\|A-lite\|A-full | triage gate |
| `phase_durations_sec` | object {explore, propose, spec, design, tasks, apply, verify, archive} | phase-telemetry |
| `coherence_scores` | array of int (only phases that ran a check) | coherence agent |
| `correction_cycles` | int | apply↔verify loop count |
| `tokens_used` | int | phase-telemetry estimate |
| `cost_estimate_usd` | float | phase-telemetry |
| `first_pass_success` | bool | verify verdict first run |

## Level B — Quality (per cycle)

| Field | Type | Source |
|-------|------|--------|
| `spec_scenarios_total` | int | spec artifact |
| `spec_scenarios_passing` | int | verify report |
| `loc_changed` | int | git diff stat |
| `commits_total` | int | git log |
| `commits_reverted` | int | git log + revert detection |
| `pr_size_loc` | int | PR API or diff stat |
| `chained_pr` | bool | review budget guard |
| `adrs_created` | int | docs/adr/ count delta |
| `adrs_superseded` | int | status field changes |
| `test_pyramid` | {unit: int, integration: int, e2e: int} | project test runner |

## Level C — Knowledge (per cycle)

| Field | Type | Source |
|-------|------|--------|
| `jurisprudence_hits` | int | mem_search results count |
| `jurisprudence_helps` | bool | did prior cycle shorten this one? |
| `lenses_used` | array of string | launch plan |
| `lenses_skipped` | array of {lens, reason} | launch plan |
| `escalations_triggered` | int | grill/ADR/journal log |

## Level D — Outcome (per cycle)

| Field | Type | Source |
|-------|------|--------|
| `verify_verdict` | PASS\|PW\|FAIL | sddk-verify |
| `merged_to_main` | bool | git ls-remote |
| `tag_version` | string | git tag |
| `lead_time_hours` | float | goal received → main merge |
| `on_main_at` | ISO8601 | git log |

## Aggregate (rolling 7d/30d)

Stored as `metrics/aggregate` in Engram. Updated by F3 tuner after each cycle.

```
first_pass_success_rate: 0.78
median_lead_time_hours: 5.1
median_correction_cycles: 1.2
median_cost_usd: 1.20
top_bottleneck_phase: apply (count: 12)
path_distribution: {B-direct: 3, A-min: 8, A-lite: 5, A-full: 2}
verdict_distribution: {PASS: 14, PW: 3, FAIL: 1}
```

## User-visible summary (result contract tail)

At the end of each cycle the orchestrator prints:

```
✓ Cycle {goal} closed
  Path: {path} (C{x}, jurisprudence: {hit_count} hits)
  Verdict: {verdict} {first_pass_badge}
  Lead time: {h}h  |  Cost: ${usd}  |  Tokens: {n}
  Spec coverage: {passing}/{total} scenarios ({pct}%)
  main @ {tag} ({sha})
  Bottleneck: {phase} ({reason})
  Saved as jurisprudence: {topic_key} {if reusable}

  vs rolling {window}:
    - first_pass_success_rate: {value} ({delta})
    - median_lead_time: {value}h ({delta})
    - top_bottleneck_phase: {phase} ({you_too|new})
```

## Privacy and storage

- Per-cycle JSONL append-only in `$SDDK_DATA_DIR/projects/<project_id>/metrics/{cycle_id}.jsonl` (XDG, outside the repo — ADR-0011)
- Engram mirror for cross-session retrieval (project scope)
- No PII in metrics; goal_pattern is normalized, not verbatim
- Cost fields are estimates, not billing data

## What NOT to measure

- Lines of code written (vanity metric)
- Number of files touched (low signal)
- Tokens without cost normalization (depends on model pricing)
- Coherence score when no coherence check ran (would dilute distribution)

---

## Level E — Teleological Indicators (NEW v2)

Based on Haidemariam (Frontiers in AI, 2026) — "synthetic teleology" framework. Three indicators measure **goal-directedness** vs raw activity:

### E1 — Teleological Coherence

> "What fraction of agent actions serve the declared goal?"

| Field | Type | Source |
|-------|------|--------|
| `teleological_coherence_pct` | float 0-100 | `spec_scenarios_passing / spec_scenarios_total * 100` |
| `spec_scenarios_total` | int | spec artifact |
| `spec_scenarios_passing` | int | verify report |
| `spec_scenarios_not_run` | int | verify report (gap signal) |

**Interpretation**:
- ≥ 90%: high coherence — agent's actions are goal-aligned
- 70-89%: medium — some drift; investigate skipped scenarios
- < 70%: low coherence — possible scope drift or wrong spec interpretation

### E2 — Adaptive Recovery

> "How fast does the system recover from failure?"

| Field | Type | Source |
|-------|------|--------|
| `adaptive_recovery_hours` | float | `(verify_pass_at - previous_verify_fail_at) / 3600` |
| `consecutive_passes_after_fail` | int | runs counter |
| `fail_to_pass_ratio_30d` | float | rolling 30d |

**Interpretation**:
- < 4h: excellent — fast iteration
- 4-24h: good — typical for complex changes
- > 24h: poor — investigate blocked states

### E3 — Reflective Efficiency

> "Does the cost of self-evaluation justify the benefit?"

| Field | Type | Source |
|-------|------|--------|
| `reflective_efficiency_ratio` | float | `cycles_saved_by_coherence / coherence_check_cost_usd` |
| `coherence_check_cost_usd` | float | sum of coherence phase costs |
| `cycles_saved_by_coherence` | int | count of cycles where coherence check blocked a downstream failure |

**Interpretation**:
- > 5: coherence checks are net positive
- 1-5: marginal — consider relaxing coherence
- < 1: coherence is overhead — F3 should tune to skip

### Why these matter

Raw process metrics (tokens, duration, cost) measure **activity**. Teleological indicators measure **purposefulness**. An agent can run for hours, consume many tokens, and still produce nothing aligned with the goal. These three indicators catch that.

Source: Haidemariam, T. (2026). *From the logic of coordination to goal-directed reasoning: the agentic turn in artificial intelligence*. Frontiers in Artificial Intelligence, 8. DOI: 10.3389/frai.2025.1728738.

---

## Per-loop cost (NEW v2)

Oracle research (March 2026): agents consume 4-15x more tokens than chat. To know which loop is expensive:

| Loop | Cost field |
|------|-----------|
| L1 triage | `cost_triage_usd` |
| L2 phase | `cost_phase_usd: {explore, propose, spec, design, tasks, apply, verify, archive}` |
| L3 per-task | `cost_per_task_usd: {task_id}` |
| L4 apply↔verify | `cost_correction_cycle_usd` |
| L5 cycle | `cost_cycle_usd` (sum) |
| L6 F3 | `cost_f3_usd` |

Stored per cycle. F3 tuner uses this to recommend: "L3 cost is 60% of cycle → reduce per_task_max_attempts to 3".

---
