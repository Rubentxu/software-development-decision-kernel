# RECEIPT — AIW-S7c — AuthorityContext negative-grant (G05+G07)

> **Slice:** `p-63676b11dc0ef88f/aiw-s7c-authority-negative-grant`
> **Status:** CLOSED-LOCALLY (no push per slice contract)
> **Scope contract:** `tests/cycle-artifacts/p-63676b11dc0ef88f/aiw-s7-secretary-attention/SCOPE-CONTRACT.md` §S7-STOP-3
> **UAT evidence:** `UAT-EVIDENCE.yaml` (same dir)

## §1 What was delivered

Tests-only slice (no `src/` changes): 13 integration tests in
`crates/sddk-engine/tests/aiw_s7c_authority_negative_grant.rs`
exercising the REAL authority + Secretary APIs with negative-grant
scenarios. Resolves S7-STOP-3 and closes AIW-S7 rows G05 and G07.

### Surface changes

| Path | Δ | Description |
|---|---|---|
| `crates/sddk-engine/tests/aiw_s7c_authority_negative_grant.rs` | new | 13 integration tests: 7 AuthorityContext matrix negatives + 1 positive control, 3 closed-set G05 tests, 3 L1 tier/unknown-template negatives. |
| `tests/cycle-artifacts/.../aiw-s7c-authority-negative-grant/{SCOPE-CONTRACT.md,UAT-EVIDENCE.yaml,RECEIPT.md}` | new | Slice cycle artifacts. |

## §2 Acceptance vs scope

| Constraint | Compliance | Evidence |
|---|---|---|
| C1: tests-only, no `src/` changes | YES | `git status` shows only the test file + artifacts |
| C2: real engine APIs | YES | `AuthorityContext::{for_test,for_cli,validate}`, `WRITABLE_SURFACE_MATRIX` (implicitly), `validate_secretary_event`, `is_secretary`, `SECRETARY_PROHIBITED_PREFIXES`, `SecretaryL1Engine::{register_template,propose}` |
| C3: no new dependency edge | YES | `sddk-domain` was already a dep of `sddk-engine` |
| C4: 1 test file + 3 artifacts | YES | see §1 |
| C5: one commit, no push | YES | see §6 |
| C6: scoped verification batch | YES | see §3 |
| C7: removed-vs-skipped discipline | YES | 0 tests removed; all 13 pass against the real API |

### UAT coverage

| UAT id | Status | Why |
|---|---|---|
| G05 | PASS | Secretary bound via `role="secretary"` + `kind=Agent` rejected with `ProhibitedEventType` on `release.complete`, `gate.passed`, `lease.released`, `receipt.replay`; `EscalationRequired` for `cycle.transitioned`; `escalation.requested` admissible; Human-with-role not bound. |
| G07 | PASS | Human rejected on `framework_bundle`/`github_releases`/`dependency_edge`; Agent rejected on `decision_record`; System rejected on `knowledge_graph_vault`; positive CLI control on `evidence_attachment`; per-surface non-spill; L1 High-tier-without-evidence and unknown-template rejected. |

## §3 Real verification output (cargo, 2026-09-21)

```
$ cargo fmt --check                      # exit 0 (after one rustfmt pass)

$ cargo clippy -p sddk-engine --all-targets -- -D warnings
    Checking sddk-engine v1.169.122
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 20s

$ cargo test -p sddk-engine --test aiw_s7c_authority_negative_grant
running 13 tests
test agent_rejected_on_decision_record_surface ... ok
test cli_admitted_on_evidence_attachment_surface ... ok
test human_rejected_on_framework_bundle_surface ... ok
test human_rejected_on_dependency_edge_surface ... ok
test negative_grant_does_not_spill_to_admitted_surface ... ok
test human_rejected_on_github_releases_surface ... ok
test secretary_bound_to_role_only_for_agent_kind ... ok
test secretary_l1_unknown_template_rejected ... ok
test secretary_prohibited_on_all_four_exclusive_prefixes ... ok
test secretary_proposal_kind_matches_template ... ok
test secretary_state_mutation_outside_closed_set_requires_escalation ... ok
test system_rejected_on_knowledge_graph_vault_surface ... ok
test secretary_l1_proposal_with_high_tier_requires_evidence ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo build --release -p sddk-engine
    Finished `release` profile [optimized] target(s) in 34.67s
```

**Counts:** 13 new integration tests, all passing. Release build clean.

## §4 Deviations from the sketch

Three shape adjustments, all forced by the real API (read before
writing, per slice instructions):

1. `RiskTier` is `#[non_exhaustive]` and not re-exported in a way the
   sketch assumed; imported from `sddk_engine::risk_approval_policy::RiskTier`.
2. `ActorRef` is private in `sddk-engine` root; imported from its real
   path `sddk_domain::event_envelope::ActorRef`.
3. `ProposalTemplate::new` takes 5 args (includes `summary`), and
   `AuthorityContext` has no `ActorKind::Guest`/`Cli` variants — the
   sketch's `Guest` test was replaced by real matrix negatives
   (`Human` on `framework_bundle`, etc.) using actual `ActorKind`
   variants (`Human`, `Agent`, `System`). The "unknown surface" draft
   test was dropped: all 11 `WritableSurface` variants are admitted in
   the matrix (the unknown-surface branch is dead code), so the
   non-spill assertion covers the intent instead.

## §5 Out of scope (not closed by this slice)

G01/G03 were closed by AIW-S7b; G04/G06 (producer→L0 stream wiring)
remain open under S7-STOP-1. No manifest edits, no release, no push.

## §6 Commit

One commit on top of the slice base:
`test(engine): AIW-S7c AuthorityContext negative-grant integration tests (G05+G07)`.
Not pushed; release script not invoked (requires operator
authorization per the AIW adoption workflow).
