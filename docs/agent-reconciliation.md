# Agent reconciliation — `sddk dev reconcile`

> Authoritative reconciliation between bundle sources and IDE configs.
> Companion to `sddk dev link`. ADR-0064.

## Overview

`sddk dev reconcile` detects and fixes drift between:

- `assets/agent-models.yaml` (model mapping)
- `agents/*.md` (bundle agent sources)
- Per-IDE config files (`opencode.json`, `zcode.json`, `~/.claude/agents/*.md`, `~/.codex/agents/*.toml`)

Unlike `sddk dev link` which only writes new entries (first-write-only, ADR-0018), reconcile updates existing entries to match the current bundle state.

## Usage

```bash
# Dry-run (default): show what would change
sddk dev reconcile

# Actually apply changes
sddk dev reconcile --apply

# Check mode: exit 1 if drift detected
sddk dev reconcile --check

# Target specific editor
sddk dev reconcile --editor opencode

# JSON output for scripting
sddk dev reconcile --format json
```

## Exit codes

| Flag | Drift | No drift | Error |
|------|-------|----------|-------|
| `--check` | exit 1 | exit 0 | exit 1 |
| `--apply` or dry-run | exit 0 | exit 0 | exit 1 |

## IDE capabilities

| IDE | mode | hidden | prompt | tools | model validator |
|-----|------|--------|--------|-------|-----------------|
| opencode | ✅ | ✅ | ✅ | ❌ | — |
| zcode | ✅ | ✅ | ✅ | ❌ | — |
| claude | ❌ | ❌ | ❌ | ✅ | ✅ sonnet/opus/haiku/inherit |
| codex | ❌ | ❌ | ❌ | ❌ | — |

## Ownership rule

Agents **not** in the bundle (user-created agents like `my-agent`) are **never reconciled or pruned** — they are invisible to the reconciliation process.

## Field preservation

When applying changes, **unknown fields are preserved**:

- **opencode/zcode**: Keys in `opencode.json`/`zcode.json` not managed by sddk (e.g., custom extensions, UI preferences) are kept.
- **claude**: Frontmatter keys other than `name`, `description`, `model`, `tools` are preserved.
- **codex**: TOML keys other than `name`, `description`, `model`, `developer_instructions` are preserved.

## Relationship with `dev link`

| Aspect | `dev link` | `dev reconcile` |
|--------|------------|-----------------|
| Purpose | Initial registration | Ongoing synchronization |
| Mode | First-write only (ADR-0018) | Updates existing entries |
| Pruning | Framework-namespaced orphans | Framework-namespaced orphans not in bundle |
| User agents | Untouched | Untouched |
| Use case | Fresh install | After bundle update |

Use `dev link` for initial setup. Use `dev reconcile` after `dev update` to propagate bundle changes to IDE configs.

## Troubleshooting

### "No drift detected" after update

If you've updated the bundle but `dev reconcile` reports no drift, the IDE config may have been customized with a non-framework prompt path. The reconcile logic preserves user-customized prompts.

### Agent skipped (no model configured)

An agent without a model in `assets/agent-models.yaml` is **skipped** (not deleted) during reconciliation. This follows the `NoModelConfigured` semantics from ADR-0017.

### Exit code 1 with no errors shown

Run with `--format json` to see the full reconciliation report including per-editor details.

## See also

- [SPEC-RECONCILE-001](./reconciliation-spec.md) — full specification
- [ADR-0064](adr/ADR-0064-sddk-authored-reconciliation.md) — architecture decision
- [`agent-models-registration.md`](./agent-models-registration.md) — model configuration
