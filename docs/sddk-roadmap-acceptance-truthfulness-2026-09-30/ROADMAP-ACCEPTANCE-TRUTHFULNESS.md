# SDDK — Roadmap de saneamiento de contratos y Acceptance Truthfulness

**Edición:** 2026-09-30  
**Baseline auditado:** `main@6fbe1990ac40826a1f52ddf5b52c145fbea07f13`  
**Release pública de referencia:** `v2.3.3`  
**Roadmap autoridad:** `docs/roadmap/ROADMAP.md`  
**Objetivo:** reparar falsos verdes, drift contractual y wiring incompleto detectados al contrastar los roadmaps históricos AIW + Context-First contra el código actual, sin reabrir indiscriminadamente hitos que sí están demostrados.

---

## 0. Decisión de integración

Este plan **no reemplaza C3j**.

C3j continúa con contexto durable / session binding / hipermedia porque sus objetivos no dependen de los defectos encontrados aquí.

Se añade una vía paralela obligatoria:

```text
                         ┌── C3j Context / Hypermedia ───────────────┐
C3i CLOSED ──────────────┤                                          ├── C4/C6
                         └── C3l → C3m → C3n ───────────────────────┘
                              │     │     │
                              │     │     └─ certificación real
                              │     └─ convergencia semántica
                              └─ acceptance truthfulness
```

### Regla de promoción

- C3j puede seguir avanzando en paralelo.
- C4 puede publicar Base siempre que no reclame capacidades afectadas como certificadas.
- Ningún perfil que reclame **AIW completo**, **Context-First completo**, **runtime enhanced**, **dynamic workflow expansion**, **Secretary producer integration** o **arquitectura conforme** puede ser `CERTIFIED` hasta cerrar C3l/C3m/C3n aplicables.
- C6 no debe consumir como primitivas “certificadas” contratos que C3l/C3m haya re-clasificado como `IMPLEMENTED` o `NOT_VERIFIED`.

---

# C3l — Acceptance Truthfulness & False-Green Elimination

**Prioridad:** P0  
**Tipo:** saneamiento correctness / certification  
**Dependencia:** C3i cerrado. Puede ejecutarse en paralelo con C3j.  
**Objetivo:** conseguir que los tests y receipts demuestren exactamente la frontera que dicen demostrar.

## C3l.0 — Re-clasificación honesta del baseline

### Objetivo

Congelar una matriz:

```text
requirement
→ implementation
→ boundary realmente ejercitada
→ test
→ evidence
→ current status
```

para AIW-S0..S8 y R0..R11 del bundle Context-First.

### Acciones

1. No borrar receipts históricos.
2. Re-clasificar únicamente los claims afectados:
   - `VERIFIED` → `IMPLEMENTED`
   - `VERIFIED` → `NOT_VERIFIED`
   - `PASS` → `NOT_RUN`
   cuando la prueba no cruza la frontera declarada.
3. Introducir `boundary_class` en nuevos UAT:
   - `PURE`
   - `IN_PROCESS`
   - `THREAD`
   - `PROCESS`
   - `FILESYSTEM`
   - `SQLITE_MULTI_HANDLE`
   - `SQLITE_MULTI_PROCESS`
   - `MCP_EXTERNAL`
   - `RELEASE_ARTIFACT`
4. Prohibir usar el sufijo `e2e`, `real`, `external`, `two-cli` o `second-binary` si la prueba no atraviesa esa frontera.

### Exit gate

La matriz puede responder, para cualquier hito, **qué frontera se observó realmente** sin leer el nombre del test.

---

## C3l.1 — DebVerify fail-closed

### Defecto

`DebVerifyKernel::reconcile` ignora `ChallengeError`; puede terminar en `ConfirmedBaseline` aunque una o más estrategias hayan fallado.

### Invariante

```text
strategy_error
    ⇒
summary != ConfirmedBaseline
```

### Diseño

Preferencia:

```text
ChallengeError
    ↓
StrategyFailure { strategy_id, typed_reason }
    ↓
ReconciliationSummary::EvidenceGap / Incomplete
```

No introducir score ni booleanos ambiguos.

### Falsificadores

- una estrategia falla;
- todas las estrategias fallan;
- una falla y otra no encuentra findings;
- una falla y otra encuentra contradicción;
- `strategies_run` cuenta ejecuciones completadas, no sólo aplicables.

### Exit gate

Imposible producir `ConfirmedBaseline` si una estrategia aplicable no terminó satisfactoriamente.

### Reapertura

`R6 / DebVerify` pasa de `VERIFIED` a `IMPLEMENTED` hasta cerrar esta slice.

---

## C3l.2 — Producer → Secretary L0 wiring real

### Defecto

`ProducerToL0Adapter::dispatch()` crea un `SecretaryL0Engine::new()` vacío, por lo que ninguna regla productiva registrada puede dispararse por la ruta pública.

### Diseño objetivo

```text
ProducerEvent
    ↓
ProducerToL0Adapter
    ↓
ReactiveEvent
    ↓
SecretaryL0Port
    ↓
configured SecretaryL0Engine
    ↓
ReactiveSignal
```

El adapter no crea el engine.

Opciones aceptables:

1. `Arc<dyn SecretaryL0Port>`;
2. `Arc<SecretaryL0Engine>`;
3. composición equivalente en application layer.

### Restricciones

