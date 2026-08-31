---
status: accepted
date: 2026-08-18
deciders: [orchestrator]
linked_cycles: [c-20260818-145237]
---

# ADR-0020 — bash+gum TUI as a thin UI shell over `sddk dev models`

## Status

Accepted — implemented in `assets/agent-models/tui.sh` + `sddk dev models`
subcommand family.

## Contexto

Managing `assets/agent-models.yaml` (ADR-0017) needs an interactive surface.
The sed/yq approach to YAML editing is fragile on quoting/nesting, cannot
guarantee atomicity, and would duplicate schema knowledge in bash.

## Decision

The TUI (`assets/agent-models/tui.sh`) contains **zero YAML logic**. All
reads, edits, validation, and writes delegate to `sddk dev models`
subcommands implemented in Rust over the validated serde-saphyr schema:

- `list` — bundle agents with current tier/overrides (stable JSON for the
  script),
- `set --file $TMP …` — staged edits, each validated, written atomically
  (`atomic_write`: temp + fsync + rename),
- `validate --file $TMP` — final gate before commit,
- `tui-path` — prints the bundle script path (`bash "$(sddk dev models tui-path)"`).

The script stages edits in a temp file and commits with a single `mv`
(rename(2) = atomic); the previous file stays intact on any failure
(AtomicWrite). Its only write target is the bundle `agent-models.yaml`
(NoRepoMutation — editor configs and `agents/*.md` are never opened).

gum is feature-detected per subcommand; absence degrades to `select`/`read`
bash primitives with the **identical state machine and exit-code contract**:
0 success · 1 user cancel · 2 validation error · 3 target/bundle
unresolvable. Model detection uses `opencode models` live (5s timeout →
static catalog + warning); zcode/claude/codex use static catalogs curated at
apply time.

The script ships under `assets/agent-models/` so it rides the existing
manifest surface (MANIFEST_SURFACES covers `assets` — zero pipeline change).

## Alternativas rechazadas

- **Edición YAML pura en bash (sed/yq)** → frágil ante quoting/anidamiento,
  sin garantía de atomicidad, duplica el conocimiento del esquema en bash.
- **TUI ratatui/Rust** → nueva dependencia y superficie de build para una
  tarea de edición de configuración; el host ya tiene gum/fzf.
- **Catálogos en vivo como única fuente** → dependencia de red; el spec exige
  fallback estático con warning.

## Consecuencias

- `dev models` añade 4 subcomandos (list/set/validate/tui-path) — la
  propuesta original solo pedía `validate`; el resto está justificado por la
  robustez del contrato de la TUI.
- El script es solo UI: su lógica es testeable e2e
  (`tests-e2e/tui/run.sh`, 24 aserciones E1–E10) contra un bundle falso.
- Sin dependencias Rust nuevas (serde-saphyr y toml 0.8 ya están en el
  workspace).
- Los comentarios en agent-models.yaml no se preservan al reescribir
  (documentado; es dato gestionado por el framework, no prosa).
