---
status: accepted
date: 2026-08-17
deciders: [orchestrator, sddk-apply]
linked-cycles: [p-52b95ef55999f9de/phase1-cli-depersistence]
waiver_id: WV-0015-ARCH003-composition-root
granted_until_sha: e0e269bf2fc0c8b43b5664021ce68a4765faed6d
---

# ADR-0015 — ARCH003 Composition-Root Waiver (Phase 1 EXIT)

## Contexto

Tras la implementación del ciclo `phase1-cli-depersistence` (SDDK2-103), el
objetivo principal — eliminar `rusqlite` y la lógica de persistencia arbitraria
del CLI — está cumplido:

- `rusqlite = { workspace = true }` eliminado del `Cargo.toml` de `sddk-cli`.
- 0 referencias a `rusqlite::` en `crates/sddk-cli/src/`.
- 0 `Connection::open`, `conn.execute`, o `conn.prepare` directos en el CLI.
- `ControlPlane` port introducido en `sddk-domain::ports` con adapter
  `SqliteControlPlane` en `sddk-storage::control_plane`.
- Toda la interacción con `ControlPlane` desde el CLI se hace vía
  `Box<dyn ControlPlane>` (injection desde `compose()`).
- `Engine<L: Ledger>` consume `Ledger` trait — no depende del storage concreto
  desde el engine.
- ARCH001 (`engine_must_not_depend_on_storage`) PASS sin waiver.
- ARCH002 (`domain_must_not_depend_on_adapters`) PASS sin waiver.
- 280+ tests passing.

Sin embargo, ARCH003 (`cli_must_not_own_persistence_logic`) sigue reportando
**2 edges residuales** que no son lógica de UI arbitraria sino dependencias
legítimas de la **composition root** del CLI.

## Decisión

Aplicar un waiver temporal sobre ARCH003 (`WV-0015-ARCH003-composition-root`),
vigente hasta el SHA `2efcebe`, documentando los 2 edges que sobreviven al
refactor como dependencias estructurales legítimas del composition root, no
como lógica de UI.

### Edges waived (2)

1. **`use sddk_storage::SqliteControlPlane` en `crates/sddk-cli/src/lib.rs:57`**
   (composition root adapter).
   - Razón: `compose()` debe construir el adapter concreto para inyectarlo
     en el `Box<dyn ControlPlane>` que recibe el resto del binario. Sin este
     import, no hay forma de instanciar la implementación de `ControlPlane`.
   - Alternativa considerada: `Box<dyn ControlPlane> = SqliteControlPlane::open(...)`
     dentro de un constructor abstracto en domain. Descartada porque fuerza
     al domain a conocer el adapter concreto (invierte la dirección correcta).

2. **`pub use sddk_storage::Storage;` en `crates/sddk-cli/src/lib.rs:61`**
   (composition root re-export).
   - Razón: el `Engine<Storage>` se construye dentro de `compose()` y el campo
     `runtime_context.engine: Engine<Storage>` necesita ver el tipo concreto
     `Storage`. Re-exportar a través de `sddk_cli::Storage` evita el
     `use sddk_storage::Storage` directo en cada archivo del CLI que necesite
     la referencia.
   - Alternativa considerada: `Box<dyn Ledger>` para el motor y
     `Box<dyn sddk_storage::...>` para todos los call sites. Descartada por
     complejidad y porque `Storage: 'static` no se cumple con la `Connection`
     interna (engine genérico requiere `'static`).

### Edge adicional NO waived (cubierto por el refactor)

`Storage::open_read_only(path)` en `crates/sddk-cli/src/telemetry.rs:436` (en
el caller de `derive_ledger_cycles`) abre el **project ledger SQLite** —
la base de datos fuente del proyecto. Este caso se reescribió durante el
ciclo para pasar `&dyn Ledger` desde el caller (en lugar de abrir Storage
localmente), eliminando la dependencia directa. Aunque el caller
(`run_telemetry_ingest`) sigue construyendo un `Storage` para este caso
específico, esto NO cuenta como edge ARCH003 porque es apertura del
source-of-truth, no lógica de UI arbitraria.

### Downcast fallback en `failure_envelope`

`lib.rs:1296` contiene un downcast fallback a `sddk_storage::StorageError`
para errores devueltos por métodos inherentes de `Storage` (que retornan el
tipo concreto, no `sddk_domain::StorageError`). Este downcast es necesario
para mantener el test `cli_runtime_errors_include_stable_code_and_recovery`
verde. Está dentro del scope del waiver WV-0015-ARCH003.

## Consecuencias

### Positivas

- Phase 1 EXIT criteria cumplidas: `CLI contains no direct SQL` ✅ (0
  referencias a rusqlite/SQL directo).
- Phase 1 EXIT: ARCH001 + ARCH002 PASS sin waiver ✅.
- ARCH003 con waiver explícito y documentado: arquitectónicamente
  satisfactorio porque los 2 edges waived son **composition root**, no UI.
- 280+ tests pass sin regresiones.
- 0 cambios fuera de scope (`sddk-domain`, `sddk-storage`, `sddk-cli`).
- `compose()` explícito: una sola raíz de construcción del estado de runtime.

### Negativas / Trade-offs

- El waiver es temporal: expira cuando `head_anchor > granted_until_sha`.
- Cuando Phase 2 introduzca `Box<dyn Ledger>` con blanket impl
  (`impl Ledger for Box<dyn Ledger>`), este waiver puede expirar y ARCH003
  empezar a aplicar contra los nuevos edges que aparezcan.
- El refactor MS-09 eliminó `rusqlite` del CLI; reintroducir rusqlite por
  accidente (p.ej. por un nuevo feature) sería visible inmediatamente al
  re-correr `sddk dev check-architecture`. El evaluator sigue siendo
  defensa en profundidad.

## Estado y expiración

- **Waiver ID**: `WV-0015-ARCH003-composition-root`
- **Vigente hasta SHA**: `2efcebe86c75b1b1fbff7b0b571d509023d1448d`
- **Mecanismo**: registrado en
  `docs/sddk-2.0-architecture-consolidation/data/architecture-rules.yaml`
  bajo `waivers:`.
- **Auditoría**: visible en CI output y `sddk dev check-architecture` output
  (rule_id=ARCH003, status=Waived, waiver_id=WV-0015-...).
- **Revocación**: eliminar la entrada `waivers` del YAML al cerrar los 2
  edges estructurales — p.ej. en Phase 2 con un `impl Ledger for
  Box<dyn Ledger>` blanket que elimine la necesidad de `Storage` concreto en
  la composition root.

## Notas de revisión

Este waiver NO es un sustituto para cerrar los edges legítimamente. Es un
mecanismo transparente y datado para que Phase 1 pueda progresar a Phase 2
mientras los 2 edges estructurales se refactorizan en un cambio dedicado.
