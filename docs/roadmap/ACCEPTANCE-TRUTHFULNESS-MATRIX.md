# ACCEPTANCE TRUTHFULNESS MATRIX — baseline AIW-S0..S8 + Context-First R0..R11

**Congelada:** 2026-09-30 (session-51, C3l.0) · **Baseline:** `main@262d2a35` · **Release de referencia:** `v2.4.0`
**Fuente de autoridad:** `docs/sddk-roadmap-acceptance-truthfulness-2026-09-30/ROADMAP-ACCEPTANCE-TRUTHFULNESS.md` (C3l.0)
**Estado del arte que audita:** receipts históricos en `docs/roadmap/receipts/` y `tests/cycle-artifacts/`, `docs/history/proposals/all-proposals/2026-09-19-adaptive-inputs-workflows/STATE-OF-AIW.md`, bundle Context-First `03-ROADMAP/ROADMAP.md` (R0→R11).

## Reglas de esta matriz (C3l.0)

1. **No se reescribe evidencia histórica.** Un receipt de un SHA pasado queda tal cual; su veredicto vale SOLO para ese SHA y perfil.
2. **Solo se re-clasifican los claims afectados** por un defecto declarado del paquete: `VERIFIED → IMPLEMENTED`, `VERIFIED → NOT_VERIFIED`, `PASS → NOT_RUN`. Las filas sin defecto declarado se dejan con su claim y su evidencia, marcadas `sin re-clasificar`.
3. **`boundary_class`** describe la frontera que la evidencia citada CRUZA DE VERDAD (vocabulario cerrado): `PURE · IN_PROCESS · THREAD · PROCESS · FILESYSTEM · SQLITE_MULTI_HANDLE · SQLITE_MULTI_PROCESS · MCP_EXTERNAL · RELEASE_ARTIFACT`. Cuando el claim declara una frontera mayor que la cruzada, la columna `Frontier real` manda.
4. **Sufijos prohibidos sin frontera cruzada** (C3l.0 acción 4): un test no puede llamarse `e2e`, `real`, `external`, `two-cli` ni `second-binary` si no atraviesa esa frontera. La aplicación mecánica de esta política es C3n.1 (taxonomy gate).
5. `NOT_RUN` no cuenta como PASS; `BLOCKED_EXTERNAL_DEPENDENCY` tampoco (C3l.4).

Leyenda de estados: `VERIFIED` (frontera observada, anclada a SHA) · `IMPLEMENTED` (existe, frontera declarada no cruzada aún) · `NOT_VERIFIED` (claim activo refutado o sin frontera) · `CANCELLED`/`DEFERRED` (por regla propia).

---

## Tabla 1 — AIW-S0..S8 (fuente de claims: STATE-OF-AIW + receipts)

