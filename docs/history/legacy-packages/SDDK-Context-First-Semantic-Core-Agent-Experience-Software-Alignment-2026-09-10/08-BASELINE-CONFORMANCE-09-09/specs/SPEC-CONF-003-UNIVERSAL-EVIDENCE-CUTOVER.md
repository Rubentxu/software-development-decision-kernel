# SPEC-CONF-003 — Universal Evidence Cutover

## Baseline contract

Closes ADR-007, SPEC-001 evidence requirements, M1 and M9 removal of planning-specific evidence duplication.

## Problem

`PlanningEvidenceKind` still exists and its complete migration has been deferred. That is incompatible with declaring M9 finished.

## Required end-state

Universal `Evidence`/`EvidenceRef` is the only core evidence substrate. Planning, execution, governance, agent contributions and verification-like workflows may add typed metadata/relations but MUST NOT define a second evidence authority hierarchy.

## Requirements

- stop all new writes of `PlanningEvidenceKind` and equivalent planning-only evidence authority types;
- map legacy values losslessly to universal evidence metadata/relations;
- preserve historical bytes/source references where exact replay matters;
- migrate `EvidenceAttachmentV1` to `EvidenceRef` plus semantic relation/fact as specified by the baseline;
- expose compatibility decoding only at read boundaries;
- enforce ARCH-SC-008 as blocking;
- remove `evidence-migration-v2` as “future debt” by executing it inside this closeout or formally proving the symbol is already unreachable production compatibility.

## Required relation semantics

The model MUST distinguish at least:

- `supports`;
- `verifies`;
- `gates`;
- `produced_by`;
- `contradicts`.

Domain-specific code may define strongly typed relation builders, not parallel evidence roots.

## Acceptance

- no production writer constructs deprecated planning evidence types;
- static dependency test rejects new references outside migration/read-compat modules;
- migrated evidence yields equivalent planning/WHY outputs;
- UAT-04, UAT-05, UAT-11 and UAT-19 remain green;
- repository search allowlist contains only explicit legacy decoder/migration locations.
