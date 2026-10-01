# RECEIPT — C3l.3 Dynamic Workflow Expansion E2E real

**Slice:** `session54-c3l3-dynamic-expansion-vertical`
**Fecha:** 2026-10-01 · **Baseline:** `dc343722` (= `origin/main`, workspace `2.4.2`)
**Commit de la slice:** `7360c32e`
**SCOPE-CONTRACT:** [`SCOPE-CONTRACT.md`](SCOPE-CONTRACT.md) (congelado **antes** de escribir código)
**UAT:** AT-UAT-006 / 007 / 008 — **PASS** · `boundary_class = IN_PROCESS/SQLITE`

---

## §1 Qué se entregó

| Path | Δ | Descripción |
|---|---|---|
| `crates/sddk-engine/src/dynamic_expansion.rs` | +615 (nuevo) | Superficie de producción de la vertical |
| `crates/sddk-engine/src/lib.rs` | +2 | `pub mod` + `pub use` |
| `crates/sddk-engine/tests/c3l3_dynamic_expansion_vertical.rs` | +660 (nuevo) | 12 tests: F1..F7 + exit gate + control negativo |
| `crates/sddk-engine/tests/aiw_s4_dynamic_expansion.rs` | doc | W02: el nombre afirmaba "one admission" y el cuerpo probaba 2. Reetiquetado como **frontera medida** de `cycle_replan`, sin tocar lógica ni aserción |
| `docs/ROADMAP-ACCEPTANCE-TRUTHFULNESS.md` | **eliminado** | Duplicado byte-idéntico (sha256 `f06b9fb5…` en ambas) de la copia commiteada en el paquete. Decisión del operador. |

## §2 El defecto, verificado contra el código

Seis huecos, cada uno con su ruta:

| # | Hueco | Evidencia |
|---|---|---|
| D1 | `cycle_replan` recibe `event_id` del caller; no deriva identidad del trigger | `cycle_replan.rs:84` |
| D2 | W02 **fijaba el defecto como contrato**: `replan_count == 2`, *"the contract is bounded counter, not dedup"* | `aiw_s4_dynamic_expansion.rs:483` (pre) |
| D3 | Replay no idempotente en la proyección: el evento se dedupea (`INSERT OR IGNORE`) pero el `UPDATE cycles` es incondicional | `event_store.rs:246` vs `storage/lib.rs:686-705` |
| D4 | `cycle_replan` no valida authority en absoluto | `grep authority` → 0 matches |
| D5 | No hay paso de orchestration entre proposal y delta | ausente |
| D6 | Nunca se toca `PlanRevisionV1`; y la ejecución incremental no existe — el compiler es **compile-only por diseño declarado** | `execution_graph_compiler.rs:9-13` |

**Conclusión:** C3l.3 no era una composición de test. La clasificación vigente `IMPLEMENTED_NOT_VERIFIED` era correcta.

### Hallazgo de profundidad

`WorkflowManifest` (el workflow del engine) **no contiene un `WorkflowIR`**: es un manifiesto de transiciones de fase, no un DAG. El substrate `plan_revision.rs` y el ciclo/ledger nunca estuvieron unidos. Por eso el padre de cada revisión se toma del **tip real del ledger**, no de un recálculo local: el linaje es reconstruible desde el ledger y sobrevive al restart.

### Hallazgo sobre la matriz de authority

`WritableSurface::PlanRevisions` admite `Human` y `Agent`, **no `System`**. Los tests W01..W11 existentes usan `ActorKind::System`. Es decir: si la ruta de replan hubiera validado authority alguna vez, esos tests ya habrían fallado — corroboración independiente de D4.

## §3 RED → GREEN

**RED (antes de tocar producción):** `error[E0432]: unresolved import sddk_engine::dynamic_expansion` + `E0599` por cada método ausente. El RED es por **superficie ausente**, no por aserción blanda.

**GREEN:** `c3l3_dynamic_expansion_vertical` **12 passed / 0 failed**.

### Correcciones propias durante el GREEN (declaradas)

Cinco errores fueron **míos**, no del defecto: `base_ir` ausente en el inicializador del trigger; `&str.as_str()` (API inestable); closure `Fn` que capturaba un `String` mutable; y un predicado de ejecución **invertido** en F5 (devolvía `true`=éxito para `t2`, así que F5 no fallaba). Ninguno de los cinco era un defecto del código de producción; el predicado invertido demonstrates que el hook de ejecución es load-bearing.

