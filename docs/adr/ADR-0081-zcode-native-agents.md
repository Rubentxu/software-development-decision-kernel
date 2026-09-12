---
status: accepted
date: 2026-09-12
deciders: [orchestrator]
linked_cycles: []
---

# ADR-0081 — ZCode native agent registration (md files + primary commands)

## Status

Accepted — implemented in `crates/sddk-cli/src/dev/editor_adapters/zcode.rs`.

## Context

ADR-0019 modeled ZCode as an OpenCode mirror: `ZCodeAdapter` upserted an
`agent` map into `~/.zcode/zcode.json` with `{file:...}` prompt references,
`mode` and `hidden` fields, and the agents symlink surface was shared with
OpenCode (`LinkProfile::ALL`).

The ZCode desktop app never reads an agent map from `zcode.json` (its user
config, `~/.zcode/cli/config.json`, holds only MCP/hooks/plugins). It
discovers user sub-agents as markdown files under `~/.zcode/agents/<name>.md`
whose YAML frontmatter MUST carry `name` and `description` (files missing
either are ignored); keys are camelCase; the body is the inline system
prompt. There is no `{file:...}` reference mechanism and no `mode: primary`
concept — the main agent is fixed and sub-agents cannot spawn further
sub-agents. Consequently every artifact the ADR-0019 zcode path produced was
invisible to the app ("Installed 0 items" in Settings → Subagents).

## Decision

ZCode joins claude/codex as an adapter-owned native-agents editor:

1. **Sub-agents** are written as real files (never symlinks) at
   `<zcode_dir>/agents/<name>.md` with frontmatter `name` (required),
   `description` (required), optional `tools` (passthrough from bundle
   frontmatter) and optional `model` resolved from `agent-models.yaml` via
   the existing `IdeKey::Zcode` tier/override mechanism. The bundle body is
   inlined as the system prompt.
2. **Primary agents** (`PRIMARY_AGENTS`) are registered as slash commands at
   `<zcode_dir>/commands/<name>.md` with frontmatter `description`,
   `argument-hint`, `model` and a `source: sddk` marker. Invoking the command
   injects the agent prompt into the ZCode main agent, which CAN dispatch the
   sddk-* sub-agents via the Agent tool — restoring OpenCode primary-mode
   semantics within ZCode's fixed-main-agent model.
3. **Link profile**: `LinkEditor::ZCode => LinkProfile::NATIVE_AGENTS` — the
   agents symlink surface is skipped; skills/prompts/workflows/assets keep
   symlinking.
4. **Capabilities**: `EditorCapabilities::for_ide(IdeKey::Zcode)` becomes
   `supports_mode/hidden/prompt_ref = false`, `supports_tools = true`, with a
   `zcode_model_valid` validator (full `provider/model` ids only).
5. **Migration**: on register/reconcile, agent-map era artifacts (symlinks,
   `name`-less stale writes under `agents/` for sddk-owned names) are
   replaced in place or removed; `dev uninstall --editor zcode` also prunes
   legacy `zcode.json` agent-map entries, marker-generated commands, and
   agents symlinks.
6. **Bounded pruning (ADR-0018)**: `agents/` orphans are removed only for
   `is_sddk_owned` names; command files are removed only when carrying the
   `source: sddk` marker, so hand-written commands sharing a framework
   namespace survive.

## Consequences

- `zcode.json` is no longer created, read, or required for agent
  registration; `dev reconcile --editor zcode` diffs native md/command files
  instead of the agent map.
- All bundle agents become visible in the ZCode Subagents UI (there is no
  `hidden` equivalent).
- Orchestrator-style agents cannot spawn sub-agents from the Subagents panel;
  users dispatch them through the generated commands instead.
- `dev doctor` stops expecting agent symlinks for zcode and instead warns
  when native agent files lack the required `name:` frontmatter
  (`agent_name_frontmatter` check).
- OpenCode behavior is unchanged; `upsert_json_agents` remains
  opencode-only in `json.rs`.

## References

- Official ZCode sub-agents documentation (file location, required
  frontmatter, no primary-mode, invocation limits).
- ADR-0019 (EditorAdapter seam), ADR-0018 (bounded pruning), ADR-0127
  (assets surface).
