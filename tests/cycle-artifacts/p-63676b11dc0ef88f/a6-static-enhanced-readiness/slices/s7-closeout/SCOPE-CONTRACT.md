# SCOPE-CONTRACT — S7 — Macro-cycle closeout integrated report

> **Slice id:** `p-63676b11dc0ef88f/a6-static-enhanced-readiness/slices/s7-closeout`
> **Macro-cycle:** `p-63676b11dc0ef88f/a6-static-enhanced-readiness`
> **Status:** closing — docs-only.

## §1 Goal

Produce the integrated closeout report for the A6 macro-cycle,
capturing the state of every slice (S1..S6), the macro-cycle exit
criterion (AC10 + `PR-UAT-024`), the operational gaps (no
`COGNICODE_MCP_BIN`, no schema-evolution contract), and the
follow-up proposals that have surfaced during the cycle.

## §2 Deliverables

| Deliverable | Path | Status |
|---|---|---|
| Closeout report (this file) | `tests/cycle-artifacts/.../slices/s7-closeout/RECEIPT.md` | ✅ |
| Per-slice map | (embedded in this file) | ✅ |

## §3 Slice inventory

| Slice | Title | State | Local commit | Pushed? | UAT coverage |
|---|---|---|---|---|---|
| S1 | UAT coverage via fake (deterministic subset of A6) | ✅ CLOSED | `897738a` | ✅ pushed at `9688ebb` (v1.169.95) | `PR-UAT-C01..C04, C06, C08, C10` PASS |
| S2 | UAT C05/C07/C09 (subset determinista) | ✅ CLOSED | `32ac759` | local-only (per operator rule) | `PR-UAT-C05, C07, C09` PASS |
| S3 | AC10 Verify integration (bridge) | ✅ CLOSED | `c917393` | local-only | AC10 + `PR-UAT-C04` + `PR-UAT-019` PASS |
| S5 | Real CogniCode EXT | ⚠️ NOT_EVALUATED | `e4fdc35` (docs in same commit as S6/S4) | local-only | 11 rows `NOT_EVALUATED` (no `COGNICODE_MCP_BIN`) |
| S6 | Fake relocation to `dev-deps` + `test-support` feature | ⚠️ NOT_PROCEED | `e4fdc35` | local-only | n/a (audit-only) |
| S4 | Durability (PR-UAT-024, ObservationSet crash/reopen) | ⏸ STOP | `e4fdc35` | local-only | n/a (pre-implementation STOP) |
| S7 | Closeout (this slice) | ✅ CLOSED | `<this commit>` | local-only | n/a |

## §4 Macro-cycle exit criterion

Per macro-cycle SCOPE §S7, the exit criterion is:
**AC10 (Static Enhanced Coverage) verified end-to-end + PR-UAT-024 (durability) closed.**

| Exit criterion | Status | Reason |
|---|---|---|
| AC10 (Static Enhanced Coverage) verified end-to-end | ✅ | S3 closes the AC10 seam: `AnalysisResult` → `ObservationSet` bridge is in `verify_kernel/evidence_source_static_provider.rs`. `StaticProviderDomain` consumes the bridged substrate. 5/5 integration + 3/3 unit + 1/1 regression pin. |
| PR-UAT-024 (durability) closed | ⏸ | S4 STOP — operator must choose between Option A (JSON shape contract with `schema_version`), Option B (typed event class), or Option C (defer to downstream cycle). Until chosen, AC10's claim of "static evidence is consumed by Verify" is **not yet durable across crash/reopen**. |

**Honest read**: AC10 closes the in-process contract. PR-UAT-024 is
the contract that makes AC10's claim trustworthy across restarts.
Until the operator picks A/B/C, the macro-cycle is **partially
closed** — S1/S2/S3 are individually green but the production-ready
exit criterion is not met.

## §5 Operational gaps recorded during this cycle

1. **`COGNICODE_MCP_BIN` not available in this environment.**
   Affects: S5 live EXT, AC10 EXT profile rows.
   Mitigation: live test is `#[ignore]`-d, de-ignore path is zero-cost
   when the binary becomes available.
   Owner: environment (CI matrix).

2. **`code_intelligence_port_fake` is `pub mod`** (not gated).
   Affects: release binary surface (currently stripped, but
   documented surface remains public).
   Mitigation: see S6 RECEIPT §2.4 for three options.
   Owner: operator decision.

3. **`LedgerEvent.payload` has no schema-evolution contract.**
   Affects: PR-UAT-024 (durability of any persisted ObservationSet).
   Mitigation: see S4 SCOPE §4 for three options.
   Owner: operator decision.

4. **2 commits ahead of `origin/main`, no publishable bump.**
   Affects: cadence of release.
   Mitigation: `scripts/release.sh` is the canonical publish path
   per the operator's no-auto-bumps rule.
   Owner: release flow on operator signal.

## §6 Follow-up proposals surfaced

| Proposal | Source | Scope | Recommendation |
|---|---|---|---|
| API-hygiene gating for `code_intelligence_port_fake` | S6 audit | Public surface | Defer until a downstream crate legitimately consumes it; the release-binary motivation is moot. |
| JSON shape contract for ledger `payload` (`schema_version`) | S4 STOP report | Durability | Adopt Option A from S4 §4.2 (lightest path to PR-UAT-024). |
| De-ignore `t_ar_6_ext_real_cognicode_run` in CI when `COGNICODE_MCP_BIN` is present | S5 | CI matrix | One-line change in `a6_cc_s1_static_graph_completeness.rs`; safe to do now (test is correctly gated). |
| Optional: more `canonical_tag` regression pins across `ObservationOrigin::*` | t2 of session | Hygiene | Already done (5 namespaces + cross-namespace collision). |

## §7 Honest limits of this closeout

1. **S4 was paused, not closed.** The macro-cycle cannot honestly
   report PR-UAT-024 as satisfied until the operator chooses A/B/C
   and the chosen option is implemented in a follow-up slice.
2. **S5 is environment-bound.** If `COGNICODE_MCP_BIN` becomes
   available, S5 can be re-validated to PASS with **zero code
   change**; the de-ignore path is documented in S5 RECEIPT §4.2.
3. **S6 was de-scoped, not closed.** The audit is durable; the
   decision is pending. If API hygiene is later prioritized, an
   amended SCOPE targeting `pub(crate)` is the cheapest path
   (no build-matrix impact).
4. **The 3 local commits ahead of `origin/main` (S3, S4+S5+S6
   docs, S7 closeout) are not bumped.** They will travel together
   on the next canonical release. No commit subject hints at a
   release; no version file is touched.

## §8 References

- Macro-cycle plan: `tests/cycle-artifacts/p-63676b11dc0ef88f/a6-static-enhanced-readiness/SCOPE-CONTRACT.md`.
- Per-slice SCOPE/RECEIPT/UAT-EVIDENCE: `tests/cycle-artifacts/.../slices/s{N}/`.
- Operator feedback on no-auto-bumps: handoff history in
  `docs/history/handoffs/all-handoffs/HANDOFF-2026-09-15-session-close.md` (operator rules).
- Pre-push hook: `githooks/pre-push` (release-flow contract).
