# RECEIPT — S4 durability (Option B)

**Slice:** `p-63676b11dc0ef88f/a6-static-enhanced-readiness/slices/s4-durability`
**Fecha de cierre:** 2026-09-20
**Status:** CLOSED — Opción B implementada; decisión #1 de STATE-OF-AIW.md §6 resuelta.

## Decisión (mejora sobre lo propuesto)

El SCOPE-CONTRACT planteaba STOP por pérdida semántica en `payload: Value`
y tres opciones (A light / B medium / C defer), asumiendo que B era
"higher scope". La investigación profunda (mandato del auto-run) mostró
que **la infraestructura de B ya existía**: `EventSchemaRegistry` +
`schema_struct!` + `CanonicalEventValidator` con 26 tipos registrados.
El coste de B colapsó; se eligió B por ser la única opción que elimina
de raíz los tres riesgos de §4.1 (forward-compat, cross-version,
discovery) sin quedar como deuda (C).

## Implementación (commit en main, 2026-09-20)

1. `crates/sddk-engine/src/observation/types.rs`: `Deserialize` añadido a
   todo el árbol de observación (el bloqueo real: cero impls de
   Deserialize impedían el reopen tipado).
2. `crates/sddk-domain/src/event_registry/schemas.rs`: schema
   `observation.set.appended` v1 registrado con envelope congelado
   `{ schema_version, observation_set, content_digest }`. Aditivo: no
   toca formatos existentes ni crea dependencia domain→engine.
3. Pin de count del registry 26→27 con comentario de procedencia.
4. Tests de contrato:
   `crates/sddk-engine/tests/observation_set_durability.rs`
   - roundtrip completo: serialize → emit_canonical_event → crash →
     Storage::open → list_events → validate (registry v1) → deserialize
     → igualdad exacta + digest estable.
   - 4 casos negativos de schema + 1 válido (set vacío).

## Evidencia (OBSERVED, 2026-09-20)

- `cargo test -p sddk-domain --lib` → 541 passed, 0 failed.
- `cargo test -p sddk-engine` → 96 líneas "test result: ok", 0 FAILED.
- `cargo test -p sddk-engine --test observation_set_durability` →
  2 passed, 0 failed.
- `cargo fmt --check` clean; clippy `-D warnings` clean en ambos crates.

## Clasificación

- No hay cambio de autoridad: el envelope v1 es nuevo, versionado desde
  el día 1, aditivo.
- Registro como **mejora sobre lo propuesto** (preámbulo del ciclo
  AIW Delivery): refinamiento forzado por blocker resuelto con
  investigación profunda.
- Desbloquea: AIW-S1b (S4 deja de ser A/B/C pendiente).
