# DELTA-CONF-004 — Cycle / Run Lifecycle Cutover

> Delta spec ejecutable de [SPEC-CONF-004-CYCLE-RUN-LIFECYCLE-CUTOVER.md](../SPEC-CONF-004-CYCLE-RUN-LIFECYCLE-CUTOVER.md).
> Ciclo: `p-63676b11dc0ef88f/conformance-closeout-2026-09-13` (fase specify).
> Cada Scenario es criterio de aceptación directamente ejecutable (dado/cuando/entonces).

## 1. Estado inicial (evidencia C0)

Estado medido contra `main` el 2026-09-13. Fuentes:

- `reference/09-09-SPEC-CROSSWALK.md:8` (SPEC-002 FAIL/PARTIAL: "remove runtime Cycle authority") y `:38` (M9 "retire runtime-specific Cycle statuses": variants remain).
- `crates/sddk-domain/src/cycle.rs:14-54` — `CycleStatus` con 11 variantes incluyendo runtime states: `Remediating` (:21), `Recovering` (:31), `UatWaiting` (:33), `ApprovalPending` (:35); partición DELIVERY/DERIVED documentada en `crates/sddk-domain/tests/sp04_cycle_status_slimming.rs:13-17`.
- `crates/sddk-domain/tests/sp04_cycle_status_slimming.rs:74` — `all_eleven_variants_reachable` (fija el baseline actual); `:137` — `derived_variants_have_documented_migration_target` (matriz de migración: `ApprovalPending -> run/authority facts + derived summary`, `UatWaiting -> run/gate facts`, `Recovering -> Run recovery state`, `Remediating -> REVIEW`).
- `crates/sddk-domain/src/cycle.rs:580-584` — test `test_cycle_status_approval_pending_roundtrip`: `ApprovalPending` aún se persiste y roundtripea como estado de Cycle.
- `uat/CONFORMANCE-FITNESS-RATCHETS.md:13` — `conf09_no_runtime_cycle_truth` MISSING (parcial): partición documentada pero **sin guard de escritura** ni mutation test.
- UAT-03 MAPPED parcialmente (`uat/UAT-09-09-CONFORMANCE-MASTER.md:9`): la aceptación de SPEC-CONF-004 exige "three heterogeneous Runs under one Cycle" con summary derivado sin contradicción; hoy los runtime states viven en el propio Cycle.

**Diagnóstico C0:** la matriz de migración existe (sp04) pero los estados runtime siguen siendo verdad canónica del Cycle: se escriben, persisten y roundtripean. Falta el guard de escritura, la derivación desde Run/Authority facts y el fixture multi-Run heterogéneo.

## 2. Estado final verificable

1. `UatWaiting`, `ApprovalPending` y `Recovering` dejan de ser estados canónicos de Cycle: no se escriben en el record persistido; `Remediating` solo sobrevive si una ADR prueba un significado delivery-level genuinamente distinto de la remediación de Run/WorkItem (SPEC-CONF-004 §Deprecated).
2. Approval waits son facts de Run/Authority; UAT waits/results son facts de Run/gate/evidence; retry/recovery son facts de ejecución de Run.
3. El status de Cycle se deriva del lifecycle durable de alto nivel + summaries de Runs hijos; nunca se persiste un runtime state.
4. Un Cycle con Run A failed, Run B approval-waiting y Run C passed no produce verdad de Cycle contradictoria (UAT-03, fixture ejecutable).
5. Las vistas legacy pueden renderizar labels de compatibilidad pero los derivan de Run facts (decode-only).
6. Código nuevo no puede pattern-matchear variantes runtime de Cycle fuera de módulos de compatibilidad, demostrado por ratchet `conf09_no_runtime_cycle_truth` con mutation test.
7. Si permanece alguna variante decode-only, cumple las 6 condiciones de SPEC-CONF-001 §Compatibility exception (`specs/SPEC-CONF-001-BASELINE-CONFORMANCE-CONTRACT.md:59-68`): sin writes, borrable sin perder datos, deriva del camino canónico, parity test, owner + removal trigger, fitness anti-nuevas-dependencias.

