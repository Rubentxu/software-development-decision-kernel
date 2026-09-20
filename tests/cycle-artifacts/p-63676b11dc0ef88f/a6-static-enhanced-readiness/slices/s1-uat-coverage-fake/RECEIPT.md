# RECEIPT — S1 — UAT Coverage via Fake

> **Slice id:** `p-63676b11dc0ef88f/a6-static-enhanced-readiness/slices/s1-uat-coverage-fake`
> **Macro-cycle:** `p-63676b11dc0ef88f/a6-static-enhanced-readiness`
> **Baseline (released):** `v1.169.94` → `dd616d0` (macro-cycle plan)
> **Cycle lead:** orchestrator (this session, auto-run)
> **Status:** **CLOSED (LOCALLY)** — push pending.

## §1 Scope adherence

| Hard constraint | Status | Evidence |
|---|---|---|
| C1: no architectural change to `CodeIntelligencePort` trait | ✅ | The slice only **consumes** the trait (and types) shipped by CC-S0 + CC-S1; no new ADTs, no signature changes. Diff in this slice = test file + cycle-artifacts only. |
| C2: zero provider DTO types in `sddk-domain` | ✅ | `sddk-domain` untouched. The test imports from `sddk_engine::code_intelligence_port` (SDDK-owned) and `sddk_engine::code_intelligence_port_fake` (SDDK-owned fake). No `cognicode::*` / `chronos::*` / `prost::*` / `tonic::*` token in scope. |
| C3: no modification of `arch-spec-021` | ✅ | `arch-acceptance-coverage-001` (acceptance contract) already covers AR-1..AR-7; this slice exercises them via tests, no amendment to either spec. |
| C4: no new dependency added | ✅ | `Cargo.toml` / `Cargo.lock` unchanged. |
| C5: `cargo test --workspace --offline` green | ✅ | 235 `test result: ok` lines; zero `failed` lines; exit 0. |

| STOP condition (per SCOPE §4) | Triggered? |
|---|---|
| Discovery that AC10 requires architectural change to Verify | No — AC10 is S3's scope; S1 does not touch Verify. |
| Discovery that the fake provider cannot faithfully represent a real consumer's claim | No — `CapabilitySnapshot::from_advertised_with_capabilities` supports the required class declarations; tests demonstrate semantics `Demonstrated` while preserving `Unknown` for inventory and operational (per M6). |
| `cargo fmt` / `cargo clippy -D warnings` failures | No — both exit 0 after one fmt iteration and one clippy iteration. |
| Discovery that PR-UAT-C0X cannot be expressed in existing types | No — every row maps to existing CC-S0/CC-S1 ADTs. |

## §2 Evidence

### 2.1 S1 test binary (12 PASS / 0 FAIL / 0 ignored)

```
$ cargo test -p sddk-engine --offline --test a6_s1_uat_coverage_fake
…
running 12 tests
test t_uat_c01_optional_provider_unavailable_yields_evidence_gap ... ok
test t_uat_c01_preferred_provider_unavailable_yields_evidence_gap ... ok
test t_uat_c02_required_provider_unavailable_yields_typed_failure ... ok
test t_uat_c03_compatible_handshake_records_capabilities ... ok
test t_uat_c03_capability_snapshot_is_content_addressed ... ok
test t_uat_c04_localized_delta_typed_result_with_basis ... ok
test t_uat_c04_no_full_graph_transfer_on_localized_delta ... ok
test t_uat_c06_cancellation_yields_incomplete_not_satisfied ... ok
test t_uat_c06_cancelled_delta_does_not_produce_partial_green_receipt ... ok
test t_uat_c08_protocol_incompatible_no_capability_inference ... ok
test t_uat_c10_repeat_deterministic_analysis_yields_stable_digest ... ok
test t_uat_c10_coverage_evaluation_is_deterministic_across_runs ... ok

test result: ok. 12 passed; 0 failed; 0 ignored
```

### 2.2 Regression checks (no green turned red)

