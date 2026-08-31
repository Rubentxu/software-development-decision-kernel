# ADR-034-PACK-MICROKERNEL — Evolve packs into a microkernel extension contract

**Status:** Accepted


## Decision
A pack may declare workflows, capabilities, agents, schemas, policies, behaviors, evidence types and moldable views. Packs declare `requires`, `integrates_with` and `conflicts_with`.

## Rule
A pack may extend kernel concepts but may not redefine kernel authority or bypass governed side effects.

## Consequences
SDD, UAT, incident and security can evolve independently while remaining composable.
