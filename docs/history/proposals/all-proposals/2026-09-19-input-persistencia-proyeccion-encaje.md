# Flujo input → persistencia → proyección: encaje con productores reales

**Estado:** propuesta de aterrizaje. Extiende y no sustituye a
[`2026-09-19-kernel-adaptativo-productores-encaje.md`](2026-09-19-kernel-adaptativo-productores-encaje.md).
**Fecha:** 2026-09-19. **Fuente inspeccionada:** `df60a8a` (incluye `8dd56bf`).
**Alcance:** revisión estructural read-only del pipeline de datos. Sin implementación, push ni release.

## 1. Tesis

La calidad de las salidas de SDDK no se consigue decorando resultados: se consigue
garantizando la cadena completa **captura → normalización → persistencia canónica →
proyección → presentación**. Cada eslabón existe parcialmente en el código; lo que
falta son dos puentes concretos, no un nuevo sistema de conocimiento.

```text
ENTRADA (productor)      NORMALIZACIÓN         PERSISTENCIA canónica        PROYECCIÓN           SALIDA
git diff / runner /      SoftwareObservation   events_v1 (ledger)          project_next/blocked  JSON + texto
proveedor estático   →   ObservationSet     →  evidence_attachments_v1  →  architecture       →  del mismo
pruebas / declaraciones  con ObservationBasis  + CAS + gate_receipts       findings/receipt      resultado
                                               projection_checkpoints_v1   (rebuildables)
```

Regla de oro: **cada eslabón conserva la naturaleza de lo observado.** Una inferencia
no persiste como hecho; una observación sin base (revisión, scope, productor) no
persiste como evidencia; una proyección no persiste como copia canónica.

## 2. Qué persistencia canónica existe realmente

Inspeccionado en `crates/sddk-storage/src/` (migraciones y contratos):

| Superficie | Qué guarda ya | Clase de estado |
|---|---|---|
| `events_v1` | Ledger append-only con cadena de hashes; `rebuild.rs` reconstruye proyecciones desde aquí | Fact |
| `work_items_v1`, `work_item_dependencies_v1`, `decision_records_v1` | Planning: identidad, dependencias, decisiones, columnas spine | Fact/Object |
| `evidence_attachments_v1` + CAS | Evidencia referida por hash `sha256:<hex>`, con `relation` universal obligatoria en escritura (fail-closed) | Fact + contenido CAS |
| `capability_receipts` / `gate_receipts` | Resultados de capacidades y gates con idempotencia y formato RID pinado | Fact |
| `workflow_runs_v1`, `node_runs_v1`, `attempts_v1`, `workflow_run_events_v1` | Ejecución de workflows con eventos de ciclo de vida append-only | Fact/Object |
| `projection_checkpoints_v1` | Checkpoints de read-models: `(projection_name, version) → state_json + last_event_*` | Projection |
| `execution_graph_revisions_v1` (cadena padre + digest), `ir_digests_v1` | Revisiones de plan con linaje | Object/Projection |
| `graph_state_v1` | Snapshot de grafo (el GraphStore persiste bajo checkpoints) | Projection |

Contrato de concurrencia vivo en `planning_substrate_contract.md`: escrituras
ATOM-PER-ROW, CAS-ORACLE con INV-1 (ningún huérfano de CAS en colisión UNIQUE),
TRANSACTIONAL con `with_busy_retry`. El pipeline de inputs debe respetar estas
clases: un productor nuevo escribe por los métodos existentes, no con SQL ad hoc.

## 3. Los dos puentes que faltan (hallazgos STRUCTURAL)

### Puente 1: observación → evidencia adjunta

El sustrato de observaciones del engine (`crates/sddk-engine/src/observation/`)
conserva contradicciones, soporte y refutación como first-class, con
`ObservationBasis` (revisión + basis hash + input digest del productor). Pero la
tabla `evidence_attachments_v1` solo admite hoy cuatro relaciones en escritura vía
`EvidenceAttachmentRecord::from_universal_relation` (`crates/sddk-domain/src/planning/mod.rs`):

```text
observed_for → Log    verifies → Metric    references → Reference    justifies → Approval
supports / gates / produced_by / contradicts → Err(UnknownRelationTag)  # fail-closed
```

El propio código lo dice: persistirlas «requires a successor record shape, not a
fake legacy kind». **Consecuencia:** una contradicción entre dos observaciones, hoy,
no puede adjuntarse a un WorkItem. El motor puede verla; el ledger no puede guardarla.
Cualquier proyección de «salidas de calidad» que dependa de contradicciones
persistentes está bloqueada por este hueco de shape, no por falta de análisis.

**Slice propuesto (futuro, sujeto a admisión):** successor shape de evidencia
observacional — nueva tabla o migración aditiva que referencie `ObservationId`
y su basis, manteniendo `evidence_attachments_v1` intacto para compatibilidad.
Fallará cerrado ante observation inexistente o basis incompleta.

### Puente 2: input de herramienta → observación normalizada

`verify_kernel_cmd.rs::run_verify` construye `ObservationSet::new()` vacío (comentario
en el código: «future: from CogniCode, Chronos, etc.»). Los runners del gateway
producen `RunOutcome` (stdout/stderr truncados, `timed_out`, estado) pero nadie lo
normaliza a `SoftwareObservation` con `ObservationBasis`. CC-S0 aporta el puerto de
proveedor estático con Fake/Null; no hay adaptador a producción.

**Slice propuesto:** un normalizador por familia de input (empezar por UNO: p. ej.
resultado de `cargo test --format json`, o el diff ya usado por
`architecture_cmd::git_changed_paths`), que emita `SoftwareObservation` con basis
completa. El UAT falsa: timeout ≠ fallo de test, salida truncada ≠ PASS, proceso
fallido con «passed» en stdout ≠ verde.

