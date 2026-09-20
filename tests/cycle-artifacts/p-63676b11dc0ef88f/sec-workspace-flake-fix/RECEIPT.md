# RECEIPT — sec-workspace-flake-fix

**Cycle:** sec-workspace-flake-fix
**Fecha de cierre:** 2026-09-20
**Status:** CLOSED — fix ya aplicado; decisión #5 de STATE-OF-AIW.md §6 resuelta.

## Qué se decidió y por qué

El SCOPE-CONTRACT proponía eliminar
`cross_surface_facades_share_the_service_instance`
(`crates/sddk-cli/tests/a6_4_shared_ticket_service.rs`) porque corría en
paralelo contra el singleton `AuthorityTicketService::process_service()`
y flakeaba gates de release.

Investigación de cierre (auto-run, 2026-09-20):

1. **El fix ya estaba aplicado y commiteado**: commit `0ca24c2`
   (`fix(sec-workspace-flake): elimina test con carrera sobre singleton
   process_service`, 2026-09-19), alcanzable desde `main`. La decisión #5
   estaba registrada sobre un estado pre-fix obsoleto.
2. **Análisis de cobertura del test eliminado** (revisión adversarial del
   argumento "reducción de cobertura sobre AuthorityTicketService"):
   - Identidad del singleton → cubierta de forma más fuerte por
     `shared_service_singleton_is_same_instance_for_both_surfaces`
     (comparación de punteros `*const _`).
   - Seq monotónico compartido entre ambas superficies → cubierta por
     `cross_surface_shared_seq_strictly_monotonic` (nivel servicio) y
     ejercitada end-to-end por los facades en
     `facade_deny_yields_zero_side_effects_on_real_service`.
   - El test eliminado era la composición de ambos, sin aserción única
     propia. Reducción de cobertura neta: cero.
3. **El flag `AuthorityContext negative-grant` de AIW-S7 no depende** de
   este test; la superficie de autoridad sigue pinada por los 4 tests
   retenidos.

## Evidencia

- `git merge-base --is-ancestor 0ca24c2 main` → reachable.
- `cargo test -p sddk-cli --test a6_4_shared_ticket_service` × 3:
  `4 passed; 0 failed` en las tres corridas (OBSERVED, 2026-09-20).

## Resultado

- Decisión #5 (STATE-OF-AIW.md §6): **RESOLVED — no operator action needed.**
- Cobertura sobre `AuthorityTicketService` no reducida (los 4 tests
  retenidos siguen ejercitando deny-path, seq monotónico, identidad
  singleton y transición de política).
- El flake de release gate queda eliminado sin pérdida de verificación.
