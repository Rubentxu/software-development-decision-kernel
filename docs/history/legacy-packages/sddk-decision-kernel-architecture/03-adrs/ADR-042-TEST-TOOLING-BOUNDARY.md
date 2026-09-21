# ADR-042 — Test-Tooling Boundary: Sequencing and Target Migration

**Status:** Accepted (user-approved 2026-08-28)

## Approval provenance

- **Date:** 2026-08-28
- **Decision:** User binding approval of test-tooling sequencing and phased migration plan; roadmap reprioritization.
- **Authority:** Amendment-002 (sha `0f711b3aa9e4ea551870260f3009dc58c2c25b99f5bd7c858fbd06cb103b22e9`) and Amendment-003 (sha `35b82978…`) as the binding decisions that produced this ADR.
- **Acceptance:** At creation, per user decision. No future-cycle acceptance step required.

## Context

The current repository has accidental accretion in test-tooling ownership (documented in `TEST-TOOLING-EVIDENCE-AUDIT.md`). Without an explicit sequencing and migration plan, debt-verify will continue flagging boundary drift cycle after cycle without a basis for prioritization.

The target architecture needs a phased migration plan that:
1. Repairs the current cycle's boundary state without disrupting active work.
2. Establishes the ownership/lint foundation in the next cycle.
3. Defers cleanup until parity evidence is available.

The ownership policy itself is owned by ADR-0069. This ADR owns sequencing and migration.

## Decision

### Ownership policy

Test-tooling ownership is bound by ADR-0069 §Decision. The four-cell declaration (Rust / Shell / Python / JS) is the canonical policy for this repository. This ADR does NOT restate the policy body — it references ADR-0069 as the single source of truth.

### Sequencing is owned here

The phased migration sequence below is binding for:
- `docs/sddk-decision-kernel-architecture/02-roadmap/ROADMAP.md`
- `docs/sddk-decision-kernel-architecture/02-roadmap/BACKLOG.md`
- `docs/sddk-decision-kernel-architecture/02-roadmap/MIGRATION-PLAN.md`
- `docs/sddk-decision-kernel-architecture/09-implementation/IMPLEMENTATION-BACKLOG.md`

## Migration plan

### Phase A — Current-cycle repair / verification

**Timeline:** This cycle (`p-52b95ef55999f9de/kernel-cli-agent-information-flow`).

**Scope:**
- Audit the 16 new shell contract tests introduced by commit `643180a`. Classify each per ADR-0069 ownership cells. Flag any that belong in Rust (binary behavior tests) for Phase B migration.
- Annotate test filenames with ownership prefix where the convention supports it (e.g., `tests/shell/test_*smoke*.sh`).
- Record findings in `TEST-TOOLING-EVIDENCE-AUDIT.md` §"Accidental accretion" + §"Concrete false positives."
- This is documentation + light annotation only. No source rewrites. No test deletions.

**Exit:** Findings documented. Phase B entries in IMPLEMENTATION-BACKLOG.md are concrete.

### Phase B — Next-cycle ownership / lint / testkit foundation

**Timeline:** The cycle after the current one.

**Scope:**
- Add `shellcheck` to the local CI gate for `tests/test_*.sh` (gap documented in `TEST-TOOLING-EVIDENCE-AUDIT.md` §"CI / local-gate gaps").
- Add a Python linter (e.g., `ruff`) to the local CI gate for `scripts/` if Python scripts are in scope.
- Evaluate ADR-0022 (sddk-testkit, currently proposed) for adoption or supersession.
- Add ownership-prefix convention to test filenames (Rust / Shell / Python / JS prefix).
- Migrate flagged Phase A false positives to their correct ownership language, after parity verification.

**Exit:** Local CI gate covers shell and Python lint. Ownership prefix convention documented. False positives migrated with parity evidence.

### Phase C — Later cleanup / removal (only after parity)

**Timeline:** Deferred. Gated on parity evidence from Phase B.

**Scope:**
- Move any remaining tests flagged as misowned, AFTER parity is verified (same test passes in new ownership language AND original test is deleted).
- Remove superseded test scaffolding only when the new path is stable for at least one release cycle.
- Do NOT execute Phase C in this cycle or the next.

**Exit:** Misowned tests consolidated or deleted with parity evidence. No redundant test surfaces remain.

## Cross-references

- `docs/adr/ADR-0069-test-tooling-ownership.md` (accepted) — ownership policy; one source of truth for the four-cell declaration.
- `docs/adr/ADR-0022-sddk-testkit.md` (proposed) — testkit proposal; pending supersession or acceptance in Phase B.
- `docs/adr/ADR-0060-prompt-layer-evidence-contracts.md` (accepted) — prompt-layer evidence contracts; related to CLI contract testing.
- `docs/adr/ADR-0068-bounded-execution.md` (cycle-44 foundation) — bounded execution; cycle-46 lockstep owned by Rust.
- `docs/sddk-decision-kernel-architecture/03-adrs/ADR-035-EVALUATION-FEEDBACK.md` (Accepted) — evaluation feedback; relevant to test-tooling evaluation.
- `docs/sddk-decision-kernel-architecture/03-adrs/ADR-039-ADAPTIVE-VERIFICATION.md` (Proposed) — adaptive verification; relevant to test-strategy evolution.
- `docs/sddk-decision-kernel-architecture/03-adrs/ADR-066-MAP-OPERATOR-ARC-BODY.md` (Accepted) — map operator arc body; note: do NOT renumber.
- `docs/sddk-decision-kernel-architecture/09-implementation/TEST-TOOLING-EVIDENCE-AUDIT.md` — verified evidence, inventory, and audit; input to this migration plan.