- no mover authority dentro del producer adapter;
- no hardcodear reglas CogniCode/Chronos en el motor;
- mantener `Unknown` silencioso;
- conservar determinismo y cooldown.

### Falsificador principal

```text
register rule
→ ProducerToL0Adapter::dispatch(real ProducerEvent)
→ expected signal
```

Está prohibido reconstruir manualmente un `ReactiveEvent` desde el test para demostrar la ruta.

### Exit gate

La API pública del adapter dispara una regla registrada y el test falla si se sustituye el engine inyectado por uno nuevo vacío.

### Reapertura

`AIW-S7a` → `NOT_VERIFIED` hasta cierre.

---

## C3l.3 — Dynamic Workflow Expansion E2E real

### Defecto

`aiw_s4_dynamic_expansion` llama directamente a `cycle_replan`; no prueba evidence → Secretary → orchestration → authority → plan revision → incremental runtime.

### Vertical mínima obligatoria

```text
Observation / EvidenceGap
        ↓
Secretary proposal
        ↓
Orchestration decision
        ↓
Authority validation
        ↓
typed ReplanDelta
        ↓
PlanRevision N+1
        ↓
incremental execution
        ↓
receipt
```

### Idempotencia

Debe existir identidad estable del trigger/proposal.

```text
apply(trigger A)
apply(trigger A)

revision_delta_count == 1
new_node_execution_count == 1
```

### Falsificadores

1. trigger duplicado;
2. proposal replay;
3. authority denied;
4. invalid delta;
5. execution failure del nuevo nodo;
6. restart entre proposal y apply;
7. plan base cambió antes del apply.

### Exit gate

- exactamente una expansión ante replay;
- nodos previos no se re-ejecutan;
- denied/invalid no cambia plan;
- nueva revisión tiene lineage y receipts.

### Reapertura

`AIW-S4` → `IMPLEMENTED_NOT_VERIFIED`.

---

## C3l.4 — External test semantics: ausencia ≠ PASS

### Defecto

Los tests reales de Chronos pueden salir verdes mediante `None => return`.

### Política

Un provider externo produce uno de:

```text
PASS_OBSERVED
FAIL_OBSERVED
BLOCKED_EXTERNAL_DEPENDENCY
NOT_RUN
NOT_APPLICABLE
```

Nunca `PASS` por ausencia.

### Implementación

Separar:

- tests ordinarios;
- `#[ignore]` / perfil EXT;
- launcher que traduce ausencia a `BLOCKED/NOT_RUN`;
- receipt con binary path, version, hash y capabilities.

### Aplicar a

- Chronos;
- CogniCode;
- futuros providers;
- JCode cuando exista adapter real.

### Exit gate

Eliminar todo patrón equivalente a:

```rust
let Some(bin) = ... else { return; };
```

dentro de tests que certifican integración externa.

---

## C3l.5 — X04: dos CLI / multi-process concurrency real

### Defecto

X04 usa dos `AgentHost` y un `Arc<InMemoryLeaseStore>` en el mismo proceso.

### Escenario real

```text
temp ledger.sqlite

process A: sddk ... acquire/update
process B: sddk ... acquire/update

             ↓

same durable store
```

### Probar

- first-writer/lease winner;
- fencing monotónico;
- loser no muta;
- crash del winner;
- expiry/reacquire;
- reopen;
- WAL/busy timeout;
- no state divergence.

### Exit gate

La prueba usa al menos dos PIDs distintos y SQLite durable compartido.

`AIW-S8/X04` no se considera verificado antes.

---

## C3l.6 — X07: segundo binario real

### Defecto

X07 usa dos handles `Storage` dentro del mismo test process.

### Objetivo

Crear un consumidor mínimo real, preferiblemente reutilizando el binario SDDK si existe una superficie read-only suficiente.

```text
writer process
  ↓
ledger.sqlite
  ↓
reader process / second binary
```

### El consumidor debe demostrar

- lectura de project/workspace/cycle/events;
- schema guard;
- read-only enforcement;
- no side effects;
- compatibilidad observable.

### Restricción

No introducir un binario permanente sólo para satisfacer el test si una superficie CLI real ya cubre la lectura.

---

## C3l.7 — Architecture checker: FAIL conocido no es verde

### Defecto

El test actual considera correcto que:

```text
sddk dev check-architecture --root .
→ ARCH001 FAIL
→ exit 1
```

### Política

Una violación severity=error sólo puede estar en uno de:

```text
OPEN_DEBT
WAIVED_UNTIL(date)
FIXED
```

No puede contarse como architecture gate PASS.

### Slice

1. identificar edge `engine → storage`;
2. decidir:
   - eliminarla; o
   - waiver explícito con owner, reason, expiry y revisit trigger.
3. separar:
   - characterization test del debt;
   - certification gate.

### Exit gate

El gate de certificación entiende la diferencia entre `expected debt` y `architecture conformant`.

---

# C3m — Semantic & Boundary Convergence

**Prioridad:** P1  
**Dependencia:** C3l.1–C3l.4 cerrados para los contratos afectados.  
**Puede convivir con C3j.**

## C3m.0 — ADR: significado canónico de KMT

### Problema

Actualmente coexisten tres significados:

```text
Knowledge Merkle Tree
Knowledge Management Tiers
Knowledge-Machine Topology
```

### Decisión obligatoria

Elegir una autoridad terminológica y semántica.

### Opción recomendada

Reservar:

```text
KMT = Knowledge Merkle Tree
```

