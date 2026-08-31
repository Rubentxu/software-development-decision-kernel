---
name: chronos-sdd
description: >
  Chronos runtime evidence integration for SDD phases.
  Trigger: Automatic when any sddk-* sub-agent needs runtime behavior analysis,
  crash investigation, performance regression detection, or concurrency validation.
  Applies to: sddk-explore (runtime bugs), sddk-verify (regression gate), sddk-apply (concurrency/memory features).
license: MIT
metadata:
  author: rubentxu
  version: "1.1"
---

## Purpose

Chronos provides **frozen execution traces** — immutable snapshots of how a program
actually ran. In SDD it fills the gap between "code looks correct" (CogniCode) and
"code runs correctly" (Chronos).

**Chronos is enhancement, not a requirement.** If the MCP server is unavailable,
proceed with the standard SDD phase workflow. Never block on Chronos.

Mental model: `debug_run(binary)` = ETL into a frozen DB. All other tools = queries
against that DB. Capture once, query in parallel as many times as needed.

---

## Capture Targets

Chronos supports two capture targets:

### Native Binary (default)

```
debug_run(program: "./target/release/my-binary", trace_syscalls: true, auto_save: true)
```

For Rust/C/C++/Go binaries compiled with debug info. Uses ptrace under the hood.

### Browser / WASM (agent-first, v0.4.0+)

After the agent-first reframe (v0.4.0), browser traces are captured **without
pre-filtering** — agents capture ALL events and query semantically after.

```
1. debug_run(program: "browser")  →  session_id
2. query_events(session_id, event_types: ["function_entry", "function_exit", "memory_write"])
```

Key differences from native capture:
- **No `function_filter`** — removed in v0.4.0. Agents don't pre-filter.
- **VecDeque ring buffer** — 100K event cap, OOM-proof
- **Position-based return detection** — 3% body_end threshold (toolchain-agnostic)
- **CancellationToken** — graceful shutdown via shared state

---

## When to Use Chronos in Each SDD Phase

### sddk-explore — Runtime Evidence for Bug Investigations

Use Chronos when the exploration topic involves **runtime behavior**: crashes,
performance regressions, data races, or unexpected variable values.

**Step 0 (before reading raw files, if runtime bug):**

```
1. debug_run(program, trace_syscalls: true, auto_save: true) → session_id
2. [get_execution_summary, debug_get_saliency_scores(limit:10), list_threads] (parallel)
```

Map symptoms to bulk tools (run in parallel after orientation):

| Symptom | Tools |
|---------|-------|
| Crash / signal | `debug_find_crash`, `debug_call_graph` |
| Data race | `debug_detect_races(threshold_ns:100)`, `list_threads` |
| Slow / CPU spike | `debug_expand_hotspot`, `performance_regression_audit` |
| Variable has wrong value | `debug_find_variable_origin(variable_name)` |
| Memory corruption | `forensic_memory_audit(address)`, `inspect_causality(address)` |

Surface Chronos findings in the **Exploration Findings** section:
```markdown
### Runtime Evidence (Chronos)
- session_id: {id}
- Top functions by CPU: {list from saliency}
- Crash: {signal + function if found}
- Races detected: {count}
```

If no runtime bug → skip Chronos entirely.

---

### sddk-verify — Performance Regression Gate

**Always run when a Chronos baseline session exists** (stored in engram as
`chronos-baseline/{project}`). This is the verify phase's structural equivalent
of `cognicode_check_architecture`.

**Step 1 — Capture current build:**
```
debug_run(new_binary, auto_save: true, trace_syscalls: true) → current_session_id
```

**Step 2 — Regression check (parallel with spec compliance matrix):**
```
[
  performance_regression_audit(baseline_session_id, current_session_id),
  debug_find_crash(current_session_id),
  debug_detect_races(current_session_id, threshold_ns: 100)
]
```

**Step 3 — Auto-baseline (if no baseline exists):**

