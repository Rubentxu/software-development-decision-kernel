# Governance bounded context

## Owns

AuthorityEngine, policy snapshots, AdmissionDecision, approvals, risk policy, waiver authorization y gates.

## Explicitly does not own

No convierte heurísticas Alignment en normas implícitas.

## Source layout

Domain types: `crates/sddk-domain/src/governance/` (except Shared Kernel -> `shared/`).

Application/use cases: `crates/sddk-engine/src/governance/` where applicable.

Concrete persistence remains in `sddk-storage` adapters/projections.