| Battery | Result |
|---|---|
| CC-S0 spike (`a6_cognicode_protocol_spike`) | 6 PASS / 0 FAIL |
| CC-S1 acceptance (`a6_cc_s1_static_graph_completeness`) | 12 PASS / 0 FAIL / 1 ignored (`t_ar_6_ext_real_cognicode_run` — S5 owns) |
| Workspace (`cargo test --workspace --offline`) | 235 `test result: ok` lines; 0 `failed`; exit 0 |
| `cargo fmt --check` | exit 0 |
| `cargo clippy -p sddk-engine --all-targets -- -D warnings` | exit 0 |
| `bash tests/test_push_prevention_hook.sh` | 35 PASS / 0 FAIL |

### 2.3 UAT matrix rows (per `07-UAT-EVIDENCE-MATRIX.md` §E + §I)

See `tests/cycle-artifacts/.../s1-uat-coverage-fake/UAT-EVIDENCE.yaml` for the durable YAML evidence schema.

| Row | Profile | Result | Test reference |
|---|---|---|---|
| `PR-UAT-C01` (provider stopped, OPTIONAL/PREFERRED) | Static | PASS | `t_uat_c01_*` (2 tests) |
| `PR-UAT-C02` (provider stopped, REQUIRED) | Static | PASS | `t_uat_c02_required_provider_unavailable_yields_typed_failure` |
| `PR-UAT-C03` (compatible startup/handshake) | Static | PASS | `t_uat_c03_*` (2 tests) |
| `PR-UAT-C04` (localized delta, typed result) | Static | PASS | `t_uat_c04_*` (2 tests) |
| `PR-UAT-C06` (cancellation/deadline) | Static | PASS | `t_uat_c06_*` (2 tests) |
| `PR-UAT-C08` (protocol major incompatible) | Static | PASS | `t_uat_c08_protocol_incompatible_no_capability_inference` |
| `PR-UAT-C10` (deterministic analysis) | Static | PASS | `t_uat_c10_*` (2 tests) |

Rows **not** in this slice (deferred to other slices):

| Row | Owner slice |
|---|---|
| `PR-UAT-C05` (impact, no DTO leakage) | S2 |
| `PR-UAT-C07` (restart/reconnect, stable refs) | S2 |
| `PR-UAT-C09` (contradiction preservation) | S2 |
| AC10 (static evidence consumed by Verify) | S3 |
| `PR-UAT-024` (durability of static-evidence artefacts) | S4 |
| Real-provider EXT (`t_ar_6`) | S5 |

## §3 Files changed by this slice

| Path | Δ | Role |
|---|---|---|
| `crates/sddk-engine/tests/a6_s1_uat_coverage_fake.rs` | nuevo | 12 UAT tests, deterministic subset. |
| `tests/cycle-artifacts/p-63676b11dc0ef88f/a6-static-enhanced-readiness/slices/s1-uat-coverage-fake/SCOPE-CONTRACT.md` | nuevo | Slice scope. |
| `tests/cycle-artifacts/p-63676b11dc0ef88f/a6-static-enhanced-readiness/slices/s1-uat-coverage-fake/UAT-EVIDENCE.yaml` | nuevo | Durable UAT evidence rows in YAML. |
| `tests/cycle-artifacts/p-63676b11dc0ef88f/a6-static-enhanced-readiness/slices/s1-uat-coverage-fake/RECEIPT.md` | nuevo | This file. |

No source file in `crates/sddk-engine/src/**` modified by this slice (test-only).

## §4 Honest limits

1. **No real CogniCode**: all rows exercised via `FakeCodeIntelligenceProvider` and `NullCodeIntelligenceProvider`. Real-provider evidence belongs to S5 (`t_ar_6_ext_real_cognicode_run` with `COGNICODE_MCP_BIN`).
2. **Inventory and operational dimensions stay `Unknown`** in these tests (the fake does not enrich them into `Demonstrated` — that is intentional per M6 of the CC-S1 SCOPE, "Unknown preserved, never invented"). The verdict is therefore `Incomplete` even when semantics is `Demonstrated`; this is correct and pinned by `t_ar_1` (CC-S1).
3. **CoverageContract for `verify-kernel::static_evidence`** is exercised in code, but no actual `verify-kernel` consumer exists yet. Wiring a real consumer is part of AC10 in S3.

## §5 Next slice

**S2 — UAT-C05, C07, C09** (impact no-DTO-leakage, restart/reconnect stable refs, contradiction preservation). Depends on S1.
