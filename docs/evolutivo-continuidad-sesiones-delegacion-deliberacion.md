# Evolutivo — Continuidad de sesiones, delegación enriquecida y Decision Memory

> **Estado:** propuesta integrada en la línea canónica SDDK
> **Baseline investigada:** v1.70.0 / `main` a 2026-09-03
> **Roadmap canónico:** `docs/sddk-decision-kernel-architecture/02-roadmap/ROADMAP.md`
> **Execution spine:** `docs/sddk-decision-kernel-architecture/02-roadmap/EXECUTION-SPINE.yaml`
> **Entrada LLM:** `docs/sddk-decision-kernel-architecture/02-roadmap/LLM-START-HERE.md`

## 1. Motivación

SDDK ya sabe reconstruir bastante bien **dónde está el runtime** después de una compactación, reinicio o nueva sesión. Lo que todavía no representa de forma uniforme, durable y machine-readable es la historia deliberativa que permite responder preguntas como:

- ¿cómo quedó exactamente la cosa ayer o hace diez sesiones?;
- ¿qué cambió desde una sesión histórica concreta?;
- ¿qué alternativas se estudiaron y cuáles se rechazaron?;
- ¿por qué se eligió A frente a B?;
- ¿qué pros, contras, riesgos, costes, incertidumbres y supuestos tenía cada opción?;
- ¿qué condiciones harían razonable reabrir una alternativa descartada?;
- ¿qué agentes participaron y qué aportó realmente cada uno?;
- ¿qué información se comprimió al pasar por un coordinator/orchestrator?;
- ¿qué disenso se resolvió y qué disenso sigue abierto?;
- ¿qué propone Secretary y sobre qué evidencia/provenance?;
- ¿podemos recorrer el razonamiento como una rama Git, volver a un fork-point y explorar otro camino sin reescribir la historia?;

El objetivo de este evolutivo es que SDDK pueda continuar trabajo a lo largo de días, sesiones, agentes y modelos sin convertir el chat ni una memoria vectorial en fuente de verdad.

## 2. Auditoría del estado actual

### 2.1 `sddk-cycle-resume`: buena reconstrucción autoritativa, no deliberativa

`skills/sddk-cycle-resume/SKILL.md` ya establece un principio correcto: el orchestrator reconstruye `cli_context` desde CLI, ledger, cycle status, artifacts dir y vault validation; nunca desde memoria conversacional.

Fortalezas actuales:

- reconstrucción pull-based;
- fail-closed;
- lease/fencing;
- ledger causal reciente;
- artifacts persistidos como autoridad;
- separación explícita frente a Engram.

Gap: responde a **dónde está el runtime**, pero no modela completamente **cómo llegamos, qué aprendimos, qué rechazamos y qué caminos razonables quedan**.

El skill también menciona `sddk-continue-options`, pero esa pieza no aparece en el árbol actual de `main`; la intención debe absorberse aquí en vez de mantener una referencia documental huérfana.

### 2.2 Engram y session summaries: útiles como memoria episódica, no como verdad estructural

`AGENTS.md` exige session summary en sesiones largas y `prompts/sddk/decision-model.md` distingue correctamente:

1. workflow state — procedimental/current run;
2. vault — conocimiento durable/canónico cuando está fresco;
3. Engram — episodios, learnings, jurisprudencia; nunca autoridad por sí sola.

La jurisprudencia ya permite recuperar patrones que funcionaron en ciclos anteriores.

Gap: una summary narrativa no permite de forma determinista:

- calcular un delta entre sesiones;
- demostrar completitud;
- preservar ramas alternativas;
- hacer merge-base/fork-point;
- conservar disenso;
- saber qué información desapareció durante una compresión;
- recorrer relaciones causales.

### 2.3 Handoff actual: persistencia buena, envelope general demasiado pobre

`skills/_shared/sddk-phase-common.md` ya obliga a:

- persistir el artifact de cada fase;
- recuperar filesystem/vault antes que memoria;
- devolver un envelope al orchestrator;
- no finalizar el subagente con una tool call, porque el padre podría perder el análisis textual.

Este último punto es especialmente importante: SDDK ya ha identificado un fallo real de **information loss entre agentes**.

Sin embargo el envelope común todavía no representa de forma homogénea:

