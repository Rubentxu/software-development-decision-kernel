# Orígenes de los inputs: qué proceso interno los produce realmente

**Estado:** auditoría de adopción de inputs. Cierra la pregunta del documento hermano
[`2026-09-19-input-persistencia-proyeccion-encaje.md`](2026-09-19-input-persistencia-proyeccion-encaje.md):
no basta con que exista el productor en código; hay que saber si **algo lo invoca**.
**Fecha:** 2026-09-19. **Fuente:** `6251af7` + bundle instalado 1.169.88
(`~/.local/share/sddk/framework/1.169.88`).
**Alcance:** read-only. Distinción clave usada en todo el documento:
**código vivo** (invocado por CLI/prompt), **código muerto-productivo** (compilado,
expuesto, sin invocador) y **código de test** (excluido de este análisis).

## 1. Respuestas directas a las preguntas

**¿Git como productor, hay proceso interno que lo extraiga y reporte?**
Sí, pero es un productor **invocado bajo demanda por el CLI**, no un proceso
residente. `sddk architecture --changed` ejecuta git internamente
(`architecture_cmd.rs::git` → `git_changed_paths`: diff rango+worktree, `-z
--no-renames` para no perder paths con caracteres no-ASCII, fail-closed si no hay
base). No hay watcher, daemon ni polling. Cada consulta recaptura; nada persiste
esa captura como input canónico.

**¿Hay otros procesos internos?**
Cuatro, todos same-process, ninguno residente:

1. **test_runner del gateway** — `dispatch()` genera y ejecuta specs para 6 familias
   (cargo-nextest, pytest, jest, go, maven, gradle) con env allowlist y truncado de
   salida. **Código muerto-productivo:** trait `pub(crate)`, sin ruta del CLI que lo
   invoque fuera de tests. `sddk dev test count-workspace` es lo único vivo, y solo
   *cuenta* tests parseando la salida, no produce input estructurado.
2. **knowledge scan/import** (`knowledge_ingest.rs`) — el CLI **sí** tiene comandos
   reales (`sddk knowledge scan|import|verify`) que clasifican conocimiento del repo
   en un plan con hashes SHA-256 y lo importan a un registry versionado. **Pero los
   prompts de fase no los invocan nunca** (0 menciones en `prompts/`). Conocimiento
   posible que nadie pide.
3. **Evidencia al ledger** — `sddk plan evidence attach` existe y es el único camino
   para persistir un input canónico adjunto a un WorkItem (relation obligatoria,
   CAS). Su **único** invocador prompteado es `phases/apply.md` línea 471
   (`sddk.cli --evidence {evidence_json}` vía evaluate-gate). Verify no adjunta
   evidencia al ledger; produce markdown en `verify-report.md`.
4. **uat-evidence** — captura tipada (screenshot/console/network/trace/DOM) en
   browser, la vía más madura de input estructurado. Solo se activa en flujos UAT.

**¿Se extraen inputs de herramientas externas vía agentes LLM?**
Parcialmente, y es la parte débil. Los prompts de fase ordenan a los LLM ejecutar
herramientas y **narrar** resultados: apply.md pide gates cargo (28 menciones a
comandos, 10 a git en verify, 0 en explore); verify.md define 6 niveles de lens
(L1 determinista, L3 reachability, L4 adjudicación…). El invariante L1 es correcto
en diseño («un LLM no puede reinterpretar un exit≠0 como éxito») pero **el registro
de ese exit code lo escribe el LLM en su informe markdown**. Los únicos inputs
LLM-derivados que llegan a persistencia canónica son: gate receipts del apply
(schema pinado), decision records y cycle transitions. El análisis de un lens
(la conclusión del LLM) vive y muere en markdown de ciclo.

**¿Se analizan los reportes de los agentes LLM?**
No por código. No existe un parser ni normalizador de informes de fase hacia
`SoftwareObservation`/Knowledge. El `sddk-verify` coordinador deduplica hallazgos
de lenses **leyéndolos como texto** (es un LLM), no consumiendo un envelope
estructurado validado. La síntesis es narrativa, no tipada.

## 2. Inventario por bounded context

### Planning — VIVO (el único con loop completo)
- **Inputs:** cycle lifecycle events, gate receipts, work items, decisiones.
- **Productores vivos:** `sddk cycle *` (40 invocaciones prompteadas), `sddk ledger *`
  (15), `sddk plan` (facade de cycle), `plan evidence attach` (solo desde apply).
- **Salida:** proyecciones JSON desde SQLite (verificado con consultas reales:
  `next` → GA-002, `blocked` → 49 items).
- **Persistencia:** canónica (ledger + tablas ATOM-PER-ROW/CAS-ORACLE).

### Arquitectura — VIVO pero efímero
- **Inputs:** declaración de contratos (YAML) + diff git on-demand.
- **Productores vivos:** `sddk architecture findings/receipt/why` — ejecutan
  DebVerify audit, generan findings con ID determinista, `--out` escribe receipt.
