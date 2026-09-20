# SCOPE-CONTRACT — S6 — Fake relocation (decision: NOT_PROCEED, with evidence)

> **Slice id:** `p-63676b11dc0ef88f/a6-static-enhanced-readiness/slices/s6-fake-relocation`
> **Macro-cycle:** `p-63676b11dc0ef88f/a6-static-enhanced-readiness`
> **Status:** planning + audit complete; **NOT_PROCEED** pending operator decision.

## §1 Goal (per macro-cycle §S6)

Move `code_intelligence_port_fake` under `dev-dependencies` +
`#[cfg(any(test, feature = "test-support"))]` with a `test-support`
Cargo feature. Audit downstream consumers first.

## §2 Audit results (this slice)

### 2.1 Module scope

| Item | Value |
|---|---|
| Module | `crates/sddk-engine/src/code_intelligence_port_fake.rs` (376 LOC) |
| Public types | `FakeCodeIntelligenceProvider`, `NullCodeIntelligenceProvider` |
| Module is `pub` in `lib.rs` | yes (line ~42) |
| Is it a `dev-dependencies` item? | **No** — exposed as a regular `pub mod` |
| Are there existing test-support features? | **No** — only `default = []` and `std = []` |

### 2.2 Downstream consumers

```
$ grep -rn "FakeCodeIntelligenceProvider\|NullCodeIntelligenceProvider" --include='*.rs' .
```

Returns **31 hits**, all in `crates/sddk-engine/tests/`:

- `crates/sddk-engine/tests/a6_s1_uat_coverage_fake.rs` (12 tests)
- `crates/sddk-engine/tests/a6_s2_uat_c05_c07_c09.rs` (4 tests)
- `crates/sddk-engine/tests/a6_s3_ac10_verify_integration.rs` (5 tests)
- `crates/sddk-engine/tests/a6_cognicode_protocol_spike.rs` (5 tests)
- `crates/sddk-engine/tests/a6_cc_s1_static_graph_completeness.rs` (12 tests)

**Zero hits** in `crates/sddk-cli`, `crates/sddk-gateway`, `crates/sddk-domain`,
`crates/sddk-storage`, or any other production source.

### 2.3 Binary footprint — empirical measurement (2026-09-20)

The slice's stated motivation in the macro-cycle was closing a
CC-S0 RECEIPT finding about the fake living in release builds.
**The audit reveals the finding is already moot in practice:**

```
$ cargo build --release -p sddk-engine --offline
   Compiling sddk-engine v1.169.95
    Finished `release` profile [optimized] target(s) in 53.94s

$ strings target/release/libsddk_engine*.rlib | grep -c "FakeCodeIntelligenceProvider"
0

$ strings target/release/libsddk_engine*.rlib | grep -c "NullCodeIntelligenceProvider"
0
```

**Both types are stripped from the release rlib.** No consumer
outside the crate uses them, so even though the module is `pub`,
dead-code elimination removes the symbols. The "release pollution"
motivation for S6 no longer applies.

### 2.4 What the relocation would still buy

A remaining valid motivation **not stated in the macro-cycle SCOPE** would be
**API hygiene**: making `FakeCodeIntelligenceProvider` /
`NullCodeIntelligenceProvider` non-public surface of the crate. This
is a public-contract change and was not in the original S6 scope.

Per AGENTS.md §2.7 (semantic ownership) and §2.9 (extension
discipline): introducing a `test-support` feature is a **material
architectural change** (changes the build matrix, the docs surface,
the IDE/CLion/rust-analyzer experience, and the
`cargo doc --no-deps` output). Such a change requires operator
alignment per the rule "no broad speculative rewrites".

## §3 Hard constraints

- C1: no fabrication of evidence.
- C2: no source modification without operator decision.
- C3: no scope drift from the original macro-cycle goal.

## §4 STOP conditions (per macro-cycle §S6)

| Condition | Action | This slice |
|---|---|---|
| Audit reveals a downstream crate legitimately depends on the fake for production-side integration testing | STOP and report | **No** — audit shows zero downstream production usage. |
| Moving the fake breaks the 6 CC-S0 tests or the 12 CC-S1 tests and the breakage cannot be fixed without exposing the fake to a wider surface | STOP and report | **Not exercised** — no move attempted. |
| **Material architectural change** (per AGENTS.md §2.7) requires operator alignment | STOP and report | **Triggered** — introducing `test-support` feature is a build-matrix change. **Reporting**. |

## §5 Decision

**NOT_PROCEED.** The slice's stated motivation (release-binary
pollution by the fake module) is already empirically moot, and
the only remaining motivation (API hygiene / non-public surface)
is **scope drift** from the original SCOPE and triggers a
material architectural change STOP.

Operator can override this decision if:

1. **API hygiene is the actual goal.** Then an amended SCOPE
   targeting "make the fake module non-public" is the correct
   slice; the feature flag is one of several ways to achieve
   it (`pub(crate)` is another, simpler option with no build
   matrix implications).
2. **A future downstream crate will need the fake in production.**
   Then the `test-support` feature is the right design, but it
   must be motivated by the new consumer, not the current
   empty set.

## §6 Deliverables

| Deliverable | Path | Status |
|---|---|---|
| Audit (this file) | `tests/cycle-artifacts/.../s6-fake-relocation/SCOPE-CONTRACT.md` | ✅ |
| RECEIPT (this slice, NOT_PROCEED) | `tests/cycle-artifacts/.../s6-fake-relocation/RECEIPT.md` | ✅ |

No `UAT-EVIDENCE.yaml` (no behaviour exercised).

## §7 References

- `tests/cycle-artifacts/p-63676b11dc0ef88f/a6-static-enhanced-readiness/SCOPE-CONTRACT.md` §S6 (macro-cycle plan).
- `crates/sddk-engine/src/code_intelligence_port_fake.rs` (376 LOC).
- `crates/sddk-engine/Cargo.toml` (no existing `test-support` feature; `default = []`, `std = []`).
- AGENTS.md §2.7 (semantic ownership) and §2.9 (extension discipline).
