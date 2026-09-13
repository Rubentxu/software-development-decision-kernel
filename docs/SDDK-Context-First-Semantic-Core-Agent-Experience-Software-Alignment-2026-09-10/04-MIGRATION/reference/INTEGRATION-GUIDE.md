# Integration guide

## Recommended destination

Import for review under:

```text
docs/sddk-semantic-core-agent-experience-consolidation/
```

After acceptance, promote selected files into canonical architecture/spec/ADR/roadmap locations rather than keeping another permanently parallel documentation tree.

## PR 1 — establish documentation authority

- add this package;
- create/update one canonical architecture entry point;
- add supersession banners to previous roadmaps/evolution packs;
- explicitly mark the earlier Semantic Core proposal superseded by this package if it was imported;
- no production code change.

## PR 2 — M0 inventory

- ownership/state-class registry;
- advisory architecture rules;
- CLI CommandSpec/golden snapshot;
- agent asset inventory (AGENTS/prompts/skills/agents/templates/rules/embedded examples);
- classify every item KEEP/MIGRATE/SPLIT/ABSORB/DEPRECATE/REMOVE.

## Subsequent PR contract

Every implementation PR states:

```text
milestone:
replaces:
keeps:
deprecates:
canonical_authority_changed?:
data_migration?:
projection_rebuild?:
agent_assets_impacted?:
command_contract_changed?:
examples_updated_and_tested?:
compatibility_test:
UAT:
```

## Existing roadmap items

Re-home rather than discard intent:

- HANDOFF → M5;
- MEMORY → M4;
- GRAPH foundation → M3;
- WHY → M8;
- prompts/agents/skills/CLI instruction migration → M7.

If code is already implemented on `main`, treat it as substrate to reconcile. Existing implementation does not justify creating another parallel model.

## Definition of Done per migration slice

- replacement path used by at least one real workflow;
- compatibility fixture where applicable;
- responsibility/source of truth documented;
- UAT passes in clean sandbox;
- old path frozen/removed per phase;
- docs, CommandRegistry examples and agent surfaces updated together;
- no active prompt/skill references deprecated internal semantics;
- execution provenance identifies new contract versions/hashes where agent execution is involved.
