---
status: accepted
date: 2026-08-18
deciders: [orchestrator]
linked_cycles: [c-20260818-145237]
---

# ADR-0018 — User-owns-IDE-config boundary (first-time registration only)

## Status

Accepted — implemented across `crates/sddk-cli/src/dev/editor_adapters/`.

## Contexto

The pre-cycle `register_opencode_agents` rewrote every framework agent entry
in `opencode.json` on each `dev link` run — including `model`, reverting user
migrations (DeepSeek/GLM IDs were silently replaced by the hardcoded MiniMax
fallback). IDE config is user-owned territory; the framework had been
violating that boundary by design.

## Decision

IDE configuration is **user-owned after first registration**. Adapters create
entries only when absent (first-time only) and never mutate any field of an
existing entry (`model`, `description`, or any other). Framework-side
maintenance is limited to pruning entries the framework itself namespaced
(`sddk-`/`sdd-`/`gentle-`) that no longer ship in the bundle. Presentation
fields (`mode`/`hidden`) are framework-owned but written only at creation
time.

This invariant applies uniformly to all four adapters: JSON entries
(opencode/zcode), native `.md` files (claude), and native TOML files (codex).

## Alternativas rechazadas

- **Reconciliación completa** (reescribir las entradas del framework en cada
  run) → exactamente el bug que se corrige: revertir silenciosamente las
  migraciones del usuario.
- **Marcadores de propiedad/checksums** dentro de los configs de los editores
  → contamina archivos de usuario con metadatos del framework; no soportado
  por todos los esquemas.
- **Nunca podar** → entradas muertas acumuladas; sin camino de limpieza para
  agentes retirados.

## Consecuencias

- Los cambios en `description`/body de los agentes del bundle **no se
  propagan** a entradas ya registradas (tradeoff documentado en
  `docs/adr/ADR-0019` y las notas de apply).
- Los archivos claude/codex escritos por el framework nunca se actualizan
  tras la primera escritura: el usuario los posee desde entonces.
- El prune es la única escritura del framework sobre namespaces existentes, y
  está acotado al namespace del framework.
