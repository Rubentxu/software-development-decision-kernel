# UAT Matrix — Hypermedia Workflow Platform

## Convenciones

Estados: `PASS_OBSERVED | FAIL | NOT_RUN | BLOCKED | NOT_APPLICABLE`.  
Cada ejecución debe registrar source SHA, command/harness, environment, inputs, output digest y limitaciones.  
Los casos de provider real identifican build/protocol/capability snapshot.

## C3i — Bootstrap / adoption / active-cycle recovery

| ID | Escenario | Resultado esperado |
|---|---|---|
| CTX-UAT-001 | Proyecto ya adoptado, ejecutar bootstrap 20 veces | no nuevas adopciones ni prompts; misma project identity |
| CTX-UAT-002 | Repo sin remote, restart proceso 10 veces | mismo project/workspace identity persistido |
| CTX-UAT-003 | Un único active cycle, no pasar cycle ID | bootstrap lo infiere y devuelve URI del cycle/run |
| CTX-UAT-004 | Cero active cycles | estado tipado `no-active-work`; action `start-workflow/cycle`, no error inventado |
| CTX-UAT-005 | Dos active cycles | typed ambiguity con ambas candidates; ninguna selección automática |
| MIG-UAT-001 | Caller legacy pasa cycle ID explícito | explicit vence inference; comportamiento compatible |

## C3j — Durable context / session / delta

| ID | Escenario | Resultado esperado |
|---|---|---|
| CTX-UAT-006 | Process A compila capsule, cierra; process B reabre | mismo context basis/digest con estado sin cambios |
| CTX-UAT-007 | WorkItem cambia entre A/B | B recibe nueva basis y delta relevante, no full resend obligatorio |
| CTX-UAT-008 | Cambio irrelevante para binding | no context injection/delta material |
| CTX-UAT-009 | Delta duplicado | reject/no-op tipado; nunca doble aplicación |
| CTX-UAT-010 | Delta out-of-order | `NonMonotonicSeq` / equivalente |
| CTX-UAT-011 | Delta stale from_revision | fail closed |
| CTX-UAT-012 | Dos host sessions mismo project, tasks distintas | bindings aislados, context reads atribuibles |
| CTX-UAT-013 | Host session restart | reattach sin importar transcript completo |
| CTX-UAT-014 | Context exceeds inline budget | capsule contiene refs/progressive disclosure, no dump ilimitado |
| CTX-UAT-015 | Expand evidence/decision ref | contenido correcto + ContextReadRecord actualizado |

## Hypermedia protocol

| ID | Escenario | Resultado esperado |
|---|---|---|
| HYP-UAT-001 | `resource get` Project/Run/Step | envelope versionado con state/basis/relations/actions |
| HYP-UAT-002 | Action mutating con basis correcta | se ejecuta y devuelve nueva representation |
| HYP-UAT-003 | Reutilizar action después de state change | stale-basis rejection |
| HYP-UAT-004 | Read relation | nunca requiere write authority |
| HYP-UAT-005 | Usuario sin authority para action | action ausente o marked unavailable; invocation fail closed igualmente |
| HYP-UAT-006 | Policy cambia tras representation | invocation reevalúa y rechaza si ya no legal |
| HYP-UAT-007 | Missing evidence bloquea completion | problem detail lista missing evidence + recovery actions |
| HYP-UAT-008 | CLI vs MCP same resource | semántica/IDs/actions equivalentes |
| HYP-UAT-009 | Unknown representation field | compatible consumer conserva seguridad según schema policy |
| HYP-UAT-010 | Canary secret en provider error | no aparece en representation/receipt/log surface definida |

## StepDefinition / StepRun

