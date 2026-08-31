# ADR-0068: Bounded Execution — cycle-44 foundation

**Autoridad**: ciclo `p-52b95ef55999f9de/kernel-cycle-44-polyglot-bounded-execution`
(rama `main`, base `ad2ff22051be72ed37b0e0d7d46a2e02a6975dcd`).

**Tipo**: decisions (foundation)
**Estado**: accepted
**Decisiones referenciadas**: D1, D2, D3, D5, D10, D12
**Suprime**: ninguna
**Relevancia invertida**: ninguna
**Amends**: ADR-0068 (this document) — 2026-08-27 cycle-45 JVM scope
expansion from four adapter families to six (`maven/test`, `gradle/test` added);
cycle-44 public runner contract unchanged.

---

## Resumen

El runtime de workflow (`WorkflowRuntime`) carece de dos garantías de terminación
exigidas por los requisitos `REQ-WF-RT-017` y `REQ-WF-RT-018`:

1. **Agotamiento de presupuesto de pared** — el tiempo de reloj de pared excede
   `max_wall_ms` → terminación con `BudgetExceeded`.
2. **Inanición por falta de progreso** — N ticks consecutivos sin mutación
   observable → terminación con `NoProgressDetected`.

Esta ADR establece la autoridad de diseño para la implementación fundacional
(cycle-44). Los adaptadores específicos de runner (cargo-nextest, pytest, jest,
go/test, maven/test, gradle/test) se delegan al cycle-45. Maven y Gradle cubren
Java + Kotlin/JVM; Android instrumentation, Kotlin/Native, Kotlin/JS y Kotlin
Multiplatform no-JVM quedan explícitamente fuera de cycle-45.

---

## Decisiones de diseño

### D1 — `execution_controller.rs` crate-private

```
crates/sddk-engine/src/execution_controller.rs   → creado
  ExecutionController, ProgressSnapshot, NodeSnapshot   → pub(crate)
  sin re-export en lib.rs
```

**Razonamiento**: ningún consumidor externo necesita estos tipos en cycle-44.
Hacerlos crate-private minimiza la superficie pública y evita la tentación de
acoplar lógica de bounds fuera del motor. Si un consumidor futuro necesita
estas garantías, puede acceder via el API pública de `WorkflowRuntime`.

**Inversibilidad**: media. Si se requiere en el futuro, mover a `pub` es
aditivo y no rompe el comportamiento existente.

### D2 — `no_progress_threshold` en `Budgets`

```
crates/sddk-domain/src/workflow_ir.rs:103  → campo añadido
  #[serde(default)]
  pub no_progress_threshold: u32   → default u32::MAX
  cinco sitios de construcción de Budgets actualizados
```

**Razonamiento**: el campo es aditivo; `#[serde(default)]` garantiza que los
IR serializados existentes (sin el campo) deserialize correctamente con
`u32::MAX` (check deshabilitado). Los cinco sitios de construcción en
`workflow_ir.rs` se actualizan explícitamente; los sitios externos al crate
reciben el default sin cambios.

**Inversibilidad**: baja. Cambiar el default rompería IRs existentes que
dependen del comportamiento actual.

### D3 — Reutilización de `Budgets.max_wall_ms` con `Instant::elapsed()`

```
controlador.pre_tick() → Instant::now() capturado en execute() entry
                          elapsed via Instant::elapsed() (NO 4th Clock)
```

**Razonamiento**: el spec dice "observable recovery boundary" — matando
preemptivamente dentro de un operador en ejecución se arriesga estado inconsistente.
El chequeo en la entrada de `tick()` es la primera frontera de recuperación
visible. No se añade un cuarto `Clock` para no complicar la API existente.

**Inversibilidad**: media. Un `Clock` separado permitiría testabilidad;
`Instant::now()` la dificulta.

### D5 — Errores aditivos `RuntimeError::BudgetExceeded` y `NoProgressDetected`

```rust
RuntimeError::BudgetExceeded {
    elapsed_ms: u64,    // ms realmente consumidos
    max_wall_ms: u64,  // máximo configurado
}
RuntimeError::NoProgressDetected {
    consecutive: u32,   // ticks consecutivos sin progreso
    threshold: u32,    // umbral configurado
}
```

**Razonamiento**: ambos errores son aditivos — `AlreadyTerminal` gana siempre.
La firma pública de `WorkflowRuntime::execute()` y `tick()` no cambia; los
errores emergen del tipo de retorno `Result` existente. El hook en `pre_tick()`
(y en `tick()` tras `apply_outcomes_to_state()`) mantiene `AlreadyTerminal`
como guard de precedencia.

