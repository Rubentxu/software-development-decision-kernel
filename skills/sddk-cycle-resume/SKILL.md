---
name: sddk-cycle-resume
description: "Trigger: orchestrator rebuilds state after session compaction, restart, or explicit /sddk-continue. Pull-based state reconstruction from authoritative CLI queries — never from in-memory phase envelopes. Returns a validated cli_context the orchestrator uses for Gate 0 pre-flight checks."
disable-model-invocation: true
user-invocable: false
license: MIT
metadata:
  author: SDDK Team
  version: "1.0"
  trigger_after_compaction: true
---

> **ORCHESTRATOR GATE**: If you loaded this skill, STOP. Run this skill
> **inline** (do NOT delegate) — it is the orchestrator's own state
> reconstruction, not a phase agent's work.

## Why this skill exists

Between phases (and especially after a session compaction or restart), the
orchestrator must rebuild its working `cli_context` from the **authoritative
CLI**, not from in-memory phase envelopes. Phase envelopes are push-based and
can be lost; the CLI is the source of truth (XDG-backed, ADR-0011).

Load and follow `skills/_shared/cli-usage-contract.md`.

## When to load

- Session start, BEFORE the first triage call.
- After any compaction event (per AGENTS.md "AFTER COMPACTION").
- Before each phase delegation in interactive mode (`Pre-flight Gate 0`).
- When `/sddk-continue` is invoked (mid-cycle resumption).

## CLI reconstruction steps

Require an absolute `PROJECT_ROOT` from the orchestrator. Run these in order.
Every command is fail-closed: capture exact argv, exit code, parsed stdout, and
stdout/stderr digests; stop reconstruction on any result other than a documented
`not_found`.

```bash
# 1. Capture the CLI version and resolve adoption/knowledge once.
sddk --version
sddk adopt status --root "$PROJECT_ROOT" --scope . --format json
sddk knowledge status --root "$PROJECT_ROOT" --scope . --format json

# 2. Rebuild cycle state only from a trusted cycle ID supplied by the launch
# packet, persisted cycle artifact, or explicit user continuation request.
sddk cycle status --root "$PROJECT_ROOT" --scope . \
  --cycle "$CYCLE_ID" --format json
sddk cycle artifacts-dir --root "$PROJECT_ROOT" --scope . \
  --cycle "$CYCLE_ID" --format json

# 3. Reconstruct the recent causal chain.
sddk ledger events --root "$PROJECT_ROOT" --scope . \
  --limit 10 --format json

# 4. Validate the exact vault_path parsed from knowledge status.
sddk vault validate --root "$PROJECT_ROOT" --scope . \
  --vault "{vault_path parsed from knowledge status}" --format json
```

Steps 2a and 2b are omitted when no trusted cycle ID exists. Parse
`PROJECT_ID`, `KNOWLEDGE_VAULT_PATH`, profile presence, vault presence, and
Engram status from the successful knowledge-status object before step 4. Set
`cli_version` from step 1 and `observed_at` at completion. Every field in
`source_commands` comes from the captured command record; no command may be
listed unless it actually ran.

The baseline has no global active-cycle discovery command. `cycle lock status`
requires a known `--cycle`, and `cycle status` already returns that cycle's
lease. The current runtime does not serialize distinct cycle IDs project-wide.
Without a trusted cycle ID, leave cycle fields null and return `blocked` for
automated start/resume with `runtime-active-cycle-discovery-unavailable`. A
human may explicitly accept that unresolved conflict risk, but the result must
not claim project-wide serialization.

## cli_context envelope

```json
{
  "cli_version": "<semver>",
  "observed_at": "<RFC3339>",
  "project": {"root": "<absolute-path>", "project_id": "<stable-uuid>", "adopted": true},
  "knowledge": {"vault_path": "<xdg-vault-path>", "profile_present": true, "engram_enabled": false},
  "cycle": {"cycle_id": "<id|null>", "status": "<status|null>", "phase": "<phase|null>", "path": "<path|null>", "updated_at": "<RFC3339|null>"},
  "lease": {"owner": "<owner>", "fencing_token": 1, "expires_at_ms": 0},
  "cycle_artifacts_dir": "<absolute-path|null>",
  "ledger_events": [/* last 10 */],
  "vault_validation": {"valid": true},
  "source_commands": [{"argv": [], "exit_code": 0, "output_digest": "<sha256>"}]
}
```

Set `lease` to `null` when cycle status returns no lease. Never invent fields
such as cycle branch/head SHA or vault drift count when the command schema does
not provide them. Git subject identity belongs to the launch plan, not runtime
cycle state.

## Hard rules

| Condition | Action |
|-----------|--------|
| Adoption is absent | BLOCK with `next_recommended=/sddk-adopt` |
| Knowledge profile or vault validation is invalid | BLOCK and return the exact CLI recovery action |
| Cycle ID is unknown | Leave cycle/lease/artifact fields null; block automated start/resume and request trusted-ID recovery or explicit human risk acceptance |
| Cycle status is closed | Return the legal next action derived from status/phase; do not resume a worker |
| Lease is absent or expired for a mutation | BLOCK or renew under `cli-usage-contract.md`; never fabricate lease flags |
| Fencing token differs from the prior trusted context | BLOCK as lease desynchronization; do not mutate cycle state |
| Any authoritative command has an invalid invocation or corrupt output | BLOCK with argv, exit code, output digest, and recovery action |

## Return Format

- status: success | partial | blocked
- executive_summary: one sentence describing the rebuilt state
- cli_context: the JSON envelope above
- next_recommended: phase to dispatch, trusted-ID recovery, or explicit human risk decision
- risks: "vault-invalid" / "cycle-id-unknown" / "lease-desync" or "None"
- rebuild_source: ["cli:adopt-status", "cli:knowledge-status", "cli:cycle-status", "cli:cycle-artifacts-dir", "cli:ledger-events", "cli:vault-validate"]

## Difference from existing patterns

- **`mem_session_summary`** (Engram) — persists narrative across sessions.
- **`sddk-continue-options`** (skill) — presents tablet-friendly options.
- **`sddk-cycle-resume`** (this skill) — rebuilds **authoritative state** from the CLI. Other patterns consume its `cli_context` output.

## Reference

- `prompts/sddk/orchestrator.md` § Pre-flight Gates (Gate 0)
- `prompts/sddk/mcw.md` § Phase 0 Step 0.2 (lock check)
- `prompts/sddk/status-query.md` (manual status queries)
- ADR-0011 — XDG-backed persistence, never repo-local SDDK state
