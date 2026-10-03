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

### El nombre del nivel decidía si una fila exigíaReceipt

Medido: **`UAT-MATRIX.md` y el overlay discrepan en el nivel de 4 filas**, y el
overlay es la fuente declarada por la propia cabecera de la matriz:

| ID | `UAT-MATRIX.md` | overlay (fuente declarada) |
|---|---|---|
| `AT-UAT-011` | `PROCESS/SQLITE_DURABLE` | `SQLITE_MULTI_PROCESS` |
| `AT-UAT-012` | `PROCESS/SQLITE_DURABLE` | `SQLITE_MULTI_PROCESS` |
| `AT-UAT-013` | `PROCESS/SQLITE_DURABLE` | `PROCESS` |
| `AT-UAT-014` | `PROCESS/SQLITE_DURABLE` | `PROCESS` |

`PROCESS/SQLITE_DURABLE` **no aparece en ningún otro documento** y lo usan
exactamente esas 4 filas. Con el nombre de la matriz las cuatro quedan exentas;
con el del overlay, **013 y 014 exigen** receipt. **Un nombre decides si una fila
exige o no**, que es la misma clase de error que el rename de los pins
`SDDK_LEGACY_CERT_*` que rompió la certificación de cinco releases: **un nombre es
un contrato con quien lo lee**, y por eso las 4 filas se reconcilian al overlay,
que es la fuente declarada. Reconciliado en `537e73e3`..(session-69s bis 5) y
verificado con el guard: las cuatro filas nombran hoy lo que nombra el overlay.


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