If this is the first verify run for the project:
```
1. Save current session as baseline:
   mem_save(topic_key: "chronos-baseline/{project}", content: {session_id, timestamp, binary_path})
2. Skip regression check — note "first run, baseline established"
```

Future verify runs will compare against this baseline automatically.

**Decision rules:**
- `regression_score > 0.15` (>15% CPU regression) → **WARNING** in verify-report
- `regression_score > 0.30` → **CRITICAL** — block verification
- New crash not in baseline → **CRITICAL**
- New races not in baseline → **WARNING**

Add a **Chronos Regression Analysis** section to the verify-report:
```markdown
### Chronos Regression Analysis

| Check | Baseline | Current | Delta | Result |
|-------|----------|---------|-------|--------|
| Regression score | — | {score} | — | ✅ <15% / ⚠️ <30% / ❌ >30% |
| Crashes | {n} | {n} | {delta} | ✅ Same / ❌ New crash |
| Races detected | {n} | {n} | {delta} | ✅ Same / ⚠️ New races |
| Baseline | {id} | {id} | — | ✅ Established / ✅ Compared |
```

---

### sddk-apply — Concurrency and Memory Feature Validation

Use Chronos **after implementing** features that touch:
- Multi-threading / async / channels
- Raw memory / unsafe blocks / FFI
- Performance-critical hot paths

**Post-implementation validation:**
```
1. debug_run(binary, trace_syscalls: true) → session_id
2. [debug_detect_races(threshold_ns: 100), get_execution_summary, list_threads] (parallel)
3. If races found: query_events(thread_id=A, event_types=[memory_write]) to pinpoint
```

If races or crashes are found → do NOT mark the task as complete. Fix first.

---

## Capture Patterns

### Minimal capture (fast, default)
```json
{ "tool": "debug_run", "params": { "program": "./binary", "auto_save": true } }
```

### Full capture (crash / race investigation)
```json
{
  "tool": "debug_run",
  "params": {
    "program": "./binary",
    "trace_syscalls": true,
    "auto_save": true,
    "timeout_secs": 30
  }
}
```

### Always save immediately — sessions are lost on server restart without `auto_save: true`.

---

## Orientation — Always Run These First in Parallel

After any `debug_run`, NEVER jump to drill-down tools. Always orient first:

```json
[
  { "tool": "get_execution_summary",     "params": { "session_id": "$S" } },
  { "tool": "debug_get_saliency_scores", "params": { "session_id": "$S", "limit": 10 } },
  { "tool": "list_threads",              "params": { "session_id": "$S" } }
]
```

These three calls cost ~1000 tokens total and prevent 10x more expensive wrong-path investigations.

---

## Anti-Patterns

| ❌ Anti-pattern | ✅ Correct |
|----------------|-----------|
| `query_events` without filters | Always add `event_types`, `function_pattern`, or time range |
| Drill-down before orientation | Always run summary + saliency + threads first |
| Sequential tool calls | Batch all independent calls in one parallel round-trip |
| Skip `save_session` | Use `auto_save: true` on `debug_run` |
| Use Chronos for static analysis | Use CogniCode for that — Chronos is runtime only |
| Block SDD phase if Chronos unavailable | Always graceful — report "no runtime data" and continue |
| Pre-filter events with `function_filter` | Capture ALL events, query semantically after (agent-first) |

---

## Compact Rules

- `debug_run` with `auto_save: true` — always, to survive server restarts
- Orientation (summary + saliency + threads) is MANDATORY before any drill-down
- Batch all parallel-safe tools in ONE call — never sequential if independent
- `regression_score > 0.30` = CRITICAL in verify, blocks pass
- New crash in verify = CRITICAL regardless of regression score
- New races in verify = WARNING (not CRITICAL unless confirmed data corruption)
- First verify run → auto-save as baseline via `mem_save(topic_key: "chronos-baseline/{project}")`
- Chronos is enhancement — if unavailable, proceed without it and note "no runtime data"
- Never use Chronos for static analysis — that is CogniCode's domain
- Browser/WASM capture: no function_filter — capture all, query after