porque es la pieza arquitectónica de invalidación incremental del roadmap original.

Renombrar los conceptos actuales:

```text
Knowledge Management Tiers
→ KnowledgeFreshness / KnowledgeBasisStatus

Knowledge-Machine Topology index
→ UnitImpactIndex / NamespaceUnitIndex
```

### No objetivo

No hacer rename masivo antes de aceptar el ADR.

---

## C3m.1 — Knowledge Merkle Tree mínimo real

Sólo si el ADR mantiene el contrato original.

### Fingerprint de unidad

```text
UnitFingerprint {
  source_hash,
  structure_hash,
  symbols_hash,
  dependency_hash,
  behavior_hash,
  knowledge_hash,
  intent_hash,
  decision_refs_hash,
  analyzer_set_hash
}
```

### Jerarquía mínima

```text
project
  → crate/package
    → module/directory
      → file
        → symbol
```

No todas las capas tienen que tener persistencia dedicada inicialmente; sí identidad y derivación determinista.

### Operaciones

- build;
- diff;
- invalidate;
- affected_units;
- as_of/ref;
- analyzer-set change.

### Falsificadores

- source cambia;
- dependency cambia sin source;
- analyzer cambia sin source;
- behavior cambia sin source;
- insertion order;
- rebuild;
- branch parcial.

### Exit gate

Un cambio en una dimensión invalida únicamente las ramas/unidades dependientes demostrables.

---

## C3m.2 — Corregir incoherencia de `KnowledgeBasis::revise`

### Problema

Documentación y código discrepan sobre si `revised_at` participa en `basis_hash`.

### Decisión

Una de dos:

**A. Content identity**
- `basis_hash` sólo depende de assertions.
- `revised_at` NO participa.
- corregir docs/tests/naming.

**B. Revision identity**
- `revised_at` participa.
- `derive_basis_hash(assertions, revised_at)`.

### Preferencia

Mantener separados:

```text
content_hash
revision_id
```

para no hacer que el mismo conocimiento tenga distinta identidad de contenido sólo por el reloj.

### Exit gate

No queda comentario ni test que afirme la semántica opuesta.

---

## C3m.3 — Runtime provider-neutral provenance

### Problema

El generic runtime path inyecta:

```text
chronos-mcp
aiw-s5-capture
ProviderKind::Null
```

### Modelo objetivo

```text
RuntimeProviderIdentity {
  provider_id,
  provider_version,
  protocol_version,
  capability_snapshot_digest
}

CaptureBasis {
  source_revision,
  request_digest,
  instrumentation,
  sampling,
  completeness
}
```

`RuntimeCaptureResult` conserva esa procedencia hasta `SoftwareObservation`.

### Restricción

Literales de Chronos sólo en:

```text
runtime_evidence_port_mcp.rs
```

o package/adapter equivalente.

### Fitness test

El engine genérico no puede contener provider ids concretos.

---

## C3m.4 — Eliminar confidence mágica de Snapshot L1

### Problema

`log_head > 0 => 0.95`, si no `0.5`.

### Sustitución

Preferencia:

```text
SnapshotEvidenceState =
    Observed { evidence_ref }
  | Empty
  | Stale
  | Conflicted
  | Missing
```

Si `SecretaryProposal.confidence` debe mantenerse por compatibilidad:

```text
confidence: Option<Confidence>
confidence_basis: Option<ConfidenceBasis>
```

y Snapshot L1 no fabrica una probabilidad.

### Exit gate

Ninguna cifra de confianza aparece sin basis declarada.

---

## C3m.5 — R0: bounded-context decision

### Problema

R0 pedía migración física; el árbol conserva más de cien módulos root-level en `sddk-engine/src`.

### No hacer big bang

Primero clasificar cada root module por ownership:

```text
planning
execution
decision
knowledge
alignment
verification
governance
agent_experience
extension
shared
composition_root
```

### Salida

`CONTEXT-MAP.yaml` canónico:

```text
module
owner_context
allowed_dependencies
migration_status
adr
```

### Estrategia

Strangler:

1. nuevos módulos sólo en context correcto;
2. mover módulos al tocarlos si coste bajo;
3. façade temporal;
4. eliminar façade cuando consumidores converjan.

### Exit gate

El fitness test valida la **arquitectura objetivo**, no una allowlist histórica de root files.

---

# C3n — Production Boundary Certification

**Prioridad:** P1  
**Dependencia:** C3l cerrado; C3m aplicable cerrado o explícitamente deferred con ADR.  
**Objetivo:** volver a certificar las capacidades afectadas usando fronteras reales.

## C3n.1 — Test taxonomy gate

Cada UAT declara:

```yaml
boundary:
mode:
provider:
binary_sha256:
storage:
process_count:
falsifier:
```

### El vocabulario de frontera, declarado aquí y en ningún otro sitio

Session-69s bis 5: medido, el conjunto de niveles «que exigen frontera» estaba
declarado **en dos sitios y no coincidían**, y el nombre del nivel decidía si una
fila exigía o no. Esto es **la autoridad canónica**; `docs/roadmap/UAT-MATRIX.md`
referencia esta sección y **no vuelve a nombrar el conjunto**.

**Vocabulario medido sobre las 26 filas `AT-UAT` del overlay** (las dos matrices
cubren los mismos 26 IDs; no hay filas huérfanas en ninguna dirección):

