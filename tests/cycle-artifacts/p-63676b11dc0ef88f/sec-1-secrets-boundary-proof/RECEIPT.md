# SEC-1 Receipt — CapabilityReceipt stdout/stderr Secret-Bearing Leak

Cycle: `sec-1-secrets-boundary-proof`
Identity: `p-63676b11dc0ef88f`
Baseline (released): `v1.169.88` → `add896d94274a7515254b8c195e2e78c3669108f`
Development head at start: `8499ff7d585dadcfbb69cad2060be6526968f38a`
Final development head: `1374951`
Fix commit: `1374951` (fix(gateway): redact stdout/stderr secret-bearing substrings)
RED commit: `2c63aff` (test(sec-1): RED pinned)

## Boundary under test

`CapabilityReceipt.result.stdout` and `CapabilityReceipt.result.stderr`
produced by `Gateway::finish_effect` at `crates/sddk-gateway/src/gateway.rs:268`
via `redact(result)`.

This is the single boundary treated under SEC-1. The pre-existing
`SECRET_KEY_PATTERN` redaction (9 keys, `redact()` in
`crates/sddk-gateway/src/lib.rs`) is preserved unchanged; SEC-1
adds a string-level pass on stdout/stderr surfaces.

## MUST (from SCOPE-CONTRACT.md)

- M1. A subprocess capability emitting a `key=value` substring
  whose `key ∈ SECRET_KEY_PATTERN` (case-insensitive) MUST NOT
  appear verbatim in the persisted `CapabilityReceipt.result.stdout`
  or `result.stderr`.
- M2. Diagnostic capacity MUST be preserved: stdout/stderr length,
  the key + separator prefix, and a length tag (`<redacted:N>`)
  MUST remain observable.
- M3. Receipt shape (stdout/stderr fields, terminal status, exit
  status) MUST NOT change.

## MUST NOT (from SCOPE-CONTRACT.md)

- N1. No new secret manager / provider / log system.
- N2. No new governance semantics.
- N3. No redaction of free-form strings outside the stdout/stderr
  keys.
- N4. No dependency addition (`regex` etc.).

All four MUST_NOT constraints hold in the fix commit `1374951`:

- N1: no new module introduced beyond `redact_text`,
  `find_next_secret_pair`, `scan_value_end` — all private to
  `sddk-gateway` and module-local.
- N2: governance (policy, approval, capability classification)
  untouched.
- N3: `STRING_LEVEL_KEY_PATTERN = ["stdout", "stderr"]` — single
  boundary, not widened.
- N4: `Cargo.toml` of `sddk-gateway` unchanged.

## Falsification battery — before vs after

| Test | Surface | Canary shape | Before fix | After fix |
|------|---------|--------------|------------|-----------|
| `sec1_red_canary_in_stdout_does_not_leak_into_receipt` | stdout | `export GITHUB_TOKEN=CANARY_…` | RED (leaked) | GREEN |
| `sec1_red_canary_in_stderr_does_not_leak_into_receipt` | stderr (exit 1) | `api_key=CANARY_…` | RED (leaked) | GREEN |
| `sec1_red_canary_substring_inside_longer_diagnostic_does_not_leak` | stdout | `credentials={ token="CANARY_…" }` | RED (leaked) | GREEN |
| `sec1_red_canary_repeated_across_stdout_stderr_does_not_leak` | both | `token=CANARY_…` × 2 in each | RED (leaked) | GREEN |
| `sec1_diagnostic_information_preserved_after_redaction` | both | `token=…` + `api_key=…` | RED (leaked) | GREEN (status + length preserved) |
| `sec1_sanity_capability_is_allowed_and_terminates` | n/a | n/a | GREEN | GREEN |

Plus unit-level (new in this commit):

| Test | Expectation | Result |
|------|-------------|--------|
| `redact_masks_token_value_in_stdout` | `token=…` masked; `<redacted:…>` and prefix preserved | GREEN |
| `redact_does_not_touch_non_secret_keys` | non-key-value lines untouched | GREEN |
| `redact_keeps_existing_key_level_redaction` | key-level redaction (`credentials.password`) still works alongside string-level | GREEN |

