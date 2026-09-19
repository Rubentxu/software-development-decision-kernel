# Handoff — SEC-1 closed, awaiting J2 proposal review

Date: 2026-09-19
Branch: `main` (HEAD local `f6e1906`, not pushed — release would
require a `[workspace.package] version` bump, not in scope here).

## A5-C status (unchanged)

- `v1.169.88` → `add896d94274a7515254b8c195e2e78c3669108f` is
  the certifying release.
- `docs/architecture/a5/A5-C-BASE-PRODUCTION-READY-CERTIFICATION.md`
  remains the authority.
- G11 = NOT VERIFIED. R14 = OPEN_NON_BLOCKER. Not retroactively
  reframed; the historical cert is preserved as-is.

## SEC-1 — closed

Boundary: `CapabilityReceipt.result.stdout` /
`.stderr` (single site: `Gateway::finish_effect` at
`crates/sddk-gateway/src/gateway.rs:268`).

Outcome: RED pinned (`2c63aff`), GREEN pinned (`1374951`),
receipt committed (`f6e1906`).

Surface-level delta:

| File | Lines | Why |
|---|---|---|
| `crates/sddk-gateway/src/lib.rs` | +120 | adds `redact_text` + `find_next_secret_pair` + `scan_value_end`; extends `redact()` with a second pass on stdout/stderr string values |
| `crates/sddk-gateway/tests/sec1_capability_receipt_redaction.rs` | +37/-11 | 6 falsification tests (RED #1..#4 + diagnostic preservation + sanity) |
| `crates/sddk-gateway/tests/sec1_redactor_unit.rs` | new, 38 | 3 unit tests for the new helper |
| `tests/cycle-artifacts/p-63676b11dc0ef88f/sec-1-secrets-boundary-proof/{SCOPE-CONTRACT,RECEIPT}.md` | new | scope + receipt |

Receipt: `tests/cycle-artifacts/p-63676b11dc0ef88f/sec-1-secrets-boundary-proof/RECEIPT.md`

### SEC-1 gates (scope-limited)

- `cargo test -p sddk-gateway --offline` → 155/155 PASS.
- `cargo clippy -p sddk-gateway --all-targets -- -D warnings` → clean.
- `cargo clippy --workspace --all-targets -- -D warnings` → clean.
- `cargo fmt --check` → clean.

### Pre-existing flake (NOT a regression)

`cargo test --workspace --offline -- --test-threads=1` fails on
`architecture_rules_yaml_parse` (sddk-domain) because that test
reads `architecture-rules.yaml` relative to `CARGO_MANIFEST_DIR`
but some other workspace test mutates the process-global CWD
mid-run (project's own warning at
`crates/sddk-cli/src/cycle.rs:158`). Reproduced with SEC-1
changes stashed → pre-existing, out of SEC-1 scope. Logged as
**SEC-WORKSPACE-FLAKE (OPEN_NON_BLOCKER)** in the receipt.

A similar flake on
`graph_store_run_persistence_deterministic` (sddk-storage) has
the same root cause.

### What SEC-1 does NOT cover (per SCOPE-CONTRACT.md)

`CapabilityReceipt.request`, `EvidenceBundle RedactionRule[]`,
`bounded_runner` env, gh auth, provider integrations, CAS,
ledger events. All kept on record; SEC-2 would address them if
authorised.

## Pending risks

1. **Workspace flake (SEC-WORKSPACE-FLAKE).** Not blocking.
   Pre-existing, documented, easy to fix in a separate cycle
   (move `architecture-rules.yaml` under `sddk-domain/test-data/`
   or refactor the test to read via `env!` only).
2. **Push discipline.** `f6e1906` is local. The hook only
   accepts pushes containing either a `Cargo.toml` version bump
   or docs-only paths. Publishing SEC-1 to `origin/main` will
   require a release (e.g. `v1.169.89`) that bundles the
   workspace bump + the fix.
3. **SEC-1 scope is narrow.** SEC-1 falsified stdout/stderr at
   one site. Other free-form output surfaces (CAS, ledger
   payload event messages, agent context serialization) are not
   in scope.

## Proposal ready for review — J2 (JCode anti-corruption adapter)

A separate doc has been written at
`docs/proposals/2026-09-19-jcode-anti-corruption-adapter-PROPOSAL.md`
(SEE BELOW). It does NOT open a cycle. It defines the JCode
anti-corruption layer between the SDDK core and the JCode
host-runtime so that JCode hosts cannot accidentally depend on
internal SDDK types or invoke capabilities the gateway would
deny.

The proposal:

- Defines ONE anti-corruption surface: `JCodeHostAdapter` (a
  trait that exposes only `cycle_id`, `CapabilityReceipt` view,
  and a typed `request_effect` method).
- Identifies the contract boundary: the adapter returns a
  narrow `JCodeReceiptView` (id, capability, status, exit
  status, masked stdout/stderr); it does NOT return the full
  `CapabilityReceipt` or any `serde_json::Value` blob.
- Lists the FALSIFICATION tests for the adapter:
  (a) the adapter cannot be invoked with a denied capability
  by the host;
  (b) the host cannot read `serde_json::Value` directly;
  (c) the masked stdout/stderr round-trip through the adapter
  is identical to the gateway view (i.e. SEC-1 redaction is
  preserved at the adapter boundary).
- Lists the MUST NOTs: no new secret manager, no new log
  system, no new capability taxonomy, no coupling to JCode's
  internal trait tree.

The next cycle, if authorised, would be `sec-2-jcode-boundary`
(adapter surface + same falsification battery, with JCode
host types as the test consumer).

## Action items

None blocking. The user's last instruction was:

1. STOP generic A5-C audits — held.
2. Authorise SEC-1 only — done, RED pinned, GREEN pinned, receipt
   written.
3. Profile at exit — held; receipt reports scoped profile clean
   and the workspace flake as out-of-scope.
4. Handoff → proposal J2 — done.

**No cycle was opened for J2.** The proposal is a markdown
document only.

## Suggested next action (operator decision)

Three options, in order of expected value:

1. **Release SEC-1 as `v1.169.89`.** Bundle the workspace
   bump + the fix + the receipt into a release; the public
   release gate is automated by `bash scripts/release.sh`. This
   pushes the SEC-1 work to `origin/main` and ships the fix to
   users.
2. **Authorise SEC-2 / `sec-2-jcode-boundary` cycle.** Closes
   the adapter surface gap (J2 proposal above) with the same
   falsification discipline. Avoid opening until the operator
   confirms J2 is the right shape.
3. **Fix the workspace flake** (SEC-WORKSPACE-FLAKE) in a
   small, dedicated cycle. Not blocking; clean-up.

I recommend option 1 → option 2 in sequence.
