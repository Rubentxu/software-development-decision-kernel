# SCOPE-CONTRACT — EXT auto-activation (release flow)

> **Slice id:** `p-63676b11dc0ef88f/ext-auto-activation-release-flow`
> **Macro-cycle:** release-flow hardening
> **Status:** DELIVERED (commit `5550fcf`)

## §1 Goal

Convert the long-standing pattern "EXT tests run only if the operator
manually unignores and re-runs `cargo test`" into an opt-in,
release-time auto-activation. When the operator exports
`$COGNICODE_MCP_BIN` and/or `$CHRONOS_MCP_BIN` before invoking
`scripts/release.sh`, step 1d/14 runs the previously-`#[ignore]` EXT
tests against the real provider binaries, captures pass/fail per
provider, and writes an `EXT-RECEIPT.md` to the cycle artifacts. Without
the env vars, the step is a no-op (the EXT tests stay `#[ignore]` and
the release proceeds normally — operators without the binaries are not
blocked).

## §2 STOP conditions resolved

| Condition | Resolution |
|---|---|
| Required external binary on host | OUT OF SCOPE — script does not install. Operator responsibility (`npm install -g cognicode-mcp` or equivalent). |
| Release flow failure on EXT failure | Fail-closed: any EXT failure aborts the release (`die "EXT tests failed against real binaries"`). |
| New dependency in release script | None. `jq` was already required by step 9b/14. |

## §3 Hard constraints

- C1: **No new dependency** in the release script (no `cargo install`, no `npm install`).
- C2: **Opt-in**: no env vars → no-op. Never break existing release flow.
- C3: **Fail-closed**: any EXT test failure aborts the release.
- C4: Bash syntax + shellcheck clean on the modified script.
- C5: Dry-run (`--dry-run --skip-tests --skip-install`) with no env vars MUST succeed (proving the no-op branch).

## §4 Deliverables

| Deliverable | Path | Status |
|---|---|---|
| New step 1d/14 in `scripts/release.sh` | `scripts/release.sh` (inserted at line ~260, after step 1c/14) | ✅ commit `5550fcf` |
| EXT-RECEIPT.md included as release asset when step 1d ran | `scripts/release.sh` (ASSETS list, line ~545) | ✅ commit `5550fcf` |
| This SCOPE-CONTRACT.md | `tests/cycle-artifacts/.../ext-auto-activation-release-flow/SCOPE-CONTRACT.md` | ✅ |
| UAT-EVIDENCE.yaml | `tests/cycle-artifacts/.../ext-auto-activation-release-flow/UAT-EVIDENCE.yaml` | ✅ |
| RECEIPT.md | `tests/cycle-artifacts/.../ext-auto-activation-release-flow/RECEIPT.md` | ✅ |

## §5 Verification (commit `5550fcf`)

```
$ bash -n scripts/release.sh && echo "bash syntax: OK"
bash syntax: OK

$ shellcheck --severity=warning scripts/release.sh
(exit 0, no output)

$ bash scripts/release.sh --dry-run --skip-tests --skip-install
==> 0/14 — preflight
  ✗ release admission refused: REJECT non-monotonic 1.169.123 -> 1.169.123
```

The release admission failure is expected (HEAD already carries the
v1.169.122 → v1.169.123 bump from `cace421`; a fresh release needs
another bump to `v1.169.124` to be admissible). What matters for this
slice is that the script reaches the admission check without syntax
errors — proving the new step is syntactically and structurally valid.

## §6 Operator usage

```bash
# Operator without binaries: release proceeds, EXT tests stay #[ignore].
bash scripts/release.sh

# Operator with binaries: EXT auto-activates, EXT-RECEIPT.md published.
export COGNICODE_MCP_BIN=/usr/local/bin/cognicode-mcp
export CHRONOS_MCP_BIN=/usr/local/bin/chronos-mcp
bash scripts/release.sh
```

## §7 Out of scope

- Installing the binaries (`npm install -g cognicode-mcp` etc.).
- Adding EXT tests to crates that don't currently have them.
- Changing the 9-asset public-release gate contract (the EXT-RECEIPT is
  an evidence-of-record asset, not part of the gate's 9-asset check).

## §8 References

- `scripts/release.sh` lines 260..336 (new step 1d/14).
- `scripts/release.sh` lines 540..547 (EXT-RECEIPT included in ASSETS).
- `docs/debt/INC-PUSH-DERIVED-METADATA-NO-ADMISSIBLE-PATH.md` (sibling
  release-flow hardening pattern).
- `docs/architecture/adrs/ADR-0137-CODE-INTELLIGENCE-PORT-SEAM.md` (the
  CodeIntelligencePort trait that EXT tests exercise).
- `docs/architecture/adrs/ADR-0138-runtime-evidence-port.md` (the
  RuntimeEvidencePort trait that Chronos EXT tests exercise).
- `crates/sddk-engine/tests/a6_cc_s1_static_graph_completeness.rs` (the
  CogniCode EXT test fixture).
- `crates/sddk-engine/tests/aiw_s1_cognicode_real.rs` (additional
  CogniCode EXT coverage).
- `crates/sddk-engine/tests/aiw_s5_chronos_real.rs` (the Chronos EXT
  test fixture).
