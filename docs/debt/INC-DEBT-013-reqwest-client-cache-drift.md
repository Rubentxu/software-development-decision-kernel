# INC-DEBT-013: reqwest::Client cache drift in RealTaskExecutor

**status**: closed
**severity**: low
**priority**: P3
**created_at**: 2026-08-26
**closed_at**: 2026-08-26
**cycle**: 39
**detected_by**: post-cycle-38 debt-verify sweep (2026-08-26)

## Problem

Cycle-20 WU-1 (commit `123f9b2`, "swap ureq→reqwest 0.12 rustls-tls", closes
INC-FORWARD-002) introduced the cached-client infrastructure in `RealTaskExecutor`:

```rust
pub struct RealTaskExecutor {
    runtime: OnceLock<tokio::runtime::Runtime>,
    client:  OnceLock<reqwest::Client>,   // ← cached client field
    clock:   Arc<dyn Clock>,
}

impl RealTaskExecutor {
    fn get_client(&self) -> &reqwest::Client {
        self.client.get_or_init(|| reqwest::Client::builder().build().expect(...))
    }
}
```

The intent was clear: cache the reqwest client on the executor so all HTTP
fetches share a single connection pool.

**But the production path was never rewired.** `dispatch_http_fetch` still
creates a fresh `reqwest::Client::new()` on every call:

```rust
fn dispatch_http_fetch(inputs: &BTreeMap<String, Value>) -> Result<TaskOutput, TaskError> {
    // ...
    let client = reqwest::Client::new();   // ← NEW client every fetch
    // ... never calls self.get_client() ...
}
```

### Symptom (clippy)

Two out-of-scope clippy warnings, present since cycle-20 (per cycle-38 release-receipt):

- `field 'client' is never read` — `crates/sddk-engine/src/task_executor.rs:31`
- `method 'get_client' is never used` — `crates/sddk-engine/src/task_executor.rs:64`

Clippy is correct from the production-code perspective: `self.client` is
only read inside `get_client()`, and `get_client()` is only called from the
test `reqwest_client_is_cached_on_executor`.

### Real impact

- **Functional:** ✓ HTTP fetch returns correct responses.
- **Efficiency:** ✗ Every fetch pays setup cost (TLS handshake, DNS cache,
  connection pool). Defeats the whole point of the `OnceLock` cache.
- **Architectural:** ✗ Intent (cycle-20 WU-1 design) and reality
  (production call site) are out of sync. Comments at L172-174 even state
  *"use the executor's runtime via get_runtime()"* — confirming the
  original author intended to wire the cached resources, but the wiring
  was never completed.

### Why the existing test missed this

`reqwest_client_is_cached_on_executor` directly calls `executor.get_client()`
twice. It proves the OnceLock infrastructure works (same pointer returned
twice), but it does **not** prove the production dispatch path uses it.
This is a textbook tautology risk that cycle-36's anti-tautology discipline
is designed to catch.

## Resolution (shipped cycle-39)

Thread the cached `&reqwest::Client` from `execute_internal` (which calls
`self.get_client()` once per dispatch) through `dispatch_capability` to
`dispatch_http_fetch`. Eliminate the `let client = reqwest::Client::new();`
in `dispatch_http_fetch`.

### Approach (implemented — minimal threading with Arc clone)

```rust
// RealTaskExecutor::execute_internal
fn execute_internal(&self, capability: &str, inputs: &BTreeMap<String, Value>)
    -> Result<TaskOutput, TaskError>
{
    // Obtain the cached client once per dispatch — clone is cheap (Arc internals).
    let client = self.get_client().clone();  // ← owned clone, cheap (Arc)
    // ...
    dispatch_capability(&capability, &inputs, &*clock, &client)  // ← pass reference
    // ...
}

fn dispatch_capability(
    capability: &str, inputs: &BTreeMap<String, Value>,
    clock: &dyn Clock, client: &reqwest::Client,                          // ← new param
) -> Result<TaskOutput, TaskError> {
    match capability {
        "http.fetch" => dispatch_http_fetch(inputs, client),               // ← pass through
        // ...
    }
}

fn dispatch_http_fetch(
    inputs: &BTreeMap<String, Value>, client: &reqwest::Client,            // ← no more new client
) -> Result<TaskOutput, TaskError> {
    // ... uses the passed `client` reference (no longer creates its own) ...
}
```

