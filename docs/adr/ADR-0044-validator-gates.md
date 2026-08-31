# ADR-0044 — Validator con 7 gates en short-circuit

**Status:** Accepted

## Context

El `WorkflowValidator` (en `crates/sddk-domain/src/validator.rs`) decide si un `WorkflowIR`
compilado puede pasar al runtime. Hay dos filosofías para reportar fallos:

- **Acumulación** — ejecuta las 7 gates, recoge todos los errores y los devuelve en lote.
- **Short-circuit** — para en el primer error y devuelve solo ese.

## Decision

El validator usa **short-circuit en el primer fallo** (gate G1 → G7 en orden). Razones:

1. **Determinismo en CI** — el primer fallo es el más accionable para arreglar; el resto suele
   ser consecuencia (un schema inválido enmascara errores de operadores).
2. **Latencia de validación** — un IR con G1 fallido no necesita ejecutar las 6 gates restantes.
3. **Simplicidad** — el flujo `Result<(), ValidateError>` es trivial de componer y testear.
4. **Mensajes de error útiles** — un solo error por ciclo de validación es más legible que un
   dump de 7 problemas simultáneos cuando el desarrollador está depurando.

El orden de las 7 gates refleja dependencias crecientes:

| Gate | Check | Por qué primero |
|------|-------|-----------------|
| G1 | Schema validity | Si falla, el resto del IR es ilegible |
| G2 | Operator well-formedness | Necesita un IR parseable |
| G3 | Cycle-free graph | Necesita un IR estructuralmente válido |
| G4 | Guards | Necesita un grafo sin ciclos |
| G5 | Budgets | Necesita nodos bien formados |
| G6 | Expansion permissions | Necesita presupuesto |
| G7 | Context capsules | Último check de coherencia |

## Consequences

- Iteración de fixing más rápida (un error a la vez).
- Tests unitarios pueden verificar cada gate en aislamiento.
- Si el operador quiere ver todos los errores a la vez, debe llamar al validator 7 veces (no
  recomendado en producción; útil para tooling de IDE).
- Modo `validate_with_template()` ejecuta también G6 con allowlist.

## Rejected

- **Acumulación total**: difícil de razonar; el primer error suele ser la causa raíz.
- **Acumulación por gate**: complica el tipo de retorno sin beneficio claro.

## Verification

- `crates/sddk-domain/tests/validator_closure.rs` proptest: para todo `m`, `validate(compile(m))` o
  es Ok o devuelve un solo error de la primera gate que falla.
- `crates/sddk-domain/tests/validator_gates.rs` (no existe aún): tabla con 7 casos, uno por gate,
  cada uno verifica que ese gate falla con su error canónico.