| Item | Requirement (resumen) | Implementación | Frontier real (boundary_class) | Test citado | Evidence | Status PRE | Status POST (re-clasificado) | Trigger reapertura |
|---|---|---|---|---|---|---|---|---|
| AIW-S0 | Preflight de un único slice | Proceso (SCOPE-CONTRACT del primer slice) | PURE | — | `tests/cycle-artifacts/p-63676b11dc0ef88f/aiw-s0-preflight/SCOPE-CONTRACT.md` | CLOSED | CLOSED (sin re-clasificar) | — |
| AIW-S1 | CogniCode real → Verify consume ObservationSet | `code_intelligence_port` + `verify_kernel` evidence source | **MCP_EXTERNAL** (binario real: exit=0, verdict `Verified`, 10 obs, digest SHA-256, EXT 5/5) | UAT-A01..A09 + EXT | `aiw-s1-cognicode-real/RECEIPT.md`, commits `f65e182`/`427bbf4` | DELIVERED (real) | **VERIFIED** (anclado a su SHA) — la frontera SÍ se cruzó; C3l.4 no re-clasifica, obliga a re-observación | C3l.4 + C3n.2 (AT-UAT-009/010) |
| AIW-S1b | Sucesora de evidencia observacional | Gate ejecutado: carencia NO demostrada | SQLITE (crash/reopen real en test) | UAT-A10 (`observation_set_durability.rs`) | `aiw-s1b-successor-shape/RECEIPT.md` | CANCELLED | CANCELLED (sin re-clasificar) | — |
| AIW-S2 | Captura estructurada de tests/gates | `runner_receipt.rs` + e2e | FILESYSTEM (receipt surface local; sin runner externo) | 10 unit + 11 E2E + bounded 18/18 | `aiw-s2-test-runner-capture/RECEIPT.md`, `ab9fe23` | DELIVERED (T08 partial) | DELIVERED (sin re-clasificar; T08 sigue parcial) | — (C3n.2: no repetir si no cambia el contrato) |
| AIW-S3 | Handoff durable entre dos tareas | `storage_adapter.rs` | SQLITE (durable, mismo proceso) | 5 integration + 6 unit | `aiw-s3-handoff-durable/RECEIPT.md`, `98614fc` | DELIVERED | DELIVERED (sin re-clasificar) | — |
| AIW-S4 | Expansión dinámica por evidencia | `dynamic_expansion.rs` (nuevo): la vertical existe como superficie de producción; `aiw_s4_dynamic_expansion.rs` sigue siendo composición directa de `cycle_replan` | IN_PROCESS/SQLITE (nueva) + IN_PROCESS (composition test antigua) | `c3l3_dynamic_expansion_vertical.rs` (12 tests: F1..F7 + 4 exit gate + 2 decisión/control negativo) | `session54-c3l3-dynamic-expansion-vertical/RECEIPT.md`, `7360c32e` | DELIVERED | **IMPLEMENTED → re-verificable** (session-54; era IMPLEMENTED_NOT_VERIFIED). AT-UAT-006/007/008 PASS. **Residual declarado:** `executed_node_ids` es un ledger de *selección y contabilidad* de nodos despachados, NO evaluación de operadores (DW-RUNTIME-003/004/005, fuera de scope del compiler). `AIW-S4` no puede pasar a VERIFIED con esta slice. | C3n.2 (re-certificar con la frontera real) |
| AIW-S5 | Evidencia Chronos de escenario REAL | Adaptador Chronos + receipts + `ext_outcome` + launcher | **MCP_EXTERNAL** (provider ausente: BLOCKED, nunca PASS; frontera ya no reportable como verde) | `ext_outcome` 4 unit (incluye enum↔launcher) + `aiw_s5_chronos_real` 1 ordinary + 2 `#[ignore]` | `aiw-s5-chronos-runtime/RECEIPT.md`, `6d826044` | DELIVERED | **IMPLEMENTED → re-verificable** (session-55: la semántica ausencia≠PASS está implementada y falsificada; la captura real sigue **BLOCKED**, `chronos-mcp` no instalado) | C3n.2 (AT-UAT-010 requiere un provider real instalado) |
| AIW-S6 | Correlación estático/runtime | Compuesta sobre A4-5a existente | IN_PROCESS | `a4_5a_intelligence_loop_composition.rs` (10+) | `aiw-s6-correlation-a8/RECEIPT.md` | DELIVERED (by-infrastructure) | DELIVERED (sin re-clasificar) | — |
| AIW-S7a | Producer→L0 stream | `ProducerToL0Adapter` compone `Arc<SecretaryL0Engine>` inyectado desde session-53: las reglas registradas disparan por `dispatch()` público; exit-gate como test propio (engine vacío ⇒ 0 señales) | IN_PROCESS (tests unit del gateway) | **falsificador principal C3l.2 en `aiw_s7a_producer_l0.rs`** (RED observado antes del fix: `with_engine` no existía; el test antiguo fijaba el defecto como esperado y reconstruía el evento a mano — prohibido por C3l.2) | commit de session-53; gateway 133/0 | DELIVERED | **IMPLEMENTED → re-verificable**: defecto C3l.2 cerrado con falsificadores; estado definitivo lo fija C3n.2 | C3n.2 (re-ejecución del suite S7a sobre el SHA nuevo) |
| AIW-S7b/c | Snapshot→SecretaryL1; AuthorityContext | Sub-slices con tests propios | IN_PROCESS | 8 + 13 tests | commits `118e969`/`7fca3d9` | DELIVERED | DELIVERED (sin re-clasificar; S7 global → IMPLEMENTED) | — |
| AIW-S8 | Fronteras CLI/host (X04/X07) | X04: dos `AgentHost` + `Arc<InMemoryLeaseStore>` EN EL MISMO PROCESO; X07: dos handles `Storage` en el mismo test | IN_PROCESS (declaraba SQLITE_MULTI_PROCESS / PROCESS) | tests X04/X07 existentes | STATE-OF-AIW + C3l.5/C3l.6 | DELIVERED (claim multi-proceso) | **NOT_VERIFIED** para los claims de concurrencia/segundo-binario (C3l.5/C3l.6) | C3l.5 + C3l.6 (AT-UAT-011..014) |

