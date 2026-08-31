---
status: accepted
date: 2026-08-18
deciders: [orchestrator]
linked_cycles: [c-20260818-145237]
---

# ADR-0017 — Tier-based model resolution from agent-models.yaml

## Status

Accepted — implemented in `crates/sddk-cli/src/dev/agent_models.rs` + `assets/agent-models.yaml`.

## Contexto

`dev link` resolved the model of every registered agent from the agent's
frontmatter `model:` key, with a hardcoded MiniMax fallback
(`framework_check.rs:65`) when the key was absent. Consequences: vocabulary
changes required editing 64 files; per-IDE overrides were impossible; and the
fallback silently restored a provider whose quota was exhausted — the exact
bug this cycle kills (user-migrated models were overwritten on every run).

## Decision

Agent→model mapping lives in a single data file, `assets/agent-models.yaml`:

- per-agent `tier` (`premium|fast`),
- optional per-IDE `overrides`,
- per-tier default model tables per IDE (`opencode`, `zcode`, `claude`, `codex`).

Resolution order is strict: per-IDE override → per-tier default table →
`NoModelConfigured` (agent skipped with a warning). There is no cross-IDE
guessing, no cross-tier fallback, and no hardcoded fallback anywhere in code.
The absence of the file is a distinct state (ConfigAbsent → register without
a model field), never a fallback.

The schema is deserialized with serde-saphyr in two phases (tolerant raw
parse → validated typed config) so validation errors name the agent and
field. Serialization is round-trip stable (BTreeMap ordering), which the
`dev models set`/TUI write path relies on.

## Alternativas rechazadas

- **Frontmatter `model:` como fuente de verdad** → 64 archivos por cambio de
  vocabulario; overrides por IDE imposibles; fuerza fallbacks para archivos
  sin la clave.
- **Un YAML por IDE** → cuatro fuentes de drift para el mismo mapping; TUI
  más compleja; más superficie de manifest.
- **Scraping de catálogos en vivo** → dependencia de red en `dev link`;
  explícitamente fuera de alcance en la propuesta.

## Consecuencias

- El `model:` del frontmatter de `agents/*.md` queda **inerte** para el
  registro (se conserva para otras herramientas).
- El drift de vocabulario se corrige editando un único archivo de datos (o
  con la TUI).
- El pipeline del bundle no cambia: `assets/` ya es una superficie de
  manifest (MANIFEST.sha256, `dev install`, `dev verify`).
- IDs de modelo no válidos para un editor se detectan en el adaptador
  (p. ej. el gate de vocabulario de Claude en ADR-0019).
