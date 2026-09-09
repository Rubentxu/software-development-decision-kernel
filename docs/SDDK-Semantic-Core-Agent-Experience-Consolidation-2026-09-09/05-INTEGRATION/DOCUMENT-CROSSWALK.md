# Document crosswalk and retention strategy

## Keep as historical evidence

- old ADRs and design investigations;
- completed execution timelines/spines;
- release notes/changelogs;
- migration reports;
- research that explains rejected alternatives.

## Supersede as normative guidance

- previous “current architecture” master specs;
- multiple competing roadmaps;
- standalone evolutivo status files that instruct future execution;
- old LLM start-here documents that point to obsolete backlogs.

## Merge into new canonical docs

- reusable hexagonal-boundary rules;
- existing universal Evidence design;
- existing CEP/event canonicalization decisions;
- existing pack-agnosticity work;
- existing handoff/synthesis invariants;
- existing ContextCapsule useful fields;
- existing PlanRevision/fork invariants that belong in Revision substrate.

## Do not mechanically delete

A document is deleted only if its information is duplicated byte-for-byte or has zero historical/audit value. Prefer moving to `docs/history/` with supersession metadata.