| Nivel | Filas | Qué frontera cruza |
|---|---|---|
| `PURE` | 6 | ninguna: función, memoria, sin proceso ni almacenamiento |
| `IN_PROCESS` | 6 | misma imagen de proceso; el «binario» es el propio binario de test |
| `IN_PROCESS/SQLITE` | 3 | **compuesto**: mismo proceso sobre almacenamiento durable |
| `SQLITE_MULTI_PROCESS` | 2 | **dos procesos** compitiendo por estado durable compartido |
| `PROCESS` | 4 | proceso hijo real del producto |
| `MCP_EXTERNAL` | 2 | proveedor externo por MCP, con binario y hash propios |
| `MIXED` | 2 | **compuesto**: atraviesa más de una frontera |
| `RELEASE_ARTIFACT` | 1 | el artefacto publicado, tal como lo instala un usuario |
| `FILESYSTEM` | 0 (la usa `AIW-S2`) | superficie de receipt en el sistema de ficheros local, sin runner externo |

**Conjunto que EXIGE receipt completo de frontera** — *decisión del operador,
session-69s: literal a esta sección*:

```text
PROCESS
MCP_EXTERNAL
RELEASE_ARTIFACT
```

El runner rechaza receipts incompletos para esos niveles. La lectura es
**literal**: una fila exige si y solo si su nivel declarado es **exactamente** uno
de esos tres.

**Y lo que esa decisión deja sin cubrir, declarado y no omitido** (session-69s
bis 5, 13 de 26 filas): `IN_PROCESS` (6, ids 002-005, 020, 021),
`IN_PROCESS/SQLITE` (3: 006-008), `SQLITE_MULTI_PROCESS` (2: 011, 012) y `MIXED`
(2: 024, 025). **Las unicas que cruzan una frontera de estado durable son
5** — 006, 007, 008, 011, 012 — y son las filas donde un verde falso es más
fácil, porque «dos procesos y no se ven» es indistinguible de «se midió y
no pasó». Queda como **hueco declarado**, no como cobertura.

**Por qué el conjunto pequeño es defendible y no una comodidad:** `IN_PROCESS` es
el mismo proceso, luego `process_count` valdría 1 trivialmente y exigirlo sería un
campo decorativo — un guard que se cumple siempre enseña a leer verde sin leer.
`PURE` (6 filas) tampoco cruza nada. El coste real está en las 5 filas durable y
en las 2 `MIXED`, y queda escrito arriba para que la decisión sea revisable.

### El nombre del nivel decidía si una fila exigía receipt

Medido: **`UAT-MATRIX.md` y el overlay discrepan en el nivel de 4 filas**, y el
overlay es la fuente declarada por la propia cabecera de la matriz:

| ID | `UAT-MATRIX.md` | overlay (fuente declarada) |
|---|---|---|
| `AT-UAT-011` | `PROCESS/SQLITE_DURABLE` | `SQLITE_MULTI_PROCESS` |
| `AT-UAT-012` | `PROCESS/SQLITE_DURABLE` | `SQLITE_MULTI_PROCESS` |
| `AT-UAT-013` | `PROCESS/SQLITE_DURABLE` | `PROCESS` |
| `AT-UAT-014` | `PROCESS/SQLITE_DURABLE` | `PROCESS` |

Con el nombre de la matriz las cuatro quedan exentas; con el del overlay, **013 y
014 exigen** receipt. **Un nombre decide si una fila exige o no**, que es la misma
clase de error que el rename de los pins `SDDK_LEGACY_CERT_*` que rompió la
certificación de cinco releases: **un nombre es un contrato con quien lo lee**, y
por eso las 4 filas se reconcilian al overlay, que es la fuente declarada.
Reconciliado en `537e73e3`..(session-69s bis 5) y verificado con el guard: las
cuatro filas nombran hoy lo que nombra el overlay.

