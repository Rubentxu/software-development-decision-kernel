# ADR-0140: Módulos root de integración agéntica (J2–J6, JCODE_CORE_GA)

- **Estado**: Accepted
- **Fecha**: 2026-09-20
- **Decisión**: aceptar los módulos root nuevos en `sddk-engine/src`:
  `agentic_session_binding`, `context_bridge`, `reactive_verify`,
  `structured_work`.

## Contexto

El mini-roadmap JCODE_CORE_GA (docs/SDDK-Production-Readiness-Alignment-2026-09-14/02-MINI-ROADMAP.md)
exige implementar los tracks J2..J6 como superficie semántica del
engine: binding de sesión host↔SDDK (arch-spec-024), entrega de
contexto por deltas (arch-spec-026), reactive verify / loop AC9
(arch-spec-025 + arch-spec-037) y structured agent work
(arch-spec-027). Los cuatro son módulos puros (sin I/O), SDDK-owned
y host-agnostic.

## Decisión

Se añaden como módulos root de `sddk-engine` (no sub-módulos de un
umbrella) porque cada uno es una frontera de contrato pública
consumida por futuros crates sdk/api (arch-spec-030) y por los
adapters host. Ninguno introduce dependencias nuevas más allá de
serde/thiserror ya presentes.

## Consecuencias

- El gate `no_new_root_level_context_module_without_adr` queda
  satisfecho: los cuatro nombres aparecen en este ADR.
- R11 (crate-split, P3) podrá moverlos sin romper el contrato: son
  módulos puros.
- Ciclos: j2-j3-agentic-session-binding (v1.169.110),
  j4-context-bridge (v1.169.111), j5-reactive-verify (v1.169.112),
  j6-structured-work (v1.169.113).
