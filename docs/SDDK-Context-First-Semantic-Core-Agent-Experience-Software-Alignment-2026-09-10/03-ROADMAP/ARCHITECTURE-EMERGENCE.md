# Arquitectura emergente — reglas de decisión

La estructura context-first se fija ahora a nivel de **módulos y ownership**, pero las decisiones costosas se posponen hasta tener evidencia.

## Se fija ahora

- ubiquitous language;
- context ownership;
- import/dependency direction;
- advisory vs normative boundary;
- provider ports;
- state authority classification;
- file/module target homes.

## Se deja evolucionar

- crate split;
- SQLite table/materialized-view strategy de projections;
- exact KMT granularity de symbols;
- gRPC transport implementation details;
- exact attention formula;
- lens evaluator implementation;
- UI framework.

## Trigger para reconsiderar

- false-stale KMT > 30% en fixtures;
- verify coste > 40% de deb-verify habitual;
- Alignment exige writes para funcionar;
- lens produce >25% `NOT_APPLICABLE` en su target declarado;
- provider RPC obliga a mapear internals 1:1;
- context cambia conjuntamente con otro >80% durante varios milestones -> reconsiderar boundary.