> #### ⚠️ CORRECCIÓN (session-69s bis 6) — una afirmación de arriba era FALSA
>
> El párrafo que sigue a la tabla decía, literally:
> *«`PROCESS/SQLITE_DURABLE` **no aparece en ningún otro documento** y lo usan
> exactamente esas 4 filas»*. **Es falso, y se comprobó con `grep` sobre todo el
> repo, no leyendo.** Aparece en **cinco sitios más**:
>
> | Sitio | Dónde |
> |---|---|
> | `docs/roadmap/ACCEPTANCE-TRUTHFULNESS-MATRIX.md:33` | columna `Frontier real` de **AIW-S8** |
> | `tests/cycle-artifacts/p-63676b11dc0ef88f/session56-c3l5-x04-multi-process-concurrency/RECEIPT.md:7` | `boundary_class:` |
> | `tests/cycle-artifacts/p-63676b11dc0ef88f/session58-c3l6-x07-second-binary/RECEIPT.md:6` | `IN_PROCESS` → `PROCESS / SQLITE_DURABLE` |
> | `crates/sddk-storage/tests/x04_multi_process_concurrency.rs:17` | **fuente de producto** |
> | (más las 4 filas de la tabla de arriba) | |
>
> **Lo que cambia es el diagnóstico, no solo la frase.** El texto daba a entender
> un *typo* local en cuatro celdas, huérfano y sinsdkel resto. **No lo es:**
> `PROCESS / SQLITE_DURABLE` es la clasificación **declarada** de `AIW-S8` en la
> matriz de aceptación, está en los **dos** receipts de X04 y X07, y **un test del
> producto lo afirma en su propio fuente**. Es decir: es una **clasificación viva**
> —y lo que ocurre es que **la propia regla 3 de `ACCEPTANCE-TRUTHFULNESS-MATRIX.md`
> la excluye**, porque su vocabulario cerrado declara `SQLITE_MULTI_PROCESS`, no
> `SQLITE_DURABLE`.
>
> **La conclusión de reconciliación NO cambia** — las 4 filas siguen yendo al
> overlay, y el guard sigue comprobándolo. Lo que cambia es su **fuerza**: ya no
> se sostiene en «ese nombre no existe en ningún otro sitio», sino en lo que sí se
> puede medir, que es que **X04 es literalmente concurrencia de ≥2 PIDs reales**
> (`x04_multi_process_concurrency.rs`, aserción D0 de identidad de actor), luego
> `SQLITE_MULTI_PROCESS` **describe lo que el test hace** y `SQLITE_DURABLE` es un
> nombre que el vocabulario del repo no reconoce.
>
> **Y el hallazgo de fondo, que es mayor que la frase equivocada:** este repo tiene
> **dos vocabularios cerrados de frontera que no coinciden**, en dos documentos que
> ambos se declaran autoridad:
>
> | | valores | solo en este |
> |---|---|---|
> | C3n.1 (este §, 8 niveles) | `PURE` `IN_PROCESS` `IN_PROCESS/SQLITE` `SQLITE_MULTI_PROCESS` `PROCESS` `MCP_EXTERNAL` `MIXED` `RELEASE_ARTIFACT` | `IN_PROCESS/SQLITE`, `MIXED` |
> | `ACCEPTANCE-TRUTHFULNESS-MATRIX.md` regla 3 (9 valores) | `PURE` `IN_PROCESS` `THREAD` `PROCESS` `FILESYSTEM` `SQLITE_MULTI_HANDLE` `SQLITE_MULTI_PROCESS` `MCP_EXTERNAL` `RELEASE_ARTIFACT` | `THREAD`, `FILESYSTEM`, `SQLITE_MULTI_HANDLE` |
>
> Se cruzan en **6 de 8 / 6 de 9**. Y la matriz **además viola su propia regla 3**:
> usa `IN_PROCESS/SQLITE` (que no declara) y `PROCESS / SQLITE_DURABLE` (que no
> declara). **Un vocabulario cerrado que su propio documento no respeta no es un
> vocabulario cerrado.** Reconciliar los dos es trabajo de C3n.3 y **no se hace
> aquí**: este bloque declara el hallazgo, no finge cerrarlo.



### Reconciliación de los dos vocabularios (session-69s bis 8)

Session-69s bis 6 declaró que este repo tenía **dos vocabularios cerrados de
frontera que no coinciden** y que la matriz de aceptación **violaba su propio
vocabulario**. Aquí se cierra, y el criterio es uno solo: **un vocabulario, declarado
en un sitio, referenciado por el resto.**

| | antes | ahora |
|---|---|---|
| Dónde se declara | dos sitios (aquí y `ACCEPTANCE-TRUTHFULNESS-MATRIX.md` regla 3) | **sólo aquí** |
| Valores | 8 aquí, 9 allí, se cruzan en 6 | **9 aquí**, el mismo para los dos documentos |
| `FILESYSTEM` | lo declara la matriz y aquí no | **aquí**, porque `AIW-S2` lo usa de verdad |
| `THREAD`, `SQLITE_MULTI_HANDLE` | los declara la matriz y **ninguna fila los usa** | **retirados**: un vocabulario cerrado que declara valores que no existen no es cerrado, es una lista de deseos |

**Las cuatro correcciones, una a una, con la fila que las motiva:**

| Fila | Antes | Ahora | Por qué |
|---|---|---|---|
| `AIW-S1b` | `SQLITE (crash/reopen real en test)` | `IN_PROCESS/SQLITE` | crash/reopen dentro del test **es** el mismo proceso sobre almacenamiento durable |
| `AIW-S3` | `SQLITE (durable, mismo proceso)` | `IN_PROCESS/SQLITE` | **la celda se contradice sola**: dice «mismo proceso» y usa un valor sin el calificador `IN_PROCESS` |
| `AIW-S8` | `PROCESS / SQLITE_DURABLE` | `PROCESS / SQLITE_MULTI_PROCESS` | X04 afirma **≥2 PIDs reales** y X07 usa un **segundo binario**: eso es multi-proceso por definición, y `SQLITE_DURABLE` no existe en el vocabulario |
| `AIW-S2` | `FILESYSTEM` (declarado en la matriz, ausente aquí) | `FILESYSTEM` (declarado aquí) | el valor es legítimo; lo que faltaba era **dónde se declara** |

`AIW-S4` usa `IN_PROCESS/SQLITE (nueva) + IN_PROCESS (composition test antigua)`:
los dos valores son canónicos, luego la fila **no se toca** — y queda dicho, porque
`+` entre dos niveles es una fila con dos boundaries a lo largo del tiempo y eso es
información, no ruido.

**Lo que este § NO hace:** no reescribe la regla 3 de la matriz, la **remplaza por
una referencia**. Y el guard nuevo comprueba que ninguna fila use un valor fuera de
este vocabulario, para que la unión no vuelva aopenedirse por la vía de los datos.


---

## C3n.2 — AIW re-certification

Re-ejecutar únicamente lo que quedó re-clasificado:

- S1 external capabilities;
- S4 dynamic expansion;
- S5 Chronos;
- S7 producer L0;
- S8 X04/X07.

