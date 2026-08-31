# SDDK Launch Plan Helper

## Schema

This document is the prompt-layer launch-plan authority. Runtime schema work is
outside this contract.

## Quick Reference

Every launch plan MUST contain:

| Field | Required | Type | Values |
|--------|----------|------|--------|
| `goal` | Yes | string | One bounded outcome |
| `phase` | Yes | string | sddk-init, sddk-explore, sddk-propose, sddk-spec, sddk-design, sddk-tasks, sddk-apply, sddk-verify, sddk-debt-verify, sddk-release, sddk-archive, sddk-coherence |
| `path` | Yes | enum | B-direct, A-min, A-lite, A-full |
| `delivery_kind` | No | enum | code-delivery, docs-delivery, vault-only-delivery, retroactive-archive-close-delivery, managed-closure-delivery — defaults to null (standard release path) |
| `execution_mode` | Yes | enum | auto, interactive |
| `context_quality` | Yes | enum | C0, C1, C2, C3, unknown |
| `knowledge_coverage` | Yes | object | roadmap_backlog, work_items, architecture_adrs, ownership, learnings — each present/missing/stale |
| `taxonomy` | Yes | object | `dominant_axes` plus material `evidence` |
| `reversibility` | Yes | enum | HIGH, MEDIUM, LOW |
| `recommended_effort` | Yes | enum | skip, verify, deepen, recommend-lenses |
| `cycle_id` | Yes | string or null | Trusted runtime cycle ID; null before safe start/resume |
| `cycle_artifacts_dir` | Yes | absolute path or null | Null whenever `cycle_id` is null |
| `vault` | Yes | absolute path | Exact path from knowledge status |
| `framework_root` | Yes | absolute path | Loaded framework bundle/source root containing `MANIFEST.sha256`; never project CWD |
| `cli_context` | Yes | object | Envelope from `sddk-cycle-resume`; never reconstructed locally |
| `subject` | Yes | object | branch, base_commit, head_commit, diff_digest |
| `testing` | Yes | object | strict_tdd and exact commands |
| `capabilities` | Yes | array | Selected capability IDs, possibly empty |
| `skills_to_load` | Yes | array | Exact skill paths, possibly empty |
| `artifact_references` | Yes | object | Existing cycle artifacts by kind, possibly empty |
| `engram_memory` | No | boolean | Enable Engram as optional cross-session memory. Default: false |
| `with_knowledge` | No | boolean | Run scan → review → import → verify. Default: false |
| `knowledge_approved_entry_ids` | No | array of strings | Explicitly reviewed changed-entry IDs allowed after `--approve`; default empty |
| `plan_version` | Yes | string | v1, v2, ... |
| `report_locale_requested` | No | string or null | Explicit BCP 47 tag; default null |
| `report_locale` | Yes | string | Resolved BCP 47 tag; default `es` |
| `report_locale_fallback` | Yes | enum | none, project, parent-language, es |
| `report_audience` | Yes | enum | novice, standard, expert; default standard |

## Example: Minimal Valid Plan

```json
{
  "goal": "Map the authentication boundary before proposing changes",
  "phase": "sddk-explore",
  "path": "A-full",
  "execution_mode": "interactive",
  "context_quality": "C1",
  "knowledge_coverage": {
    "roadmap_backlog": "missing",
    "work_items": "missing",
    "architecture_adrs": "present",
    "ownership": "present",
    "learnings": "missing"
  },
  "taxonomy": {
    "dominant_axes": ["boundary_seam", "coupling_connascence"],
    "evidence": "shallow modules in auth/ found via grep"
  },
  "reversibility": "MEDIUM",
  "recommended_effort": "deepen",
  "cycle_id": "auth-boundary-20260824",
  "cycle_artifacts_dir": "/xdg/projects/p-123/cycle-artifacts/auth-boundary-20260824",
  "vault": "/xdg/knowledge/p-123",
  "framework_root": "/xdg/framework/1.42.1",
  "cli_context": {
    "cli_version": "1.42.1",
    "observed_at": "2026-08-24T12:00:00Z",
    "project": {"root": "/work/auth", "project_id": "p-123", "adopted": true},
    "knowledge": {"vault_path": "/xdg/knowledge/p-123", "profile_present": true, "engram_enabled": false},
    "cycle": {"cycle_id": "auth-boundary-20260824", "status": "OPEN", "phase": "explore", "path": "A-full", "updated_at": "2026-08-24T12:00:00Z"},
    "lease": null,
    "cycle_artifacts_dir": "/xdg/projects/p-123/cycle-artifacts/auth-boundary-20260824",
    "source_commands": [{"argv": ["sddk", "cycle", "status", "--root", "/work/auth", "--scope", ".", "--cycle", "auth-boundary-20260824", "--format", "json"], "exit_code": 0, "output_digest": "<sha256>"}]
  },
  "subject": {"branch": "main", "base_commit": null, "head_commit": null, "diff_digest": null},
  "testing": {"strict_tdd": true, "commands": ["cargo test --workspace"]},
  "capabilities": [],
  "skills_to_load": ["skills/sddk-explore/SKILL.md"],
  "artifact_references": {},
  "engram_memory": false,
  "with_knowledge": false,
  "knowledge_approved_entry_ids": [],
  "report_locale_requested": null,
  "report_locale": "es",
  "report_locale_fallback": "es",
  "report_audience": "standard",
  "plan_version": "v1"
}
```

## Knowledge Authority

When `with_knowledge` is true, use `scan -> reviewed plan -> import -> verify`.
Import always consumes the exact scan `plan_id`. Pass `--approve` only with the
comma-separated IDs in `knowledge_approved_entry_ids`; these IDs can authorize
only compatible changes to existing entries. New import candidates follow the
plan disposition. Unapproved changed candidates are still persisted as
`NeedsReview` and may create incidences; they are not promoted to trusted
authority. An empty approval list is valid and is not a reason to skip import.

## Versioning Rules

- Start with `v1` for each new change
- Increment when: scope changes materially, new lenses are added, different phase is targeted
- Never decrease version
- The version is stored in the artifact so downstream phases can detect stale plans

## Validation Checklist (Orchestrator)

Before injecting a launch plan into a phase prompt, verify:

- [ ] All required fields present
- [ ] `phase` matches the agent being launched
- [ ] `knowledge_coverage` reflects actual inventory
- [ ] `taxonomy.dominant_axes` has at least one entry if not skip
- [ ] `adaptive_lenses` match the recommended_effort gate
- [ ] `artifact_references` point to existing artifacts for this change
- [ ] `cycle_id`, artifact directory, vault, framework root, subject, and `cli_context` agree
- [ ] Every knowledge approval is an explicitly reviewed entry ID from the current plan
- [ ] Report locale is valid, resolved under `phase-contracts.md`, and not inferred from environment/chat
- [ ] `plan_version` is incremented if this is a revised plan

## Anti-patterns

- `context_quality: unknown` without a blocking question → BLOCKED
- `recommended_effort: deepen` with no adaptive_lenses → add or explain
- `engram_memory: true` with no Engram MCP available → WARN (persistence degraded)
- Boolean approval field or bare `--approve` → BLOCK as invalid authority
- Same `plan_version` for materially different plans → increment version
