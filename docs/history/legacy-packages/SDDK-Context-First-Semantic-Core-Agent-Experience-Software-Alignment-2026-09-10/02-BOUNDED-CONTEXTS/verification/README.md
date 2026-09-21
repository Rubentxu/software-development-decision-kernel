# Verification bounded context

## Owns

Verify/DebVerify use cases, scopes, deepening policy, receipts, evidence-gap handling y orchestration de providers/alignment.

## Explicitly does not own

No implementa heurísticas de diseño ni provider internals.

## Source layout

Domain types: `crates/sddk-domain/src/verification/` (except Shared Kernel -> `shared/`).

Application/use cases: `crates/sddk-engine/src/verification/` where applicable.

Concrete persistence remains in `sddk-storage` adapters/projections.