No repetir S2 si no cambia su contrato.

### Exit gate

Nueva tabla:

```text
AIW-S0 ... AIW-S8
IMPLEMENTED / VERIFIED / BLOCKED / N/A
evidence SHA
```

#### Tabla de exit gate — medida en session-69s bis 6

**Evidence SHA: `43c7aac2`** (el commit de C3n.1; el árbol estaba limpio).
Provider: `cognicode-mcp` **PRESENTE** (`~/.cognicode/shims/cognicode-mcp`);
`chronos-mcp` **AUSENTE**.

| Item | Comando | Resultado medido | Veredicto |
|---|---|---|---|
| `AIW-S1` | `cargo test -p sddk-engine --test aiw_s1_cognicode_real` | `2 passed; 3 ignored` | — ver abajo |
| `AIW-S1` | ídem, con `COGNICODE_MCP_BIN` + `--ignored` | **`3 passed; 0 failed`, 141,60 s** | **VERIFIED, pero fuera del gate** |
| `AIW-S4` | `--test aiw_s4_dynamic_expansion` | `7 passed` | **IMPLEMENTED** (residual intacto) |
| `AIW-S4` | `--test c3l3_dynamic_expansion_vertical` | `12 passed` | ídem |
| `AIW-S5` | `--test aiw_s5_chronos_real` | `2 passed; 2 ignored` | — |
| `AIW-S5` | ídem, con `--ignored` | `BlockedExternalDependency { env_var: "CHRONOS_MCP_BIN", found: "not on PATH and env var unset or empty" }` | **BLOCKED**, nunca PASS |
| `AIW-S7a` | `--test aiw_s7a_producer_l0` | `5 passed` | **IMPLEMENTED** (ver el hallazgo de abajo) |
| `AIW-S8` X04 | `-p sddk-storage --test x04_multi_process_concurrency` | `9 passed` | **VERIFIED** |
| `AIW-S8` X07 | `-p sddk-cli --test aiw_s8_x07_real_binary_boundary` | `6 passed` | **VERIFIED** |

**Lo que la tabla de arriba NO puede afirmar, y es lo más importante de ella:**

- **`AIW-S4` NO se promueve.** La matriz declara un residual vivo —`executed_node_ids` es un ledger de *selección y contabilidad* de nodos, **no** evaluación de operadores— y ese residual **no ha cambiado**. Sus 19 tests pasan, y pasar no cierra un residual que nadie ha tocado. Se queda en `IMPLEMENTED`.
- **`AIW-S5` es `BLOCKED`, y el bloqueo tiene tipo.** No es «no ejecutado»: es `BlockedExternalDependency`, que por la regla 5 de la matriz **no cuenta como PASS**. Correcto como está.
- **`AIW-S1` es el caso raro, y por eso lleva dos filas.** El producto **funciona** contra el provider real: `3 passed` en **141,60 s**. Pero **por defecto el gate no lo ve**: sin `--ignored` la suite da `2 passed; 3 ignored`, y **uno de los dos que corren es `a03_spawn_failure_is_unavailable`**, un test que afirma que el provider **no** está disponible. O sea: el `VERIFIED` de la matriz **solo es reproducible si alguien exporta una variable y pasa un flag**, y **`cargo test --workspace` —el gate de release— no lo cruza nunca**. El coste está medido: **141,60 s** por publicación es lo que costaría, y por eso no se ha conectado. **Es cobertura de sesión, no de pipeline**, igual que el estado O6 de `dev doctor`.

> #### ⚠️ HALLAZGO MEDIDO — la regla 4 no tiene guard, y la fila que la invoca viola la regla 4
>
> `ACCEPTANCE-TRUTHFULNESS-MATRIX.md` **regla 4** dice: *«un test no puede llamarse
> `e2e`, `real`, `external`, `two-cli` ni `second-binary` si no atraviesa esa
> frontera. **La aplicación mecánica de esta política es C3n.1 (taxonomy gate).**»*
>
> **Las dos mitades de esa frase son medidas y las dos son falsas.**
>
> **1. El guard de C3n.1 no aplica esa política.** Lee las 26 filas `AT-UAT` de
> `UAT-MATRIX.md` y de su overlay. **No lee `ACCEPTANCE-TRUTHFULNESS-MATRIX.md`,**
> que es donde vive la regla 4, **ni mira ningún nombre de test.** Cubre un
> conjunto de filas distinto del que la regla dice cubrir.
>
> **2. La propia fila de la matriz que nombra C3n.2 como re-apertura es la que
> viola la regla 4.** En `AIW-S7a` hay **cinco** tests, y **tres** se llaman
> `*_e2e`: `cognicode_finding_e2e`, `chronos_crash_e2e`, `chronos_race_e2e`.
> **Los tres se leen, los tres, y ninguno cruza la frontera que su nombre
> promete:**
>
> | Test | Lo que el nombre dice | Lo que el test hace (leído) |
> |---|---|---|
> | `cognicode_finding_e2e` | finding de CogniCode, extremo a extremo | construye `ProducerEvent::CogniCodeFinding` **a mano** y lo despacha in-process. Su propio comentario lo dice: *«same matcher shape the adapter emits for CogniCode findings»* — **la forma, no el finding** |
> | `chronos_crash_e2e` | crash de Chronos, extremo a extremo | construye `ProducerEvent::ChronosCrash` **a mano**. **No hay Chronos, no hay crash, no hay proceso** |
> | `chronos_race_e2e` | carrera de Chronos, extremo a extremo | construye `ProducerEvent::ChronosRace` **a mano** y afirma `signals.is_empty()` |
>
> La fila declara su `boundary_class` como **`IN_PROCESS` (tests unit del
> gateway)**, que **no es un nivel exigente** del vocabulario de C3n.1. Los tres
> tests **sí hacen trabajo real y valioso** —demuestran que la ruta pública
> `dispatch()` dispara la regla registrada, que es el defecto de C3l.2— y por eso
> **`AIW-S7a` NO se degrada**: se queda en `IMPLEMENTED` y el falsificador de C3l.2
> sigue valiendo. Lo que no vale es el **nombre**: `e2e` aquí no describe lo que
> el test cruza, y **un nombre es un contrato con quien lo lee** — el mismo
> criterio con el que se reconciliaron las 4 filas de `AT-UAT` justo arriba.
>
> **Lo que NO se hace aquí, y es deliberado:** renombrar los tests, tocar
> `crates/`, ni degradar la fila. Este bloque **declara y mide**; la corrección
> (un guard de política de nombres, o el rename) es trabajo con su propio ciclo.
> **Un hallazgo declarado que nada vigila envejece**, así que queda escrito como
> siguiente paso preciso, no como nota al pie.

