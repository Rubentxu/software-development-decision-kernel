# C1-H01-RECEIPT — fix(engine): reject unknown descriptors in shape_matches

> **Slice id:** `p-63676b11dc0ef88f/c1-h01-shape-matches-descriptor-validation`
> **Baseline SHA:** `eae4f7f`
> **Fix SHA:** `74dfcc9`
> **Bump SHA:** `4159052` (post-bump, admission ACCEPT)
> **Date (UTC):** 2026-09-21T13:55:00Z (start) → 2026-09-21T14:16:00Z (end)
> **Status:** PASS_OBSERVED — fix landed, regression test green, no-regression confirmed at workspace level

## §1 Summary

ROADMAP §2 C1 H01: `structured_work::shape_matches` no debe devolver `true` para descriptor no soportado.

**Before:**
```rust
match shape {
    "string" => v.is_string(),
    "u64" => v.is_u64(),
    "bool" => v.is_boolean(),
    "array<string>" => v.as_array().is_some_and(|a| a.iter().all(|x| x.is_string())),
    _ => true, // unknown descriptors are pass-through (coarse schema)
}
```

**After:**
```rust
match shape {
    "string" => v.is_string(),
    "u64" => v.is_u64(),
    "bool" => v.is_boolean(),
    "array<string>" => v.as_array().is_some_and(|a| a.iter().all(|x| x.is_string())),
    // H01 of ROADMAP §2 C1: unknown descriptors must NOT match.
    // Previously this arm was `_ => true` (pass-through), which silently
    // accepted any shape, allowing the executor to fabricate
    // `Contributed` outcomes for descriptors that were not part of
    // the supported allow-list ("string" | "u64" | "bool" |
    // "array<string>"). Any future descriptor must be added here
    // explicitly with its matcher.
    _ => false,
}
```

Single-line semantic change. Comment expanded to document the ROADMAP rationale.

## §2 Tests added

| Test | Purpose |
|---|---|
| `saw007_unknown_descriptor_rejected` | Unit-level: 4 known-descriptor regressions + 7 unknown-descriptor rejections. |
| `saw008_unknown_descriptor_violation_not_contribution` | End-to-end: `StructuredWorkExecutor` with unknown descriptor in `return_schema` produces `SchemaViolation`, NOT `Contributed`. Receipt shows `outcome_kind = SchemaViolation`. |

## §3 Test results

### RED (pre-fix)
```
test saw007_unknown_descriptor_rejected ... FAILED
   assertion failed: !shape_matches(&json!("hello"), "this-descriptor-does-not-exist")
test saw008_unknown_descriptor_violation_not_contribution ... FAILED
   expected SchemaViolation, got Contributed(ContributionV2 { request_id: "req-1", fields: ... })
test result: FAILED. 5 passed; 2 failed; 0 ignored; 1313 filtered out
```

### GREEN (post-fix)
```
test saw007_unknown_descriptor_rejected ... ok
test saw008_unknown_descriptor_violation_not_contribution ... ok
test saw001_002_typed_request_schema_result ... ok
test saw003_invalid_output_visible_not_fabricated ... ok
test saw004_same_adapter_two_modes ... ok
test saw005_contribution_not_authority ... ok
test saw006_receipt_provenance ... ok
test result: ok. 7 passed; 0 failed; 0 ignored; 1313 filtered out
```

### No-regression (workspace)
```
$ cargo test --workspace
TOTAL passed: 4968
TOTAL failed: 0
TOTAL ignored: 15
```

Comparison vs baseline (`C0-WORKSPACE-TESTS.md` @ `62494ae`: 4966/0/15): +2 passed (SAW-007 + SAW-008), 0 failed, 15 ignored unchanged.

## §4 Binary verification

```
$ cargo build --release -p sddk-cli
   Compiling sddk-cli v1.169.131 (...)
    Finished `release` profile [optimized] target(s) in 3m 10s

$ /var/home/rubentxu/cargo-targets/release/sddk --version
sddk 1.169.131

$ sha256sum /var/home/rubentxu/cargo-targets/release/sddk
42d5c065d7ce50b0b0d15f142e45f0dfb4af24316ead204cbe2c7128f315ca3d  /var/home/rubentxu/cargo-targets/release/sddk
```

## §5 Acceptance gates

| Gate | Result | Evidence |
|---|---|---|
| **G0 — Fix is minimal and surgical** | PASS | 1-line semantic change in `shape_matches`; comment expanded with rationale. |
| **G1 — RED characterization captured** | PASS | Tests fail pre-fix with verbatim output. |
| **G2 — GREEN captured post-fix** | PASS | 7/7 SAW tests pass; SAW-007 + SAW-008 specifically. |
| **G3 — No-regression at crate level** | PASS | 1313+ existing engine tests still green. |
| **G4 — No-regression at workspace level** | PASS | 4968 / 0 / 15 workspace tests. |
| **G5 — Admission gate restored** | PASS | `release_admission_check HEAD` → ACCEPT 1.169.130 -> 1.169.131. |
| **G6 — Pre-push admit** | PASS | rule A (real Cargo.toml change). |
| **G7 — Binario built and verified** | PASS | `sddk 1.169.131`, sha256 `42d5c065...`. |
| **G8 — History preserves record** | PASS | SCOPE-CONTRACT, UAT-EVIDENCE, RECEIPT all under `docs/roadmap/receipts/c1/eae4f7f/`. |

## §6 Risks and limitations

- **Strict tightening**: any consumer that sent an unknown descriptor before now receives `SchemaViolation`. This is the correct semantic per ROADMAP H01. No internal consumers broken (4968 workspace tests pass).
- **Surface area**: only `crates/sddk-engine/src/structured_work.rs`. 1 call site at line 154.
- **Future extensions**: if SDDK adds new descriptors (e.g. `i64`, `f64`, `bytes`), they must be added explicitly with matchers. The `_ => false` makes the allow-list closed.

## §7 Next action

Operator:
```bash
cd ~/Proyectos/agentesIA/sddk-framework
git fetch origin main
git checkout main && git pull --ff-only
# HEAD should be 4159052; admission ACCEPT.
source scripts/lib/release_admission.sh && release_admission_check HEAD
bash scripts/release.sh   # publica v1.169.131
```

Or, if operator prefers the cleaner `reset --hard 74dfcc9 + release.sh` flow (publishing the H01 fix without the bump commit), see `OPERATOR-RUNBOOK.md` §2.

After operator publishes, the next slice is **H02** (`structured_work::submit` + `IdempotencyKey` semantics + error interpolation). SCOPE-CONTRACT for H02 will be emitted in a new cycle.
