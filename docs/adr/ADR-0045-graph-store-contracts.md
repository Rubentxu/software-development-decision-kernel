# ADR-0045 — GraphStore port con 6 métodos IR-revision

**Status:** Accepted

## Context

En el ciclo 1 (`kernel-workflow-ir-contracts` v1.29.0) se añadieron al trait `GraphStore`
7 métodos default-implemented, pero 6 de ellos quedaron como `unimplemented!()` — un LSP
violation (Liskov Substitution Principle): un cliente que reciba un `GraphStore` por
trait-object no puede asumir que estos métodos funcionan. El debt-verify del ciclo 1
marcó esto como W-DV-1 (HIGH).

Los 6 métodos son:

- `record_ir_digest(ir_hash, ir_json)` — deduplicación de IRs compilados
- `record_graph_revision(rev)` — persistir una `ExecutionGraphRevision` con su cadena de padres
- `load_node_attempts(run_id, node_id) -> Vec<Attempt>` — histórico de intentos
- `attempt_count(run_id, node_id) -> u32` — contador para budget
- `load_revision(run_id, rev_id) -> Option<ExecutionGraphRevision>` — punto en el tiempo
- `latest_revision(run_id) -> Option<ExecutionGraphRevision>` — estado actual

## Decision

Los 6 métodos se implementan en `SqliteGraphStore` (en `crates/sddk-storage/src/graph_store.rs`)
contra las 5 tablas añadidas en MIGRATION_11 (ciclo 1):

- `ir_digests_v1` para `record_ir_digest`
- `execution_graph_revisions_v1` para `record_graph_revision`, `load_revision`, `latest_revision`
- `attempts_v1` para `load_node_attempts`, `attempt_count`

Los `default impl` se eliminan del trait (LSP limpio); los métodos ahora son **requeridos**
sin body. Los tests que no sean contra `SqliteGraphStore` deben mockear el trait completo
(vía `mockall` si se añade, o un struct de test local).

## Consequences

- LSP KL 0.06 → 0.0 en el grafo de Arch (cierra W-DV-1).
- `workflow_runs_v1` no se usa directamente aquí — solo garantiza el FK referencial en
  las otras tablas (insert de attempts sin run válido falla por FK).
- El campo `parent: Option<Box<ExecutionGraphRevision>>` no se deserializa en `load_revision`
  para evitar recursión infinita; los clientes que necesiten el chain hacen N lookups por
  `parent_revision_id`. Documentado en el módulo.
- `RunId` para `record_graph_revision` se infiere del primer nodo presente (workaround
  pragmático; ADR-0046 propuesto para ciclo 3: añadir campo `run_id` explícito al struct).

## Rejected

- **Mockear los 6 con `unimplemented!()` para tests**: perpetúa el LSP violation.
- **Convertir los métodos en asociados en vez de trait**: rompe la portabilidad del testkit.

## Verification

- `crates/sddk-storage/tests/graph_store_roundtrip.rs` — roundtrip in-memory para los 6 métodos.
- `cargo clippy --workspace -- -D warnings` verde.
- `cargo test --workspace` verde.