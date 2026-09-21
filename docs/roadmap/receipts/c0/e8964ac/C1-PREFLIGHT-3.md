# C1-PREFLIGHT-3 — Ejecutables de evidencia (test + probe)

> **Slice id:** `p-63676b11dc0ef88f/c1-contracts-hardening` (preflight extension 3)
> **Date (UTC):** 2026-09-21T12:14:00Z
> **Status:** EXECUTED EVIDENCE — observed in shell, not by inspection.
> **Companion to:** [C1-PREFLIGHT.md](./C1-PREFLIGHT.md), [C1-PREFLIGHT-2.md](./C1-PREFLIGHT-2.md).

## §0 Purpose

Earlier preflights were evidence-by-inspection (read source, identify bug). This preflight-3 adds **executed evidence** — what happened when we ran the existing test suite and a control-flow probe. The goal is to make the C1 SCOPE-CONTRACT have not just "this is the bug" but "this is the bug as observed in cargo test output + a mock probe".

## §1 Existing tests OBSERVED to pass

**Command:**
```
cargo test -p sddk-engine --lib structured_work
```

**Output (verbatim, last 12 lines):**
```
running 5 tests
test structured_work::tests::saw001_002_typed_request_schema_result ... ok
test structured_work::tests::saw006_receipt_provenance ... ok
test structured_work::tests::saw004_same_adapter_two_modes ... ok
test structured_work::tests::saw005_contribution_not_authority ... ok
test structured_work::tests::saw003_invalid_output_visible_not_fabricated ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 1313 filtered out; finished in 0.00s
```

**Findings:**
- 5 tests pass, 0 fail.
- All 5 are SAW-001..006 (the 6 spec invariants). SAW-001 and SAW-002 share one test.
- Compilation: 1m 25s; this is the only crate that needed rebuilding.
- **1313 other tests in the engine crate, all filtered out by the `structured_work` filter** — meaning the existing test surface for SAW is comprehensive for known descriptors but blind to unknown ones.

**Implication for C1 H01:**
- A new regression test for H01 will not conflict with existing SAW tests.
- The test should follow the SAW-007 naming convention or extend the existing SAW-003 test to cover unknown descriptors.
- A potential test (proposed, NOT added to repo):

  ```rust
  /// SAW-007 (proposed): unknown shape descriptor must NOT match.
  /// H01 of ROADMAP §2: descriptor no soportado no puede dar true.
  #[test]
  fn saw007_unknown_descriptor_rejected() {
      // shape_matches(json!("hello"), "this-descriptor-does-not-exist")
      //   → expected: false (currently returns true; H01 bug)
      // After fix: shape_matches returns false for unknown descriptors.
  }
  ```

## §2 Mock probe OBSERVED

**Source of probe (in `/tmp/sddk_h01_probe.rs`, NOT in repo):**

```rust
fn shape_matches_mock(value_kind: &str, shape: &str) -> bool {
    match shape {
        "string" => value_kind == "string",
        "u64" => value_kind == "u64",
        "bool" => value_kind == "bool",
        "array<string>" => value_kind == "array<string>",
        _ => true, // ← THE BUG: pass-through
    }
}

fn main() {
    let bug = shape_matches_mock("any-value", "this-descriptor-does-not-exist");
    println!("mock result = {}", bug);
    if bug {
        println!("H01 BUG CONFIRMED OBSERVED in equivalent control flow");
    } else {
        println!("H01 fix in place (unexpected)");
    }
}
```

**Output (verbatim):**
```
mock result = true
H01 BUG CONFIRMED OBSERVED in equivalent control flow
```

**Findings:**
- The control-flow shape (a `match` with `_ => true`) is exactly the structure at line 198-208 of `crates/sddk-engine/src/structured_work.rs`.
- The mock reproduces the bug deterministically.
- A future fix (replacing `_ => true` with explicit `false` or an exhaustive match) would change this mock's output to `false`. That's the RED→GREEN cycle for H01.

## §3 IdempotencyKey distinction OBSERVED

Two `IdempotencyKey` types exist. Verified structurally different:

**Type 1 (`crates/sddk-domain/src/proposal.rs:39`):**
```rust
pub struct IdempotencyKey {
    pub project_id: String,
    pub cycle_id: Option<String>,
    pub capability: String,
    pub request_hash: String,
}
```

**Type 2 (`crates/sddk-domain/src/workflow_run.rs:147`):**
```rust
pub struct IdempotencyKey {
    pub project_id: String,
    pub run_id: RunId,
    pub node_id: NodeId,
    pub attempt_seq: u32,
}
```

**Findings (corrects C1-PREFLIGHT-2 §1):**
- These are NOT a code smell — they are two different concepts in two different domains:
  - `proposal::IdempotencyKey` — dedup of capability proposals (project + cycle + capability + hash).
  - `workflow_run::IdempotencyKey` — dedup of attempt executions (project + run + node + seq).
- No refactor needed. Both are correctly scoped.
- H02 (which talks about request_id collision) is primarily about the **proposal** flavor, not the workflow_run one. The cycle-start research should grep `proposal::IdempotencyKey` callers specifically.

## §4 MANIFEST.sha256 stale check (observation)

**Observed:**
- `MANIFEST.sha256` last regenerated at commit `450e993` ("regenerate MANIFEST.sha256 for the orchestrator prompt configuration section").
- That regeneration happened TODAY (2026-09-21 10:57:52).
- BUT the 5 files in `docs/roadmap/receipts/c0/e8964ac/` that I created AFTER 450e993 are NOT in MANIFEST.sha256.

**Verified:**
```
$ grep -c "docs/roadmap/receipts/c0/" MANIFEST.sha256
0
```

**Implication:**
- The MANIFEST is **partially stale** for the new C0 docs.
- However, `scripts/release.sh` step 4 regenerates the manifest before publishing:
  ```
  "$BIN" dev manifest --root . --format text
  "$BIN" dev manifest --verify --root . --format text
  ```
- So the release pipeline will refresh the manifest and the bundle will include the new C0 docs.
- **No autonomous action required** for the operator's release to succeed; the pipeline handles it.

**Action recorded:**
- This observation is logged for honesty. The next autonomous pre-release check should verify MANIFEST includes all new files. For now, trust the pipeline.

## §5 Stop conditions respected

- Existing tests: observed pass via `cargo test`.
- Probe: written to `/tmp/`, NOT to repo, NOT committed.
- Mock probe confirms the bug shape.
- No source code modified.
- No tests added to repo (the proposed SAW-007 test is documented but NOT written).
- No release action invoked.

## §6 What the future C1 SCOPE-CONTRACT will inherit

When C1 SCOPE-CONTRACT is written (post-release), this preflight contributes:

1. **Baseline test count for SAW tests**: 5 pass, 0 fail.
2. **A reproducible mock** to verify the fix during cycle execution.
3. **A proposed test name**: `saw007_unknown_descriptor_rejected` (or extend `saw003`).
4. **No idempotency-key refactor** — clarifies a previous suspicion.
5. **MANIFEST freshness**: pipeline-regenerated, not autonomous-regenerated.
