# Definition of Done — Architecture Consolidation Work

A roadmap item is not done merely because code compiles.

## Required where applicable

- behavior/use-case tests pass;
- architecture dependency rules pass;
- new events have schema/version fixtures;
- replay/projection rebuild tests pass;
- no governed side effect can bypass capability policy;
- evidence/receipt lineage exists for authoritative actions;
- CLI compatibility tests updated intentionally;
- docs generated/validated from canonical sources where possible;
- ADR/spec updated if semantics changed;
- migration path documented for public/state schema changes;
- new complexity does not merely move from prompts into giant runtime modules;
- new deferred follow-ups have revisit triggers.

## Pack-specific

- manifest validates;
- `requires`/`integrates_with` declared;
- pack can disable cleanly;
- fixtures are deterministic;
- no direct cross-pack implementation calls for coordination;
- optional integrations degrade gracefully.