- **Hueco:** el cruzado diff×contratos (ChangeBasis) se recalcula en cada invocación
  y **no se persiste como evidencia** ni alimenta al Planning. Dos contextos
  (architecture findings y planning evidence) que no se tocan.

### Knowledge — Código vivo con pipeline huérfano
- **scan/import/verify** completos y con concurrencia cuidada; **cero adopción** en
  prompts, agents o skills. Su único uso prompteado es `sddk knowledge path/status`
  para *localizar* el vault (grep/ls por parte del LLM), no para producir inputs
  tipados.
- El vault se consulta con grep por diseño; nada de lo que el LLM lee ahí vuelve
  como input estructurado.

### Verification — el más fragmentado
| Pieza | Estado |
|---|---|
| `sddk verify` (ledger) | Vivo, prompts lo usan como proof-check |
| `sddk verify-kernel --domain architecture` | Vivo, consumido por architecture receipt |
| `verify_kernel` genérico +其他 dominios | **Muerto-productivo**: sin invocador CLI |
| `run_verify` con `ObservationSet::new()` vacío | Vivo pero **sin inputs**: verifica claims contra nada |
| test_runner (6 familias) | **Muerto-productivo** en gateway, `pub(crate)`, sin ruta CLI |
| Evidencia de tests como input | Narrada por el LLM en markdown; no persistida tipada |

### Secretary / ContextCompiler / AgentHost — VIVOS como librería, muertos como pipeline
- L0/L1/L2 con tests propios, **sin wiring** a Planning o Knowledge desde CLI/prompts.
- `cold_start`/RecoveryCapsuleInputs: solo tests y librería.
- `AgentContributionEnvelope` + síntesis: ídem. `ExecutionRequest`/`ExecutionOutcome`
  de arch-spec-007 **no existen como tipos Rust**; el receipt acepta su ausencia
  (`MissingExecutionOutcome` es warning, no error).

### Agéntico — el input real de hoy
Lo que realmente fluye por el sistema en un ciclo A-*: **texto narrado por LLMs**
(apply-report.md, verify-report.md, explore-report.md) + gate receipts con schema +
transition records. Los reportes markdown no se reingieren: el siguiente agente los
lee como contexto. Es un pipeline de **contexto por convención**, no de datos.

## 3. Síntesis: el mapa real de producción de inputs

```text
VIVO con persistencia canónica:
  cycle/gate/transition events ──→ ledger ──→ proyecciones planning (JSON)
  plan evidence attach (solo apply) ──→ CAS + evidence_attachments_v1
  uat-evidence (solo flujo UAT) ──→ capture tipado

VIVO pero efímero (calcula y no persiste):
  sddk architecture --changed ──→ ChangeBasis ──→ findings markdown/receipt --out
  sddk knowledge verify ──→ drift report

CÓDIGO MUERTO-PRODUCTIVO (compilado, expuesto, sin invocador):
  test_runner 6 familias (RunOutcome completo)
  verify_kernel dominios no-architecture
  Secretary L0/L1/L2, ContextCapsule wiring, AgentHost, Envelope
  Observation→Knowledge pipeline (CC-S0 es solo el puerto)

LO QUE EL LLM PRODUCE Y EL SISTEMA NO ANALIZA:
  verify-report.md / apply-report.md: exit codes, hallazgos, coverage,
  reachability → narrativa sin parser ni normalizador.
```

## 4. Qué implica para «inputs de calidad»

1. **El gap dominante no es de productores sino de adopción.** El gateway puede
   ejecutar tests de 6 ecosistemas y nadie lo llama; knowledge puede ingerir y nadie
   lo pide; verify-kernel existe y recibe observaciones vacías. Conectar > construir.
2. **El informe LLM es hoy un producto de residuos.** La herramienta más usada del
   sistema (agentes en fases) produce los inputs menos estructurados. Invertirlo:
   que cada gate que el LLM ya ejecuta deje un receipt/tipado, y que el informe sea
   una *presentación* de esos receipts, no la fuente.
3. **No hay que añadir ningún proceso nuevo.** El orden del documento hermano se
   confirma y afina:
   - Puente 2 (normalizador de input) tiene un caso aún más barato de lo descrito:
     **exponer test_runner al CLI** y parsear su `RunOutcome` ya tipado. Cero parsers
     nuevos para la familia cargo.
   - Puente 1 (successor shape de evidencia) sigue siendo prerrequisito para
     contradicciones y para que architecture findings y planning evidence converjan.
   - Antes de ampliar ObservationSet del verify-kernel, decidir su consumidor real:
     hoy verifica contra nada.
4. **Los prompts son parte del sistema y también auditarlos.** Una feature «existe»
   para este proyecto cuando código + comando CLI + prompt/agent la invocan. Las
   cuatro verticales muertas-productivo deberían listarse en el A5-ROADMAP como
   deuda de adopción, no como capacidades disponibles.

