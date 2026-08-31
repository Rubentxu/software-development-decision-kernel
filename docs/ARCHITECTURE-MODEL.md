# Architecture Model

Inspired by `asdf-vm` (tool versions, shims per version, `path:` override).
Canonical spec: `docs/responsibility-separation/SPEC.md`.

## Three Separate Roles

| Role | Location | Content | Adopted | Linked |
|------|----------|---------|---------|--------|
| **Development repo** | `~/Proyectos/agentesIA/sddk-framework/` (CWD) | `crates/`, `docs/`, `agents/`, `skills/`, `prompts/`, CI, releases | NO | NO |
| **Runtime bundle** | `~/.local/share/sddk/framework/<v>/` | Snapshot: `agents/`, `skills/`, `prompts/`, `workflows/`, `assets/` | — | YES → `$HOME/.config/{opencode,claude,kilo,codex}/` |
| **Usage workspace** | User repos | Project + optional `.sddk-versions` | YES | NO |

## Version Resolution (lookup order)

1. `$PWD/.sddk-versions`
2. `.sddk-versions` in parent directories up to root
3. `$SDDK_DATA_DIR/framework/current` (global symlink)

Format (managed by the developer, NEVER by the framework):
```text
sddk 1.5.3
sddk current         # follows global symlink
sddk path:../..      # dogfooding (CWD = sddk-framework)
sddk system          # system installation
```

## Zero Intrusion

| Operation | Before (wrong) | Now (correct) |
|-----------|---------------|---------------|
| Adoption | `workflow/workflow.yaml` planted in repo | receipt in `~/.local/share/sddk/projects/<id>/` |
| Cycle artifacts | `sddk/{change}/...` in repo | `~/.local/share/sddk/projects/<id>/cycle-artifacts/{cycle_id}/` |
| Generated docs | `docs/generated/` in repo | `~/.local/share/sddk/projects/<id>/generated/` (or `--in-repo` for dogfooding) |
| Telemetry | `~/.local/share/sddk/uat-results.sqlite` | always XDG, never in repo |