- findings tipados;
- coverage;
- decisiones propuestas;
- alternativas rechazadas;
- pros/contras;
- evidence por claim;
- supuestos;
- incertidumbre/confianza;
- preguntas abiertas;
- context deltas;
- dissent;
- qué contenido fue sintetizado/omitido.

### 2.4 Verify muestra el patrón correcto

`prompts/sddk/phases/verify.md` ya usa un modelo más rico: findings tipados ligados al subject, ubicación, clasificación, severidad, confianza, reachability y evidencia. El coordinator espera workers, deduplica, sintetiza y no puede rebajar fallos deterministas.

Conclusión: no debemos inventar otra filosofía. Debemos **generalizar el patrón typed contribution → coordinator synthesis → durable evidence** a las delegaciones importantes.

### 2.5 Roles: claros en prompts, no suficientemente enforceables

El orchestrator principal tiene una frontera razonable: gestiona, delega, valida y sintetiza; no ejecuta phase work. Leaf agents no delegan; verify/debt-verify son coordinadores declarados.

Pero la mayor parte de las responsabilidades todavía vive como prosa en wrappers/prompts. El kernel no puede preguntar/validar de forma uniforme:

- qué rol tiene cada agent;
- a quién puede delegar;
- qué estado puede mutar;
- qué tool scopes tiene;
- qué input/output schema debe cumplir;
- quién posee el join/synthesis;
- qué budget tiene;
- qué autoridad posee;
- qué acciones están prohibidas.

Los orchestrators especializados (`auto-grill-loop-orchestrator`, `studio-orchestrator`, etc.) son laboratorios útiles, pero hoy codifican patrones propios que deberían ascender a contracts comunes cuando demuestren valor.

### 2.6 Auto-Grill contiene una semilla muy valiosa

Auto-Grill ya mantiene explícitamente:

- goal model;
- evidence index;
- coverage map;
- question backlog;
- answered questions;
- decision log;
- assumption log;
- risk log;
- validation log;
- context patch log;
- ADR candidate log;
- working summary;
- ledger;
- checkpoint;
- rejection reason/remedy/alternative/proxy learning.

Este modelo es mucho más rico que una session summary y debería inspirar el Decision Memory general.

Limitación: su resume está acoplado a ficheros/fecha. Para SDDK, identidad y continuidad deben depender de refs/revisions/provenance, no del día del calendario.

### 2.7 Dynamic Workflow: vocabulario de grafos sí; memoria deliberativa durable todavía no

`prompts/sddk/dynamic-workflow.md` ya contempla routing, parallelism, voting, orchestrator-worker, evaluator-optimizer, group-chat, adaptive planning, HITL, hierarchical teams, saga y blackboard.

Eso cubre **cómo ejecutar** ciertos grafos. Falta representar de manera durable **por qué se escogió una rama, qué otras ramas existían y qué resultado real tuvo cada decisión**.

### 2.8 Secretary: debe entrar por el mismo canal enriquecido

`SPEC-042-secretary-runtime` y ADR-0072/ADR-0073 fijan proposal-only/closed-set/budgets/authority. Stage 1+ no es todavía un camino completo en `main`.

Por tanto este es el momento adecuado para fijar que Secretary:

- consume Decision Plane + ResumeView;
- devuelve Contribution/ContinuationCandidate tipado;
- adjunta evidence/provenance;
- nunca crea un side-channel de memoria o autoridad;
- queda incluido en el mismo DelegationGraph que cualquier otro agent.

## 3. Investigación externa y lecciones transferibles

### 3.1 OpenAI Agents SDK

La documentación actual aporta varios patrones relevantes:

- `Sessions` persiste historia entre runs y permite reanudar runs interrumpidos;
- tracing registra generaciones, tools, handoffs, guardrails y eventos custom;
- `RunResult.new_items` conserva items ricos con metadata de agent/tool/handoff/approval, no sólo el texto final;
- handoffs admiten input tipado y filtros de contexto;
- manager-style orchestration mantiene un único synthesis owner; un handoff real transfiere el control;
- structured outputs ayudan a validar interfaces entre agents.

Lección para SDDK: **no reducir una ejecución rica al final_output**. Conservar items/contributions y dejar que las summaries sean proyecciones.

### 3.2 Anthropic: long-running agents y multi-agent research

Anthropic documenta dos ideas muy alineadas con SDDK:

- sesiones nuevas deben poder continuar mediante artifacts estructurados + progreso durable + historia de Git;
- en sistemas multi-agent, guardar outputs de subagentes en filesystem y devolver referencias ligeras al lead reduce el “game of telephone”.