| ID | Escenario | Resultado esperado |
|---|---|---|
| STEP-UAT-001 | Register valid namespaced StepDefinition | digest estable y discoverable |
| STEP-UAT-002 | Missing task_kind | compile/admission fail |
| STEP-UAT-003 | Wildcard capability | reject |
| STEP-UAT-004 | Missing required input | workflow compile fails before run |
| STEP-UAT-005 | Output schema mismatch | child output rejected, no downstream propagation |
| STEP-UAT-006 | Same StepDefinition used in 2 workflows | same contract/digest, independent StepRuns |
| STEP-UAT-007 | Retry same contribution idempotency key | no duplicate evidence/decision |
| STEP-UAT-008 | Retry requires new AttemptId | lineage preserved |
| STEP-UAT-009 | Side-effect step marked reusable cache | policy rejects memoization |
| STEP-UAT-010 | Step blocked waiting event | state survives restart |
| STEP-UAT-011 | Step completion missing required contribution | remains blocked/running, cannot fake success |
| STEP-UAT-012 | Human contribution | same validation + provenance human |

## Workflow compiler/runtime

| ID | Escenario | Resultado esperado |
|---|---|---|
| WF-UAT-001 | Simple sequence | deterministic IR digest + correct execution order |
| WF-UAT-002 | Parallel supported path | bounded concurrency and deterministic join semantics |
| WF-UAT-003 | Map within limits | child lineage and max_concurrency honored |
| WF-UAT-004 | Unsupported operator semantic/version | compile reject, not runtime surprise |
| WF-UAT-005 | Illegal cycle | compile reject |
| WF-UAT-006 | Required capability unsatisfied | compile/admission typed failure depending resolution timing |
| WF-UAT-007 | Discover capability permitted | plan revision recorded and bounded |
| WF-UAT-008 | Discover not permitted | fail closed, no graph mutation |
| WF-UAT-009 | Replan | new revision, prior execution history intact |
| WF-UAT-010 | Restart mid-workflow | resumes from persisted StepRun, no completed step repetition |
| WF-UAT-011 | Legacy default workflow shadow compare | equivalence contract passes for selected slice |
| WF-UAT-012 | LLM-generated invalid workflow | compiler rejects before execution |

## Augmentation / skills / capabilities / providers

| ID | Escenario | Resultado esperado |
|---|---|---|
| AUG-UAT-001 | Step task_kind matches skill | deterministic skill selection |
| AUG-UAT-002 | context key absent | skill requiring it not selected |
| AUG-UAT-003 | skill declares required cap | aggregated capability inherits required status |
| AUG-UAT-004 | same cap optional+required | required wins |
| AUG-UAT-005 | CogniCode READY with `code.dependencies` | selected without workflow provider name |
| AUG-UAT-006 | CogniCode unavailable, capability preferred | Base path + visible gap if step permits |
| AUG-UAT-007 | CogniCode unavailable, required | Step blocked; no fake evidence |
| AUG-UAT-008 | Chronos selected only for runtime-sensitive step | unrelated doc/static step does not invoke it |
| AUG-UAT-009 | two providers same capability | deterministic policy ordering + receipt |
| AUG-UAT-010 | explicit provider override compatible | override selected and recorded |
| AUG-UAT-011 | override lacks capability | fail closed |
| AUG-UAT-012 | provider protocol incompatible | status incompatible preserved |
| AUG-UAT-013 | runtime Discover requests expensive cap beyond budget | rejected/budget action offered |
| AUG-UAT-014 | `explain augmentation` | complete provenance of skill/cap/provider decisions |

## Observation / evidence

