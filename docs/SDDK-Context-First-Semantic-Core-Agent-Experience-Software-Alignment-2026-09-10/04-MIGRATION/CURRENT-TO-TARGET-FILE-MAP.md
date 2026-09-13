# Current -> target file map

Este mapa se aplica **antes** de añadir los nuevos modelos. Las rutas reflejan la taxonomía observada en el repositorio actual; el inventario R0 confirma cada archivo antes del movimiento.

## sddk-domain — candidatos principales

| Actual | Target module | Acción |
|---|---|---|
| `cycle.rs`, `goal.rs`, `planning.rs`, `spine.rs`, `proposal.rs` | `planning/` | MOVE/SPLIT según runtime leakage |
| `workflow.rs`, `workflow_ir.rs`, `workflow_run.rs`, `execution_graph_compiler.rs`, `execution_scope.rs`, `transition_ast.rs` | `execution/` | MOVE |
| `plan_revision.rs`, `fork.rs`, decision-related models | `decision/` | MOVE/SPLIT common revision primitive to `shared/` |
| `graph.rs`, `staleness.rs`, `context_read.rs` | `knowledge/` | MOVE; graph remains projection semantics |
| `evidence.rs`, `event_envelope.rs`, identity/schema primitives | `shared/` | MOVE/KEEP facade |
| `rules.rs`, operator/authority contracts where normative | `governance/` | MOVE/SPLIT |
| `pack.rs`, extension schemas | `extension/` | MOVE |
| `metrics.rs`, generic `view/projections` | cross-context projection support or owner context | SPLIT by owner, do not create Metrics BC |
| `uat.rs` | pack/UAT integration | KEEP outside Alignment core |

## sddk-engine — candidatos principales

| Actual family | Target module |
|---|---|
| `build_work_graph`, planning/cycle preparation | `planning/` |
| `execution_controller`, `workflow_runtime`, retry/fanout/join/run-view runtime logic | `execution/` |
| `decision_lab`, human decision data, memory-specific logic | `decision/` or `governance/` depending authority |
| `active_graph`, staleness/up_to_date, knowledge projection helpers | `knowledge/` |
| **new Alignment types/evaluators/workbooks** | `alignment/` from day one |
| `converge_verification`, engineering assurance/debt verification orchestration | `verification/` after ownership audit |
| `authority`, gates, risk approval, signed gates, human approval | `governance/` |
| agent host/role/contribution/synthesis/context capsule | `agent_experience/` |
| provider router/generic pack contracts/pack registry | `extension/` |
| telemetry/workflow metrics | projection/observability support; owner-specific outputs |

## sddk-cli

`verify_cmd.rs` se convierte en adapter fino de `verification::VerifyUseCase`; no contiene lógica de ledger/capability stitching.

`debt.rs` deja de ser el sitio donde nace el modelo global de debt. Sus subcomandos migran hacia Verification/Decision/Governance según ownership y CLI sólo orquesta.

## Regla de commit

- Commit A: move + reexports + tests.
- Commit B: dependency enforcement.
- Commit C+: semantic changes.

No mezclar las tres cosas en un mega-refactor.
