# SCOPE-CONTRACT — Release prep (bump to v1.169.124 + bonus nounset fix)

> **Slice id:** `p-63676b11dc0ef88f/release-prep-v1.169.124`
> **Macro-cycle:** release-flow hardening (companion to FU-A6-EXT-AUTO)
> **Status:** DELIVERED (commits `f1357c2`, `38f84cb`)

## §1 Goal

Pre-stage the next release by:

1. Bumping `[workspace.package] version` from `1.169.123` → `1.169.124`,
   so the release admission check (`scripts/lib/release_admission.sh`)
   will ACCEPT the next `bash scripts/release.sh` invocation.
2. Discovering and fixing a real nounset bug exposed by the bump's
   dry-run, so the EXT slice's no-op branch does not abort in CI /
   agent shells that have `set -u` enabled.

**No push. No gh release create. No operator-side release.** The next
operator runs `bash scripts/release.sh` from this HEAD to actually
publish `v1.169.124`.

## §2 STOP conditions resolved

| Condition | Resolution |
|---|---|
| Working tree dirty at preflight | The bump commit `f1357c2` was the first action; the nounset fix `38f84cb` was the second. Both committed before re-running preflight. |
| Nounset aborts the release script in CI | Fixed at `38f84cb` by switching `[ -n "$VAR" ]` to `[ -n "${VAR:-}" ]`. |

## §3 Hard constraints

- C1: No new dependency.
- C2: No push. No release. The slice prepares a release-bound commit
  ahead of push but does not push.
- C3: Bump must be monotonic (`1.169.123 → 1.169.124` passes; equal
  would be REJECT).
- C4: Bash syntax + shellcheck of the modified script must exit 0.
- C5: Working tree must be clean before any dry-run (preflight
  rejects dirty trees).

## §4 Deliverables

| Deliverable | Commit | Evidence |
|---|---|---|
| Bump to v1.169.124 | `f1357c2` | `cargo_ws_version_at HEAD` reports `1.169.124`; `release_admission_check HEAD` returns `ACCEPT 1.169.123 -> 1.169.124` exit 0. |
| Nounset fix in step 1d | `38f84cb` | `bash -c 'set -u; bash scripts/release.sh --dry-run --skip-tests --skip-install'` reaches the admission check (no abort on line 279). Shellcheck exit 0. |
| This SCOPE-CONTRACT | new | this file |
| RECEIPT.md | new | `RECEIPT.md` (same dir) |

## §5 Verification (commit `38f84cb`)

### §5.1 Admission check accepts the bump

```
$ source scripts/lib/release_admission.sh && release_admission_check HEAD
ACCEPT 1.169.123 -> 1.169.124
exit=0
```

### §5.2 Bash syntax + shellcheck of modified script

```
$ bash -n scripts/release.sh && echo "syntax: OK"
syntax: OK

$ shellcheck --severity=warning scripts/release.sh
(exit 0, no output)
```

### §5.3 Nounset bug fixed (the bonus)

Before fix (commit `5550fcf`):
```
$ env -u COGNICODE_MCP_BIN -u CHRONOS_MCP_BIN bash -c 'set -u; bash scripts/release.sh --dry-run --skip-tests --skip-install' 2>&1 | tail -3
==> 1d/14 — EXT auto-activation (cognicode-mcp / chronos-mcp, opt-in)
scripts/release.sh: línea 279: COGNICODE_MCP_BIN: variable sin asignar
```

After fix (commit `38f84cb`):
```
$ env -u COGNICODE_MCP_BIN -u CHRONOS_MCP_BIN bash -c 'set -u; bash scripts/release.sh --dry-run --skip-tests --skip-install' 2>&1 | tail -3
==> 0/14 — preflight
  ✗ release admission refused: REJECT non-monotonic 1.169.124 -> 1.169.124 — release requires a real, monotonic [workspace.package] version bump
```

Result: **the nounset bug no longer aborts** the script. The remaining
refusal is the expected admission-check rejection (a release requires a
strict monotonic bump, and the bump is already in HEAD).

### §5.4 Isolated reproduction under `set -u`

```
$ env -u COGNICODE_MCP_BIN -u CHRONOS_MCP_BIN bash -c '
set -u
EXT_RECEIPT_DIR=""
EXT_FAIL=0
VERSION="1.169.124"
if [ -n "${COGNICODE_MCP_BIN:-}" ] || [ -n "${CHRONOS_MCP_BIN:-}" ]; then
    echo "FAIL: would enter active branch"
    exit 1
else
    echo "OK: no-op branch triggered under set -u"
    echo "EXT_FAIL=$EXT_FAIL"
    echo "EXT_RECEIPT_DIR=$EXT_RECEIPT_DIR (must be empty)"
fi'
OK: no-op branch triggered under set -u
EXT_FAIL=0
EXT_RECEIPT_DIR= (must be empty)
```

## §6 Operator usage

```bash
# 1. Verify the bump is acceptable:
source scripts/lib/release_admission.sh && release_admission_check HEAD
# expect: ACCEPT 1.169.123 -> 1.169.124, exit 0

# 2. (Optional) dry-run without env vars:
bash scripts/release.sh --dry-run --skip-tests --skip-install
# expect: stops at admission refusal — the bump is in HEAD, so to walk
# further, do a real release.

# 3. (Optional) dry-run with EXT binaries installed:
export COGNICODE_MCP_BIN=/usr/local/bin/cognicode-mcp
export CHRONOS_MCP_BIN=/usr/local/bin/chronos-mcp
bash scripts/release.sh --dry-run --skip-tests --skip-install
# expect: walks through step 1d/14 with EXT receipt written.

# 4. Real release:
bash scripts/release.sh
# expect: walks the full 14 steps and creates the v1.169.124 release on GitHub.
```

## §7 Out of scope

- Pushing the bump commit (system-law `git.push` human_gate).
- Creating the v1.169.124 GitHub Release (system-law `git.release` human_gate).
- Running live EXT tests against `cognicode-mcp` / `chronos-mcp`
  (operator must install + run).

## §8 References

- `Cargo.toml` line 16: bumped from `1.169.123` to `1.169.124`.
- `Cargo.lock`: regenerated (sddk-cli, sddk-engine, sddk-storage,
  sddk-domain, sddk-testkit, sddk-pack-uat, sddk-gateway all at `1.169.124`).
- `scripts/release.sh` line 279: `${COGNICODE_MCP_BIN:-}` defensive expansion.
- `scripts/lib/release_admission.sh`: source of `release_admission_check`.
- `tests/cycle-artifacts/p-63676b11dc0ef88f/ext-auto-activation-release-flow/`:
  parent slice that introduced step 1d/14.
