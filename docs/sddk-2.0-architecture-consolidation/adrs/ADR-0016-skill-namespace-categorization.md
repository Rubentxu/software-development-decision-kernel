# ADR-0016: Skill Namespace & Categorization

**Status:** Proposed (aligned with SPEC-006, M4 Pack Runtime milestone) · **Date:** 2026-08-16

## Context

22 deep-* skills flat in `skills/`. Hard to navigate. No physical grouping possible without CLI changes.

## Decision

**Option A now, Option B for sddk-2.0.**

- A: Add `metadata.category` + `subcategory` to all skills. Zero CLI changes.
- B: Extend CLI to recurse one extra level. sddk-2.0 (SPEC-006 + M4).

## Consequences

### Positive
- Zero risk, 22 skills self-describing.
- Existing skill-registry infra supports scope labels.
- SPEC-006 + M4 already plan this.

### Negative
- Repo still has 22 directories in `skills/`.

## Implementation

### Phase A (done)
1. ✅ Add `metadata.category: deep-research` + `metadata.subcategory` to all 22 skills.
2. ✅ Create `docs/skill-categorization.md`.
3. ✅ Update `skills/DEEP-RESEARCH-INDEX.md`.

### Phase B (sddk-2.0, M4)
1. Extend `link_editor`, `doctor`, `uninstall`, `bootstrap.sh` to recurse 1 level.
2. Extend `write_skill_registry` with category column.
3. Migrate at least one domain to nested form.

## When to revisit

When next domain grows >30 skills, or contributor requests, or M4 starts.

## References

- `docs/skill-categorization.md`
- `skills/DEEP-RESEARCH-INDEX.md`
- `docs/sddk-2.0-architecture-consolidation/specs/SPEC-006-pack-runtime.md`
- `crates/sddk-cli/src/dev/registry.rs`