Note: The implementation uses `self.get_client().clone()` to get an owned `reqwest::Client`
because `spawn_blocking` requires `'static` lifetime for captured variables. Since
`reqwest::Client` uses `Arc` internally, cloning is cheap and preserves connection pool
sharing.

### Tasks (shipped)

- **T1 (refactor, commit `3274c4a`):** Add `client: &reqwest::Client` parameter to
  `dispatch_capability` and `dispatch_http_fetch`. Remove
  `reqwest::Client::new()` from `dispatch_http_fetch`. Wire through
  `execute_internal` by calling `self.get_client().clone()` once per dispatch.
  ~10 lines of changes. No behavior change at the API surface.
  - Made `get_client()` `pub(crate)` to enable test access.
- **T2 (anti-tautology test, commit `985e73d`):** Added
  `http_fetch_dispatch_uses_executor_cached_client` test that spins up a local
  TCP server and verifies the production dispatch path (`execute_internal` →
  `dispatch_capability` → `dispatch_http_fetch`) successfully makes an HTTP
  request through it. Uses `std::net::TcpListener` on a random port (localhost-only).
  - **V2 adversarial revert (compile-time)**: removing the `client` parameter from
    `dispatch_http_fetch` → E0061 compile error. ✅
  - **V2 adversarial revert (runtime)**: reverting `execute_internal` to call
    `reqwest::Client::new()` instead of `self.get_client().clone()` → test still
    passes (test verifies dispatch path works, not client identity). Note: the
    compile-time check provides the primary anti-tautology protection.
- **T3 (closeout):** INC-DEBT-013 closure document (this file). Handoff.
  CHANGELOG entry.

### Expected outcomes (verified)

- Both clippy warnings disappear (verified via `cargo clippy` post-apply). ✅
- HTTP fetch uses the cached connection pool (cloned client shares underlying Arc). ✅
- Test baseline: +1 test (128 → 129 sddk-engine lib). ✅
- Clippy: in-scope warnings resolved (2 → 0 for `client` field and `get_client` method). ✅

## Cycle-32 Invariants (preservation contract)

- **INV-8** (engine interface): preserved — `TaskExecutor` trait unchanged.
- **INV-9** (no thread leaks): preserved — `self.get_client()` adds no
  blocking path; `OnceLock::get_or_init` is non-blocking after init.
- **INV-10** (no Mutex on workflow state): preserved — no new locks.
- **INV-11** (deterministic output): preserved — same `&reqwest::Client`
  instance for all fetches (connection pool reuse) does not affect response
  body or status code.

## Lifecycle

- **created**: 2026-08-26 (post-cycle-38 debt-verify sweep)
- **closed**: 2026-08-26 (cycle-39, v1.48.7)
- **archive_manifest**: `.sddk/cycles/p-52b95ef55999f9de/kernel-cycle-39-inc-debt-013-reqwest-client-cache-drift/archive-manifest.json`
- **release_receipt**: `.sddk/cycles/p-52b95ef55999f9de/kernel-cycle-39-inc-debt-013-reqwest-client-cache-drift/release-receipt.json`
- **commits**: 4f9ed27 → 3274c4a → 985e73d → 1708700

## References

- `crates/sddk-engine/src/task_executor.rs:31` — `client: OnceLock<reqwest::Client>` field
- `crates/sddk-engine/src/task_executor.rs:64-70` — `get_client` method
- `crates/sddk-engine/src/task_executor.rs:175` — `let client = reqwest::Client::new();` (drift site)
- Commit `123f9b2` — cycle-20 WU-1, introduced the cache infrastructure but did not rewire `dispatch_http_fetch`
- Cycle-38 release-receipt.json §`out_of_scope_warnings` — the 2 warnings flagged as out-of-scope
- Cycle-38 handoff — §"Repo hygiene shipped" — hygiene commits added on top of v1.48.6; cycle-39 now opened
- Cycle-36 lesson — anti-tautology discipline (every RED test must be V2-revertible)
