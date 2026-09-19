# SEC-1 — Secrets Boundary Proof

> Cycle: `p-63676b11dc0ef88f/sec-1-secrets-boundary-proof`
> Status: **IN PROGRESS**

## Scope contract

**SECRET HANDLING / OBSERVABLE OUTPUT BOUNDARY**

ONE boundary under test, ONE battery of falsification tests.

### MUST

- Avoid secrets supplied by the supported credential mechanisms
  appearing in the observable outputs covered by this cycle.
- Preserve the ability to diagnose failures without revealing the
  secret value itself.
- Distinguish inspected flows, uninspected flows, and flows that
  require a follow-up cycle.

### MUST_NOT

- Introduce a new secret manager.
- Introduce a new credential provider.
- Introduce a second logging system.
- Introduce new Governance semantics or a parallel observability
  architecture.

## Baseline identity

| Field | Value |
|---|---|
| `released_baseline` | `v1.169.88` → `add896d94274a7515254b8c195e2e78c3669108f` |
| `development_head` | `8499ff7d585dadcfbb69cad2060be6526968f38a` |
| `workspace_version` | `1.169.88` |
| `cycle_id` | `sec-1-secrets-boundary-proof` |

## Selected boundary (first shared chokepoint)

`CapabilityReceipt.result.stdout` and `.result.stderr` produced by
`Gateway::execute_governed` and finalized by
`Gateway::finish_effect` in `crates/sddk-gateway/src/gateway.rs`.

The `redact()` function in `crates/sddk-gateway/src/lib.rs` masks
values under `SECRET_KEY_PATTERN` (9 keys: `api_key`,
`api_key_id`, `authorization`, `auth_token`, `cookie`,
`credential`, `password`, `secret`, `token`) but **only at JSON key
level**, not inside string values. When a capability emits a
secret via stdout/stderr that is not wrapped in one of those JSON
keys, the secret passes through verbatim into the persisted
`CapabilityReceipt.result` (which is observable via `sddk
capability get`, doctor, and the JSON dump in cycle-artifacts).

This is the first shared chokepoint the cycle addresses.

## Surfaces inventory (preflight)

| Surface | Under test in SEC-1 | Notes |
|---|---|---|
| `CapabilityReceipt.result.stdout` (capability output) | **YES** | primary boundary |
| `CapabilityReceipt.result.stderr` (capability output) | **YES** | primary boundary |
| `CapabilityReceipt.request` (capability input) | YES (existing test) | `redact()` covers named keys; not a fresh leak path |
| `EvidenceBundle` (`sddk-domain::RedactionRule[]`) | NO | declarative rules by JSON pointer; pre-existing surface, not a new chokepoint |
| `bounded_runner` env allowlist (`bounded_runner_contract.rs`) | NO | already covered by existing tests (S7 GITHUB_TOKEN excluded from child env) |
| `gh auth` / git credential helper | NO | external to SDDK; covered by `git_push_credential.rs` |
| `sddk-cli` argv / env reads | NO | CLI does not read `GITHUB_TOKEN`/`SDDK_TOKEN` directly; secrets arrive via the gateway |
| Provider integrations (CogniCode, JCode) | NO | not yet implemented; future work |
| Agent context serialization | NO | not in scope of A5; future work (SEC-2 if needed) |
| CAS payloads, ledger events | NO | not a secret-bearing path in A5 |

## Falsification plan

1. RED test: a capability emits `CANARY_GH_TOKEN_abc123XYZ` via
   stdout; after `execute_governed` + `finish_effect`, the
   persisted `CapabilityReceipt.result.stdout` must NOT contain
   the canary value.
2. RED test (cross-channel): same canary via stderr; same
   expectation.
3. RED test (substring): canary embedded inside a longer
   diagnostic line; same expectation.
4. RED test (repetition): canary emitted multiple times; the
   receipt must contain zero occurrences after redaction.
5. GREEN after fix: all four tests pass; existing tests still
   pass.

## Diagnostic preservation check

The fix must not erase useful information. After redaction:

- `stdout` length and a non-secret snippet prefix (first N chars)
  must remain so operators can confirm "the capability emitted N
  bytes" without seeing the secret.
- Error messages must still include the capability name, the
  exit_status, and a non-secret stderr snippet.
- The number of redactions must be observable (so operators can
  answer "yes, the redaction layer fired").

## Acceptance criteria

PASS requires:

1. RED test is committed before the fix (falsification pinned).
2. GREEN test is committed after the fix (proof of closure).
3. All four RED→GREEN cases pass.
4. Diagnostic preservation checks pass.
5. No existing test in `sddk-gateway/tests/` regresses.
6. Profile: `cargo fmt --check`, `cargo clippy --workspace
   --all-targets -- -D warnings`, `cargo test --workspace
   --offline -- --test-threads=1`.

## Disposition at exit

Will report:
- Which surfaces were verified by executable tests.
- Which surfaces remain unverified.
- Which leaks were found and closed (or none).
- Whether G11 / R14 can advance, stay at NOT VERIFIED / OPEN, or
  partially close.

No retroactive change to A5-C certification.