---

## C3n.3 — Context-First re-certification

Re-evaluar únicamente:

- R0 context map;
- R2/KMT;
- R5 incremental invalidation;
- R6 DebVerify;
- R8 runtime semantics;
- architecture gate.

R3 Agent Experience y las partes ya sólidas de R4 no se reabren salvo regresión observada.


### Tabla de exit gate — medida en session-69s bis 9

**Evidence SHA: `87831cc5`.** Binario de HEAD, storage real, sin copia. Criterio de
esta tabla, el mismo que en C3n.2: **una fila que no nombra un artefacto ejecutable
no se re-certifica; se declara no re-certificable, y eso ya es un veredicto.**

| Fila | Qué cita como evidencia | ¿Ejecutable en HEAD? | Medido | Veredicto |
|---|---|---|---|---|
| `R0` contexts | `09-09-CONFORMANCE-RECEIPT` + commit `0c2ca56` | el commit resuelve; el receipt vive en `docs/history/legacy-packages/…`, o sea **un paquete histórico** | — | **VERIFIED sólo para su SHA**, no re-verificable aquí |
| `R2` KMT/substrate | «tests knowledge existentes» | **NO** — es una descripción, no una ruta | — | **IMPLEMENTED** (sin cambio) |
| `R5` invalidación | «tests staleness (SPEC-012)» | **NO** — ídem | — | **IMPLEMENTED / NOT_VERIFIED** (sin cambio) |
| `R6` DebVerify | `debverify_kernel/tests.rs::c3l1_falsifiers` | **SÍ** | **`6/6` passed**; `debverify_kernel` completo **`34/34`** | **IMPLEMENTED → re-verificable** |
| `R8` runtime | «tests runtime» | **NO** — ídem | — | **IMPLEMENTED** (sin cambio) |
| `R10` architecture gate | `verdict.rs` 11/11 + `check_architecture_gate.rs` 4/4 | **SÍ** | `check_architecture_gate` **`4/4`**; `verdict` **`15` por filtro**, **`8` en el fichero** | **IMPLEMENTED**, y el `11/11` **no reconcilia** |

### ⚠️ El resultado de C3n.3 no es «todo verificado»: es que la mitad de las filas no se pueden verificar

**De las seis filas que C3n.3 manda re-evaluar, sólo DOS nombran un artefacto
ejecutable.** Las otras cuatro citan una **descripción** de tests («tests knowledge
existentes», «tests staleness (SPEC-012)», «tests runtime») o un **receipt de un
paquete histórico**.

Eso no es un detalle de redacción. Es la diferencia entre un claim que se puede
volver a comprobar y uno que no: `VERIFIED` cuya evidencia **no se puede ejecutar**
es un claim que sólo existe como texto, y el día que el texto se queda viejo nadie
lo nota porque no había nada que ejecutar. **Es la misma clase que el conjunto de
niveles declarado en dos sitios, y que el nombre `*_e2e` sin frontera: una
afirmación que parece verificable porque tiene una casilla donde va su evidencia.**

**Por qué las filas AIW no tenían este problema y las R sí:** las filas `AIW-S*`
nombran **ficheros de test** (`aiw_s5_chronos_real.rs`, `x04_multi_process_concurrency.rs`),
así que re-ejecutarlas es escribir el nombre. Las filas `R*` nombran **receipts,
commits y descripciones**, así que re-ejecutarlas exige primero *encontrar* qué
tests son, y ese trabajo **no está hecho**.

**Lo que C3n.3 deja escrito como trabajo, no como nota:**

1. Nombrar, para `R2`, `R5` y `R8`, el **fichero de test** que respalda cada claim.
   Sin eso, esas tres filas no son re-certificables por construcción.
2. `R0` tiene una decisión de fondo que no es de este bloque: su evidencia está en
   `docs/history/legacy-packages/`, y **verificar contra un paquete histórico es
   verificar contra algo que por política de traslados ya no se mueve**. O se acepta
   que `R0` es `VERIFIED` **para `0c2ca56` y nada más**, y se dice en la fila, o se
   reconstruye la evidencia contra HEAD. Hoy la fila dice «VERIFIED (histórico)» en
   una columna y «VERIFIED **para su SHA**» en la otra, lo cual es correcto, y por
   eso **no es un defecto**: es la fila que mejor está escrita de todas y sirve de
   modelo para las otras tres.