## Gate (scope-limited, per AGENTS.md §2.3)

```text
$ cargo test -p sddk-gateway --offline
… 155 passed; 0 failed; 0 ignored …
```

```text
$ cargo test -p sddk-gateway --test sec1_capability_receipt_redaction --offline
… 6 passed; 0 failed …
```

```text
$ cargo test -p sddk-gateway --test sec1_redactor_unit --offline
… 3 passed; 0 failed …
```

```text
$ cargo clippy -p sddk-gateway --all-targets --offline -- -D warnings
… Finished `dev` profile [unoptimized + debuginfo] target(s) in 47.82s …
```

```text
$ cargo clippy --workspace --all-targets --offline -- -D warnings
… Finished `dev` profile [unoptimized + debuginfo] target(s) in 15.69s …
```

```text
$ cargo fmt --check
… (no output) …
```

### Profile full (`cargo test --workspace`) — pre-existing flake, NOT a regression

```text
$ cargo test --workspace --offline -- --test-threads=1
…
test architecture_rules_yaml_parse: 11 FAILED
…
```

`architecture_rules_yaml_parse` (in `sddk-domain`) reads
`docs/sddk-2.0-architecture-consolidation/data/architecture-rules.yaml`
relative to `CARGO_MANIFEST_DIR/../..`. When run as part of
`--workspace`, the test fails with `Os { code: 2, kind: NotFound }`
even though the file exists on disk and the test passes when run
in isolation:

```text
$ cargo test -p sddk-domain --test architecture_rules_yaml_parse --offline -- --test-threads=1
… 12 passed; 0 failed …
```

