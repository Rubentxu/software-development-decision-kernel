# Decision bounded context

## Owns

Decision Memory, refs/reflog semánticos, decisions, assumptions, dissent references y TradeoffRecord.

## Explicitly does not own

No posee workbook cells, métricas ni provider evidence.

## Source layout

Domain types: `crates/sddk-domain/src/decision/` (except Shared Kernel -> `shared/`).

Application/use cases: `crates/sddk-engine/src/decision/` where applicable.

Concrete persistence remains in `sddk-storage` adapters/projections.
