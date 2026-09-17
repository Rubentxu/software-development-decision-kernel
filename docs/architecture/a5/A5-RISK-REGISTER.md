# A5 — Risk Register

> Cycle: `p-63676b11dc0ef88f-a5-plan-base-production-ready`
> Status: **A5-PLAN deliverable (planning only)**
> Severity vocabulary: `docs/architecture/a5/A5-PRODUCTION-READINESS-CONTRACT.md` §20.

The register is the *risk* view; `A5-DEBT-DISPOSITION.md` is the *debt*
view. A risk without an owner/workstream is a planning defect.

| ID | Risk | Sev | Detected by (gate) | Workstream | Status |
|---|---|---|---|---|---|
| R1 | Crash between canonical append and projection rebuild loses or diverges state | **P0** | G2, G3 | A5-2 | open |
| R2 | Corrupt / missing CAS object silently accepted | **P0** | G2 | A5-2 | open |
| R3 | Two writers overwrite (`last writer wins`) | **P0** | G4 | A5-3 | open |
| R4 | Authority decision → effect TOCTOU window (side effect after stale decision) | **P0** | G5 | A5-3 | open |
| R5 | Denied action still produces a side effect / `RequireApproval` bypass | **P0** | G5 | A5-3 | open |
| R6 | Retry double-applies an effect | **P1** | G5 | A5-3 | open |
| R7 | Release tag ≠ certified SHA; version drift | **P1** | G7, G16 | A5-1 | open |
| R8 | Corrupt / partial / stale public asset; CDN serves previous binary | **P1** | G8 | A5-1 | open |
| R9 | Installed binary depends on repo checkout / only works via `cargo run` | **P1** | G9, G12 | A5-5 | open |
| R10 | Push/release protocol requires a ceremonial empty `chore(release)` marker | **P2** | release audit | A5-1 | open (INC-A5-PUSH-RELEASE-MARKER-FRICTION) |
| R11 | Flaky test hidden by `couldn't reproduce → closed` | **P1** | G13 | A5-5 | open |
| R12 | Non-blocking Parallel path sender-drop bug (documented in two ignored tests) | **P1** | G4, G13 | A5-3 | open |
| R13 | Dead compatibility code (`paradigm_lens::evaluate_lens`) kept forever | **P2** | G14 | A5-4 | open |
| R14 | Secret leaks into log / receipt / error / telemetry / CAS | **P0** | G11 | A5-5 | open |
| R15 | Rollback to previous certified release fails | **P1** | G15 | A5-1 / A5-5 | open |
| R16 | An A5 change silently breaks an A4 certified contract | **P0** | G1 | every cycle | open |
| R17 | Operator cannot diagnose a production failure | **P2** | G10 | A5-4 | open |
| R18 | Ignored tests become invisible debt | **P2** | G13 | A5-5 | open |
| R19 | Deprecated-pattern lints stay `allow` with no disposition | **P3** | G14 | A5-4 | open |
| R20 | Migration of persisted state breaks an upgrade | **P1** | G6 | A5-2 | open |

## Notes on high-severity risks

- **R4 (TOCTOU):** `A5-PRODUCTION-READINESS-CONTRACT` §G5 requires that the
  decision→effect atomicity boundary be **named and tested**. If no safe
  boundary exists, that is a `BASE_PRODUCTION_READY` blocker.
- **R12 (Parallel sender-drop):** two ignored tests in
  `crates/sddk-engine/tests/parallel_spec_scenarios.rs` document a
  "sender-drop bug that prevents `record_node_run_for_run` from being
  called for child nodes". This is a **runtime** signal, not just test
  debt; A5-3 must reproduce and classify it. If real, it is P0/P1.
- **R14 (secrets):** the CAS stores payloads, receipts and handoffs store
  metadata, and telemetry may carry provider context. A leak path is P0.

## Register discipline

- Every risk maps to exactly one primary workstream.
- A risk is closed only with evidence (a receipt in the A5 body of
  evidence), never by assertion.
- New risks discovered during A5 cycles are added here, not silently
  absorbed.
