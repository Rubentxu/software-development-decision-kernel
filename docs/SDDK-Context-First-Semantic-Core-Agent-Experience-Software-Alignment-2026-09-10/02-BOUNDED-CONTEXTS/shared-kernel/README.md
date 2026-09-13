# Shared Kernel bounded context

## Owns

Identity/value references, canonical event envelope, universal Evidence primitives, CAS/ObjectId/Revision primitives y contratos mínimos compartidos.

## Explicitly does not own

No contiene Planning, Alignment, Verification, provider logic ni utilidades arbitrarias.

## Source layout

Domain types: `crates/sddk-domain/src/shared_kernel/` (except Shared Kernel -> `shared/`).

Application/use cases: `crates/sddk-engine/src/shared_kernel/` where applicable.

Concrete persistence remains in `sddk-storage` adapters/projections.
