# Rust Testing Reference

Patterns specifically for the **Rust** half of the pyramid. For deep Rust design (generics, error handling, async, smart pointers), load `rust-patterns` — this file covers the **testing chapter** only.

## Unit tests (the foundation)

```rust
// Inside the module — can access private items
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_expired_token() {
        let token = Token::new(valid_user(), expires_at: past());
        assert!(matches!(token.validate(), Err(AuthError::Expired)));
    }
}

// In a sibling tests/ dir — public API only
// crates/<crate>/tests/integration.rs
use <crate_name>::{Token, validate};

#[test]
fn integration_validates_issued_token() { ... }
```

**Rules**:
- `#[cfg(test)]` next to the code, for white-box tests.
- `tests/` for black-box public API tests (one file per concern).
- Doc tests for every public function whose signature is part of the contract.

## Property-based tests

Use `proptest` for parsers, serializers, and invariant-checking code.

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn roundtrip_serde_json(payload in any::<MyMessage>()) {
        let json = serde_json::to_string(&payload).unwrap();
        let back: MyMessage = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(payload, back);
    }
}
```

For async, use `proptest!` inside a `#[tokio::test]` via `tokio::task::spawn_blocking` or use `proptest-attr` macros for async.

## Async tests (Tokio)

```rust
#[tokio::test]
async fn handler_returns_201_on_valid_payload() {
    let app = build_test_app().await;
    let res = app.post("/v1/executions").json(&valid_payload()).await;
    assert_eq!(res.status(), 201);
}
```

- **One runtime per test.** Do not reuse a global `Runtime`.
- Use `tokio::time::pause()` for time-dependent code.
- Use `tokio::task::yield_now()` to surface scheduling bugs.
- For axum / actix: `tower::ServiceExt::oneshot` to call handlers without a real socket.
- For WebSockets: `tokio-tungstenite` client against an in-process server.

## Database / SQL integration

```rust
#[sqlx::test(migrations = "./migrations")]
async fn execution_persists_and_loads(pool: PgPool) {
    let repo = ExecutionRepo::new(pool);
    let id = repo.create(sample_execution()).await.unwrap();
    let loaded = repo.get(id).await.unwrap();
    assert_eq!(loaded.status, ExecutionStatus::Pending);
}
```

- `sqlx::test` spins up an ephemeral DB per test, applies migrations, drops on teardown.
- For Postgres-specific features, run **testcontainers** Postgres in CI.
- For unit-level DB code, mock at the trait boundary with `mockall` instead of hitting a DB.

## HTTP / API integration

Two patterns, pick by fidelity:

| Need | Pattern |
|---|---|
| Handler logic + status codes | `tower::ServiceExt::oneshot` (no socket, no port) |
| Full stack incl. middleware | `axum::serve` on a random port + `reqwest` client |
| External contract guarantee | `wiremock` / `httpmock` for downstream services |

```rust
// Pure handler test — fast
let app = build_app(state);
let res = app.oneshot(Request::post("/v1/x").body(json).unwrap()).await.unwrap();
assert_eq!(res.status(), 201);

// End-to-end with real server
let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
let addr = listener.local_addr().unwrap();
tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
let client = reqwest::Client::new();
let res = client.post(format!("http://{addr}/v1/x")).json(&json).send().await.unwrap();
```

## Mocks and fakes

| Strategy | When |
|---|---|
| `mockall` | Trait-based dependency, behavior assertions needed |
| Hand-written `#[cfg(test)]` fake | Simpler than mockall, type-safe, fast to read |
| `wiremock` / `httpmock` | Downstream HTTP API in tests |
| `testcontainers` | Real Postgres / Redis / NATS / Kafka in CI |
| `sqlx::test` (in-memory or real) | SQL code paths |

**Rule**: prefer a hand-written fake over `mockall` for repositories — the fake becomes a runnable spec of the contract.

## Benchmarks (criterion)

```rust
// benches/parse_request.rs
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_parse(c: &mut Criterion) {
    c.bench_function("parse_execution_request", |b| {
        b.iter(|| parse_execution_request(black_box(SAMPLE_JSON)))
    });
}
criterion_group!(benches, bench_parse);
criterion_main!(benches);
```

Run with `cargo bench -p crate_name`. Use for hot paths and before/after perf changes.

## Mutation testing (optional, signal-rich)

`cargo-mutants` rewrites code and checks if your tests still pass. If they do, your tests are weak. Run periodically, not on every PR.

## Workspace testing

```bash
# Run a single crate's tests
cargo test -p <crate_name>

# Run a single test by name substring
cargo test -p <crate_name> <test_name>

# Run a single integration test file
cargo test -p <crate_name> --test <integration_file>

# Workspace-wide (slow)
cargo test --workspace

# With coverage
cargo llvm-cov --workspace --html

# Bench
cargo bench -p <crate_name>
```

## References to load

- `rust-patterns` — Ch. 14 (testing), Ch. 9 (error handling), Ch. 16 (async).
- `diagnose` — For hard flakiness or test-timeout bugs.
- `work-unit-commits` — Keep tests in the same commit as the behavior.
