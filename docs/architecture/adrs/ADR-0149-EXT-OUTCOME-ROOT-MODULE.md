---
id: ADR-0149-EXT-OUTCOME-ROOT-MODULE
status: accepted
proposed_at: 2026-10-01
accepted_at: 2026-10-01
accepted_by_cycle: c3l
date: 2026-10-01
deciders: ["operador (gates pre-aprobados, session-55)", "orchestrator"]
related:
  - ADR-0148-DYNAMIC-EXPANSION-ROOT-MODULE
  - ADR-0145-DURABLE-CONTEXT-STORE-ROOT-MODULES
---

# ext_outcome: la ausencia de un provider externo nunca es PASS

## Contexto

`aiw_s5_chronos_real.rs` resolvía el provider así:

```rust
let bin = match mcp_bin() { Some(b) => b, None => return };
```

Medido en session-55, con `CHRONOS_MCP_BIN` y `COGNICODE_MCP_BIN` ambos
ausentes:

```text
test result: ok. 3 passed; 0 failed; 0 ignored    (finished in 0.00s)
```

Dos de los tres tests **no se ejecutaron** y el run lo reportaba como verde.
`finished in 0.00s` es la prueba material.

El dato que hace el diagnóstico completo: en el **mismo** entorno,
`aiw_s1_cognicode_real.rs` reportaba `2 passed; 3 ignored`, porque ya usaba la
convención correcta (`#[ignore]` + `expect`). El defecto estaba aislado a dos
sitios y **la solución correcta ya existía en el repo**. No era un problema de
diseño, era un patrón propio sin generalizar.

## Decisión

Nuevo módulo root `crates/sddk-engine/src/ext_outcome.rs`, con cinco estados y
tres invariantes:

1. **Cinco estados, no dos.** `PassObserved` · `FailObserved` ·
   `BlockedExternalDependency` · `NotRun` · `NotApplicable`. Un provider
   externo no puede producir nada más.

2. **`is_pass()` es `true` solo para `PassObserved`.** Es la invariante
   load-bearing: convierte "hay una vía por la que esto se declara verde" en una
   propiedad del tipo y no del código circundante.

3. **Resolver un binario nunca devuelve un pass.** `resolve_provider` devuelve
   `NotRun` aunque encuentre el ejecutable, porque *resolver* es precondición y
   *observar* es el contrato. Sin esta distinción, "encontré el binario" se
   degrada en "el contrato se sostiene".

Complemento: `tests/ext_provider_gate.sh` es la otra mitad del contrato —
cinco estados, cuatro exit codes (0 pass / 1 fail / 2 blocked / 3 not_run) y un
recibo con path/sha256/version/capabilities. **Un provider resuelto cuyo perfil
falla es `fail_observed`, nunca `blocked`**: colapsar los dos es exactamente
cómo una regresión real se reporta como problema de entorno. Los dos lados se
pinean mutuamente por test.

## Consecuencias

- `ProviderKind::Null`, que codifica la ausencia **en el tipo** para el runtime
  evidence port, sigue abierto. Este ADR cierra el falso verde **por harness**;
  el falso verde **por tipos** lo cierra la consolidación provider/capability
  (C3m.3 + C3m.5). Anotar esto evita leer el cierre de uno como cierre del otro.
- El recibo del gate es artefacto de run y no se versiona; la observación
  durable vive en `docs/roadmap/UAT-MATRIX.md` (AT-UAT-009).
- `AT-UAT-010` (captura real de Chronos) queda **BLOCKED**, no PASS: el contrato
  está implementado y falsificado, pero la ejecución real no se ha observado
  porque `chronos-mcp` no está instalado.

## Alternativas descartadas

- **Solo `#[ignore]`** — suficiente para el patrón de CogniCode, pero no da un
  modelo de estado reutilizable ni un recibo. Los providers futuros repiten el
  error si no hay contrato.
- **Skip silencioso con `eprintln!`** — sigue siendo un verde para quien
  lee la línea de resumen.
