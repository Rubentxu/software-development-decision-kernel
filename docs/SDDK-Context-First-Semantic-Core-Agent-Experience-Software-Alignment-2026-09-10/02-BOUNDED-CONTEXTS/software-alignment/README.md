# Software Alignment bounded context

## Owns

ArchitecturalIntentSnapshot, UniversalConcern, LensDefinition, AlignmentAssessment/Tension/Opportunity, advisory workbooks y attention suggestions.

## Explicitly does not own

No posee Authority, gates, code mutation ni instrucciones imperativas.

## Source layout

Domain types: `crates/sddk-domain/src/software_alignment/` (except Shared Kernel -> `shared/`).

Application/use cases: `crates/sddk-engine/src/software_alignment/` where applicable.

Concrete persistence remains in `sddk-storage` adapters/projections.
