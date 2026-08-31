---
name: chronos-mcp
description: >
  Time-travel debugging for AI agents via Chronos MCP tools. Capture once, query everything in parallel.
  Trigger: When debugging programs, investigating crashes, performance regressions, data races, memory corruption,
  variable tracing, or comparing execution traces across builds/environments.
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.0"
---

## Mental Model

Chronos is NOT an interactive debugger. It is a **frozen trace database**.

```
debug_run(program) → session_id (immutable, read-only)
                          ↓
          query N tools IN PARALLEL → synthesize
```

Think: `debug_run` = ETL. All other tools = SQL queries against a frozen DB.

---

## The 5-Level Pyramid (never skip levels)

```
1. CAPTURE      → debug_run / debug_attach
2. ORIENTATION  → [summary + saliency + threads] ALWAYS parallel, ALWAYS first
3. BULK         → crash / races / hotspot / regression  (parallel, symptom-driven)
4. FORENSICS    → memory_audit / causality / variable_origin  (parallel, address-driven)
5. DRILL-DOWN   → query_events / call_stack / variables  (parallel, event-driven)
```

**Rule:** Never call drill-down tools without orientation data. Never use `state_diff` on random timestamps.

---

## Tool Taxonomy (29 tools)

| Tier | Tools |
|------|-------|
| 🔴 Capture | `debug_run`, `debug_attach` |
| 🟢 Orientation (always first, parallel) | `get_execution_summary`, `debug_get_saliency_scores`, `list_threads` |
| 🟡 Bulk analysis | `debug_find_crash`, `debug_detect_races`, `debug_expand_hotspot`, `performance_regression_audit`, `debug_call_graph` |
| 🟡 Forensics | `forensic_memory_audit`, `inspect_causality`, `debug_find_variable_origin` |
| ⚪ Drill-down | `query_events`, `get_call_stack`, `evaluate_expression`, `debug_get_variables`, `state_diff`, `debug_diff`, `get_event` |
| ⚪ Raw | `debug_get_memory`, `debug_get_registers`, `debug_analyze_memory` |
| ⚪ Sessions | `save_session`, `load_session`, `list_sessions`, `delete_session`, `drop_session`, `compare_sessions` |

---

## Critical Patterns

### Step 1 — Capture
```json
{ "tool": "debug_run", "params": { "program": "./my-binary", "trace_syscalls": true } }
```
Wait for `session_id`. For long-running services, add `"auto_save": true`.

**Python / Node.js / Java / Go** — must specify debug port:
```json
{
  "tool": "debug_run",
  "params": {
    "program": "my_script.py",
    "program_language": "python",
    "debug_host": "127.0.0.1",
    "debug_port": 5678,
    "wait_for_connection": true
  }
}
```

### Step 2 — Orientation (ALWAYS parallel)
```json
[
  { "tool": "get_execution_summary",      "params": { "session_id": "$S" } },
  { "tool": "debug_get_saliency_scores",  "params": { "session_id": "$S", "limit": 10 } },
  { "tool": "list_threads",               "params": { "session_id": "$S" } }
]
```

### Step 3 — Bulk (choose by symptom, run parallel)

| Symptom | Tools to run in parallel |
|---------|--------------------------|
| Crash / signal | `debug_find_crash`, `debug_call_graph`, `debug_expand_hotspot` |
| Data race | `debug_detect_races`, `list_threads`, `debug_call_graph` |
| Slow / CPU | `debug_expand_hotspot`, `debug_get_saliency_scores`, `performance_regression_audit` |
| Memory growth | `forensic_memory_audit(address)`, `debug_expand_hotspot` |

### Step 4 — Forensics (only after bulk narrows scope)
```json
[
  { "tool": "forensic_memory_audit",      "params": { "session_id": "$S", "address": $ADDR, "limit": 100 } },
  { "tool": "inspect_causality",          "params": { "session_id": "$S", "address": $ADDR } },
  { "tool": "debug_find_variable_origin", "params": { "session_id": "$S", "variable_name": "count", "limit": 50 } }
]
```

### Step 5 — Drill-down (only after forensics identifies event_id or time range)
```json
[
  { "tool": "get_call_stack",       "params": { "session_id": "$S", "event_id": $EV } },
  { "tool": "debug_get_variables",  "params": { "session_id": "$S", "event_id": $EV } },
  { "tool": "debug_get_registers",  "params": { "session_id": "$S", "event_id": $EV } }
]
```

---

## Anti-Patterns (DO NOT do these)

