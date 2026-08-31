---
id: INC-DEBT-010-rename-on-disk-field-diff-consumers
title: "Rename-on-disk: diff_existing_target emits name diff but no adapter's apply block handled it"
status: closed
severity: medium
priority: P2
cluster_id: CL-COUPLING
created: 2026-08-25
created_by: sddk-debt-verify (cycle-35 closeout)
owner: orchestrator
cycle_source: p-52b95ef55999f9de/kernel-cycle-36-inc-debt-010-rename-on-disk-field-diff-consumers
attribution: pre_existing
base_sha_when_discovered: 86d0940 (cycle-35 archive)
---

# INC-DEBT-010 — Rename-on-disk for FieldDiff consumers

> Closed at cycle-36.

## Context

`diff_existing_target` (cycle-35 wired) emits `FieldDiff { field_name: "name", ... }` when names differ. None of the 4 adapter apply blocks handled this diff:
- `json.rs` apply: fell to `_ => {}` (L290 pre-cycle-36)
- `claude.rs` apply: rewrote file from `agent.*`, ignored diffs (L215-246)
- `codex.rs` apply: rewrote file from `agent.*`, ignored diffs (L198-238)

Result: even if rename-detection were added, the file/entry wouldn't actually be renamed on disk.

## Resolution

Cycle-36:
- Added `"name"` arm to `json.rs` apply block: renames key in agents map (`agents.insert(new, agents.remove(old))`) inside the existing single atomic_write. R5 mitigation: skip if target key exists.
- Added rename path to `claude.rs` apply block: write new file + best-effort `remove_file(<old>)`. R3 mitigation: if remove fails, push to `report.errors` (orphan correctable via existing prune logic).
- Added rename path to `codex.rs` apply block: same as claude but for `.toml` files.
- Added 3 tests: `apply_renames_json_key_on_name_diff`, `apply_renames_claude_file_on_name_diff`, `apply_renames_codex_file_on_name_diff`.

**Dormant in production today**: all 4 adapters set `existing.name = lookup_key = bundle_agent_name`, so `existing.name == target.name` is invariantly true and the name diff is never emitted. The apply handlers are wired for future rename-detection mechanisms (out of scope for cycle-36).

## Lifecycle

| Date | Actor | Change | Evidence |
|------|-------|--------|---------|
| 2026-08-25 | sddk-debt-verify (cycle-35) | created | deferred item in INC-DEBT-009 spec Part C |
| 2026-08-25 | sddk-apply (cycle-36) | status: open → closed | apply handlers wired; 3 tests pass |

## Future work

Detection mechanism (pre-pass scan, bundle manifest, CLI subcommand, fuzzy similarity) — deferred to future cycle. Will activate the dormant apply handlers.
