# RECEIPT — AIW-S7b — StorageSnapshot → SecretaryL1 consumer

> **Slice:** `p-63676b11dc0ef88f/aiw-s7b-storage-snapshot-l1-consumer`
> **Status:** CLOSED-LOCALLY (pending `scripts/release.sh` push authorization)
> **Scope contract:** `tests/cycle-artifacts/p-63676b11dc0ef88f/aiw-s7-secretary-attention/SCOPE-CONTRACT.md` §S7-STOP-2
> **UAT evidence:** `UAT-EVIDENCE.yaml` (same dir)

## §1 What was delivered

A gateway consumer (`SnapshotL1Consumer`) that converts a durable
`StorageSnapshot` (from `sddk_engine::context_compiler::storage_adapter`)
into a closed-set `SecretaryProposal` via the real
`SecretaryL1Engine::propose()`. Closes G01 (reconciled evidence ref from
`(adapter_id, log_head)`, confidence tracks durability) and G03 (no live
`Storage` handle anywhere in the loop).

### Surface changes

| Path | Δ | Description |
|---|---|---|
| `crates/sddk-gateway/src/storage_snapshot_l1_consumer.rs` | new | `SnapshotConsumerError`, `SnapshotL1Consumer::{new,unregistered,with_secretary,register,consume}`. 4 in-module unit tests. |
| `crates/sddk-gateway/src/lib.rs` | +1 LOC | `pub mod storage_snapshot_l1_consumer;` |
| `crates/sddk-gateway/tests/aiw_s7b_snapshot_l1_consumer.rs` | new | 4 integration tests (G01 + G03 + negatives). |
| `tests/cycle-artifacts/.../aiw-s7b-storage-snapshot-l1-consumer/{UAT-EVIDENCE.yaml,RECEIPT.md}` | new | Slice cycle artifacts. |

## §2 Acceptance vs scope

| Constraint | Compliance | Evidence |
|---|---|---|
| C1: no changes under `crates/sddk-engine/src/` | YES | engine untouched |
| C2: engine imports via root reexports + public module path | YES | `use sddk_engine::{BoundedWindow, ClosedSetKind, ProposalTemplate, RiskTier, SecretaryId, SecretaryL1Engine, SecretaryL1Error, SecretaryProposal}` (root reexports, lib.rs L238-239) + `sddk_engine::context_compiler::storage_adapter::StorageSnapshot` (public module) |
| C3: no new dependency edge | YES | `sddk-engine` already a path dependency of gateway (promoted in the S7-prep commit `92a4cb8`); no new crate |
| C4: three deliverables only | YES | consumer module, lib.rs decl, integration test file |
| C5: sync consumption | YES | `consume` is a plain sync fn returning `Result<SecretaryProposal, SnapshotConsumerError>` |
| C6: scoped checks | YES | see §3 |
| C7: one feature commit, no push, no release script | YES | see §6 |

### UAT coverage

| UAT id | Status | Why |
|---|---|---|
| G01 | PASS | evidence ref = `storage:{adapter_id}:{log_head}` (reconciled pair, A blocks B), asserted in unit + integration. **La cláusula de `confidence` 0.95/0.5 se retira de esta fila: no estaba en la fila canónica de G01 ni en el SCOPE que la transcribe** (ADR-0156, INC-DEBT-066). El `PASS` se sostiene sobre el `evidence_ref`, que es lo que la fila pide. Ver «Reconciliación de G01» más abajo. |
| G03 | PASS | proposal issued from `StorageSnapshot` alone; no `Storage` in scope anywhere |

## §3 Real verification output (cargo, 2026-09-21)

```
$ cargo fmt --check                      # exit 0

$ cargo clippy -p sddk-gateway --all-targets -- -D warnings
    Checking sddk-gateway v1.169.122
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.28s

$ cargo test -p sddk-gateway --lib storage_snapshot_l1_consumer
running 4 tests
test storage_snapshot_l1_consumer::tests::accepts_durable_snapshot ... ok
test storage_snapshot_l1_consumer::tests::empty_log_head_reduces_confidence ... ok
test storage_snapshot_l1_consumer::tests::rejects_empty_adapter_id ... ok
test storage_snapshot_l1_consumer::tests::rejects_when_template_not_registered ... ok

test result: ok. 4 passed; 0 failed; ...; 123 filtered out

$ cargo test -p sddk-gateway --test aiw_s7b_snapshot_l1_consumer
running 4 tests
test empty_adapter_id_e2e ... ok
test durable_snapshot_e2e ... ok
test no_template_e2e ... ok
test empty_log_head_e2e ... ok

test result: ok. 4 passed; 0 failed; ...

$ cargo build --release -p sddk-gateway
    Finished `release` profile [optimized] target(s) in 3.45s
```