## 4. Cadena de calidad por tipo de input

| Input | Productor ejecutable hoy | Normalización necesaria | Persistencia canónica | Proyección de salida |
|---|---|---|---|---|
| Cambios del repo | `git_changed_paths` / `changed_basis` (`architecture_cmd.rs`) | Ya emiten ChangeBasis | Como evidencia `observed_for` adjunta al WorkItem del ciclo | `architecture findings` ya cruza con contratos |
| Contratos declarados | `load_declaration` | Ya tipados en la declaración | En el paquete de declaración; findings derivados | `findings` / `receipt` / `why` (existe) |
| Resultado de tests | `gateway::test_runner::dispatch` → `RunOutcome` | **Pendiente**: parser por testcase con basis | CAS body + adjunto `verifies` al WorkItem | Claim verificación / agenda de verificación faltante |
| Observación estática (CogniCode) | Puerto `CodeIntelligencePort` (solo Fake/Null hoy) | `Observation` textual → typed variant (trabajo CC-S1+) | **Bloqueada por Puente 1** para contradicciones; `observed_for` disponible para lo básico | Verify con evidencia estructural real (AC10) |
| Decisiones | `decision_records_v1` con rationale obligatorio | Ya tipadas | Tabla canónica con FK | `plan decision` / provenance chain (`build_provenance_chain_v2`) |
| Ejecución de workflow | `workflow_run_events_v1` append-only | Ya emitida por runtime | Eventos + attempts | Reconstrucción de estado / cold_start (existe) |

Regla transversal: **si el productor no puede llenar la `ObservationBasis`
(revisión, basis, digest de entrada), el input no persiste como evidencia; persiste
como artefacto bruto en CAS sin autoridad**. Eso evita que logs huérfanos se cuelen
como hechos.

## 5. Salidas: proyecciones, no informes

El modelo de datos ya distingue Fact (ledger) de Projection
(`projection_checkpoints_v1` + `rebuild.rs` canónico). Las salidas de calidad son:

1. **Proyecciones deterministas reconstruibles**: misma base semántica → misma salida
   (campos semánticos idénticos; timestamps de presentación pueden variar). Los
   tests de determinismo de `projections` ya pinan esto para planning.
2. **Presentación dual desde el mismo resultado**: JSON estructurado para agente,
   texto/plantilla para humano (`render_json` + ramas textuales ya existen en el
   CLI). Nunca dos fuentes de verdad.
3. **Procedencia visible**: toda salida agrega refs al evidence/gate/decision que la
   sostiene; `build_provenance_chain_v2` ya recorre la cadena.

Nueva proyección = nuevo `projection_name` + versión + `rebuild` desde `events_v1`.
No una tabla espejo escrita a mano por el productor: eso duplicaría autoridad y
violaría el modelo Fact/Projection.

## 6. Qué NO hacer (descartes con causa)

- **No ampliar `EvidenceKind`** (`crates/sddk-domain/src/evidence.rs`) para colar
  observaciones: es vocabulario de captura UAT/gobernada, y la autoridad de escritura
  es `relation`, no `kind`.
- **No persistir `ObservationSet` completo como blob JSON** en una tabla nueva «por
  comodidad»: perdería identidad por observación e invalidación por basis. El
  successor shape del Puente 1 debe ser fila-por-observación referenciando
  `ObservationId`.
- **No escribir proyecciones desde los productores.** El productor emite Fact/observación;
  la proyección se reconstruye. `rebuild.rs` es fail-closed ante error de apply:
  mantener ese contrato.
- **No duplicar el KnowledgeBasis del engine en SQLite con otro hash.** La base
  semántica ya tiene BasisHash SHA-256 canónico (`knowledge.rs`); un segundo esquema
  de hashing crearía dos identidades para el mismo hecho. (Contrastar con el
  `DigestSha256` FNV del spike CC-S0: identidad provisional, no durable.)

## 7. Orden de adención sugerido (sin abrir roadmap nuevo)

1. **Puente 2 (normalizador de UN input)** — desbloquea evidencia real en Verify con
   shapes existentes (`observed_for`/`verifies`). Ciclo pequeño, UAT falsable.
2. **Puente 1 (successor shape de evidencia observacional)** — migración aditiva +
   ADR de shape; prerequisite para contradicciones persistentes y para el valor
   completo de A6/CogniCode en el ledger.
3. **Proyección de atención mínima** — `project_next`/`project_blocked` + evidencia
   adjunta en una vista reconciliada con el roadmap vivo. Solo después de 1–2; si no,
   proyectaría huecos disfrazados de agenda.

Cada paso mantiene el encaje declarado en el documento hermano: A6 sigue siendo la
línea de implementación; este pipeline es el mismo trabajo visto desde el dato, no
un evolutivo paralelo.

## 8. Evidencia y límites

**STRUCTURAL:** tablas y migraciones citadas; `insert_evidence_attachment` (orden
INSERT→CAS, INV-1); `from_universal_relation` fail-closed; `rebuild.rs` canónico;
`run_verify` con `ObservationSet` vacío; `RunOutcome` con truncado/timeout.
**OBSERVED:** consultas previas `sddk plan roadmap next/blocked` (reporte hermano).
**DERIVED:** puentes, orden y UAT propuestos — pendientes de admisión como ciclos.
**DOCUMENTED:** clases de estado y contratos de concurrencia de
`planning_substrate_contract.md`.

No se ejecutó la batería Rust; no se validó ningún UAT propuesto; los cambios
pendientes del flake (`a6_4_shared_ticket_service.rs`) siguen intocados y fuera de
este alcance.
