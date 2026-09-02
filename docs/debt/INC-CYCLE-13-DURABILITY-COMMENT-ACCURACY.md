---
id: INC-CYCLE-13-DURABILITY-COMMENT-ACCURACY
title: "Weak // durability-required: comment on adoption.rs same_basename_different_remotes test"
status: resolved
severity: low
priority: P3
fingerprint: "4cc2dbe418749e4e"
fingerprint_aliases: ["4cc2dbe418749e4ebd8f16ba2e12b493d0255bea0275ea7b3410c403e8c017ae"]
cluster_id: CL-DOC-QUALITY
created: 2026-08-22
created_by: sddk-verify
owner: orchestrator
resolved_by: p-63676b11dc0ef88f/cycle-50-housekeeping-p3
resolved_at: 2026-09-01
---

# INC-CYCLE-13-DURABILITY-COMMENT-ACCURACY — weak durability comment on one adoption test

> Durable record for one debt finding across cycles. See ADR-0047 §3.2.

## Context

Cycle-13 added `// durability-required:` comments to all 11 non-migrated tests
in `crates/sddk-engine/tests/adoption.rs`. One of those comments
(`same_basename_different_remotes_and_scopes_do_not_collide`, line 44-46)
is technically weak:

```
// durability-required: apply_adoption calls Storage::open(&plan.paths.ledger) internally,
// opening a separate connection; the plan's paths.ledger resolves to a TempDir path,
// not an in-memory database, so in-memory migration is not byte-equivalent.
```

The comment describes `apply_adoption`'s internal connection pattern, but
the test itself only calls `plan_adoption` (no apply_adoption). The
test's three assertions are:

```rust
assert_ne!(first.identity.project_id, second.identity.project_id);
assert_ne!(first.identity.project_id, scoped.identity.project_id);
assert_ne!(first.paths.ledger, second.paths.ledger);
```

These assertions do not require cross-call durability. The test could
technically be migrated to `Fixture::new_in_memory()` and the assertions
would pass byte-identically. The conservative choice (keep on file-based)
preserves byte-equivalence but the stated reason is inaccurate.

## Rationale

- **Severity = low**: this is a documentation accuracy issue, not a
  behavioral defect. The test still passes; the file-based storage is
  harmless; the assertion coverage is unchanged. A reader who trusts the
  comment may misjudge the byte-equivalence claim, but the migration is
  correct and the byte-equivalence AC is preserved.

- **Priority = P3**: opportunistic, when convenient. The recommended
  fix is a comment reword:

  > `// File-based for fixture consistency with surrounding suite; this test
  >  does not require durability but uses Fixture::new() to match the
  >  adoption suite's helpers.`

  Alternative: migrate the test to `Fixture::new_in_memory()` and remove
  the comment.

- **Cluster = `CL-DOC-QUALITY`** (documentation accuracy family).

## Lifecycle

| Date | Actor | Change | Evidence |
|------|-------|--------|----------|
| 2026-08-22 | sddk-verify | created | `verify-report.md` §Issues → SUGGESTION (DEBT-CYCLE-13-DURABILITY-COMMENT-ACCURACY); adoption.rs lines 44-46 |

## References

- `crates/sddk-engine/tests/adoption.rs:44-46` — the weak comment
- `crates/sddk-engine/tests/adoption.rs:48-61` — the test body (only calls `plan_adoption`)
- `~/.local/share/sddk/projects/p-52b95ef55999f9de/cycle-artifacts/p-52b95ef55999f9de/kernel-cycle-13-m1-hexagonal-ports/verify-report.md` — verify phase output

## Closure Evidence

Closed by `p-63676b11dc0ef88f/cycle-50-housekeeping-p3` (v1.66.4).

- **Resolution:** Comment reworded from inaccurate `// durability-required: apply_adoption calls...` to accurate `// File-based for fixture consistency with surrounding suite; this test does not require durability but uses Fixture::new() to match the adoption suite's helpers.`
- **Closing commit:** `cbd8ad7` — docs(rustdoc): reword adoption.rs durability comment (cycle-50 commit #3)
- **Release tag:** [v1.66.4](https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v1.66.4)

> Filled by `sddk-archive` (cycle-50); consumed by `sddk-debt-verify` for cross-cycle correlation via fingerprint.