**Counts:** 8 new tests (4 in-module + 4 integration), all passing; 123
existing gateway lib tests unaffected; release build clean.

## §4 Deviations

Two spec-shape adjustments, both forced by the real engine API (read
before writing, per slice instructions):

1. `ProposalTemplate::new` takes 5 args (includes `summary`); the sketch
   showed 4. `SecretaryL1Error` is `PartialEq` but not `Eq`, so
   `SnapshotConsumerError` derives `PartialEq` only.
2. Added `SnapshotL1Consumer::unregistered(now_ms)` (empty-engine raw
   constructor) so the "template not registered" path is exercised
   through the real engine instead of mutating private fields.

No STOP condition hit; no private engine API required.

## §5 Out of scope (not closed by this slice)

Live MCP transport wiring, durable persistence, other AIW-S7 rows,
manifest edits, release, push.

## §6 Pending release.sh authorization

This slice is committed locally on top of the S7a feature commit but
**not pushed and not released**. Running `bash scripts/release.sh`
requires explicit operator authorization per the AIW adoption workflow.
Until then, local state is: workspace version `1.169.122`, development
HEAD = this feature commit, no new tag.

## Reconciliación de G01 (2026-10-03, ADR-0156)

Esta fila declaraba `G01 | PASS` incluyendo una cláusula de `confidence 0.95/0.5`
que **no está en la fila canónica de G01** (`UAT-MATRIX.md` §G01: «snapshot
Planning reconciliado, A bloquea B» / «Agenda indica candidato/causa y refs; NO
autorización de ejecución por `project_next`») **ni en el SCOPE que la transcribe**
(`aiw-s7-secretary-attention/SCOPE-CONTRACT.md` §S7-STOP-2), que no la menciona en
ninguna de sus líneas.

Medido, además, que el valor no decidía nada: el campo `confidence` de una
`SecretaryProposal` **no lo leía ningún código de producto** — sus únicos
consumidores eran los dos tests que comprobaban que valía 0.95 o 0.5. El número
existía porque un test lo afirmaba, y el test lo afirmaba porque el número existía.

**Qué se hace y qué no:**

- Se retira la cláusula de esta fila y de `UAT-EVIDENCE.yaml`.
- Los dos tests se renombran para afirmar lo que G01 sí pide
  (`empty_log_head_is_empty_not_weaker`,
  `evidence_state_reflects_whether_the_log_had_anything_e2e`) y comprueban el
  `EvidenceState` (`Observed` / `Empty`) en vez de una magnitud.
- **La fila canónica NO se modifica.** El error estaba en la evidencia que la
  sostenía, no en el criterio, y reescribir el criterio para acomodar a una
  evidencia equivocada habría sido la forma de hacer permanente el defecto.
- El `//!` del módulo, que citaba un `SCOPE-CONTRACT.md` de este ciclo que nunca
  se commiteó, pasa a citar las dos rutas que existen.

**Lo que este `PASS` sí sigue significando:** que la agenda indica
candidato/causa y refs mediante el par reconciliado `(adapter_id, log_head)`, y
que no se emite autorización de ejecución. Eso es exactamente la fila.

**Lo que este `PASS` nunca debe significar:** que existe una propiedad de
confianza medida. No la hay, y ahora el registro lo dice en vez de inventarla.

**Una afirmación del SCOPE de este ciclo que la implementación desmintió, y que se
deja escrita porque el número estaba publicado.** Ese SCOPE afirmaba «un sitio
escribe `confidence`, ninguno lo lee». Al implementar apareció un **segundo**
escritor en producción, `dynamic_expansion.rs:415`, que pasa `trigger.confidence`
— un `f64` que además participaba en la identidad content-addressed del trigger y
por eso parecía un discriminante. Medido: `ExpansionTrigger` tiene **un solo punto
de construcción en todo el repo**, y es un helper de test con `0.9` constante, de
modo que el componente del hash nunca ha discriminado nada. Pasa a `EvidenceState`
con ADR-0156. *El grep que produjo la afirmación anterior buscaba el literal y no
vio la variable: es la quinta vez en esta sesión que se mide mención donde se iba
a medir uso, y la primera que falsea algo ya publicado.*
