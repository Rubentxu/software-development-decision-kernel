# AIW ↔ PR-GAP crosswalk

> **Purpose:** map each `PR-GAP-*` row in
> `docs/SDDK-Production-Readiness-Alignment-2026-09-14/01-GAP-AND-DRIFT-REGISTER.md`
> to the AIW slices (`AIW-S*`) and macro-cycle A6 slices (`A6-S*`) that
> already exercise the underlying contract. This is a **claim index**,
> not an authority change to the gap register — every entry must be
> ratified by the operator before the gap register row is touched.
>
> **Status of this file:** non-authoritative. The operator (or a future
> cycle) ratifies claims before the gap register row is moved.

## Crosswalk table

Legend:

- **Covers** — AIW/A6 slice demonstrably exercises the gap row's requirement, with evidence in `tests/cycle-artifacts/`.
- **Partial** — only a subset of the requirement is exercised; remainder remains open.
- **Compat** — production path is correct; a compatibility path remains with documented removal trigger.
- **Open** — no AIW/A6 work currently addresses the gap.
- **Outside AIW scope** — by-design (e.g. Section A is closed by A0/A1/A2 work, not AIW).

| PR-GAP | Title | AIW/A6 slice(s) | Status | Evidence pointer |
|---|---|---|---|---|
| PR-GAP-012 | Physical bounded-context cut (R0) | A0 work (slice log) | **Outside AIW scope** | `docs/SDDK-Production-Readiness-Alignment-2026-09-14/12-A0-IMPLEMENTATION-INVENTORY.md` slice log |
| PR-GAP-013 | Knowledge model / KnowledgeBasis / KMT | A3/A4 (slice log) | **Outside AIW scope** | Same |
| PR-GAP-014 | Agent advisory/instruction split | A3/A4 (slice log) | **Outside AIW scope** | Same |
| PR-GAP-015 | Software Alignment core | A4 (alignment lens kernel) | **Outside AIW scope** | `crates/sddk-engine/tests/a4_4b_alignment_lens_kernel.rs` |
| PR-GAP-016 | Verify delta synchronization | A4/A6-S3 | **Outside AIW scope** | `tests/cycle-artifacts/.../a6-static-enhanced-readiness/slices/s3-ac10-verify-integration/` |
| PR-GAP-017 | DebVerify reconciliation | A4/A6 | **Outside AIW scope** | Same |
| PR-GAP-018 | Completion vs intelligence providers | A6 / S1 | **Outside AIW scope** | `tests/cycle-artifacts/.../a6-static-enhanced-readiness/slices/s1-uat-coverage-fake/` |
| PR-GAP-019 | Provider absence semantics | A6-S1 + AIW-S1 base green | **Covers (compat candidate)** | `crates/sddk-engine/tests/a6_s1_uat_coverage_fake.rs::t_uat_c01_optional_provider_unavailable_keeps_base` + `t_uat_c02_required_provider_unavailable_yields_typed_failure`. AIW-S1 RECEIPT §"Base (sin proveedor): A03 negativo + A06 digest → 2 passed". |
| PR-GAP-020 | CogniCode static intelligence integration (R7) | AIW-S1 | **Covers** | `tests/cycle-artifacts/.../aiw-s1-cognicode-real/RECEIPT.md`. Gap register says `NOT_STARTED`; AIW-S1 placed `STATIC_ENHANCED` candidate profile. **Operator decision pending**: ratify claim and move row to `PASS` / `PASS_WITH_COMPAT`. |
| PR-GAP-021 | Chronos runtime intelligence integration (R8) | AIW-S5 | **Covers (compat candidate)** | `tests/cycle-artifacts/.../aiw-s5-chronos-runtime/RECEIPT.md`. Vertical E2E with `chronos-mcp v0.1.0`. AIW-S5 is **vertical**, not full A7 closure. Gap register says `NOT_STARTED`; AIW-S5 produced first acceptable real run. **Operator decision pending**: full A7 multi-program coverage remains an open slice. |
| PR-GAP-022 | Cross-provider reconciliation | AIW-S6 + A4-5a | **Covers (compat candidate)** | `crates/sddk-engine/tests/a4_5a_intelligence_loop_composition.rs::contradictory_composition_preserved` + AIW-S6 RECEIPT. Composition covers P09..P12 (correlate, contradictions preserved, no provider required, deterministic id). **Compat residual**: full E2E with two real providers simultaneously deferred to "Fully enhanced" profile. **Operator decision pending**. |
| PR-GAP-023 | Provider reproducibility basis | AIW-S1 + AIW-S5 + A6-S5 | **Covers (compat candidate)** | AIW-S1 RECEIPT documents `protocol_version`, `capability_snapshot`, `observation_set_canonical_digest`, `basis_hash_hex` for CogniCode. AIW-S5 documents the same for Chronos. **Compat residual**: every enhanced receipt must carry the full reproducibility basis (not yet enforced as a schema requirement on all receipts). **Operator decision pending**. |
| PR-GAP-024 | Workbooks/control tower (R9) | A8 territory | **Open** | A8 P2; counterfactual/proof-carrying novelty must not delay AC1..AC8. |
| PR-GAP-025 | Governance ratchets + enriched WHY (R10) | (partial) | **Partial** | A0/PASS evidence shows Authority/WHY gates are green. New ratchets not yet implemented. |
| PR-GAP-026 | Crate split evaluation (R11) | — | **Open by design** | R11 DEFERRED. Public Agentic API crates are separate external release-boundary justification. |
| PR-GAP-027 | Agentic Workspace / JCode | J0→J9 | **Open (STOP)** | All J-milestones blocked by R11 (public SDK boundary justification) and external JCode SDK pinning. AIW-S7/S8 STOP reports cover parts of this territory (X02 denial surface, X04 concurrency, J5 reactive verify). |

## How to use this file

1. Identify a PR-GAP row with status `Covers` or `Covers (compat candidate)`.
2. Open the evidence pointer; verify the claim is honest.
3. File an operator-decision ticket (or ask in session) to ratify the claim.
4. Operator moves the gap register row to `PASS` / `PASS_WITH_COMPAT` with:
   - owner (who maintains the evidence),
   - fixture (which test holds the invariant),
   - removal trigger (when `PASS_WITH_COMPAT` becomes `PASS` or is removed).

This file does **not** itself move any gap register row. It is a
research aid.

## Why this file exists

The AIW roadmap (`docs/proposals/2026-09-19-adaptive-inputs-workflows/`)
and the production-readiness gap register
(`docs/SDDK-Production-Readiness-Alignment-2026-09-14/01-GAP-AND-DRIFT-REGISTER.md`)
were authored independently and use different scope vocabularies.
The AIW slices close most of section C (`PR-GAP-019..023`) without
updating the gap register. This crosswalk closes the documentation gap
without changing authority.
