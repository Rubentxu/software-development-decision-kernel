# DELTA-CONF-002 — Canonical Event Log Cutover

> Delta spec ejecutable de [SPEC-CONF-002-CANONICAL-EVENT-LOG-CUTOVER.md](../SPEC-CONF-002-CANONICAL-EVENT-LOG-CUTOVER.md).
> Ciclo: `p-63676b11dc0ef88f/conformance-closeout-2026-09-13` (fase specify).
> Cada Scenario es criterio de aceptación directamente ejecutable (dado/cuando/entonces).

## 1. Estado inicial (evidencia C0)

Estado medido contra `main` el 2026-09-13. Fuentes:

- `reference/09-09-SPEC-CROSSWALK.md:7` (SPEC-001 FAIL/PARTIAL) y `:34` (M9 "remove duplicate event paths": `events_v1` + `ledger_events` coexisten).
- `crates/sddk-storage/src/event_store.rs:3-4` — "`ledger_events` (legacy ledger bookkeeping) — independent tables" dentro del mismo `ledger.sqlite`; `:25` — sin transacciones cruzadas entre tablas.
- `crates/sddk-engine/src/canonical_event_log.rs:9-25` — `CanonicalEventLog` port (CA-001/CA-006) existe: un writer lógico por `project_id`, envelope determinista `FactEnvelopeV1`, CasRef > 4096B.
- `crates/sddk-engine/tests/phase_events_integration.rs:153` — `pe04_ledger_coexistence_events_v1_only` documenta (no prohíbe) la coexistencia: `SqliteEventStore` escribe solo en `events_v1`, `ledger_events` escribe el camino `Storage`/`Ledger`.
- `crates/sddk-storage/tests/cross_ledger_consistency.rs:130,173,198` — `verify_cross_ledger_consistency_passes_when_aligned`, `..._detects_orphan_in_events_v1`, `..._detects_orphan_in_ledger_events` detectan divergencia pero no impiden escrituras dobles.
- `uat/CONFORMANCE-FITNESS-RATCHETS.md:9-10` — ratchets `conf09_one_event_append_authority` MISSING (parcial) y `conf09_no_legacy_event_writes` MISSING, sin mutation test.

**Diagnóstico C0:** existen dos superficies de escritura de eventos alcanzables en producción (`Ledger` → `ledger_events` y `SqliteEventStore`/`CanonicalEventLog` → `events_v1`). Coexistencia aceptable durante strangler; NO aceptable como estado final M9.

## 2. Estado final verificable

1. `CanonicalEventLog` es la única autoridad de append de eventos de dominio; `append` de eventos de dominio a `ledger_events` está eliminado o hard-disabled (falla en compile-time o en boundary runtime con error tipado).
2. No existe escritura a `ledger_events` fuera del camino de migración read-only/demostrado por test de arquitectura (`conf09_no_legacy_event_writes`) y su mutation test inyecta una segunda autoridad y espera fallo.
3. Toda proyección reconstruye solo desde el stream canónico; borrar storage de proyecciones + rebuild produce estado equivalente.
4. Repos limpio y repo migrado pasan fixtures idénticos de eventos/proyecciones; la migración es dry-runnable, restart-safe e idempotente bajo failure injection.
5. Los event ids/orden usados en Explanation/WHY permanecen estables o tienen receipt de mapping explícito.
6. Si `ledger_events` permanece legible, cumple las 6 condiciones de la Compatibility exception de SPEC-CONF-001 §Compatibility exception (`specs/SPEC-CONF-001-BASELINE-CONFORMANCE-CONTRACT.md:59-68`): sin writes canónicos, borrable sin perder source-of-truth, deriva del camino canónico, parity test, owner + removal trigger, fitness que impide nuevas dependencias. Entradas de allowlist write-capable están prohibidas (`CONFORMANCE-FITNESS-RATCHETS.md:42`).

## 3. Tests de contención que NO se pueden romper

| Test | Localización | Qué fija |
|---|---|---|
| `pe04_ledger_coexistence_events_v1_only` | `crates/sddk-engine/tests/phase_events_integration.rs:153` | `SqliteEventStore` escribe solo `events_v1` |
| `cross_ledger_consistency` suite (`verify_cross_ledger_consistency_passes_when_aligned` / `detects_orphan_in_events_v1` / `detects_orphan_in_ledger_events` / `tolerance_is_tolerated`) | `crates/sddk-storage/tests/cross_ledger_consistency.rs:130,173,198,238` | detección de huérfanos entre tablas |
| `pe01_dual_emit_sequence_and_payload` | `crates/sddk-engine/tests/phase_events_integration.rs:49` | semántica de em dual en el camino v1 |
| `command_spec_tests::clap_surface_and_command_specs_are_in_sync` | `crates/sddk-cli/tests/command_spec_tests.rs:187` | superficie CLI sincronizada (cualquier flag/comando nuevo del cutover debe pasarla) |
| `cli_golden::cli_golden_surface_matches_blessed_snapshot` | `crates/sddk-cli/tests/cli_golden.rs:71` | snapshot blessed de la CLI |
| `event_envelope_golden` / `event_replay_equality` | `crates/sddk-domain/tests/event_envelope_golden.rs`, `event_replay_equality.rs` | determinismo de envelope y replay |

