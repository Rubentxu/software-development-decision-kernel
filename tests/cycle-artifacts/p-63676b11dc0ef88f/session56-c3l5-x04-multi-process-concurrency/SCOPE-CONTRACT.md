# SCOPE-CONTRACT — C3l.5 X04: concurrencia real multi-proceso

**Slice:** `session56-c3l5-x04-multi-process-concurrency`
**Roadmap:** `docs/roadmap/ROADMAP.md` §C3l.5 · **Paquete:** `docs/sddk-roadmap-acceptance-truthfulness-2026-09-30/ROADMAP-ACCEPTANCE-TRUTHFULNESS.md:298`
**Workflow:** `A-lite` (debt_verification ejecutada en el PRE-FLIGHT: 0 deuda critical/high abierta)
**Baseline:** `ccf7ada9` (5 commits sin publicar)

---

## 1. Defecto verificado

`crates/sddk-engine/tests/aiw_s8_x04_two_cli_concurrency.rs` (4 tests) usa
`Arc<InMemoryLeaseStore>` + `MockClock`: **un solo proceso, estado en memoria,
reloj falso**. El exit gate del paquete exige *"al menos dos PIDs distintos y
SQLite durable compartido"*. La prueba no puede cumplirlo porque **la superficie
no existe**:

```
$ grep -rn 'impl LeaseStore' --include=*.rs crates/
crates/sddk-engine/src/agent_host.rs:114:impl LeaseStore for InMemoryLeaseStore {
```

**Una sola implementación, en memoria.** Y el propio doc del trait (agent_host.rs:89)
dice *"Real implementations forward to `sddk_storage::Ledger`"* — **una intención
documentada sin implementación**.

### Causa raíz (no el síntoma)

`LeaseStore` es un **puerto** que vive en `sddk-engine`, mientras el patrón
canónico del repo es *puerto en `sddk-domain/src/ports.rs` + implementación en
`sddk-storage`* (exactamente lo que hace `Ledger`). El puerto en el engine con
una única impl en memoria es la razón estructural por la que X04 no puede
verificarse: no es un test débil, es una capa que nunca se cerró.

## 2. Diseño: mover el puerto, no añadir un atajo

| Opción | Descripción | Veredicto |
|---|---|---|
| A | `SqliteLeaseStore` en `sddk-engine` + `rusqlite` como dep del engine | Mezcla persistencia en el engine; contradice la separación existente |
| B | `SqliteLeaseStore` en `sddk-storage` → storage depende de engine | Invierte la jerarquía (storage es la capa inferior) |
| **C** | **Mover `LeaseStore`/`LeaseRecord`/`LeaseError` a `sddk-domain`, re-exportar desde `sddk-engine`, implementar `SqliteLeaseStore` en `sddk-storage`** | **Coincide con el patrón de `Ledger` ya existente. Una sola autoridad por concepto (§2.7)** |

Se elige **C**. La re-export desde `sddk-engine` mantiene compilando los
consumidores actuales; no se reescribe historia.

## 3. Entregable

| Path | Δ | Descripción |
|---|---|---|
| `crates/sddk-domain/src/ports.rs` | +~70 | `LeaseStore`, `LeaseRecord`, `LeaseError` (el puerto baja a la capa de puertos) |
| `crates/sddk-engine/src/agent_host.rs` | −~70 / +re-export | El puerto sale; `LeaseStore` se re-exporta para no romper consumidores |
| `crates/sddk-storage/src/agent_lease_store.rs` | +~180 (nuevo) | `SqliteLeaseStore`: acquire/release sobre SQLite con `busy_timeout` y compare-and-swap por (owner, fencing_token) |
| `crates/sddk-storage/tests/x04_multi_process_concurrency.rs` | +~250 (nuevo) | ≥2 PIDs reales contra un ledger durable compartido |

## 4. Los 8 puntos del exit gate del paquete

| # | Condición | Cómo se observa |
|---|---|---|
| G1 | first-writer / lease winner | Un solo proceso acquires; el otro recibe `Conflict` con el owner y token del ganador |
| G2 | fencing monotónico | Tras release + reacquire, el token **sube**, nunca se repite |
| G3 | loser no muta | El perdedor no avanza el contador ni cambia el owner; re-leer el store lo confirma |
| G4 | crash del winner | El proceso ganador muere sin release; el otro **no** puede tomar la lease mientras no expire |
| G5 | expiry / reacquire | Pasado `expires_at_ms`, otro proceso puede adquirir con token mayor |
| G6 | reopen | Un `SqliteLeaseStore` nuevo sobre el mismo fichero ve el mismo estado |
| G7 | WAL / busy timeout | Escrituras concurrentes no devuelven `database is locked` |
| G8 | no state divergence | Lo que el ledger durable dice es lo que ambos procesos observan |

## 5. Falsificadores congelados

- **F6 (mutación):** hacer que `SqliteLeaseStore::release` ignore el
  `fencing_token` ⇒ G3 y G2 deben caer.
- **F7 (mutación):** quitar el `busy_timeout` / retry ⇒ G7 debe caer bajo
  contención.
- **F8 (mutación):** hacer que `acquire` acepte un holder vivo (no comprobar
  expiración) ⇒ G4 debe caer.
- **F9 (mutación):** hacer que el estado viva en memoria en vez de en SQLite
  ⇒ **G6 debe caer** (dos PIDs no se ven entre sí).

## 6. Reglas

- Los 4 tests W0x existentes **no se tocan**: prueban semántica de lease
  intra-proceso, que sigue siendo válida. Se **añade** la frontera
  multi-proceso.
- El test nuevo **no puede** usar `InMemoryLeaseStore` en ningún punto: si
  aparece, el exit gate no se cruzó.
- **≥2 PIDs reales**, verificado con `std::process::id()` afirmado, no
  asumido.

## 7. Límites declarados por adelantado

1. Este slice **verifica X04**, no certifica "dos CLIs reales de SDDK". Los
   procesos son el propio binario de test re-ejecutado, no `sddk` compilado
   como dos invocaciones de producción.
2. `AIW-S8` en su conjunto **no** pasa a VERIFIED: X07 (segundo binario) sigue
   abierto y es C3l.6.
3. Mover el puerto toca la firma pública de `sddk-engine`. La re-export
   preserva la compatibilidad; si algún consumidor de otro crate rompe, se
   registra como regresión y se corrige, no se maquilla.