| ID | Escenario | Resultado esperado |
|---|---|---|
| OBS-UAT-001 | Real CogniCode observation | provider + subject + capability basis persisted |
| OBS-UAT-002 | Same subject/basis next session | reuse observation when policy permits |
| OBS-UAT-003 | Source SHA changes | old static observation stale |
| OBS-UAT-004 | Analyzer/config changes | previous observation stale if declared basis changed |
| OBS-UAT-005 | Chronos partial capture | status Partial, never Complete |
| OBS-UAT-006 | Chronos timeout | TimedOut preserved + recovery action |
| OBS-UAT-007 | Static/runtime contradiction | both preserved, no last-writer overwrite |
| OBS-UAT-008 | Provider returns malicious instruction text | stored as untrusted data/evidence, not InstructionSource |
| OBS-UAT-009 | Heavy raw graph/trace | SDDK persists stable ref/digest + semantic facts, not mandatory full copy |
| OBS-UAT-010 | Observation used to verify WorkItem | EvidenceRef + relation traversable |
| OBS-UAT-011 | Provider absent in Base workflow | Base remains usable |
| OBS-UAT-012 | Required provider absent | only affected operation blocked |

## Contribution reconciliation

| ID | Escenario | Resultado esperado |
|---|---|---|
| CON-UAT-001 | Agent returns decision contribution | DecisionRecord persisted and linked |
| CON-UAT-002 | Agent returns evidence contribution | body CAS + EvidenceRef + relation |
| CON-UAT-003 | Tasks output decomposes 3 items | 3 WorkItems + declared dependencies |
| CON-UAT-004 | Same envelope replayed | no duplicates |
| CON-UAT-005 | One contribution invalid in atomic bundle | transaction/typed partial failure per declared boundary |
| CON-UAT-006 | Markdown renderer unavailable | structured state remains canonical; human projection can fail separately |
| CON-UAT-007 | Direct DB write attempt by agent path | unavailable/blocked by architecture surface |
| CON-UAT-008 | Completion with missing evidence tier | StepRun not succeeded |
| CON-UAT-009 | Apply commits linked to WorkItem | why traversal reaches commit/evidence |
| CON-UAT-010 | Verify results | EvidenceRefs linked to subject/work item |
| CON-UAT-011 | ContextDelta contribution stale | rejected before canonical mutation |
| CON-UAT-012 | Human edits decision | new revision/provenance, no silent overwrite |
| CON-UAT-013 | Crash after CAS before projection | recover/reconcile idempotently; no dangling success |
| CON-UAT-014 | Crash after event append before response | retry returns existing result, no duplicate fact |

## Packs / custom workflows / domain generality

| ID | Escenario | Resultado esperado |
|---|---|---|
| PACK-UAT-001 | Project custom workflow uses registered steps | compiles without editing framework prompts |
| PACK-UAT-002 | Missing pack | affected workflow blocked; SDDK Base starts |
| PACK-UAT-003 | Framework/user/project definitions same name | deterministic documented precedence/version resolution |
| PACK-UAT-004 | Incompatible step version | compile reject with alternatives |
| PACK-UAT-005 | Pack attempts wildcard capability | reject |
| PACK-UAT-006 | Pack attempts direct authority bypass | impossible/rejected |
| PACK-UAT-007 | Software workflow migrated | no core behavior regression in applicable gates |
| PACK-UAT-008 | Book workflow E2E | same runtime/context/evidence mechanisms |
| PACK-UAT-009 | `content.write-section` map over N chapters | independent child StepRuns + deterministic join |
| PACK-UAT-010 | Book process restarts after chapter 3 | resumes pending chapters only |
| PACK-UAT-011 | Content research evidence reused by chapters | refs/provenance, not repeated full research by default |
| PACK-UAT-012 | Remove software pack in isolated test | generic core schemas/tests still build/run where applicable |

## Release / migration regression

| ID | Escenario | Resultado esperado |
|---|---|---|
| MIG-UAT-002 | Open DB previous schema | additive migration + integrity checks |
| MIG-UAT-003 | Crash during new context migration | preflight/reopen deterministic typed outcome |
| MIG-UAT-004 | Legacy prompt workflow before cutover | unchanged behavior |
| MIG-UAT-005 | New runtime feature flag disabled | legacy path remains functional during window |
| MIG-UAT-006 | New runtime enabled for selected slice | no duplicate side effects |
| MIG-UAT-007 | Downgrade/read old data policy | explicit support or fail-closed, never silent corruption |
