# HANDOFF: ADR-0083 authored + 3 stale INCs closed (continued hygiene arc)

**Date**: 2026-09-11
**Audit**: `p-63676b11dc0ef88f/inc-hygiene-2026-09-11` (extension #4)
**Path**: A-min scope for ADR authoring; B-direct for INC closures
**Outcome**: ADR-0083 authored; 3 stale INC closures (INC-DKA-ORPHAN-REVIEW-PHASE, INC-DKA-MANAGED-CLOSURE-VAULT-ROUTE, INC-VAULT-PHANTOM-ADR-0070-PROMPT-DISCIPLINE)

## Summary

Continued the inc-hygiene arc with three closures that build on the P1-zero milestone:

1. **INC-DKA-ORPHAN-REVIEW-PHASE** (P3 low) — Verified Phase::Review variant is NOT present in `crates/sddk-domain/src/cycle.rs` (9 variants, not 10). Cleanup per ADR-0074 occurred before bootstrap import; INC was filed against prior project state and carried forward as stale. **Closed**: invariant enforced.

2. **INC-DKA-MANAGED-CLOSURE-VAULT-ROUTE** (P2 medium) — Verified `archive.vault.complete` transition IS declared in `workflow/workflow.yaml:490` with all expected gates (vault-receipt-verified, vault-index-current, release-bypass-declared), artifacts, and DeliveryKind guard. Route is operational. **Closed**: implementation verified.

3. **INC-VAULT-PHANTOM-ADR-0070-PROMPT-DISCIPLINE** (P2 medium) — Authored **ADR-0083-prompt-apply-discipline** to provide the real decision authority for REQ-C13-APPLY-DISCIPLINE (the "5-line rule" for `test_aggregate.total_workspace_tests` sourcing). The phantom ADR-0070-prompt-discipline is NOT back-filled (per the 2026-09-03 conservative neutralization invariant); ADR-0083 explicitly supersedes it. REQ-C13 decision_authority restored. **Closed**: real authority declared.

## ADR-0083 — Prompt apply discipline: test count sourcing rule

**Vault path**: `~/.sddk-knowledge/sddk-framework/adrs/ADR-0083-prompt-apply-discipline.md`

**Key decisions**:

- The apply phase MUST source `test_aggregate.total_workspace_tests` from the verbatim stdout of `sddk dev test count-workspace`.
- Hand-rolled counts (per-package sums, heuristic estimates) are FORBIDDEN.
- Drift is allowed only with explicit `test_count_adjustment_reason:` YAML field naming the commit and new count.
- The verify-phase coherence trigger evaluates equality between the apply envelope and the verify-phase `cargo test --workspace` re-run count.

**Considered options** (3):

| Option | Verdict |
|--------|---------|
| Hand-rolled counts (status quo pre-cycle-13) | Rejected — recurring CL-REPORTING-DEFECT |
| Per-package sums with delta logging | Rejected — two sources of truth, drift not eliminated |
| Verbatim sourcing from `sddk dev test count-workspace` | **Accepted** — single source of truth, equality check |

**Consequences**:

- **Positive**: CL-REPORTING-DEFECT cannot recur under hand-rolled counts; apply↔verify coherence becomes deterministic.
- **Negative**: Apply agent gains a mandatory tool call (minor cognitive overhead).
- **Neutral**: Verify-phase coherence trigger mechanics unchanged.

## INC counts after this handoff

| Status | Before | After |
|--------|--------|-------|
| open   | 27 | 24 |
| closed | 13 | 16 |
| resolved | 7 | 7 |
| tracked | 1 | 1 |

Total: 46 INCs.

## Files changed

- `~/.sddk-knowledge/sddk-framework/adrs/ADR-0083-prompt-apply-discipline.md` — **new ADR**
- `~/.sddk-knowledge/sddk-framework/specs/cycle-13-debt-sweep/REQ-C13-APPLY-DISCIPLINE.md` — `decision_authority` restored + changelog entry
- `~/.sddk-knowledge/sddk-framework/incs/INC-DKA-ORPHAN-REVIEW-PHASE.md` — closed with invariant citation
- `~/.sddk-knowledge/sddk-framework/incs/INC-DKA-MANAGED-CLOSURE-VAULT-ROUTE.md` — closed with workflow.yaml:490 citation
- `~/.sddk-knowledge/sddk-framework/incs/INC-VAULT-PHANTOM-ADR-0070-PROMPT-DISCIPLINE.md` — closed with ADR-0083 reference
- `.sddk/cycles/p-63676b11dc0ef88f/inc-hygiene-2026-09-11/audit-log.md` — extended (gitignored, persistent)
- `docs/handoff/HANDOFF-2026-09-11-adr-0083-stale-closures.md` — this handoff

No code changes. No Cargo.toml changes. No version bump.

## Conservative invariant preserved

The 2026-09-03 vault repair neutralized the phantom ADR-0070-prompt-discipline reference and explicitly forbade fabricating the missing ADR. ADR-0083 honors this invariant:

- ADR-0070-prompt-discipline is **NOT created** (phantom remains unsuppressed in spirit, only superseded).
- ADR-0083 is a **new node** with its own slug and traceable provenance.
- ADR-0083 explicitly states "supersedes the phantom ADR-0070-prompt-discipline reference" in its body.

This preserves audit-trail honesty: the phantom reference's history is preserved (the 2026-09-03 neutralization is documented in both REQ-C13 and INC changelogs), and the new ADR provides the real authority without back-filling the gap.
