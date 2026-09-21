# RECEIPT — Release prep (v1.169.124 + bonus nounset fix)

> **Slice:** `p-63676b11dc0ef88f/release-prep-v1.169.124`
> **Status:** DELIVERED (commits `f1357c2`, `38f84cb`)
> **Scope contract:** `SCOPE-CONTRACT.md` (same dir)

## §1 What was delivered

1. **Bump to `1.169.124`** (`f1357c2`): `Cargo.toml` workspace
   version raised; `Cargo.lock` regenerated so all seven workspace
   crates report `1.169.124`. The release admission check now
   `ACCEPT`s the next `bash scripts/release.sh` invocation.
2. **Bonus nounset fix** (`38f84cb`): step 1d/14 of the release
   script (`scripts/release.sh` line 279) was defensively expanded
   from `[ -n "$COGNICODE_MCP_BIN" ]` to `[ -n "${COGNICODE_MCP_BIN:-}" ]`
   so the no-op branch survives `set -u` in the parent shell.

## §2 Surface changes

| Path | Δ | Description |
|---|---|---|
| `Cargo.toml` | +1/-1 | `version = "1.169.123"` → `"1.169.124"` at line 16. |
| `Cargo.lock` | regenerated | All 7 workspace crates bumped to `1.169.124`. |
| `scripts/release.sh` | +1/-1 | Line 279: defensive expansion of `COGNICODE_MCP_BIN` and `CHRONOS_MCP_BIN`. |
| `tests/cycle-artifacts/.../release-prep-v1.169.124/{SCOPE-CONTRACT.md,RECEIPT.md}` | new | Slice cycle artifacts. |

## §3 Real verification output

### §3.1 Admission check after bump

```
$ source scripts/lib/release_admission.sh && release_admission_check HEAD
ACCEPT 1.169.123 -> 1.169.124
exit=0
```

### §3.2 Bash syntax + shellcheck after fix

```
$ bash -n scripts/release.sh && echo "syntax: OK"
syntax: OK

$ shellcheck --severity=warning scripts/release.sh
exit 0 (no output)
```

### §3.3 Nounset bug fixed (real, observed before/after)

**Before** (commit `5550fcf`, original step 1d):
```
$ env -u COGNICODE_MCP_BIN -u CHRONOS_MCP_BIN bash -c 'set -u; bash scripts/release.sh --dry-run --skip-tests --skip-install'
==> 1d/14 — EXT auto-activation (cognicode-mcp / chronos-mcp, opt-in)
scripts/release.sh: línea 279: COGNICODE_MCP_BIN: variable sin asignar
```

**After** (commit `38f84cb`, with fix):
```
$ env -u COGNICODE_MCP_BIN -u CHRONOS_MCP_BIN bash -c 'set -u; bash scripts/release.sh --dry-run --skip-tests --skip-install'
==> 0/14 — preflight
  ✗ release admission refused: REJECT non-monotonic 1.169.124 -> 1.169.124 — release requires a real, monotonic [workspace.package] version bump
```

The dry-run now **reaches** the admission check (the actual next gate).
It no longer aborts at the EXT guard. The remaining rejection is by
design (a release needs a fresh bump, and the bump is in HEAD).

### §3.4 Isolated reproduction under `set -u`

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

## §4 Acceptance vs scope

| Constraint | Compliance | Evidence |
|---|---|---|
| C1: no new dependency | YES | only `Cargo.toml`/`Cargo.lock` (version only) and `scripts/release.sh` line 279. |
| C2: no push, no release | YES | `git log origin/main..HEAD --oneline` shows 19 commits ahead; no `git push` invoked. `gh release create` not invoked. |
| C3: bump must be monotonic | YES | `release_admission_check HEAD` returns `ACCEPT 1.169.123 -> 1.169.124` exit 0. |
| C4: bash syntax + shellcheck clean | YES | see §3.2. |
| C5: working tree clean before dry-run | YES | dry-run was run after both commits; preflight reaches admission check, no "dirty tree" refusal. |

## §5 Outstanding items (operator-only)

- **`git push origin main`**: required to expose the bump to the
  remote. System-law `git.push` human_gate.
- **`bash scripts/release.sh`**: required to actually publish
  `v1.169.124`. System-law `git.release` human_gate. The script
  is now ready: admission check will ACCEPT, EXT step will work
  under `set -u`, no-op branch will be triggered if binaries are
  not installed, active branch will run if they are.

## §6 References

- `Cargo.toml` line 16: `version = "1.169.124"`.
- `Cargo.lock`: regenerated.
- `scripts/release.sh` line 279: `${COGNICODE_MCP_BIN:-}` defensive expansion.
- `scripts/lib/release_admission.sh`: source of `release_admission_check`.
- `tests/cycle-artifacts/.../ext-auto-activation-release-flow/`:
  parent slice that introduced step 1d/14.
