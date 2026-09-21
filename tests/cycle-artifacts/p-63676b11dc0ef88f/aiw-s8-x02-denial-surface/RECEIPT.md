# RECEIPT — AIW-S8 X02 — Denial surface (raw args/secretos)

> **Slice:** `p-63676b11dc0ef88f/aiw-s8-x02-denial-surface`
> **Date:** 2026-09-21
> **Base HEAD:** 7fca3d9
> **Closes:** SCOPE-CONTRACT §S8-STOP-1 (X02)

## What was built

- `crates/sddk-gateway/src/denial_surface.rs` (NEW): denial layer for host
  action requests. `HostActionRequest::evaluate(&DenialPolicy)` denies at
  evaluation time: empty action id, secret-prefixed args, raw-byte args
  (NUL), non-UTF-8 args, oversized payloads (> 64 KiB default). Default
  secret prefixes: `SECRET_`, `TOKEN_`, `PASSWORD_`, `API_KEY_`.
- `crates/sddk-gateway/src/lib.rs`: `pub mod denial_surface;` added.
- `crates/sddk-gateway/tests/aiw_s8_x02_denial_surface.rs` (NEW): 7
  integration tests incl. CLI-shaped full-path and rendered-verdict
  zero-leak scan.

## Verification (OBSERVED)

| Gate | Result |
|---|---|
| `cargo fmt --check` | exit 0 |
| `cargo clippy -p sddk-gateway --all-targets -- -D warnings` | 0 warnings |
| `cargo test -p sddk-gateway --lib denial_surface` | 6 passed; 0 failed |
| `cargo test -p sddk-gateway --test aiw_s8_x02_denial_surface` | 7 passed; 0 failed |
| `cargo build --release -p sddk-gateway` | Finished `release` profile |

Total new tests: **13** (6 unit + 7 integration).

## Zero-leak guarantee

`DenialReason` variants carry only: the matched prefix string, payload
sizes, or the bare fact of malformed bytes. Pinned by dedicated tests that
assert the secret value (`SECRET_TOKEN=hello` → only `SECRET_`-class prefix
escapes) and that the `Debug`-rendered verdict contains no raw arg content.

## Surprises / notes

- `missing_docs` (workspace-wide deny) required docs on enum-variant fields
  and on the restored `OversizedPayload` variant doc.
- Host-side adoption (wiring `evaluate()` into `AgentHost` dispatch in
  `sddk-engine`) is explicitly out of scope for this slice; the surface is
  public and ready for that follow-up.