## 6. Cobertura extendida: skills, agentes y comandos top-level (segunda pasada)

Tras la primera auditoría se revisó también el catálogo completo: **30 comandos
top-level** del CLI vs los 19 invocados desde el bundle. Resultado por comando
(menciones en prompts/skills/agents del bundle 1.169.88):

**Vivos con adopción real (≥3 menciones):** cycle (60), ledger (19), knowledge (11,
solo path/status), release (5), dev (5), adopt (4), plan (3), artifact (3),
uat (22), vault (2), verify (1).

**Vivos pero con adopción mínima o tangencial (1-2):** capability, lint, generate,
status, ship, run, recover, init, archive. Los tres primeros tienen usos aislados y
legítimos pero puntuales: `sddk lint`/`generate docs --in-repo` solo en
document-catalog con permiso explícito; `sddk capability` en core-contract-review.

**Muertos de adopción (0 menciones en todo el bundle):** `backlog`, `permission`,
`validate`, `agent-result`, `git`, `graph`, `metrics`, `analytics`, `telemetry`,
`approval`, `pack`, `why`.

Matices honestos sobre los ceros:
- `git`, `validate`, `agent-result`, `capability`: son superficies para **consumo
  externo/agentes no-prompteados**, no flujo interno. Su falta de mención en prompts
  no los invalida; su contrato es ser usados por otro actor. `agent-result` además
  es fachada legacy de conversión.
- `graph`, `metrics`, `analytics`, `telemetry`, `backlog`, `why`, `approval`, `pack`:
  **sí son flujo interno esperado** y tienen backend real (backlog_store,
  analytics.rs, telemetry.rs, projection rebuild). Cero adopción = capacidad
  construida y no integrada al workflow prompteado. `why` es especialmente notable:
  es la superficie de explicación que el doc hermano propone como consumidor de
  salidas, y ningún prompt lo llama.

**Sobre los 70 agentes del bundle:** 52 no son `sddk-*` (auto-grill-*, studio-*,
uat-*, jd-*). Los 18 sddk-* cubren el ciclo canónico. Las skills sddk (13) invocan
un repertorio estrecho: cycle transition/status/inventory/start/lock/artifacts-dir,
knowledge status, ledger verify/events, release plan/apply, adopt status, vault
validate. **Ninguna skill sddk invoca plan roadmap, architecture, graph, metrics,
analytics, backlog, why, evidence attach ni knowledge scan/import.**

**Corrección al mapa de la sección 3:** el conteo «plan: 3» corrige la impresión del
desglose anterior; `plan evidence attach` sigue siendo el único uso de evidence, y
`plan roadmap` (proyecciones next/blocked) **tampoco aparece en ninguna skill** —
las consultas que proyectan el estado del spine no están integradas al flujo
agéntico, se consumen manualmente.

**Regla operativa resultante:** además de código+CLI+prompt, la auditoría debe
incluir **skills**, porque son la capa que convierte comandos en hábitos agénticos.
Con esa regla, el set de capacidades realmente adoptadas es aún menor que el
estimado en la primera pasada: los comandos de análisis (graph/metrics/analytics/
backlog/why) son infraestructura viva esperando un consumidor, igual que test_runner
y knowledge ingest.

## 5. Evidencia y límites

**OBSERVED:** segunda pasada: 30 comandos top-level vs 19 adoptados; conteos por comando con grep sobre prompts+skills+agents (backlog 0, permission 0, validate 0, agent-result 0, git 0, graph 0, metrics 0, analytics 0, telemetry 0, approval 0, pack 0, why 0; uat 22). Primera pasada: inventario de menciones en prompts del bundle 1.169.88 (`grep -c`
por fichero: cycle 40, ledger 15, dev 5, knowledge 1 —solo path—; plan evidence:
solo apply.md:471; git: verify 10, apply 28, explore 0; knowledge scan/import:
0 en prompts). Consultas previas del doc hermano. Bundle resuelto por `sddk version`.
**STRUCTURAL:** `pub(crate) test_runner` sin ruta CLI (grep de `test_runner` en
sddk-cli solo da usos del propio gateway y artifact/release, no test execution);
`run_verify` con `ObservationSet::new()`; `MissingExecutionOutcome` como warning;
ausencia de tipos ExecutionRequest/ExecutionOutcome en crates/.
**DERIVED:** clasificación vivo/efímero/muerto-productivo y las implicaciones de la
sección 4 — pendientes de decisión del operador.
**DOCUMENTED:** arch-spec-007, contratos de fase del bundle.

Excluido de la auditoría: contenido de tests (por petición explícita). Los reportes
markdown de ciclos pasados no se parsearon; solo se contaron artefactos. No se
ejecutó batería Rust ni se modificó nada del árbol (flake pendiente intacto).
