# Alignment Workbook catalog

Cada workbook define grid/query/charts/controls y drill-down. Todos son projections.

| Workbook | Foco | Tablas principales | Visualizaciones | Advice/control |
|---|---|---|---|---|
| Universal Boundaries | responsabilidad/cohesión/coupling | units, boundary edges | heatmap, trend | tensions |
| Hexagonal | expected vs observed layers | units, forbidden deps, ports/adapters | DSM, bars, trend | drift |
| Bounded Context | ownership semántico | contexts, concepts, context map | coupling matrix | leakage |
| OO SOLID/GRASP | heurísticas OO | principle assessments | bars/trend | tensions only |
| Functional | purity/effects/composition | effects, partiality, state | trend/matrix | tensions only |
| Connascence | strength × distance | edges | scatter/heatmap | high distant tension |
| Smells | smells temporales | findings | Pareto/trend | persistence |
| ADT/State | illegal states/modeling | types | bars/trend | opportunities |
| Tradeoffs | decisions proyectadas | options/forces/triggers | balance sheet | REVIEW_DUE |
| Invariants | contract status | invariants/evidence | pie/trend | explicit contract results |
| Dependencies/Hotspots | topology + churn | modules/hotspots | scatter/DSM | attention |
| Static/Runtime | declared/static/runtime | edges/scenarios | matrix/trend | runtime drift |

`Knowledge Health` vive en Knowledge BC aunque puede enlazarse desde Alignment Control Tower.