### Dos recuentos de la matriz que no reconcilian, corregidos aquí

| Recuento publicado | Medido en `87831cc5` | Dónde está |
|---|---|---|
| «**5** falsificadores C3l.1» | **`6`**: `strategy_failure_is_typed_and_carryable`, `all_strategies_failing_yields_incomplete`, `failure_plus_clean_strategy_still_incomplete`, `failure_plus_contradiction_surfaces_contradiction_not_baseline`, `one_strategy_failure_never_yields_confirmed_baseline`, `strategies_run_counts_completed_not_merely_applicable` | `R6` |
| «`verdict.rs` **11/11** unitarios» | **8** en el fichero; **15** si se filtra por nombre | `R10` |

**Los dos se corrigen con su cifra, no con una nota**, porque un recuento que no
reconcilia es exactamente lo que hace que una matriz deje de describir la realidad.
**Y ninguno de los dos cambia el veredicto de su fila**: `R6` sigue siendo
`IMPLEMENTED → re-verificable` con sus falsificadores en verde, y `R10` sigue sin
poder sustentar «architecture conformant». Lo que se corrige es el número que
quien lea la fila da por bueno.

**Nota de método, y es la quinta vez en esta sesión:** los dos recuentos se
obtuvieron **contando**, no leyendo. Una regex mia devolvió `0` para
`c3l1_falsifiers` porque no toleraba atributos intermedios, y la verdad —**6**— la
dijo `cargo test`. **Cuando dos instrumentos discrepan, el que ejecuta es el
instrumento**, y un pattern que no reproduce lo que el runner ve es un patron
roto, no una fila distinta.

---

## C3n.4 — Release admission integration

Añadir al perfil que reclame estas capacidades:

1. DebVerify false-clean falsifier;
2. Producer→L0 public-path test;
3. S4 replay/idempotency E2E;
4. EXT status semantics;
5. two-process concurrency;
6. second consumer;
7. provider-neutral provenance;
8. architecture debt/waiver check.

Base puede omitir providers externos si el claim de producto no los incluye.

---

# Orden recomendado de ejecución

```text
C3l.0
 ├─ C3l.1 DebVerify
 ├─ C3l.2 Producer L0
 ├─ C3l.3 Dynamic expansion
 └─ C3l.4 EXT semantics
       ↓
 C3l.5 + C3l.6 + C3l.7
       ↓
 C3m.0 ADR KMT
       ├─ C3m.1 KMT
       ├─ C3m.2 KnowledgeBasis identity
       ├─ C3m.3 runtime provenance
       ├─ C3m.4 confidence
       └─ C3m.5 context map
       ↓
 C3n re-certification
```

Paralelamente:

```text
C3j puede continuar
```

pero C6 no debe promover capacidades afectadas mientras C3n no las haya re-certificado.

---

# Stop conditions

Detener una slice y abrir decisión/ADR si:

1. arreglar una aceptación requiere cambiar autoridad de dominio;
2. aparece una segunda fact log;
3. se propone un segundo semantic graph canónico;
4. Dynamic Expansion necesita bypass de Authority;
5. KMT obliga a duplicar Decision Memory;
6. el adapter de proveedor empieza a filtrar tipos SDK al dominio;
7. se necesita romper una API pública únicamente para satisfacer un test;
8. una prueba denominada E2E no puede señalar una frontera observable.

---

# Política de tests

## Durante `apply`

Tests quirúrgicos:

```text
unit
contract
integration de la slice
falsifier
```

## En `verify`

- crate afectado;
- architecture fitness aplicable;
- UAT del hito.

## En integración/release

- `cargo fmt --check`;
- `cargo clippy --workspace --all-targets -- -D warnings`;
- `cargo test --workspace`;
- shell/contract/UAT profile;
- EXT únicamente cuando el perfil lo reclama y el provider está disponible.

Un EXT no disponible:

```text
BLOCKED/NOT_RUN
```

no hace fallar Base y no certifica Enhanced.

---

# Definition of Done

Una slice sólo está `VERIFIED` si:

1. criterio de aceptación falsable;
2. test RED observado cuando corresponde;
3. implementación mínima;
4. GREEN en SHA exacto;
5. falsifier rompe el comportamiento si se neutraliza la garantía;
6. receipt con boundary real;
7. sin claim mayor que la evidencia;
8. CURRENT/STATE/UAT actualizados después de observar el resultado;
9. Conventional Commit atómico;
10. deuda residual con owner + trigger.

`CERTIFIED` sigue reservado a los gates del perfil de producto.

---

# Resultado esperado

Al cerrar C3n:

- DebVerify no puede false-clean.
- Producer→L0 funciona por la ruta pública.
- Dynamic Workflow Expansion está probado como circuito completo.
- Provider ausente nunca aparece como PASS.
- X04 cruza procesos reales.
- X07 usa segundo consumidor/binario real.
- KMT tiene una sola definición canónica.
- Provenance runtime es provider-neutral.
- Snapshot L1 no inventa confianza.
- El context map describe la arquitectura deseada y su migración incremental.
- Los claims AIW / Context-First vuelven a significar exactamente lo que sus UAT demuestran.
