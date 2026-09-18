---
id: ADR-0132-GITHUB-RELEASES-MIGRATION-PATTERN
status: accepted
supersedes_history: false
adopted_at: 2026-09-18
adoption_cycle: p-63676b11dc0ef88f/a6-2-github-releases-migration
package_local_id: null
package_source: null
accepted_at: 2026-09-18
accepted_by_cycle: p-63676b11dc0ef88f/a6-2-github-releases-migration
superseded_by: []
related_adrs:
  - ADR-0130-FENCED-ADMISSION-TICKETS
  - ADR-0131-MIGRATION-PATTERN-FOR-WRITABLE-SURFACES
  - ADR-0102-UNIFIED-AUTHORITY-ENGINE
stale_after: 2027-09-18
---

# ADR-0132 — `github_releases` Migration Pattern (R4-B, A6-2 onward)

> New ADR written in repo (no package source).

## Context

`ADR-0131-MIGRATION-PATTERN-FOR-WRITABLE-SURFACES.md` (A6-1) defined a
generic surface-migration pattern. A6-2 applies that pattern to the
`github_releases` surface, which is the second High-band unguarded
surface named in `INC-R4-DECISION-EFFECT-ATOMICITY-BOUNDARY.md`.

A6-1 wrapped a single physical write path (`framework_bundle`'s prefix
writes). The `github_releases` surface is different: it is a **chain**
of operations (`pr.create` → `pr.merge` → `release.create`) executed
through the `apply_release(...)` sink. The "side effect" is the whole
chain.

## Decision

The wrap pattern for `github_releases` is:

1. **Admit-time check (preserved)**: `authorize_release(...)` on the
   permission policy continues to run before any forge work. R4-A
   semantics intact.

2. **Ticket issue point**: the helper issues the ticket immediately
   after the admit-time check, *before* any `apply_release(...)`
   invocation. The ticket carries `(decision, policy_digest, fence_token=0,
   issued_at_seq=0)` per the A6-0 primitive.

3. **Apply-chain under consume**: `apply_release(gateway, plan, forge,
   version_lockstep_passed)` runs inside the wrapper's `consume(...)`.
   If the consume refuses, **none** of the three steps
   (`CreatePr`, `MergePr`, `CreateRelease`) run — they live behind the
   `apply_release` boundary. The ticket covers the whole chain.

4. **Forge unchanged**: `GitHubForge` and `MockForge` (in
   `crates/sddk-gateway/src/forge.rs`) are not modified. The ticket
   protects at the CLI level; the gateway stays agnostico.

## Honest limits (inherited from ADR-0131)

T2 (PolicyChanged via policy_digest) is NOT directly pinned at this
call site — the A6-3 upgrade is required to thread the live
`PolicySnapshot` from `AuthorityEngineRunner` into the wrappers. The
A6-0 primitive pins T2 at the engine layer; A6-1 and A6-2 pin T6
(verdict-anchored consume) at their helpers.

## Non-goals

- **No modification of `apply_release(...)`, `GitHubForge`, `MockForge`.**
- **No `Local` route ticket** (Local route touches `git.push` /
  `git.tag` only; that's a different concern and a future cycle).
- **No Medium/Low-band migration.**
- **No A4 semantic change.**

## See also

- `docs/architecture/adrs/ADR-0130-FENCED-ADMISSION-TICKETS.md`
- `docs/architecture/adrs/ADR-0131-MIGRATION-PATTERN-FOR-WRITABLE-SURFACES.md`
- `docs/architecture/a6/A6-1-RECEIPT.md` (prior data point)
- `docs/architecture/a6/A6-2-RECEIPT.md` (this cycle's evidence)
- `docs/architecture/a6/A6-2-PLAN.md`
