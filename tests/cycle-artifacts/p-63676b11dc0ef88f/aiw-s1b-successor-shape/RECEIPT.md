# RECEIPT — AIW-S1b (forma sucesora de evidencia observacional, CONDICIONAL)

**Ciclo:** aiw-s1b-successor-shape
**Fecha:** 2026-09-20
**Status:** CANCELLED per roadmap — carencia NO demostrada.

## Ejecución del gate (MILESTONES.md, AIW-S1b)

> "Probar primero writer actual. Si no permite guardar dos observaciones
> contradictorias con identity+basis+relation sin pérdida, aprobar
> AIW-ADR-03 mediante número canónico y migración aditiva; dual-read
> compatibility, no segundo store, writer canónico. **Gate: UAT-A10..A13.**
> Si no se demuestra carencia, cancelar slice."

## Sonda UAT-A10 (OBSERVED)

Test `uat_a10_current_writer_preserves_two_contradictory_observations_across_reboot`
(`crates/sddk-engine/tests/observation_set_durability.rs`, commit 2026-09-20):

- Dos `SoftwareObservation` del mismo sujeto (misma `SoftwareRelation`),
  una `Affirms` (producer-a, ev:affirm) y una `Denies` (producer-b,
  ev:deny), cada una contra su propio `ObservationBasis`.
- Identidades distintas (stance y basis participan del hash de identidad).
- Persistencia vía `observation.set.appended` v1 (S4, commit `3e37300`).
- Crash/reopen con `Storage::open` nuevo sobre el mismo fichero.
- Tras reboot: set exacto, 2 observaciones, ambas por `for_relation`,
  sin latest-wins, sin score agregado, sin pérdida.

Negativo sancionado: identidad byte-idéntica colapsa por insert
idempotente (no es latest-wins: mismo id = mismo contenido hash).

## Veredicto

- **Carencia no demostrada.** El writer actual (ObservationSet + identity
  + basis + relation, hoy duradero gracias a S4) ya preserva
  contradicciones sin pérdida.
- **AIW-S1b CANCELADO** por la regla del propio roadmap.
- **AIW-ADR-03 NO se promueve** (su condición de activación era la
  carencia).
- UAT-A11..A13 quedan fuera de S1b (no hay slice que gatear); A12
  (compatibilidad legacy) aplica solo si hubiera migración, y no la hay.

## Evidencia

- `cargo test -p sddk-engine --test observation_set_durability` →
  4 passed, 0 failed (2026-09-20).
- fmt clean; clippy `-D warnings` clean.