## 3. Tests de contención que NO se pueden romper

| Test | Localización | Qué fija |
|---|---|---|
| `sp04_cycle_status_slimming` suite (`all_eleven_variants_reachable`, `delivery_variants_serialize_roundtrip`, `derived_variants_have_documented_migration_target`) | `crates/sddk-domain/tests/sp04_cycle_status_slimming.rs:74` y ss. | baseline/partición/matrix de migración; al remover variantes, la suite se actualiza en el mismo commit con receipt y la parte DELIVERY no se debilita |
| `test_cycle_status_approval_pending_roundtrip` | `crates/sddk-domain/src/cycle.rs:579` | wire-compat de deserialización de repos antiguos; tras el cutover debe seguir pasando como **decode-only** o su sucesor declara explícitamente la nueva garantía |
| `cross_ledger_consistency` suite | `crates/sddk-storage/tests/cross_ledger_consistency.rs:130,173,198` | consistencia entre tablas del ledger mientras se migran los facts |
| `pe04_ledger_coexistence_events_v1_only` | `crates/sddk-engine/tests/phase_events_integration.rs:153` | boundary de escritura de eventos (los facts de Run/Authority se emiten por el camino canónico) |
| `command_spec_tests::clap_surface_and_command_specs_are_in_sync` | `crates/sddk-cli/tests/command_spec_tests.rs:187` | cualquier cambio de superficie CLI del cutover |
| `cli_golden::cli_golden_surface_matches_blessed_snapshot` | `crates/sddk-cli/tests/cli_golden.rs:71` | snapshot blessed; si cambia la salida de `sddk cycle status`, se re-blessa con receipt |
| `cli_walks_cycle_with_fencing_and_rebuilds_state` | `crates/sddk-cli/tests/cli.rs:1109` (UAT-12) | walking/rebuild del ciclo con fencing |
| `workflow_run_pause_resume_preserves_budget` | `crates/sddk-domain/tests/workflow_run_smoke.rs:135` | semántica de Run preservada |
| `workflow_run_lifecycle_events` | `crates/sddk-storage/tests/workflow_run_lifecycle_events.rs` | eventos de lifecycle de Run |

## 4. Ratchet activado y scenarios

### 4.1 Ratchet `conf09_no_runtime_cycle_truth`

**R-004.1** — Guard de escritura, demostrado por inyección.

```gherkin
Scenario: escritura de estado runtime en Cycle es rechazada (mutation test)
  Dado el ratchet conf09_no_runtime_cycle_truth implementado como guard de escritura
    (arquitectura/lint/compile) sobre los constructores/paths de persistencia de CycleStatus
  Cuando se inyecta la mutación "añadir una transición que persiste CycleStatus::ApprovalPending
    en el record de Cycle desde un módulo de producción" en el fixture
    (análogo a CONFORMANCE-FITNESS-RATCHETS.md §Mutation tests :50 "add CycleStatus::ApprovalPending write -> fail")
  Entonces el guard falla nombrando el call-site y la variante runtime
    Y el guard pasa sobre el árbol sin mutar
    Y CLOSE-03 (uat/UAT-09-09-CONFORMANCE-MASTER.md:38: approval wait transition ->
      Run/Authority changes; no canonical runtime Cycle write) queda demostrado por test
```

**R-004.2** — Approval wait es Run/Authority fact.

```gherkin
Scenario: aprobación pendiente vive en Run/Authority, no en Cycle
  Dado un ciclo con un Run en espera de aprobación (high-band action, UAT-04)
  Cuando se consulta el estado de espera y luego se resuelve la aprobación
    (camino de referencia: cli_approval_grant_resolves_pending en crates/sddk-cli/tests/cli_approval_e2e.rs:259)
  Entonces el hecho de espera y su resolución se registran como facts de Run/Authority
    Y el status persistido del Cycle no contiene ApprovalPending en ningún momento
    Y el Cycle summary derivado refleja la espera vía child Run summary
```

**R-004.3** — UAT wait/result es Run/gate/evidence fact.