**Inversibilidad**: baja para los variantes; la firma pública no cambia.

### D10 — ADR única como autoridad

```
docs/adr/ADR-0068-bounded-execution.md   → creado
  D1 + D2 + D3 + D5 + D10 como autoridad unificada
  D6 (bounded-process header doc) folded into cycle-45 adapter edge
  D11 (ROADMAP edge) folded into cycle-45
```

**Razonamiento**: mantener una ADR por decisión multiplica el trabajo de
seguimiento sin valor. Las decisiones fundacionales de cycle-44 tienen
acoplamiento fuerte entre sí; una ADR unificada captura mejor las trade-offs
compartidas.

**Inversibilidad**: N/A (documental).

### D12 — Adapter families are runner-based

```text
cycle-45 bounded-runner adapters, by build/test runner:
  cargo-nextest  → Rust
  pytest         → Python
  jest           → JavaScript / TypeScript
  go/test        → Go
  maven/test     → Java + Kotlin/JVM (Maven Surefire/Failsafe)
  gradle/test    → Java + Kotlin/JVM (Gradle test task)

Excluded from cycle-45:
  Android instrumentation, Kotlin/Native, Kotlin/JS,
  non-JVM Kotlin Multiplatform
```

**Razonamiento**: la abstracción de adapter es por **build/test runner**, no por
lenguaje fuente. Cada adapter construye un `RunSpec` acotado y propaga el
`RunOutcome`; cycle-45 no interpreta reportes JUnit, TestNG o Kotest. Maven y
Gradle cubren las tareas JVM sin modificar `RunSpec`, `RunOutcome` ni
`run(&RunSpec)`. La precedencia entre wrappers y ejecutables del sistema se
decide en el spec/design de cycle-45, no en esta enmienda.

**Inversibilidad**: baja. Añadir familias es aditivo; cambiar el contrato público
del runner requeriría una enmienda explícita.

---

## Modelo de datos

```
ProgressSnapshot(BTreeMap<OperatorId, NodeSnapshot>)
NodeSnapshot { state, attempt_count, outputs_hash: u64, terminal: bool }
outputs_hash = DefaultHasher::new() sobre serde_json de outputs
```

**Razonamiento**: `tick_seq` es actividad, no progreso (spec S3). Solo los
cambios observables (estado, conteo de intentos, outputs, terminal) cuentan.
`DefaultHasher` es determinista dentro de un proceso y suficiente para
detectar diferencias.

---

## Contrato de recuperación

| Error | Punto de recuperación | Acción |
|-------|------------------------|--------|
| `BudgetExceeded` | entrada de `tick()` | `fail("budget exceeded")` |
| `NoProgressDetected` | tras `apply_outcomes_to_state()` | `fail("no progress")` |
| `AlreadyTerminal` | cualquier punto | retorna error, no sobreescribe estado |

`AlreadyTerminal` gana sobre ambos errores nuevos (spec: "AlreadyTerminal wins").

---

## Edge cycle-44 ↔ cycle-45

- **cycle-44**: foundation (`ExecutionController`, `Budgets.no_progress_threshold`,
  `validate_pass_evidence`, bounded-process header doc).
- **cycle-45**: adaptadores de runner específicos (cargo-nextest, pytest, jest,
  go/test, maven/test, gradle/test) que implementan el contrato de
  bounded-process-execution. Maven y Gradle cubren Java + Kotlin/JVM; Android
  instrumentation, Kotlin/Native, Kotlin/JS y Kotlin Multiplatform no-JVM
  quedan explícitamente fuera.

---

## Métricas de riesgo

| Risk | Likelihood | Impact | Mitigation |
|------|-------------|--------|------------|
| `Instant::elapsed()` no es deterministic | Low | Medium | Spec no exige deterministic wall time; solo bound |
| `DefaultHasher` collision | Very Low | Low | Doshash en práctica; `PartialEq` en snapshot来做 final check |
| `u32::MAX` sentinel fragile | Low | Low | Check explícito `!= u32::MAX` |
| JVM wrapper/toolchain variance (Maven/Gradle) | Medium | Medium | cycle-45 fixtures constrain bounded `RunSpec`; wrapper precedence remains a cycle-45 spec/design decision |

---

## Referencias

- REQ-WF-RT-017 (Bounded Workflow Execution)
- REQ-WF-RT-018 (Bounded Process Execution Contract)
- REQ-IPV (Independent Pass Verification)
- proposal v5 (`65faa6f6…4747e`)
- design.md (`e3d2fff7…314c2`)