## §4 Falsadores OBSERVED

El verde se commiteó **antes** de mutar (lección de session-50). Se mutó la producción, se Midió, se restauró; `git diff` vacío = byte-idéntico al commit.

| M | Mutación | Resultado |
|---|---|---|
| M1 | Guard de replay neutralizado (`.filter(\|_\| false`) | **5 FAIL** — f1, f2, f6, exit_gate, receipt |
| M2-bis | Validación de authority eliminada | **2 FAIL** — f3, exit_gate |
| M3 | Pin de base neutralizado (`if false`) | **1 FAIL** — f7 |
| M4 | Selección incremental → todos los nodos | **6 FAIL** — f2, f5, f6, exit_gate, receipt, proposal |

**Error de medición propio, declarado:** el primer intento de M2 gateó la comprobación detrás de `std::env::var("C3L3_NEVER").is_err()`, es decir la dejó activa en el caso normal → **mutación nula, 12/12 PASS**. Casi se concluye que el check de authority no era load-bearing. Se repitió como M2-bis (eliminación literal) y sí cayó. Cuarta vez que un falsador mal construido da un resultado falso, y la quinta que el patrón se repite: **un guard detrás de una env var que no se activa no falsifica nada**.

## §5 Exit gate (las 4 cláusulas)

1. **Exactamente una expansión ante replay** — `f1` + `exit_gate_vertical_holds_end_to_end`: `applied`/`!applied`, mismo `revision_id`, `lineage_len == 2` tras el replay.
2. **Nodos previos no se re-ejecutan** — `f2`: `executed_node_ids == ["t2"]`; `t1` nunca aparece.
3. **Denied/invalid no cambia el plan** — `f3` (System no admitido), `f4` (delta vacío), `f7` (base obsoleta): los tres dejan eventos, tip y `replan_count` idénticos.
4. **Lineage y receipts** — `new_revision_carries_lineage_and_receipt`: padre = tip real, mutación `NodesChanged`, `expansion-receipt.json` atómico con identidad estable en el replay.

**Control negativo:** `exit_gate_fresh_secretary_cannot_expand` — un Secretary sin template registrado no puede expandir. Sin él, una vertical que ignorara al Secretary pasaría los otros 11 tests en verde.

## §6 Gates observados

| Gate | Resultado |
|---|---|
| `c3l3_dynamic_expansion_vertical` | **12 / 0** |
| `aiw_s4_dynamic_expansion` (W01..W11, intocada) | **7 / 0** |
| `cargo fmt --check` | limpio |
| `cargo clippy -p sddk-engine --all-targets -- -D warnings` | exit 0 (1 `too_many_arguments` annotate con justificación, mismo trade que `cycle_replan`) |
| `cargo test -p sddk-engine` (perfil scoped completo) | consignado en §8 |

## §7 Límites declarados

1. **`executed_node_ids` es selección y contabilidad, no evaluación.** Escribirlo en el SCOPE-CONTRACT *antes* del código es lo que impide reinterpretar el alcance después. `AIW-S4` queda en **IMPLEMENTED → re-verificable**, no VERIFIED.
2. **La selección incremental usa `base_ir` que el propio trigger declara.** Un trigger deshonesto (declarando un `base_ir` más pobre que el real) podría sobre-despachar. Mitigado, no cerrado: la defensa real sería cruzar `base_ir` contra el `normalized` del evento aplicado previo.
3. **`cycle_replan` no se modificó.** Sigue sin idempotencia por diseño actual; su frontera queda documentada en W02 en vez de fingir un fix.
4. **La revisión raíz es sintética** (`sha256("plan-root|" + manifest de cycle.start)`), porque no existe un `PlanRevisionV1` inicial para el ciclo. Es content-derived y reconstruible, pero no es una revisión de plan real.
5. **No se ejecutó el perfil completo del workspace** al escribir este recibo (corresponde a `verify`/release, no a `apply`).
6. **Sin release.** El bump ceremonial queda para el flujo canónico con autorización del operador (§8 de AGENTS.md).

## §8 Siguiente paso

**C3l.4 — External test semantics: ausencia ≠ PASS** (los tests Chronos pueden salir verdes vía `None => return`). Después C3l.5 (X04 concurrencia real `SQLITE_MULTI_PROCESS`) y C3l.6 (X07 segundo binario real).
