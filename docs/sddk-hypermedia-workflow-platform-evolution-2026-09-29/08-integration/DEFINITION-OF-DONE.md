# Definition of Done por slice

Un slice de esta evolución sólo está `VERIFIED` cuando cumple todo lo aplicable:

## Scope
- baseline SHA exacto;
- objective falsable;
- invariants/no-goals;
- consumer real identificado;
- migration/compatibility impact;
- STOP conditions.

## Code
- una autoridad por concepto;
- no duplication temporal sin owner/removal trigger;
- no provider types leaked to domain;
- no hidden wall-clock/RNG in replay-sensitive identity paths;
- typed errors for epistemically distinct failures.

## Tests
- RED/characterization cuando hay bug existente;
- happy + negative + stale/retry/crash según riesgo;
- affected tests during apply;
- integration/full profile only at correct boundary;
- mutation test para contracts cuya regresión sea fácil de reintroducir.

## UAT
- real environment cuando el claim sea provider/agent/product integration;
- mocks/fakes claramente etiquetados;
- input/output digests;
- limitations;
- no PASS by code reading.

## Persistence
- restart/reopen cuando exista durable state;
- idempotent retry;
- no dangling refs;
- schema migration path si cambia storage.

## Agent experience
- el agent consumer no necesita recipe duplicado que el kernel ya posee;
- hypermedia/action schema suficiente para el slice migrado;
- context basis/provenance visible.

## Documentation
- ADR/spec actualizados sólo tras decisión/implementación correspondiente;
- roadmap delta reconciliado, no historia reescrita;
- receipt + CURRENT/STATE/JOURNAL tras observación.

## Release
- Conventional Commits atómicos;
- no release/push/tag automático fuera del contrato operativo vigente;
- certification profile sólo si sus gates aplicables pasan en el mismo SHA.