| ❌ Anti-pattern | ✅ Correct |
|----------------|-----------|
| `query_events` with no filters | Always filter by `event_types`, `function_pattern`, or time range |
| Sequential tool calls | Batch all parallel-safe calls in ONE round-trip |
| `get_event` in a loop | `query_events` with filters for N events at once |
| Drill-down before orientation | Always run orientation first |
| `state_diff` with random timestamps | Only after `debug_find_crash` narrows the window |
| Forget to `save_session` | Use `auto_save: true` or call `save_session` immediately after |
| Python/JS without `debug_port` | Always include `debug_port` + `wait_for_connection` |

---

## Prompt Examples

### "The service crashed with SIGSEGV"
```
1. debug_run(program) → session_id
2. [get_execution_summary, debug_find_crash, list_threads] (parallel)
3. [get_call_stack(crash_event_id), debug_get_variables(crash_event_id)] (parallel)
```
Extract: crash function, call stack, variables at crash point, signal type.

---

### "P99 latency went up 40ms after deploy"
```
1. debug_run(v1, auto_save:true) + debug_run(v2, auto_save:true) (parallel)
2. [performance_regression_audit(baseline=v1, target=v2), debug_get_saliency_scores(v1), debug_get_saliency_scores(v2)] (parallel)
```
Extract: `regression_score`, which functions degraded, new functions in hot path.

---

### "Intermittent crash, suspect data race"
```
1. debug_run(program) → session_id
2. [debug_detect_races(threshold_ns=100), list_threads, get_execution_summary] (parallel)
3. If races found: [query_events(thread_id=A, event_types=[memory_write]), query_events(thread_id=B)] (parallel)
```
Extract: racing addresses, timestamp ordering of conflicting writes.

---

### "Variable `total` becomes -1 unexpectedly"
```
1. debug_run(program) → session_id
2. [get_execution_summary, debug_find_variable_origin(variable_name="total")] (parallel)
3. [query_events(function_pattern="*total*", timestamp_range)] if needed
```
Extract: all mutations to `total`, first assignment of -1, call path.

---

### "Load last Tuesday's production incident"
```
1. load_session("incident_0420_1432")
2. [get_execution_summary, debug_find_crash, debug_get_saliency_scores] (parallel, all on same session_id)
```
Sessions are immutable — reproducible analysis any time, by any agent.

---

### "Staging works, production fails"
```
1. load_session("staging_baseline") + load_session("prod_trace") (parallel)
2. [compare_sessions(a=staging, b=prod), performance_regression_audit(baseline=staging, target=prod)] (parallel)
```
Extract: functions present in prod but not staging, call count differences.

---

### "CI gate: fail build if > 10% regression"
```
1. debug_run(new_binary, auto_save:true) → current_id
2. load_session("baseline_sha") + performance_regression_audit(baseline, current) (parallel)
```
Decision: `if regression_score > 0.1 → fail build`

---

## Session Management

```json
// Save (persist across server restarts)
{ "tool": "save_session", "params": { "session_id": "$S", "language": "rust", "target": "./service" } }

// Load saved session
{ "tool": "load_session", "params": { "session_id": "incident_0420_1432" } }

// Compare two sessions
{ "tool": "compare_sessions", "params": { "session_a": "baseline", "session_b": "current" } }
```

Sessions are immutable: re-query hours later, share with teammates, load in CI — identical results.

---

## Language Quick Reference

| Language | Prerequisites | Key params |
|----------|---------------|------------|
| Rust / C / C++ | Debug symbols (`-g`) | `trace_syscalls: true` optional |
| Python | `python -m debugpy --listen 127.0.0.1:5678 --wait-for-client script.py` | `program_language: python`, `debug_port: 5678`, `wait_for_connection: true` |
| Node.js | `node --inspect=127.0.0.1:9229 server.js` | `program_language: nodejs`, `debug_port: 9229` |
| Java | JVM arg: `-agentlib:jdwp=transport=dt_socket,server=y,address=*:5005` | `program_language: java`, `debug_port: 5005` |
| Go | Delve: `dlv exec --headless --listen=:38657 ./server` | `program_language: go`, `debug_port: 38657` |

---

## Token Economy

- `get_execution_summary` → ~500 tokens ✅
- `debug_find_crash` → ~200 tokens ✅
- `query_events` **without filters** → potentially millions of events ❌ overflow
- Always use `limit`, `timestamp_start`, `timestamp_end`, `function_pattern` to constrain `query_events`

Stop as soon as you have an answer. Not every investigation needs all 5 levels.