También remarcan que una delegación necesita objetivo, output format, herramientas/fuentes y límites claros para evitar duplicación y gaps.

Lección para SDDK: el orchestrator no necesita copiar todo el raw output a su prompt, pero sí debe recibir **un índice semántico rico y referencias lossless al original**.

### 3.3 A2A Protocol

A2A separa `contextId` y `taskId`, mantiene lifecycle y permite `artifacts` + history. Es útil como inspiración de interoperabilidad futura.

Lección: identidad de contexto, identidad de tarea y artifact outputs no deben mezclarse.

### 3.4 LangGraph persistence/time travel

LangGraph guarda checkpoints de graph state, permite history, replay y fork desde checkpoints sin mutar el histórico original.

Lección: SDDK debe poder crear **counterfactual branches** desde un DecisionCheckpoint y mantener la rama oficial intacta.

### 3.5 Git como modelo conceptual principal

Git aporta la semántica que mejor encaja con la memoria que buscamos:

- objetos content-addressed;
- commits con uno o varios parents;
- refs/branches como punteros baratos;
- `HEAD` como ref activa;
- tags;
- reflog;
- ancestry/ranges;
- `merge-base`/fork-point;
- notes como metadata adjunta sin reescribir el objeto original.

No se propone almacenar la Decision Memory dentro de `.git`. Se propone adoptar sus **propiedades de modelo** sobre el storage local-first de SDDK.

## 4. Decisión arquitectónica: Continuity, Delegation & Deliberation Plane (CDD)

Introducir un capability plane transversal provisionalmente llamado:

**CDD — Continuity, Delegation & Deliberation Plane**

CDD **no es una nueva fuente de verdad**. Sus proyecciones derivan de:

```text
Event Ledger
 + Planning Ledger
 + WorkflowRun / Decision Plane
 + cycle/run artifacts
 + accepted decisions / vault
 + agent contributions
 + optional Engram episodes
        ↓
CDD projections
```

Invariante:

> Si una proyección CDD se borra, debe poder reconstruirse a partir de estado/eventos/artifacts canónicos sin inventar hechos.

## 5. Git-like Decision Memory

### 5.1 Objetivo

La memoria no será una lista cronológica de resúmenes. Será un **DAG versionado y content-addressed** cuya vista más común puede renderizarse como un árbol estilo `git log --graph`.

Ejemplo conceptual:

```text
                         experiment/cache-strategy
                        o M7  [evidence]
                       / \
main-memory   M1---M2---M3---M6---M9  ← HEAD
                    \       /     \
                     M4---M5       M8
                  option/sqlite   secretary/recovery
```

Cada nodo explica no sólo “qué pasó”, sino su delta deliberativo respecto a sus parent(s).

### 5.2 Objetos

#### `DecisionMemoryCommit`

```yaml
id: sha256(canonical_payload)
parents: [memory_commit_id]
tree: memory_tree_id
author:
  actor_type: human | agent | system
  actor_id: ...
timestamp: ...
project_id: ...
work_item_id: ...
cycle_run_id: ...
subject_revision: ...
planning_revision: ...
workflow_revision: ...
event_cursor: ...
message: ...
reason: ...
provenance_refs: []
```

Un merge/synthesis puede tener varios parents. Nada se reescribe en sitio.

#### `DecisionMemoryTree`

Snapshot semántico inmutable de pointers, por ejemplo:

```text
goal/
decisions/
options/
assumptions/
risks/
questions/
frontier/
delegations/
contributions/
dissent/
artifacts/
knowledge/
```

No implica duplicar los artifacts; normalmente contiene refs + hashes + pequeños objetos tipados.

#### `DecisionMemoryBlob`

Payload content-addressed para:

- Decision;
- Option;
- EvidenceRef;
- Assumption;
- Risk;
- Question;
- ContributionIndex;
- SynthesisReceipt;
- ContextDelta;
- RevisitTrigger;
- NegativeKnowledge.

### 5.3 Refs

```text
refs/heads/canonical
refs/heads/session/<session-id>
refs/heads/decision/<decision-id>/<option>
refs/heads/what-if/<experiment-id>
refs/tags/cycle/<cycle-id>
refs/tags/release/<version>
refs/tags/milestone/<work-item-id>
HEAD -> refs/heads/canonical
```