```gherkin
Scenario: espera y resultado de UAT viven en Run/gate/evidence
  Dado un Run cuya gate de UAT está pendiente y luego emite resultado con evidencia
  Cuando se consulta el estado del ciclo
  Entonces la espera y el resultado son facts de gate/evidence del Run
    Y el status persistido del Cycle no contiene UatWaiting
    Y gate_receipt_pass_evidence::passed_with_all_three_fields_is_accepted
      (crates/sddk-domain/tests/gate_receipt_pass_evidence.rs:24) permanece verde
```

**R-004.4** — Recovery/retry es fact de ejecución de Run.

```gherkin
Scenario: retry/recovery actualiza Run, no la autoridad del Cycle
  Dado un Run que falla y entra en recovery/retry
  Cuando se ejecuta el recovery
  Entonces el estado de recovery/retry se registra como fact de ejecución del Run
    Y el status persistido del Cycle no contiene Recovering ni Remediating
    Y workflow_run_pause_resume_preserves_budget y workflow_run_lifecycle_events permanecen verdes
```

**R-004.5** — Multi-Run heterogéneo sin contradicción (UAT-03).

```gherkin
Scenario: un Cycle con Run A failed, Run B approval-waiting y Run C passed
  Dado un ciclo con tres Runs hijos en estados heterogéneos (failed / approval-waiting / passed)
  Cuando se calcula el summary del Cycle
  Entonces el summary se deriva del lifecycle durable + summaries de Runs sin contradicción
    y es determinista ante re-derivación
    Y ningún runtime state está persistido en el record del Cycle
    Y el scenario queda cubierto por un test nuevo (fixture UAT-03 multi-run, a crear si no existe;
      hoy no hay test que componga tres Runs heterogéneos bajo un Cycle — ver estado C0)
```

**R-004.6** — Pattern-matching prohibido fuera de compat.

```gherkin
Scenario: match de variantes runtime fuera de módulos de compatibilidad falla
  Dado las variantes UatWaiting/ApprovalPending/Recovering reducidas a decode-only (o eliminadas)
  Cuando se inyecta la mutación "añadir un match/construct de una variante runtime
    en un módulo de producción fuera de compat" en el fixture del ratchet
  Entonces el guard falla (compile-time o lint deny) nombrando el módulo
    Y derived_variants_have_documented_migration_target
      (crates/sddk-domain/tests/sp04_cycle_status_slimming.rs:137) se actualiza en el mismo commit
      que cualquier cambio de la matriz, con receipt
```

**R-004.7** — Paridad de vistas legacy.

```gherkin
Scenario: vistas legacy derivan labels de compatibilidad desde Run facts
  Dado un repo con ciclos históricos persistidos con runtime states (pre-cutover)
  Cuando las vistas legacy (UI/CLI summary) renderizan el estado de esos ciclos
  Entonces los labels se derivan de Run/Authority facts vía mapping documentado (decode-only)
    Y un fixture de paridad legacy confirma que la salida visible es equivalente a la pre-cutover
    Y test_cycle_status_approval_pending_roundtrip (crates/sddk-domain/src/cycle.rs:579)
      permanece verde como prueba de wire-compat decode-only
```

## 5. Restricciones de migración

- **Strangler, no flag-day** (SPEC-CONF-004 §Migration): identificar writers/readers → introducir derived summary mapping desde Run/Authority facts → stop new writes → migrar fixtures/decoders → variantes decode-only → remoción tras ventana de compatibilidad si wire/schema lo permite.
- `Remediating` requiere ADR dedicada para sobrevivir con significado delivery-level; sin ADR, misma disposición que las demás variantes (matriz sp04: "REVIEW").
- Read-compat decode-only solo bajo SPEC-CONF-001 §Compatibility exception (6 condiciones, incluyendo parity test y removal trigger).
- El ratchet `conf09_no_runtime_cycle_truth` cuenta como PASS solo con mutation test inyectado y esperando fallo en CI (`CONFORMANCE-FITNESS-RATCHETS.md:46-60`).
