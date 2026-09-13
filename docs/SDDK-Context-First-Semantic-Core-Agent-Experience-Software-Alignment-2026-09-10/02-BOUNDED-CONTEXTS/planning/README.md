# Planning bounded context

## Owns

Goal, WorkItem, planning dependencies, desired manifest/reconciliation y planificación antes de compilar ejecución.

## Explicitly does not own

No posee Run runtime states ni approvals.

## Source layout

Domain types: `crates/sddk-domain/src/planning/` (except Shared Kernel -> `shared/`).

Application/use cases: `crates/sddk-engine/src/planning/` where applicable.

Concrete persistence remains in `sddk-storage` adapters/projections.
