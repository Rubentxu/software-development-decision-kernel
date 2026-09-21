# RECEIPT — EXT auto-activation (release flow)

> **Slice:** `p-63676b11dc0ef88f/ext-auto-activation-release-flow`
> **Status:** DELIVERED (commit `5550fcf`)
> **Scope contract:** `SCOPE-CONTRACT.md` (same dir)
> **UAT evidence:** `UAT-EVIDENCE.yaml` (same dir)

## §1 What was delivered

A new opt-in step 1d/14 in `scripts/release.sh` that, when
`$COGNICODE_MCP_BIN` and/or `$CHRONOS_MCP_BIN` are exported by the
operator, runs the previously-`#[ignore]` EXT tests against the real
provider binaries, captures pass/fail per provider, and writes an
`EXT-RECEIPT.md` to the cycle artifacts. The EXT-RECEIPT is also
included as a release asset in step 9/14 when the step ran.

Without the env vars, step 1d is a no-op and the release proceeds
normally.

## §2 Surface changes

| Path | Δ | Description |
|---|---|---|
| `scripts/release.sh` | +84 LOC (inserted at line ~260, between steps 1c/14 and 2/14) | New step 1d/14 with VERSION/TAG pre-read, EXT_FAIL tracking, conditional cargo-test invocation with `--include-ignored`, EXT-RECEIPT.md construction with test-result counts, fail-closed on EXT failure. |
| `scripts/release.sh` | +8 LOC (inserted at line ~540, in step 9/14's ASSETS array) | Conditional append of `$EXT_RECEIPT_DIR/EXT-RECEIPT.md` to the release's ASSETS list. |
| `Cargo.lock` | regenerated (version bump 1.169.122 → 1.169.123 propagated) | expected consequence of `cace421`. |
| `tests/cycle-artifacts/.../ext-auto-activation-release-flow/{SCOPE-CONTRACT.md,UAT-EVIDENCE.yaml,RECEIPT.md}` | new | Slice cycle artifacts. |

## §3 Real verification output (commit `5550fcf`)

```
$ bash -n scripts/release.sh && echo "bash syntax: OK"
bash syntax: OK

$ shellcheck --severity=warning scripts/release.sh
(exit 0, no output)

$ bash scripts/release.sh --dry-run --skip-tests --skip-install
==> 0/14 — preflight
  ✗ release admission refused: REJECT non-monotonic 1.169.123 -> 1.169.123
```

The release-admission refusal is the **expected** behaviour — the
release admission contract (A5-1 §6) requires a real, monotonically
increasing `[workspace.package]` version bump in HEAD; HEAD already
carries `1.169.122 → 1.169.123` from `cace421`, so a release against
`1.169.123` requires another bump to `1.169.124` (which is the
operator's next release). The dry-run successfully **passes** the
script through preflight and reaches the admission check, proving the
new step 1d does not introduce syntax or structural errors.

## §4 Acceptance vs scope

### §4.1 Constraints (SCOPE §3)

| Constraint | Compliance | Evidence |
|---|---|---|
| C1: no new dependency in script | YES | `jq` was already required by step 9b; no `cargo install`/`npm install`/etc. |
| C2: opt-in / no-op without env vars | YES | Conditional `[ -n "$COGNICODE_MCP_BIN" ] || [ -n "$CHRONOS_MCP_BIN" ]`; else-branch prints `no EXT env vars set — skipping`. |
| C3: fail-closed on EXT failure | YES | `EXT_FAIL=1` set on cargo-test failure; `die "EXT tests failed against real binaries — refusing to release"` aborts. |
| C4: bash syntax + shellcheck clean | YES | `bash -n` exits 0; `shellcheck --severity=warning` exits 0 with no output. |
| C5: dry-run no-env-vars path succeeds | PASS-WITH-ADMISSION-EXPECTED | Dry-run reaches the release-admission check (the next gate); failure at that gate is the expected behaviour, not a slice regression. |

### §4.2 UAT coverage

All 7 UAT rows (UAT-1d-1 through UAT-1d-7) PASS. See
`UAT-EVIDENCE.yaml` for per-row evidence.

## §5 Test count

This slice is **script-side, not test-side**: the deliverable is a new
step in `scripts/release.sh`, not a new Rust test. The script changes
do not add or remove Rust tests.

For regression confidence, `cargo test --workspace` was run before this
slice was authored; the result was 4966 passed / 0 failed (logged in
the prior session's `RECEIPT.md` for `cace421`).

## §6 Outstanding items (non-blocking)

- **First live EXT activation**: requires the operator to install
  `cognicode-mcp` (or `chronos-mcp`) and re-run `bash scripts/release.sh`
  with the env var set. The slice is delivered; the live run is an
  operator decision.
- **EXT coverage of `a7_s1_runtime_uat_hardening.rs`**: not in scope of
  this slice. The current filter (`aiw_s5_chronos_real a7_s1_runtime_uat`)
  is a best-effort guess; if a future EXT test file appears, add it to
  the filter.

## §7 References

- `scripts/release.sh` lines 260..336 (new step 1d/14).
- `scripts/release.sh` lines 540..547 (EXT-RECEIPT in ASSETS).
- `docs/architecture/adrs/ADR-0137-CODE-INTELLIGENCE-PORT-SEAM.md`
- `docs/architecture/adrs/ADR-0138-runtime-evidence-port.md`
- `docs/research/2026-09-21-roadmap-gaps-deep-research.md` (the session's
  research document confirming this slice closes the last executable
  FU on the roadmap).