## Tabla 2 — Context-First R0..R11 (fuente de claims: bundle `03-ROADMAP/ROADMAP.md` + conformance receipt 09-09)

| Item | Requirement (resumen) | Implementación | Frontier real | Test citado | Evidence | Status PRE | Status POST | Trigger |
|---|---|---|---|---|---|---|---|---|
| R0 | Context boundary cut (módulos domain/engine + fitness) | Layout de módulos vigente; conformance receipt histórica | IN_PROCESS + RELEASE_ARTIFACT (receipt anclado a `0c2ca56`, SDDK 1.169.19) | 09-09-CONFORMANCE-RECEIPT (100% PASS, SOLO su SHA) | bundle 08-BASELINE-CONFORMANCE | VERIFIED (histórico) | VERIFIED **para su SHA** + gate actual comprometido (ver fila gate) | C3l.7 + C3m.5 (AT-UAT-015/022) |
| R1 | Consolidación Semantic Core (Fact/Object/Projection/Ephemeral, Evidence universal, Goal→…→Run) | Crates domain/engine/storage según ADRs 0044..0097 | IN_PROCESS | suites engine/storage | `docs/architecture/README.md` + ADRs | VERIFIED | VERIFIED (sin re-clasificar; sin defecto del paquete) | — |
| R2 | Decision + Knowledge substrate (KnowledgeAssertion/Basis, KMT v1, SemanticGraph) | Decision Memory (SPEC-004) estable; **KMT con TRES significados coexistentes** (C3m.0) y `KnowledgeBasis::revise` con docs/código discrepantes (C3m.2) | IN_PROCESS | tests knowledge existentes | C3m.0/C3m.2 del paquete | VERIFIED (substrate) | **IMPLEMENTED** — substrate decision OK; claims de conocimiento con terminología/identidad sin resolver | C3m.0 + C3m.2 (AT-UAT-016/019) |
| R3 | Agent Experience advisory split | `advisory_context` + fitness Alignment≠EffectiveInstructions | IN_PROCESS | fitness tests existentes | C3n.3: NO se reabre | VERIFIED | VERIFIED (sin re-clasificar) | solo regresión observada |
| R4 | Software Alignment core BASE (lenses, workbooks) | Núcleo sólido; **Snapshot L1 con confidence mágica 0.95** (C3m.4) | IN_PROCESS | tests alignment | C3m.4 | VERIFIED (BASE) | BASE sólido VERIFIED; **snapshot → IMPLEMENTED** | C3m.4 (AT-UAT-021) |
| R5 | Verify delta knowledge sync (KMT diff, invalidación incremental) | **La invalidación incremental requiere KMT real que no existe** (C3m.1); staleness/cards sí | IN_PROCESS | tests staleness (SPEC-012) | C3m.1 + C3n.3 | VERIFIED (sync) | sync parts IMPLEMENTED/VERIFIED; **invalidación incremental → NOT_VERIFIED** | C3m.1 (AT-UAT-017/018) |
| R6 | DebVerify global reconciliation | `reconcile` respeta `ChallengeError` desde session-52 (`22459708`): `StrategyFailure{strategy_id, reason}` tipado, variante `Incomplete`, `ConfirmedBaseline`/`AcceptedDebt` inalcanzables con fallos, `strategies_run` cuenta completadas | IN_PROCESS (tests unit del kernel) | **5 falsificadores C3l.1 en `debverify_kernel/tests.rs::c3l1_falsifiers`** (RED observado antes del fix por tipos ausentes; GREEN 34/34 después) | commit `22459708`; engine 1358/0 | VERIFIED | **IMPLEMENTED → re-verificable**: el defecto C3l.1 está cerrado con falsificadores; el estado definitivo (VERIFIED) lo fija C3n.2 re-ejecutando AT-UAT-002/003 sobre el nuevo SHA | C3n.2 (re-ejecución de los falsificadores ya en el suite) |
| R7 | Static Enhanced provider (CogniCode RPC) | Adaptador real ejercitado con binario real (misma evidencia que AIW-S1) | MCP_EXTERNAL (su SHA) | EXT S1 | receipt S1 | VERIFIED | VERIFIED (su SHA) + trigger C3l.4/C3n.2 | C3l.4 (AT-UAT-009/010) |
| R8 | Runtime Enhanced provider (Chronos RPC, fingerprints) | Provenance hardcodeada `chronos-mcp`/`ProviderKind::Null` en el path genérico (C3m.3); confidence mágica (C3m.4) | IN_PROCESS (declaraba provider-neutral) | tests runtime | C3m.3/C3m.4 | VERIFIED | **IMPLEMENTED** | C3m.3 + C3m.4 (AT-UAT-020/021) |
| R9 | Control tower + lens ecosystem | Dashboards UAT + kits presentes | RELEASE_ARTIFACT (dashboards generados) | uat dashboard renders | `docs/uat`, kits | IMPLEMENTED/VERIFIED parcial | sin re-clasificar (sin defecto del paquete) | — |
| R10 | Governance ratchets, waivers expirables | Waivers existen; **el gate de arquitectura cuenta un FAIL error como PASS esperado** (C3l.7): `OPEN_DEBT/WAIVED/FIXED` sin distinguir | IN_PROCESS + RELEASE_ARTIFACT | `check-architecture` + su test | C3l.7 | IMPLEMENTED | IMPLEMENTED + **claim "architecture conformant" → NO VÁLIDO** hasta que el gate distinga deuda esperada de conformidad | C3l.7 (AT-UAT-015) |
| R11 | Evaluate crate splits (por métricas) | Sin split; condicionado a métricas (C5) | — | — | ROADMAP C5 | DEFERRED | DEFERRED (sin re-clasificar) | C5 triggers |

