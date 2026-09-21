# A0 — PR-GAP-006 AuthorityEngine cutover investigation

**Audit basis:** `main@e43f929`. **Status:** RESOLVED via option **B+** (`ADR-0111-EXPLICIT-APPROVAL-POLICY.md`). The engine now derives approval only from explicit policy; `ENFORCEMENT_STAGE = All`. The naive-flip evidence below is retained as the rationale.

## Requirement (gap register)

> Enumerate governed effects; route all through one AuthorityEngine decision path; complete the `LowMedium → All` rollout or replace that staging mechanism with an equally explicit cutover; prove zero bypass.

## Current code reality

- `crates/sddk-cli/src/admission.rs`: `ENFORCEMENT_STAGE = EnforcementStage::LowMedium`. `EnforcementStage::All` exists but is unused.
- `enforce_gated` is the single choke point. With `LowMedium`, `RequireApproval` on a High-band surface yields `AdvisoryHighApproval` (the governed effect **proceeds**, decision still recorded). With `All`, it yields `AwaitingApproval` and `enforce_admission_or_block_ctx` returns `Err("ADMISSION: approval required …")`.
- The engine (`authority_engine.rs:541`) emits `RequireApproval` when `policy.approval_required_for` contains the action **or** `policy.risk_band == High`.
- The runner uses `bridge::default_policy_for_surface` (`runner.rs:71`), which assigns `RiskBand::High` to the surfaces: `cycle_state`, `gate_receipts`, `plan_revisions`, `transition_records`, `framework_bundle`, `github_releases`.
- Therefore, at `All`, **every governed action on those six High surfaces blocks pending human approval** — including ordinary cycle transitions, gate receipts and releases.
- Approval loop is live: `sddk approval list|grant|deny`; `cli_approval_loop_e2e` green.

## Measured impact of a naive flip to `All`

Experiment: set `ENFORCEMENT_STAGE = All`, rebuild, run `cargo test -p sddk-cli`.

Result: **16 integration tests in `crates/sddk-cli/tests/cli.rs` fail** (core workflows), plus the 3 M1-advisory unit tests that assert the old behavior. Representative failures:

- `cli_cycle_evaluate_gate_explicit_failed_persists_failed_receipt` → `ADMISSION: approval required before mutating 'gate_receipts' (decision_id=approval-system-cli_run); no changes were made`.
- `cli_release_apply_local_*`, `cli_release_complete_success_path_unchanged`, `cli_release_recover_*`,
- `cli_phase_build_remediate_*`, `cli_no_arbitrary_release_pending_to_build_jump`,
- `cli_full_runtime_pipeline_dogfood`, `cli_walks_cycle_with_fencing_and_rebuilds_state`, `cli_closing_cycle_auto_captures_metrics_record`.

Cause: gate receipts / cycle transitions run on High surfaces, so `All` turns routine verify/release/phase operations into approval-gated operations.

## Conclusion

Flipping `ENFORCEMENT_STAGE` to `All` is **not** a drop-in completion: with the current coarse `default_policy_for_surface` mapping, it converts ordinary cycle/release/gate workflows into blocking-approval workflows. This is a product/authority decision (how much human oversight to require in production), not a mechanical rollout. It is therefore escalated rather than applied unilaterally.

## Bounded options

- **A — Complete the rollout as-is (`All` + coarse policy).** Flip, update the 3 advisory unit tests, and update the 16 workflow tests to grant approvals or assert blocking. Highest literal compliance; every gate/release/transition op requires approval in production.
- **B — Refine the policy first, then flip.** Make `default_policy_for_surface` require approval only for genuinely dangerous actions (e.g. `CycleSupersede`, release/`github_releases`, `framework_bundle`) and treat read-like/derived writes (gate receipts, plan revisions) as Medium, then flip to `All`. Requires an ADR amending ADR-069/072's surface→band table; preserves zero-bypass with a target that matches operator intent.
- **C — Replace the compile-time staging with an equally explicit, policy-driven cutover.** Remove the binary `LowMedium|All` stage; drive blocking from explicit configured policy per surface/action (`ENFORCE` only when a policy says so). Largest change; best long-term, needs an ADR.

Recommendation: **B** — it satisfies "complete the rollout" while keeping ordinary workflows usable, and it is the smallest change that yields a coherent production semantics. **A** only if the product intent is explicitly "all High-surface mutations require human approval".

## Zero-bypass evidence (independent of the stage decision)

- Single choke point: `enforce_gated` + `enforce_admission_or_block_ctx`; call sites use `admit_governed`.
- `c4.authority_single_admission` doctor check + `arch_lint` legacy-authority guard.
- Unit coverage: `all_high_surfaces_block_on_require_approval_at_production_stage` (added during the experiment; asserts each High surface maps to the single choke point outcome).

Enumerating every governed effect class and adding the negative/bypass test to the committed suite is part of whichever option is chosen.