Romper cualquiera de estos tests invalida el delta; el cutover debe preservarlos o sustituirlos con sucesores que mantengan la misma garantía y se citen en el receipt.

## 4. Ratchets activados y scenarios

### 4.1 Ratchet `conf09_one_event_append_authority`

**R-002.1** — Única autoridad de append, demostrada por inyección.

```gherkin
Scenario: segunda autoridad de eventos es rechazada (mutation test)
  Dado el ratchet conf09_one_event_append_authority implementado como test de arquitectura
    Y el marker arch_lint existente canonical_event_log_owner_recognised
      (crates/sddk-cli/src/dev/arch_lint.rs, pruebas canon: canonical_event_log.rs:9-25)
  Cuando se inyecta la mutación "registrar un segundo EventStore/append authority
    de dominio alcanzable desde producción" en el fixture de la suite de arquitectura
  Entonces el ratchet falla con diagnóstico que nombra los dos propietarios de append
    Y el mismo ratchet pasa sobre el árbol sin mutar
    Y la mutación queda registrada en CI según CONFORMANCE-FITNESS-RATCHETS.md §Mutation tests (:48 "add second EventStore authority -> fail")
```

**R-002.2** — Equivalencia de lecturas canónicas entre repos.

```gherkin
Scenario: repo limpio y repo migrado leen el stream canónico de forma equivalente
  Dado un repo recién creado con eventos solo en el camino canónico
    Y un repo migrado cuyo histórico legacy se importó por el camino de migración idempotente
  Cuando se aplican los mismos fixtures de eventos/proyecciones a ambos
  Entonces ambos producen lecturas canónicas idénticas (mismos fact_ids, mismo orden, mismo estado de proyección)
    Y los tests de contención de replay (event_replay_equality, rebuild_happy_path
      en crates/sddk-cli/tests/cli_projection_rebuild.rs:86) permanecen verdes
```

**R-002.3** — Rebuild solo desde el stream canónico.

```gherkin
Scenario: delete projection + rebuild desde eventos canónicos
  Dado un repo con proyecciones materializadas y stream canónico íntegro
  Cuando se borra todo el storage de proyecciones y se ejecuta el rebuild
  Entonces el estado reconstruido es equivalente al previo (fixture de parity)
    Y ningún paso del rebuild lee tablas legacy como autoridad
```

### 4.2 Ratchet `conf09_no_legacy_event_writes`

**R-002.4** — Escritura legacy rechazada.

```gherkin
Scenario: intento de escritura de dominio a ledger_events falla
  Dado el cutover aplicado (estrategia strangler, no flag-day)
  Cuando un llamador de producción intenta append de un hecho de dominio vía el camino legacy ledger_events
  Entonces la operación falla en compile-time (símbolo/ability no alcanzable)
    O en runtime boundary con error tipado documentado (p.ej. event_store:<code> del contrato R2 de event_store.rs:11-20)
    Y CLOSE-01 (uat/UAT-09-09-CONFORMANCE-MASTER.md:36 "Attempt legacy event write -> rejected/unreachable") queda demostrado por test ejecutable
```

**R-002.5** — Camino de compatibilidad read-only acotado.

```gherkin
Scenario: lectura legacy permitida solo vía decoder read-only conforme a SPEC-CONF-001
  Dado filas legacy ledger_events que deben seguir leyéndose durante la ventana de compatibilidad
  Cuando se accede a ellas
  Entonces el acceso es exclusivamente vía decoder/migración idempotente read-only
    Y la entrada de allowlist declara los 8 campos de CONFORMANCE-FITNESS-RATCHETS.md §Allowlist policy (:29-40)
      con read_or_write = read y parity_test nombrado
    Y ningún módulo fuera del allowlist referencia la escritura legacy (test de dependencia estática)
    Y pe04_ledger_coexistence_events_v1_only sigue verde o su sucesor documenta el nuevo boundary
```

**R-002.6** — Estabilidad de ids para Explanation/WHY.

```gherkin
Scenario: ids y orden de eventos usados en WHY permanecen estables o tienen receipt de mapping
  Dado eventos ya referenciados por Explanation/WHY en repos migrados
  Cuando se completa el cutover y cualquier importación/replay
  Entonces los event ids y su orden permanecen idénticos
    O existe un receipt de mapping explícito (old_id -> new_id) versionado en los artefactos del ciclo
    Y graph_rebuild_then_query_then_why (crates/sddk-cli/tests/cli_graph_e2e.rs:143) permanece verde
```

## 5. Restricciones de migración

- **Strangler, no flag-day:** la secuencia es identificar writers → redirigir a `CanonicalEventLog` → hard-disable del write legacy → ventana read-only → remoción. Ningún paso rompe `cross_ledger_consistency` ni los fixtures de replay.
- **No cross-table atomicity** tras el cutover (SPEC-CONF-002 Requirements); `:25` de `event_store.rs` documenta que hoy no existe — el estado final no debe introducirla.
- **Export/recovery fixture antes de cualquier borrado destructivo de tabla** (SPEC-CONF-002 Acceptance 5; `reference/MIGRATION-CUTOVER-CHECKLIST.md`).
- Ratchet promoted a blocking (deja de ser advisory) solo cuando su mutation test exista y esté en CI.
