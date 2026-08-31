---
name: uat-evidence
description: "Trigger: uat-evidence, captura evidencia, screenshot UAT. Capture and hash UAT evidence in the browser (clipboard API, MediaRecorder) and reference it by SHA-256 in uat-session.yaml. Documents the full human flow: plan → wizard → finalize → JSON → ingest → failures → agent study."
disable-model-invocation: true
user-invocable: false
license: MIT
metadata:
  author: sddk-framework
  version: "1.0"
  delegate_only: true
---

> **ORCHESTRATOR GATE**: If you loaded this skill, STOP. Delegate to `uat-runner`.

## Purpose

Evidence is what makes a PASS/FAIL defensible. Every verdict should be backed by a screenshot, log snippet, or note — referenced by hash so the chain ties to the ledger (ADR-003).

## Full human-in-the-loop flow

```
uat-planner          →  uat-plan.yaml            (script)
uat-guide            →  uat-plan.yaml enriched   (junior-friendly)
uat open             →  local guided server      (same-origin ingest)
[ tester executes ]  →  verdicts + evidence      (one scenario per screen)
Finalizar y exportar →  uat-session-<rel>.json   (canonical UatSession)
uat ingest           →  CP uat_results + ledger
uat failures         →  JSON / text with FAIL/BLOCKED details
[ agent reads failures and studies each one ]
```

The tester (junior or architect) follows the `plain_steps` written by `uat-guide`. The agent NEVER writes `executor: human` sessions — see `agents/uat-runner.md` craft rule 1.

## Canonical `UatSession` shape

The JSON produced by the dashboard's "Finalizar y exportar reporte" button MUST match the `UatSession` schema in `crates/sddk-domain/src/uat.rs`:

```yaml
schema_version: 2
plan_version: 2
session_id: uat-<uuid>
plan_ref: <release candidate>
release: v1.5.0
executor: human                # NEVER written by an agent
executed_by: <tester name>
started_at: 2026-08-07T13:00:00Z
finished_at: 2026-08-07T13:14:00Z
metadata:
  tester: { id: T-0001, display: <tester name> }
  completed_at: 2026-08-07T13:14:00Z
  duration_ms: 840000
  build: { commit: <tested commit>, branch: <branch>, tag: v1.5.0, dirty: false }
results:
  - scenario_id: S-1
    status: NOT_RUN | PASS | FAIL | BLOCKED | PARTIAL
    comment: <free text>
    evidence:
      - kind: file | screenshot | command_output | assertion | metric | note
        ref: "sha256:<hash>"
        note: <optional>
        captured_at: 2026-08-07T13:03:00Z
        size_bytes: <bytes>
        mime: image/png
    duration_minutes: <int>
```

The CLI `uat ingest` validates this shape before ingesting (guards: `executor: human` requires `executed_by` + `finished_at` + evidence or non-PASS).

## Capture patterns

| Kind | How | Notes |
|------|-----|-------|
| `screenshot` | Ctrl+V paste in the guided wizard (clipboard API) | The wizard stores a data-URL preview and records `sha256:<hash>` |
| `log` | Copy a console/terminal snippet into the comment box | Reference `sha256:<hash>` of the snippet |
| `note` | Free text | For observations that are not visual |

## What the agent does with failures

```bash
sddk uat failures --release v1.5.0 --sessions uat-session-*.json
sddk uat failures --release v1.5.0 --sessions ... --format json
```

Output per finding: scenario_id, status, feature, priority, assignee, session_id, executed_by, rationale, comment, evidence. The agent uses this to:

1. Locate the failing scenario in the plan.
2. Read `rationale` to understand intent.
3. Open the codebase area touched by the scenario (use `requirement_ref` as the architectural anchor).
4. Cross-check `comment` + evidence refs against the implementation.
5. Decide: fix in a follow-up cycle, document as known issue, or mark as acceptable risk.

## Integrity rules

1. Every evidence entry has `kind` + `ref` in `sha256:<hash>` form.
2. The payload itself is stored in XDG artifacts (`~/.local/share/sddk/projects/<id>/uat/`), never in the project repo (ADR-0011).
3. The session file only carries the reference — it stays small and diffable.
4. Agents MUST NOT fabricate `executor: human` sessions.
5. Missing scenarios are exported as `NOT_RUN`; absence is never interpreted as PASS.

## Browser capabilities used

- Clipboard API (`navigator.clipboard.read` / paste event) for screenshots — no external tools.
- `crypto.randomUUID()` / `Date.now()` for session_id (with sha256 prefix in evidence).
- `localStorage`: session progress survives reloads; the same key (`sddk-<release>`) is used by the wizard so closing/reopening the browser keeps state.

## References

- `agents/uat-runner.md` — pre-flight execution with evidence
- ADR-003 (ledger hash chain), ADR-012 §4 (evidence) in the knowledge vault
- `crates/sddk-cli/src/uat.rs` — `uat ingest`, `uat failures`, integrity guard
- `assets/uat-dashboard/kit/storage.js` — `finalizeAndExport`, `fromLegacy`
