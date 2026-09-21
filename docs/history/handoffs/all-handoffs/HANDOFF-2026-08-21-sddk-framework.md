# HANDOFF — sddk-framework — 2026-08-21

> **Cycle:** `kernel-cycle-7b-durable-debt-runtime` (kernel)
> **Released as:** v1.36.0
> **HEAD:** `0c256e0` (ADR-0047 runtime surface; JSON Schema + INC template + gates)
> **Tag:** v1.36.0

## Done (cycle-7b, 2026-08-21)

**kernel-cycle-7b-durable-debt-runtime** — Runtime contract surface for ADR-0047 shipped (JSON Schema + INC template + agent vocabulary + workflow gates + prompt updates). Gate wiring deferred to cycle-8+. Released as v1.36.0.

Archive: `~/.sddk-knowledge/sddk-framework/archive/2026-08-21-kernel-cycle-7b-durable-debt-runtime/`

| Metric | Value |
|--------|-------|
| LOC | 119 (≤150 budget) |
| Commits | 10 (9 + 1 fixup) |
| Verdict | PASS_WITH_WARNINGS (41 ACs; 1 deferred to cycle-8+) |
| Debt verdict | PASS_WITH_WARNINGS (8 findings; 4 medium-P2, 4 low-P3) |
| Rust changes | 0 |
| Tests | 1045 pass / 0 fail |

**Artifacts:**
- docs/debt/debt-report.schema.json (NEW)
- docs/debt/INCIDENCE-TEMPLATE.md (NEW)
- docs/debt/README.md (touch-up)
- agents/sddk-debt-verify.md (Vocabulary)
- agents/sddk-archive.md (INC generation)
- workflow/workflow.yaml (2 gate definitions)
- prompts/sddk/orchestrator.md (Debt lifecycle)
- prompts/sddk/phase-contracts.md (file contracts)
- prompts/sddk/arsenal.md (gate references)

**Warnings:**
- W1: AC-K7-008-2 (gate wiring) deferred to cycle-8+
- W2: Gate evaluator runtime (cycle-8+)
- W3: INC file generator runtime (cycle-8+)
- W4: Fingerprint generator runtime (cycle-8+)

## Cycle-7a completed (v1.35.0)

**kernel-cycle-7-durable-debt-spec** — ADR-0047 ratified as Accepted. Severity + Priority taxonomies in `docs/debt/`. AGENTS.md §4 references framework. Cycle-7b deferred.

Archive: `~/.sddk-knowledge/sddk-framework/archive/2026-08-21-kernel-cycle-7-durable-debt-spec/`

| Metric | Value |
|--------|-------|
| LOC | 81 (≤150 budget) |
| Commits | 4 |
| Verdict | PASS_WITH_WARNINGS (11/11 ACs; 1 false-positive grep) |
| Debt verdict | PASS_WITH_WARNINGS (0 CRIT, 4 WARN for cycle-7b) |

## Cycle-6 completed (v1.34.0)

**kernel-cycle-6-rfc3339-consolidation** — RFC 3339 wrapper deleted; 16 use-sites migrated to Stack A (`time` crate). Released as v1.34.0 (tag `0b062447`).

Archive: `~/.sddk-knowledge/sddk-framework/archive/2026-08-21-kernel-cycle-6-rfc3339-consolidation/`

| Metric | Value |
|--------|-------|
| LOC | 101 (≤150 budget) |
| Commits | 8 |
| Verdict | PASS |
| Debt verdict | PASS (0 CRIT, 1 WARN) |

## Current state (cargo test / clippy)

```
cargo test --workspace  ✓ green (all crates)
cargo clippy --workspace ✓ 0 errors (pre-existing event_envelope_golden.rs::unused_mut warning is lint-level, not blocking)
```

## What changed (10 commits)