`refs/heads/canonical` es la proyección deliberativa aceptada. Las ramas `what-if` son advisory y no poseen autoridad de runtime.

### 5.4 Reflog

Cada movimiento de ref se registra append-only:

```yaml
ref: refs/heads/canonical
old: M6
new: M9
actor: orchestrator
reason: cycle-72 closed
receipt_ref: ...
```

Esto permite preguntar dónde estaba `HEAD` ayer aunque la rama haya avanzado.

### 5.5 Merge

Una convergencia de ramas requiere `DecisionMemoryMergeReceipt`:

```yaml
parents: [M7, M8]
merge_base: M3
conflicts: []
selected_claims: []
rejected_claims: []
dissent_preserved: []
evidence_refs: []
authority: ...
```

No se hace “merge textual automático” de decisiones. Un conflict resolver/policy/human decide según el tipo de conflicto.

### 5.6 Notes/annotations

Metadata posterior —por ejemplo nueva evidencia que invalida una vieja assumption— puede adjuntarse como annotation/edge sin modificar el hash del commit histórico.

## 6. Decision Memory Projection: el árbol que recorre el LLM

El store es DAG; el LLM no debe cargar todo el DAG. El Context Compiler produce una **DecisionMemoryProjection** según pregunta/objetivo.

### `resume` projection

```text
HEAD M9
|
|-- Current goal
|-- Current Work Item
|-- Runtime frontier
|-- Decisions still binding
|-- Assumptions still active
|-- Open risks
|-- Open questions
|-- Negative knowledge
|-- Pending delegations
`-- Continuation candidates
```

### `history` projection

```text
M9 cycle close
|
M8 secretary recovery proposal [merged]
|
M6 design decision
|\
| M7 rejected cache strategy
|
M3 exploration fork point
```

### `decision` projection

```text
Question: persistence strategy?
├─ A SQLite
│  ├─ pros ...
│  ├─ cons ...
│  ├─ evidence ...
│  └─ selected → Decision D12
├─ B RocksDB
│  ├─ pros ...
│  ├─ cons ...
│  └─ rejected: operational complexity
└─ C event-only
   └─ revisit_when: projection cost > threshold
```

### `delegation` projection

```text
orchestrator
├─ explore-agent → Contribution C1
├─ architecture-agent → Contribution C2
├─ security-agent → Contribution C3
└─ coordinator merge → Synthesis S1
   ├─ selected claims C1/C2
   ├─ preserved dissent C3.F4
   └─ Decision D12
```

## 7. Operaciones estilo Git propuestas

No tienen que copiar la CLI de Git literalmente, pero la semántica debe resultar familiar:

```text
sddk memory status
sddk memory log --graph
sddk memory show <ref|commit>
sddk memory diff <A>..<B>
sddk memory branch
sddk memory branch <name> <ref>
sddk memory merge-base <A> <B>
sddk memory ancestors <ref>
sddk memory why <decision|state>
sddk memory reflog
sddk memory tag <name> <ref>
sddk memory fork <checkpoint> --as what-if/foo
sddk memory compare canonical what-if/foo
```

Para continuidad entre sesiones:

```text
sddk resume explain
sddk resume explain --at <ref|timestamp>
sddk session diff <yesterday-ref> HEAD
```

## 8. Contratos de delegación

### 8.1 `AgentRoleContract`

```yaml
role_id: sddk-explore
role_kind: leaf | coordinator | orchestrator | evaluator | advisor
responsibilities: []
capabilities: []
may_dispatch: false
allowed_workers: []
input_schema: ...
output_schema: ...
read_scopes: []
write_scopes: []
mutation_authority: []
tool_scopes: []
budgets: ...
synthesis_owner: ...
forbidden_actions: []
required_artifacts: []
```

Debe existir validación estática y runtime de topología de roles.

### 8.2 `ContextLease`

Cada delegación ve una revision explícita:

```yaml
context_id: ...
context_revision: ...
subject_revision: ...
planning_revision: ...
workflow_revision: ...
decision_memory_head: ...
objective: ...
authoritative_refs: []
advisory_refs: []
staleness_policy: ...
```

El contribution declara contra qué revision trabajó. Si cambia el mundo, el coordinator decide revalidar o marcar stale.

### 8.3 `AgentContributionEnvelope`

Generaliza el patrón de verify:

```yaml
contribution_id: ...
agent_id: ...
role_id: ...
delegation_id: ...
context_revision: ...
objective: ...
status: success | partial | blocked
coverage:
  satisfied: []
  missing: []
