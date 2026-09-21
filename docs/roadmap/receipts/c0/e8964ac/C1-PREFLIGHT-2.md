# C1-PREFLIGHT-2 — Verificación de H02/H05/H06 (READ-ONLY)

> **Slice id:** `p-63676b11dc0ef88f/c1-contracts-hardening` (preflight extension)
> **Date (UTC):** 2026-09-21T12:10:00Z
> **Status:** EVIDENCE — observations against `main@a5babb7`. NO code touched.
> **ROADMAP ref:** [C1 §2](../../ROADMAP.md) lines 36-43.
> **Companion to:** [C1-PREFLIGHT.md](./C1-PREFLIGHT.md) (which covered H01).

## §0 Purpose

C1-PREFLIGHT.md covered H01 (shape_matches). This extension verifies H02/H05/H06 with evidence so the future C1 SCOPE-CONTRACT can plan with substance instead of speculation. Read-only grep/read against existing tree. No code changes.

## §1 H02 — `request_id` dedup verification

**Question:** How is `request_id` deduplicated? Is it just by ID, or also by payload hash?

**OBSERVED:**

```
$ grep -rn "IdempotencyKey\|compute_digest" crates/ --include='*.rs' | head -10
crates/sddk-gateway/src/capability.rs:277:    fn compute_digest_is_deterministic() {
crates/sddk-storage/tests/concurrency_record_attempt.rs:19:    Attempt, AttemptOutcome, ContextCapsuleRef, IdempotencyKey, NodeRun, NodeRunState, Route,
crates/sddk-domain/src/proposal.rs:36:/// detect duplicate requests and ensure exactly-once semantics.
crates/sddk-domain/src/proposal.rs:39:pub struct IdempotencyKey {
crates/sddk-engine/src/human_decision.rs:352:    fn compute_digest(request_id: &str, decision: &HumanDecision) -> String {
crates/sddk-engine/src/human_decision.rs:379:    fn fetch(&self, request_id: &str) -> Option<HumanDecisionRequest> {
crates/sddk-storage/src/lib.rs:2110:                // Parse "project_id:run_id:node_id:attempt_seq" back to IdempotencyKey.
crates/sddk-storage/src/lib.rs:2115:                        key: sddk_domain::workflow_run::IdempotencyKey {
```

**Findings:**

1. **`IdempotencyKey` is the canonical dedup primitive**, defined in `crates/sddk-domain/src/proposal.rs:39`:

   ```rust
   pub struct IdempotencyKey {
       pub project_id: String,
       pub cycle_id: Option<String>,
       pub capability: String,
       pub request_hash: String,  // ← payload hash, not just request_id
   }
   ```

2. The `as_str` method produces a composite key:
   ```rust
   format!("{}/{}/{}/{}", project_id, cycle_id, capability, &request_hash[..16])
   ```

3. **There is a second `IdempotencyKey` in `crates/sddk-domain/src/workflow_run.rs`** (referenced from storage tests). Two `IdempotencyKey` types in the same domain crate is suspicious — need to verify if they are the same type or duplicates. Grep shows `crates/sddk-domain::workflow_run::IdempotencyKey` separate from `crates/sddk-domain::proposal::IdempotencyKey`.

**Implication for H02:**

- The dedup **already uses request_hash + capability + project_id + cycle_id**, not just `request_id`. So "mismo `request_id`, payload distinto" is NOT the only thing to verify — the question becomes: what happens if `project_id+capability+request_hash` collides? Is the existing dedup correct?
- The dual `IdempotencyKey` types is a code smell; consolidating them or documenting the distinction is a separate refactor.
- Schema-typed errors with no sensitive interpolation: not directly verified yet. The error path is in `proposal.rs` (line 25-30 enum), need to grep `#[error(…)]` for interpolation patterns.

**Sub-questions for cycle-start research:**
- Is there a test asserting "same key + different payload → reject" or is the dedup only "same key + same payload → idempotent"?
- Are there two `IdempotencyKey` types or one with a re-export? Need a deeper diff.

## §2 H05 — `set_process_service_for_tests` verification

**Question:** Does this seam exist? Is it isolated?

**OBSERVED:**

```
$ grep -rn "pub fn set_.*for_tests\|fn.*_for_tests\b" crates/ --include='*.rs' | head -5
crates/sddk-storage/src/lib.rs:347:    pub fn connection_for_tests(&self) -> &Connection {
crates/sddk-engine/src/authority_ticket_service.rs:111:pub fn set_process_service_for_tests(
```

**Confirmed at `crates/sddk-engine/src/authority_ticket_service.rs:111`:**

```rust
#[doc(hidden)]
pub fn set_process_service_for_tests(
    svc: AuthorityTicketService,
) -> Option<AuthorityTicketService> {
    // OnceLock::set returns Result<(), T>; we discard it and use get_mut
    // semantics — the API intentionally returns Option<prev> for clarity.
    PROCESS_SERVICE.set(svc).err()
}
```

**Critical finding — ISOLATION GAP:**