## Fila transversal: gate de arquitectura

El claim «arquitectura conforme» de CUALQUIER perfil queda **no válido** desde esta matriz: hoy `sddk dev check-architecture` puede terminar `ARCH001 FAIL, exit 1` y el test lo considera correcto, sin distinguir `OPEN_DEBT`/`WAIVED_UNTIL(date)`/`FIXED` (C3l.7). Ningún perfil puede reclamar conformidad arquitectónica hasta cerrar C3l.7, y C6 no consume esa conformidad como primitiva (regla de promoción del paquete).

## Qué NO cambia (explícito)

- Receipts históricos: **intactos**. Sus veredictos valen para su SHA, no para el HEAD.
- AIW-S2/S3/S6, S7b/c, S1b, R1, R3, R9, R11: sin defecto declarado del paquete → **sin re-clasificación** (regla: solo claims afectados).
- AIW-S1/R7: la frontera MCP_EXTERNAL SÍ se cruzó en su día (binario real observado); C3l.4 no re-clasifica eso — obliga a que los tests ordinarios dejen de poder ponerse verdes por ausencia del provider, y C3n.2 re-observa.

## Uso de esta matriz

- Cada slice C3l/C3m que cierre **promueve** las filas que reabrió (su UAT AT-UAT-* lo demuestra) y registra el nuevo SHA de evidencia aquí, sin borrar la fila anterior (append de columna de cierre o sección de historial).
- C3n.2/C3n.3 re-certifican **solo** las filas en `IMPLEMENTED`/`NOT_VERIFIED`.
- Ningún perfil puede reclamar CERTIFIED para capacidades cuyas filas no estén en `VERIFIED` con SHA de la re-certificación (regla de promoción del paquete).
