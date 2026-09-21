# SCOPE-CONTRACT — AIW-S7c — AuthorityContext negative-grant (G05+G07)

> **Slice id:** `p-63676b11dc0ef88f/aiw-s7c-authority-negative-grant`
> **Status:** ✅ CLOSED (tests-only, no src/ changes)
> **Parent STOP:** `tests/cycle-artifacts/p-63676b11dc0ef88f/aiw-s7-secretary-attention/SCOPE-CONTRACT.md` §S7-STOP-3

## §1 Goal

Unblock and close AIW-S7 rows **G05** and **G07** by adding integration
tests that exercise the existing `AuthorityContext` + Secretary L1 /
closed-set paths with negative-grant scenarios. This is the third
operator decision from S7 §4 ("Add AuthorityContext negative-grant
integration tests").

## §2 Constraints

| # | Constraint | |
|---|---|---|
| C1 | Tests-only: NO new `src/` module in any crate | enforced |
| C2 | Use real APIs: `authority`, `secretary_closed_set`, `secretary_l1` | enforced |
| C3 | No new dependency edge (`sddk-domain` already a dep of `sddk-engine`) | enforced |
| C4 | Deliverables: 1 test file + 3 cycle artifacts | enforced |
| C5 | One commit, no push | enforced |
| C6 | Scoped verification batch only | enforced |
| C7 | Non-passing draft tests are REMOVED, not skipped, with reason in RECEIPT §6 | enforced (0 removed) |

## §3 Deliverables

| Path | Δ |
|---|---|
| `crates/sddk-engine/tests/aiw_s7c_authority_negative_grant.rs` | new — 13 integration tests |
| `tests/cycle-artifacts/.../aiw-s7c-authority-negative-grant/SCOPE-CONTRACT.md` | new (this file) |
| `tests/cycle-artifacts/.../aiw-s7c-authority-negative-grant/UAT-EVIDENCE.yaml` | new |
| `tests/cycle-artifacts/.../aiw-s7c-authority-negative-grant/RECEIPT.md` | new |

## §4 S7-STOP-3 resolution (G05+G07)

- **G05 (SEC): Secretary intenta release/gate/lease/receipt fuera
  closed-set** — closed by exercising
  `validate_secretary_event` with a role-bound Secretary `ActorRef`
  against all four `SECRETARY_PROHIBITED_PREFIXES` (`release.*`,
  `gate.*`, `lease.*`, `receipt.*`), asserting
  `SecretaryClosedSetError::ProhibitedEventType`; plus the
  `EscalationRequired` path for a state-mutating type outside the
  closed set and the admissibility of `escalation.requested`.
- **G07 (NEG): autoridad no otorgada** — closed by exercising
  `AuthorityContext::validate` against the real
  `WRITABLE_SURFACE_MATRIX`: Human rejected on `framework_bundle`,
  `github_releases`, `dependency_edge`; Agent rejected on
  `decision_record`; System rejected on `knowledge_graph_vault`; plus
  a positive CLI control on `evidence_attachment` and a
  per-surface non-spill assertion. Tier-gate L1 negatives
  (High tier without evidence, unknown template) close the
  Secretary-side negative-grant surface.

No new engine API was required; the existing authority surface was
sufficient.

## §5 Verification batch (scoped per change-scoped-testing)

SUT: `crates/sddk-engine` (tests-only delta).

```bash
cargo fmt --check
cargo clippy -p sddk-engine --all-targets -- -D warnings
cargo test -p sddk-engine --test aiw_s7c_authority_negative_grant
cargo build --release -p sddk-engine
```

## §6 References

- Parent: `tests/cycle-artifacts/p-63676b11dc0ef88f/aiw-s7-secretary-attention/SCOPE-CONTRACT.md` §S7-STOP-3.
- `crates/sddk-engine/src/authority.rs` (ADR-069/070/072 matrix).
- `crates/sddk-engine/src/secretary_closed_set.rs` (ADR-0073 + AMENDMENT-1).
- `crates/sddk-engine/src/secretary_l1.rs` (ADR-090 closed-set proposals).
