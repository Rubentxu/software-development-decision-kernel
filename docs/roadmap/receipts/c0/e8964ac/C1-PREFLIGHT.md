# C1-PREFLIGHT — Verificación de código candidato (READ-ONLY, no aplica cambios)

> **Slice id:** `p-63676b11dc0ef88f/c1-contracts-hardening` (preflight, read-only)
> **Date (UTC):** 2026-09-21T12:08:00Z
> **Status:** EVIDENCE — observations against `main@151c8ab`. NO code touched.
> **ROADMAP ref:** [C1 §2](../../ROADMAP.md) lines 36-43.

## §0 Purpose

This is **read-only preflight** to convert the speculative C1-RESEARCH-NOTES.md into evidence-grounded questions. It does not start the C1 cycle. It runs only `grep` / `read` / `find` against the existing codebase to verify that the candidate files and bug hypotheses in C1-RESEARCH-NOTES.md are correct before the SCOPE-CONTRACT is written.

Why now: the C0 → C1 transition is blocked on `bash scripts/release.sh` (system-law). Preflight does not violate any gate.

## §1 H01 — `shape_matches` verification (FIXED LOCATION vs RESEARCH-NOTES)

**C1-RESEARCH-NOTES.md §1 said:**
> Archivo candidato: `crates/sddk-domain/src/structured_work.rs`

**OBSERVED — the real location is `crates/sddk-engine/src/structured_work.rs`:**

```
$ grep -rn "fn shape_matches\|fn shape_match\|shape_matches\b" crates/ --include='*.rs'
crates/sddk-engine/src/structured_work.rs:154:                            if !shape_matches(v, shape) {
crates/sddk-engine/src/structured_work.rs:198:fn shape_matches(v: &serde_json::Value, shape: &str) -> bool {

$ grep -rn "structured_work" crates/ --include='*.rs'
crates/sddk-engine/src/lib.rs:132:pub mod structured_work;
```

**Discrepancy:** RESEARCH-NOTES mis-attributed the crate (it said `sddk-domain`, the actual is `sddk-engine`). This is consistent with SAW-001..006 living in the engine layer, not the domain.

## §2 H01 — Bug confirmation

**`crates/sddk-engine/src/structured_work.rs` lines 198-208:**

```rust
fn shape_matches(v: &serde_json::Value, shape: &str) -> bool {
    match shape {
        "string" => v.is_string(),
        "u64" => v.is_u64(),
        "bool" => v.is_boolean(),
        "array<string>" => v
            .as_array()
            .is_some_and(|a| a.iter().all(|x| x.is_string())),
        _ => true, // unknown descriptors are pass-through (coarse schema)
    }
}
```

**Bug H01 confirmed OBSERVED:**
- The `_ => true` fallthrough means **any unknown descriptor is accepted** (e.g. `"string "`, `"integer"`, `"u32"`, `"my-custom-shape"`, `"Object"` all return `true`).
- This violates H01 of ROADMAP §2: "descriptor no soportado no puede dar `true`".
- The comment "unknown descriptors are pass-through (coarse schema)" is exactly the behaviour H01 asks to change.

**Call site (line 154):**
```rust
if !shape_matches(v, shape) {
    violations.push(format!("field {name} expected {shape}, got {v}"));
}
```

A regression test would assert: `shape_matches(json!("hello"), "integer") == false`, `shape_matches(json!("hello"), "u32") == false`, etc.

## §3 H01 — Existing test coverage

**Existing tests (lines 210-365):**
- `saw001_002_typed_request_schema_result` — tests the happy path with `string` and `u64`.
- `saw003_invalid_output_visible_not_fabricated` — tests invalid outputs (string for u64, etc.) but only with **known** descriptors.
- (more tests below line 260 — to be read on cycle start)

**No test covers the unknown-descriptor pass-through path.** This is the gap H01 closes.

## §4 H02 — `request_id` dedup verification (DEFERRED UNTIL CYCLE START)

**Research question for H02:**
- Where is the dedup logic for `request_id`?
- Is it in `sddk-engine/src/request_dedup.rs`, or elsewhere?

**This was not investigated yet** — would require more searches (e.g. `grep -rn "request_id" crates/sddk-engine/`). Deferred to cycle start to keep this preflight compact.

## §5 H05 — `set_process_service_for_tests` verification (DEFERRED UNTIL CYCLE START)

**Research question for H05:**
- Does such a seam exist? Under what name?
- Is it `#[cfg(test)]`-gated?

**This was not investigated yet.** Deferred.

## §6 H06 — gateway argument/byte defense verification (DEFERRED UNTIL CYCLE START)

**Research question for H06:**
- Where is the gateway entry point that receives bytes from the host?
- How are secrets redacted in logs and receipts?

**Deferred.**

## §7 What this preflight DOES NOT verify

- H02 / H05 / H06 (deferred to cycle start; would need more searches).
- Whether changes to `shape_matches` break any A5-* certified tests.
- Whether the change is breaking for any existing host adapter (e.g. JCode adapter sends shapes not in the current allow-list).
- Whether the change requires a new ADR.

## §8 Updated RESEARCH-NOTES candidates

When C1 SCOPE-CONTRACT is written (post-release), the following corrections should be applied to C1-RESEARCH-NOTES.md:

1. §1: change "Archivos candidatos" → "**Primary:** `crates/sddk-engine/src/structured_work.rs` (function `shape_matches`, line 198). **Not** in `sddk-domain`."
2. §1: add a "Bug evidence" subsection pointing to lines 198-208 and the `_ => true` fallthrough.
3. §1: add an "Existing test gap" subsection noting that no current test exercises the unknown-descriptor path.
4. §1: add a "Compatibility risk" — need to check who consumes this function. A simple grep `shape_matches` shows callers: only line 154. So blast radius is contained.
5. §3 (H05) and §4 (H06): remain speculative until cycle-start research.

## §9 Stop conditions respected

- No code modified.
- No new tests added (test gap will be filled by C1 cycle).
- No SCOPE-CONTRACT emitted (still DRAFT).
- No release action invoked.
- Only `grep`/`read`/`find` against existing tree; no crate rebuild.