findings: []
proposals: []
alternatives: []
rejected_options: []
assumptions: []
uncertainties: []
risks: []
open_questions: []
pros_cons: []
evidence_refs: []
artifact_refs: []
context_delta: []
recommendation: ...
confidence: high | medium | low
metrics: {}
```

El artifact completo se conserva. El envelope es un índice semántico, no una sustitución del original.

### 8.4 `OrchestrationSynthesisReceipt`

```yaml
synthesis_id: ...
consumed_contributions: []
omitted_contributions: []
conflicts: []
dissent: []
resolved_by: []
selected_option: ...
alternative_options: []
evidence_refs: []
compression_refs: []
information_loss_checks: []
next_candidates: []
```

Así el orchestrator no puede hacer desaparecer silenciosamente una conclusión incómoda de un worker.

## 9. Continuidad temporal

### 9.1 `SessionCheckpoint`

Un session checkpoint será preferentemente una **tag/ref sobre Decision Memory**, no otro snapshot independiente de la verdad.

```yaml
session_checkpoint_id: ...
memory_commit: ...
created_at: ...
project_id: ...
work_item_id: ...
cycle_run_id: ...
phase: ...
latest_event_cursor: ...
```

El contenido detallado se obtiene recorriendo el memory tree del commit.

### 9.2 `SessionDelta`

`diff(A,B)` debe proyectar cambios de:

- runtime/cycle;
- planning;
- artifacts;
- decisions;
- options/rejections;
- assumptions;
- risks;
- questions;
- contributions;
- knowledge freshness;
- source subject;
- continuation frontier.

### 9.3 `ContinuationCandidate`

Evolución del actual `next_recommended`:

```yaml
candidate_id: ...
action: ...
kind: resume | continue | recover | investigate | decide | replan
prerequisites: []
pros: []
cons: []
risks: []
reversibility: high | medium | low
confidence: ...
evidence_refs: []
expected_value: ...
expected_cost: ...
uncertainty: ...
blocks: []
unlocks: []
requires_human: ...
```

El orchestrator presenta pocas opciones, pero la Decision Memory conserva el conjunto completo y la razón de poda/selección.

## 10. Árbol/grafo de decisiones dinámico y trazable

### Stage A — core determinista

1. Decision Plane calcula acciones legales.
2. Policy elimina acciones prohibidas.
3. Se generan `ContinuationCandidate` para las acciones restantes.
4. Se puntúan dimensiones explícitas:
   - dependencias desbloqueadas;
   - riesgo;
   - reversibilidad;
   - evidencia;
   - incertidumbre;
   - coste/latencia;
   - blast radius;
   - autoridad humana.
5. Se calcula frontier de Pareto.
6. Si una alternativa domina y policy lo permite, puede seleccionarse.
7. Si existe empate material, alto riesgo o baja confianza, se escala.
8. Se persisten candidatos, pruning, evidencia y selección en Decision Memory.

### Stage B — bounded lookahead

Cuando el problema justifique planificación multi-step:

- branch/fork desde un DecisionMemoryCommit;
- beam/best-first bounded search;
- límites de depth/nodes/tokens/time;
- environment feedback;
- explicit pruning receipts;
- branch hashes;
- merge/promotion sólo mediante policy/authority.

### Stage C — experimental en Workflow Lab

Evaluar, no asumir:

- Tree of Thoughts;
- Graph of Thoughts;
- MCTS/LATS-like strategies;
- alternative ranking/evaluator policies.

No introducir MCTS o similares como default del kernel. Sólo promover una estrategia si el Lab demuestra mejor quality/éxito con coste, estabilidad, trazabilidad, rollback y policy bounds aceptables.

## 11. Pensamiento lateral

### 11.1 Negative Knowledge como ramas rechazadas

Una opción descartada no desaparece:

```text
refs/heads/rejected/<decision>/<option>
```

Puede llevar `revisit_when`. El sistema evita repetir una idea ya descartada salvo que cambien sus condiciones.

### 11.2 Decision Debt

Una decisión temporal genera deuda de decisión:

```yaml
decision: adapter X temporal
reason: provider Y carece de Z
revisit_when: provider Y soporta Z
```

Al cumplirse el trigger se crea una nueva rama candidata, no se reescribe la decisión histórica.

### 11.3 Semantic compression chain

```text
raw artifact / trace
   ↓