- The function is `#[doc(hidden)]` but **NOT `#[cfg(test)]`-gated**.
- This means production code can invoke `sddk_engine::authority_ticket_service::set_process_service_for_tests(...)` if it knows the path.
- Grep confirms it is NOT called from anywhere yet (`grep -rn "set_process_service_for_tests" crates/` only finds the def + the docstring reference). So **no active exploit**, but **the door is unlocked**.

**Implication for H05:**

- ROADMAP H05 asks "test-seam con semántica real y aislamiento de producción".
- The minimum fix is to gate the function with `#[cfg(test)]` or move it to a `test_support` module gated behind a test-only feature flag.
- A regression test would assert that the function is not exported from the public API surface (e.g. `cargo doc` does not show it, or `cargo public-api` does not list it as public).

## §3 H06 — Gateway defense + redaction

**Question:** Where is the gateway entry? How are secrets redacted?

**OBSERVED:**

```
$ ls crates/sddk-gateway/src/
artifact_store.rs   capability.rs        computer_use.rs  denial_surface.rs
evidence.rs         filesystem.rs        forge.rs         gateway.rs
git.rs              lib.rs               oracles.rs       permissions.rs
playwright.rs       policy.rs            producer_l0_adapter.rs  release.rs
runner_receipt.rs   runner.rs            semantic.rs      storage_snapshot_l1_consumer.rs
test_runner/        uat_policy.rs
```

**Confirmed canonical redaction pattern at `crates/sddk-gateway/src/lib.rs`:**

```rust
pub fn redact(value: Value) -> Value {       // line 233
    // ...
}
fn redact_text(input: &str) -> String {      // line 277
    // ...
}
```

**Two test files cover the contract:**

- `crates/sddk-gateway/tests/sec1_redactor_unit.rs` — unit tests (redact_masks_token_value_in_stdout, redact_does_not_touch_non_secret_keys, redact_keeps_existing_key_level_redaction).
- `crates/sddk-gateway/tests/sec1_capability_receipt_redaction.rs` — integration tests (sec1_diagnostic_information_preserved_after_redaction).
- `crates/sddk-gateway/tests/runner_receipt_e2e.rs:123` — `t06_canary_in_stdout_redacted_in_receipt`.

**Defense gateway:**

- `crates/sddk-cli/src/uat_serve.rs:125,170` returns `413 payload too large` for oversized payloads.
- `crates/sddk-engine/src/canonical_event_log.rs:38` returns `PayloadTooLargeForInlineStorage { size, limit }` for storage-layer limits.

**Other findings:**

- `crates/sddk-gateway/src/test_runner/mod.rs:33` re-exports `detect_secret_like` for tests only.
- `is_secret_like` in `env_allowlist` module flags keys matching `_TOKEN`, `_SECRET`, `_KEY` suffixes or exact `GITHUB_TOKEN`.

**Implication for H06:**

- The redaction helper IS canonical (single `pub fn redact` + `fn redact_text`). Good.
- The defense gateway has SOME size limits (413 on HTTP, payload-too-large on storage). But there's no central "argument validator" that catches all entry points.
- The gaps to verify in cycle-start:
  - Are ALL entry points using `redact()` consistently? Or does each crate have its own ad-hoc redaction?
  - Is the env-allowlist pattern (test_runner-only) the right one for production, or should it be promoted to a shared module?

## §4 Summary — what is verified vs deferred

| Item | Status | Evidence |
|---|---|---|
| H01: shape_matches location + bug | ✅ CONFIRMED | C1-PREFLIGHT.md §1-§2 |
| H02: IdempotencyKey shape | ✅ CONFIRMED | this file §1 |
| H02: same-key-different-payload rejection | 🔲 DEFERRED | needs test grep + diff between two IdempotencyKey types |
| H02: error interpolation patterns | 🔲 DEFERRED | needs grep on `#[error(…)]` |
| H05: seam location | ✅ CONFIRMED | this file §2 |
| H05: seam gating | ❌ GAP CONFIRMED | `#[doc(hidden)]` only, NOT `#[cfg(test)]` |
| H05: active exploit | ✅ NONE OBSERVED | grep confirms zero callers |
| H06: canonical redaction helper | ✅ CONFIRMED | `redact()` at gateway/src/lib.rs:233 |
| H06: defense size limits | ✅ CONFIRMED | 413 + PayloadTooLargeForInlineStorage |
| H06: all entry points use `redact()` | 🔲 DEFERRED | needs grep across crates |

## §5 Updated RESEARCH-NOTES candidates

When C1 SCOPE-CONTRACT is written (post-release):

1. §2 (H02): add the **dual `IdempotencyKey` smell** as a separate sub-task. Either consolidate or document.
2. §2 (H02): add a "Same-key-different-payload → reject" test as a regression must-have.
3. §3 (H05): the priority is to **gate the seam with `#[cfg(test)]`** or move to `test_support`. Simple fix, contained.
4. §3 (H05): add a regression test that the public-api surface does not list the seam.
5. §4 (H06): the central `redact()` exists; the cycle should verify it's applied at every entry point. A grep for `redact(` callsites is a good first step.

## §6 Stop conditions respected

- No code modified.
- No new tests added.
- No SCOPE-CONTRACT emitted (still DRAFT).
- No release action invoked.
- Only `grep`/`read`/`find` against existing tree.