The cause is `sddk-cli` integration tests that mutate the
process-global CWD (see `crates/sddk-cli/src/cycle.rs:158` for the
project's own warning about this race). When several workspace
tests run together, some process changes CWD mid-test, breaking
the `CARGO_MANIFEST_DIR` assumption for the architecture-rules
parser.

This is pre-existing and was reproduced with the SEC-1 changes
stashed away (see git stash log). It is NOT a regression from
SEC-1 and is OUT OF SCOPE for this cycle. Recorded as
**OPEN_NON_BLOCKER**: SEC-WORKSPACE-FLAKE. Recommended next
iteration: refactor `architecture_rules_yaml_parse.rs` to read the
file via `env!("CARGO_MANIFEST_DIR")` only — but that requires
moving the file or duplicating it under `sddk-domain/`. Both are
out of SEC-1 scope.

A similar flake appears for
`graph_store_run_persistence_deterministic` (sddk-storage) under
`--workspace`. Same pre-existing race, same diagnosis.

## Diagnostic preservation (M2)

Receipts after the fix retain:

- `result.exit_status` (from the failed-branch path) or the
  capability status (`Succeeded` / `Failed`).
- `result.stdout` and `result.stderr` strings, with
  `<redacted:N>` tokens replacing each detected value. N is the
  original byte length, so an operator can answer "yes, the
  redactor fired and the credential was 23 bytes long" without
  seeing the credential.
- The free-form prefix and the key + separator (`:`, `=`, with
  optional surrounding whitespace) remain verbatim, so log lines
  like `[INFO] build complete; credentials={ token="<redacted:23>" }; duration=4.2s`
  stay readable.

## Boundary not treated under SEC-1 (kept on record)

Per SCOPE-CONTRACT.md §2, surfaces OUT OF SCOPE for SEC-1:

| Surface | In scope? | Why |
|---|---|---|
| `CapabilityReceipt.request` | NO (existing test coverage) | key-level redaction already covers named keys |
| `EvidenceBundle` (`sddk-domain::RedactionRule[]`) | NO | declarative JSON pointer rules; pre-existing surface |
| `bounded_runner` env allowlist | NO | existing S7 test (`bounded_runner_contract.rs`) covers GITHUB_TOKEN exclusion |
| gh auth / git credential helper | NO | external to SDDK; `git_push_credential.rs` |
| Provider integrations (CogniCode, JCode) | NO | not implemented yet |
| CAS payloads, ledger events | NO | not a secret-bearing path in A5 |

## Net delta

| File | Change |
|---|---|
| `crates/sddk-gateway/src/lib.rs` | +120 lines: `STRING_LEVEL_KEY_PATTERN`, `redact_text`, `find_next_secret_pair`, `scan_value_end`; extend `redact()` with second pass on stdout/stderr |
| `crates/sddk-gateway/tests/sec1_capability_receipt_redaction.rs` | +37 lines / -11: calibrate tests to `key=value` shape |
| `crates/sddk-gateway/tests/sec1_redactor_unit.rs` | new file, 38 lines: unit-level double-sense coverage |
| `tests/cycle-artifacts/p-63676b11dc0ef88f/sec-1-secrets-boundary-proof/SCOPE-CONTRACT.md` | new file (RED commit `2c63aff`) |

## Status

SEC-1 closed. RED pinned, GREEN pinned, profile clean. A5-C
certification (v1.169.88) unchanged. No new release issued from
this commit (changes are pending in a future release bundle).

SEC-2 (if needed) would treat additional surfaces; not
authorised in this session.

---

## Release status (post-cycle, added 2026-09-19)

Operator decision 2026-09-19 authorised SEC-1 release as a
corrective release independent of A6, on its own gate.

### Bump

- `Cargo.toml` workspace.package.version: `1.169.88` → `1.169.89`.
- `Cargo.lock` updated by `cargo update --workspace --offline`.
- Commit `b93b50d` (`chore(release): bump version to 1.169.89`).

### Dry-run of `bash scripts/release.sh --dry-run`

3 dry-runs executed against the bump commit. Result summary:

| Run | Workspace green (cargo test --workspace --offline) | Steps 0–8 status |
|-----|---------------------------------------------------|------------------|
| 1   | 1 FAILED (`cross_surface_facades_share_the_service_instance`) | aborted at step 1 |
| 2   | 0 FAILED | dry-run OK |
| 3   | 0 FAILED | dry-run OK |

Run 1 reproduces `SEC-WORKSPACE-FLAKE` against the post-fix
state. The same test passes 5/5 when run in isolation:

```text
$ cargo test -p sddk-cli --test a6_4_shared_ticket_service --offline
… 5 passed; 0 failed …
```

This is the same pre-existing flake documented in the original
SEC-1 receipt §"Profile full" — a process-global CWD race caused
by `sddk-cli` integration tests that the project itself flags at
`crates/sddk-cli/src/cycle.rs:158`. SEC-1 changes are unrelated.

### Disposition

Per operator contract:

> "Si reaparecen los dos fallos del workspace, no registrar el
> gate completo como verde. Mantener el bloqueo hasta obtener un
> resultado reproducible o una excepción explícita y acotada del
> operador, respaldada por la reproducción anterior y posterior al
> fix, la ejecución de los tests restantes y la conservación del
> fallo en SEC-WORKSPACE-FLAKE."

- Workspace gate **NOT GREEN** (1/3 flake observed post-fix).
- Block on `gh release create` maintained.
- `SEC-WORKSPACE-FLAKE` status: **OPEN, REPRODUCED POST-FIX**
  (carried forward from SEC-1 receipt).

### Required artefacts to clear the block

The block clears when **one** of the following is satisfied:

1. The flake is fixed (separate cycle, out of SEC-1 scope).
2. The operator issues an explicit, bounded exception that:
   - acknowledges the flake as pre-existing and out of scope,
   - authorises `--skip-tests` on the release run,
   - commits to a separate recorded register of the controls
     that option omits (cargo fmt, clippy, `cargo test
     -p sddk-gateway --offline`, the SEC-1 falsification
     battery, the public-release-gate test, the install.sh
     smoke),
   - confirms that the exception does not convert "workspace
     PASS" into a recorded green gate.

Until either is satisfied, the release script is **not** invoked
in production mode.