AgentContributionEnvelope
   ↓
OrchestrationSynthesisReceipt
   ↓
DecisionMemoryCommit
   ↓
ResumeView / Context Capsule
```

Cada capa apunta a la anterior.

### 11.4 Information-loss budget

Un validator impide que una compresión elimine:

- blockers;
- riesgos high/critical;
- dissent no resuelto;
- mandatory evidence;
- rejected options con revisit trigger;
- authority decisions;
- open questions que afectan aceptación.

### 11.5 Staleness-aware checkout

“Checkout de memoria” no restaura blindly un estado antiguo. Compara:

- source subject;
- planning revision;
- workflow revision;
- Decision Memory HEAD;
- ADR/spec hashes;
- dependencies.

El Context Compiler marca qué nodos históricos siguen validos y cuáles requieren revalidación.

### 11.6 Counterfactual branches

Workflow Lab puede crear:

```text
what-if/option-B
```

desde un fork-point histórico, replay/re-evaluar y comparar con canonical sin mutar producción.

### 11.7 `blame/why` semántico

No sólo “quién cambió una línea”, sino:

```text
sddk memory why decision:D12
```

que devuelve la ruta mínima:

```text
Requirement → Evidence → Question → Options → Decision → Outcome
```

### 11.8 Garbage collection con reachability

Inspirado en Git, blobs/projections no alcanzables desde refs canónicas, tags, audit retention o ramas activas pueden compactarse/archivarse según policy. Nunca borrar evidence sometida a retención/auditoría.

## 12. Responsabilidades propuestas

### Top-level orchestrator

Debe:

- reconstruir runtime authority;
- resolver Decision Memory HEAD/ResumeView;
- crear delegations;
- validar role/context/contribution contracts;
- preservar raw artifact refs;
- sintetizar y registrar dissent;
- emitir SynthesisReceipt;
- calcular ContinuationCandidates;
- respetar policy/HITL.

No debe:

- rehacer internals del specialist;
- convertir summary en evidence;
- ocultar dissent material;
- aceptar contribution stale sin revalidación;
- dar autoridad a una rama what-if.

### Coordinator

- bounded fan-out/join;
- immutable ContextLease;
- espera workers declarados;
- valida schemas;
- synthesis owner único;
- devuelve ContributionEnvelope + artifact refs + synthesis evidence.

### Leaf

- bounded objective;
- no delega;
- no muta lifecycle/planning authority;
- persiste artifact;
- devuelve structured contribution.

### Evaluator/Judge

- subject inmutable;
- findings/evidence;
- no corrige el objeto evaluado dentro de la misma evaluación salvo contrato explícito.

### Secretary

- consume Decision Plane + ResumeView;
- produce Candidate/Contribution;
- nunca muta workflow directamente;
- atraviesa policy/authority;
- deja rationale + evidence refs;
- aparece como nodo normal del DelegationGraph.

## 13. Integración propuesta en roadmap

### H4 — AgentHost, Context Compiler & CDD

Después de `CTX-COMPILER-002`:

1. `CDD-ROLE-001` — `AgentRoleContract` + topology validator.
2. `CDD-HANDOFF-001` — `DelegationRequest`, `ContextLease`, `AgentContributionEnvelope`.
3. `CDD-HANDOFF-002` — `OrchestrationSynthesisReceipt`, dissent + information-loss guard.
4. `CDD-MEMORY-001` — Git-like Decision Memory object model, content-addressing, parents, refs, HEAD, tags y reflog.
5. `CDD-MEMORY-002` — `log/tree/show/diff/merge-base/branch` projections + SessionCheckpoint/SessionDelta.
6. `CDD-CONTINUE-001` — `ResumeView` + `ContinuationCandidate` integrado con AgentHost/orchestrator.

H5 Human/Secretary consume estos contratos; no crea otros.

### H6 — Workflow Lab

Después de las capacidades básicas del Lab:

7. `LAB-DECISION-001` — bounded branch/fork/lookahead con beam/best-first + Pareto baseline sobre Decision Memory.
8. `LAB-DECISION-002` — experimentos reproducibles ToT/GoT/MCTS/LATS-like y promotion decision.

### H9 Active Graph/Cockpit

No duplicar datos: visualizar Decision Memory/DelegationGraph como proyecciones del mismo modelo.

### H10 GCI

Process mining debe incluir decision outcomes, rejected branches, revisit triggers y handoff quality.

## 14. UAT mínimos

### Continuidad

- cerrar sesión A, abrir sesión B sin chat history y reconstruir el mismo ResumeView;
- `session diff A B` reporta sólo cambios reales;
- una summary Engram borrada no destruye continuidad canónica.

### Git-like memory

- mismos payloads canónicos producen mismo object id;
- un commit no cambia después de creado;
- branch mueve ref, no reescribe history;
- fork conserva base original;
- merge conserva ambos parents y receipt;
- merge-base es estable;
- reflog permite localizar HEAD histórico;
- what-if no puede autorizar runtime mutation.

### Handoff

- coordinator recibe todos los ContributionEnvelope requeridos;
- raw artifacts siguen recuperables;
- missing high-risk finding en synthesis falla information-loss gate;
- stale ContextLease fuerza revalidación;
- worker no autorizado no puede fan-out.

### Secretary

- proposal entra como Contribution/Candidate;
- no puede escribir canonical ref directamente;
- accepted proposal produce policy/authority receipt y Decision Memory commit.

### Decision search

- budgets cortan branch explosion;
- pruning deja receipt;
- baseline deterministic strategy siempre disponible;
- experimental strategy nunca bypassa HITL/policy;
- counterfactual branch no modifica canonical HEAD.

## 15. Métricas

- cold-start reconstruction success rate;
- resume context precision/recall frente a artifacts obligatorios;
- handoff information-loss incidents;
- contribution stale rate;
- dissent-lost rate (objetivo 0);
- duplicate/repeated investigation rate;
- negative-knowledge reuse rate;
- decision revisit precision;
- branch search quality uplift;
- tokens/latency por decisión;
- percentage de decisiones con evidence path completo;
- percentage de orchestrator claims trazables a contribution/evidence.

## 16. No objetivos iniciales

- copiar Git internals o almacenar la memoria dentro de `.git`;
- crear otra base de datos de verdad paralela al ledger/vault;
- persistir chain-of-thought privado de modelos;
- almacenar transcripts completos como requisito de reconstrucción;
- ejecutar MCTS como default;
- permitir a Secretary/worker mover canonical HEAD sin policy;
- convertir Engram en autoridad.

## 17. Conclusión

La pieza que falta no es “más memoria”. Es **memoria estructural y navegable**.

SDDK debería poder tratar su historia cognitiva-operativa como Git trata la historia del código:

```text
inmutable objects
+ parentage
+ cheap refs
+ branches
+ merge/fork
+ diff
+ provenance
+ projections
```

Con esta base, un LLM nuevo puede llegar mañana, resolver `HEAD`, ver el árbol de decisiones relevante, hacer diff con la sesión anterior, recuperar contributions originales, entender pros/contras y continuar sin depender de lo que sobrevivió en el chat.

## 18. Referencias externas

- OpenAI Agents SDK — Sessions: https://openai.github.io/openai-agents-python/sessions/
- OpenAI Agents SDK — Handoffs: https://openai.github.io/openai-agents-python/handoffs/
- OpenAI Agents SDK — Results: https://openai.github.io/openai-agents-python/results/
- OpenAI Agents SDK — Tracing: https://openai.github.io/openai-agents-python/tracing/
- Anthropic — Effective harnesses for long-running agents: https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents
- Anthropic — Multi-agent research system: https://www.anthropic.com/engineering/multi-agent-research-system
- Anthropic — Effective context engineering: https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents
- LangGraph — Persistence: https://docs.langchain.com/oss/python/langgraph/persistence
- LangGraph — Time travel: https://docs.langchain.com/oss/python/langgraph/use-time-travel
- A2A protocol: https://a2a-protocol.org/latest/specification/
- Git internals — objects: https://git-scm.com/book/en/v2/Git-Internals-Git-Objects
- Git internals — refs: https://git-scm.com/book/en/v2/Git-Internals-Git-References
- Git revisions: https://git-scm.com/docs/gitrevisions
- Git merge-base: https://git-scm.com/docs/git-merge-base
- Git notes: https://git-scm.com/docs/git-notes
- Tree of Thoughts: https://arxiv.org/abs/2305.10601
- Graph of Thoughts: https://arxiv.org/abs/2308.09687
- LATS: https://arxiv.org/abs/2310.04406