1. `feat(docs): add debt-report.schema.json (REQ-K7-004)` — +45 LOC
2. `feat(docs): add INCIDENCE-TEMPLATE.md (REQ-K7-005)` — +35 LOC
3. `docs(debt): touch-up README markers (REQ-K7-004/005)` — +2/-2 LOC
4. `docs(agents): add Vocabulary section (REQ-K7-006)` — +8 LOC
5. `docs(agents): add INC generation section (REQ-K7-007)` — +8 LOC
6. `feat(workflow): add 2 gate definitions (REQ-K7-008)` — +18 LOC
7. `feat(prompts): add Debt lifecycle to orchestrator (REQ-K7-009)` — +8 LOC
8. `feat(prompts): add 2 file contracts to phase-contracts (REQ-K7-009)` — +10 LOC
9. `feat(prompts): add gate refs to arsenal (REQ-K7-009)` — +4 LOC
10. `fix(workflow): desacoplar gates debt de requires hasta cycle-8+` — -12 LOC

## Done (cycle-7c, 2026-08-21)

**kernel-cycle-7c-cli-gate-receipt-fix** (docs-only correction)
**Goal**: Investigate the supposed "CLI gate receipt persistence bug" (HIGH, since cycle-3).
**Outcome**: Premise rejected. The real ledger lives at `~/.local/state/sddk/...` (XDG state_home per ADR-0006) with 18 tables, 510 receipts, 536 events. The stub 0-byte at `data_home` is an orphan NOT on the engine read path.
**Root cause**: STORAGE_NOT_FOUND errors were CLI arg format mistakes (`--gate-receipt "gate_name=receipt_id"` instead of `"receipt_id"`), not a Rust bug.
**Lessons**: When a "pre-existing bug" persists without manifesting, verify the premise first. XDG path confusion (data_home vs state_home) explained all historical symptoms.
**Archive**: `~/.sddk-knowledge/sddk-framework/archive/2026-08-21-kernel-cycle-7c-cli-gate-receipt-fix/`
**Artifacts**: explore-report.md (305 lines) + spec.md (4 REQs)
**Next**: cycle-8 (gate evaluator runtime + INC generator + fingerprint generator)

## Cycle-8 (kernel-cycle-8-debt-runtime-implementation) — DONE

- Released: **v1.35.0** (`1ed973d`), GitHub Release published.
- 14 commits (a8f3f21..1ed973d), 1,066 tests pass, 0 fail.
- 60/65 ACs PASS, 3 PASS_WITH_NOTE (LOC budget exception, forward remediation, etc.), 1 DEFERRED, 0 FAIL.
- 10 debt findings (6 medium, 4 low); 1 forward entry for cycle-9.
- Recovery loops: 6 of 14 commits were recovery/lint/manifest regen. Lesson: apply phase should run `-D warnings` clippy itself.
- Forward: DEBT-CYCLE-8-LOC-OVERAGE → cycle-9 hardening.
- Bundle: ~/.local/share/sddk/framework/1.35.0/
- Archive: ~/.sddk-knowledge/sddk-framework/archive/2026-08-21-kernel-cycle-8-debt-runtime-implementation/

## Next candidates

| # | Candidate | Status | Notes |
|---|-----------|--------|-------|
| 1 | cycle-8: Gate evaluator runtime + INC generator + fingerprint generator | P2 | ADR-0047 §3 implementation. ~400 LOC Rust + schema. |
| 2 | cycle-8+: INC backfill for cycle-3..7a + schema migration tooling | P3 | ADR-0047 §Compatibility. ~150 LOC data. |

**Deferred from cycle-7b (W1-W4):** Gate wiring, gate evaluator runtime, INC generator runtime, fingerprint generator runtime — all cycle-8+.

## Recovery cheat sheet

```bash
# Verify LOC targets
wc -l AGENTS.md          # expect ≤100
wc -l docs/RELEASING.md  # expect ≥50

# Check zero now_rfc3339 orphan references
rg "crate::uat_common::time::now_rfc3339" crates/  # expect 0

# Verify variant guards (negative test)
# Edit Phase count 10→11 in cycle.rs → cargo check must fail

# Rollback this cycle
git revert <merge-sha> && git tag -d v1.36.0
```

## Anchors (apply-phase verified)

- D-1: Phase=10, CycleStatus=10 (not "likely 7" from user spec)
- D-2: 13 call sites in 9 files (not "~15 in 7")
- D-3: `sddk-domain` already a `sddk-cli` dep — no Cargo.toml edit for WU-K5-1
- D-5: PM-3 clippy fixes applied; pre-existing `event_envelope_golden.rs::unused_mut` not modified (not my responsibility)
