# Supersession policy — exactly one normative roadmap

## Adoption action

Create or update a canonical docs entry point such as:

```text
docs/architecture/README.md
```

that names this package/current promoted documents as the sole normative architecture + roadmap.

## This package supersedes

If previously imported, mark `SDDK-Semantic-Core-Consolidation-2026-09-09` as superseded by this **Semantic Core + Agent Experience Consolidation** package. Its architectural core is incorporated here; the major addition is the explicit M7 agent/prompt/skill/CLI contract evolution and resulting M8/M9 renumbering.

## Repository historical packages

Do not erase useful history. Existing architecture consolidation, decision-kernel architecture, stabilization, responsibility-separation and standalone evolutivo folders should receive a banner:

> **Historical / superseded.** This document records previous design context. Current normative architecture and roadmap are linked from `docs/architecture/README.md`.

The repository currently contains multiple such evolution/roadmap families, so adoption is incomplete until the canonical entry point clearly distinguishes historical context from current instructions.

## ADR preservation

Old ADRs remain. Mark superseded ADR status and reference replacement. Map this package's local ADR IDs to repository-native numbering during integration rather than renumbering repository history.

## Roadmap authority

Exactly one document is designated Current Roadmap. Old Execution Spines/timelines/backlogs become migration/history inputs unless explicitly imported as current WorkItems through reconciliation.

## Merge strategy

1. import package temporarily;
2. review crosswalk against current code and completed feat commits;
3. assign repository-native ADR/work-item IDs;
4. promote one canonical architecture/roadmap entry point;
5. mark old packages historical;
6. migrate M0 inventory;
7. only remove historical docs proven redundant after traceability is preserved.
