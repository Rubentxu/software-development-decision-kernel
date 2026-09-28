#!/usr/bin/env bash
# Pin the CI release staging directory (INC-DEBT-034).
#
# Observed on v2.2.6 (session-31): the workflow staged release assets into
# `assets/`, which is ALSO the framework bundle's own asset directory — 17
# versioned files, 55 entries in MANIFEST.sha256. The `assets/*` glob then
# uploaded the bundle surface into the GitHub release and died on a
# subdirectory:
#
#     Post ".../assets?label=&name=agent-models": read assets/agent-models:
#     is a directory
#
# Two observable consequences: the release shipped `agent-models.yaml` (a
# bundle file, not a release asset), and the publish step exited non-zero, so
# the signing, unified-artifact and smoke-test jobs were skipped.
#
# This test extracts the real staging/upload paths from the workflow and
# asserts they are not the repo's bundle `assets/`. A mutation back to
# `assets/` must make it fail.
set -uo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WF="$ROOT/.github/workflows/release.yml"
FAILURES=0
pass() { echo "  ok   — $1"; }
fail() { echo "  FAIL — $1"; FAILURES=$((FAILURES + 1)); }

if [ ! -f "$WF" ]; then
    fail "release.yml not found at $WF"
    echo "RESULT: FAIL"
    exit 1
fi

# The staging root the workflow creates. Extracted, not restated.
STAGE_LINE="$(grep -nE '^\s*STAGE=' "$WF" | head -1 | sed -E 's/^[0-9]+:[[:space:]]*//')"
if [ -z "$STAGE_LINE" ]; then
    fail "no STAGE= assignment — the isolated staging root disappeared"
    echo "RESULT: FAIL"
    exit 1
fi
echo "extracted: $STAGE_LINE"

# ── Case 1: the staging root must not be the bundle's assets/ ─────────────
# `STAGE="assets"` is the exact defect. So is any value that resolves to it.
if echo "$STAGE_LINE" | grep -qE '^STAGE="?assets"?$'; then
    fail "STAGE is the repo's bundle assets/ directory (the v2.2.6 defect)"
else
    pass "STAGE is not the bundle assets/ directory"
fi

# ── Case 2: the staging root must live outside the tracked tree surface ───
# The bundle tarball is built from `agents skills prompts/sddk assets`, so a
# staging dir inside one of those would be swept into the bundle too.
if echo "$STAGE_LINE" | grep -qE 'dist-out|/tmp|mktemp|\$RUNNER_TEMP'; then
    pass "STAGE lives outside the bundle surface (dist-out / tmp)"
else
    fail "STAGE does not obviously live outside the bundle surface"
fi

# ── Case 3: every glob that uploads must use the staging root ─────────────
# A bare `assets/*` in an upload is the defect returning in another costume.
# The staging path itself ends in `release-assets/*`, so the match must be
# anchored to a path boundary — otherwise the correct line matches itself.
# The bundle-tarball and framework jobs legitimately mention `assets` as an
# *input* (`tar czf … agents skills prompts/sddk assets`), so this check
# targets upload/glob contexts only.
GLOBS="$(grep -nE '(upload|artifact|path:)' "$WF" \
    | grep -vE 'tar czf|^\s*#' \
    | grep -E '(^|[[:space:]"/])assets/\*')"
if [ -z "$GLOBS" ]; then
    pass "no upload or artifact path globs the bare bundle assets/*"
else
    fail "an upload/artifact path still globs the bare assets/*:"
    echo "$GLOBS" | sed 's/^/         /'
fi

# ── Case 4: the upload must reference the staging root at all ─────────────
if grep -qE 'gh release upload "\$TAG" .*release-assets/\*' "$WF"; then
    pass "gh release upload targets the isolated staging root"
else
    fail "gh release upload does not target the isolated staging root"
fi

# ── Case 5: the sign job must DOWNLOAD to the directory it SIGNS ──────────
# Found by execution, not by reading (session-31, second pass). The sign job
# downloaded artifacts to `path: assets` while its signing loop iterated
# `dist-out/release-assets/*` — a directory that job never creates. The two
# halves disagreed and the job published zero signatures.
#
# It failed SILENTLY, which is what makes it worth pinning: the loop body
# opens with `[ -f "$f" ] || continue`, so a glob that matches nothing at all
# and a glob that matches only directories are indistinguishable — the job
# exits 0, prints nothing, and the release ships unsigned.
#
# This asserts the *relationship* between the two paths rather than restating
# either one, so it survives a rename of the staging root.
SIGN_JOB="$(awk '/^  sign:/{f=1} f{print} /^  smoke-test:/{exit}' "$WF")"
# Anchor on the `with:` block of the download step, not a fixed line count:
# the explanatory comment above `path:` grew, and a `grep -A6` window
# silently started returning nothing. A pin that breaks when a comment
# gets longer is a pin that gets deleted instead of fixed.
DL_PATH="$(echo "$SIGN_JOB" | awk '/download-artifact@/{w=1} w && /path:/{sub(/^[[:space:]]*path:[[:space:]]*/,""); print; exit}')"
# The signing loop hardened into `for f in "${SIGNED[@]}"` over an explicit
# array whose entries each fail closed via `test -s`. Extracting the loop
# variable alone now yields the literal string '"${SIGNED[@]}"', which
# compares as a mismatch against any real directory — a false RED on a
# correct workflow. So: take the SIGNED=( ... ) block when present and
# derive the common directory of its entries; fall back to the inline-glob
# form only for older loop shapes. This asserts the *relationship* between
# the download directory and the signed payloads' directory instead of
# restating either path, so it survives a rename of the staging root.
SIGN_ARR_BLOCK="$(echo "$SIGN_JOB" | sed -n '/^[[:space:]]*SIGNED=(/,/^[[:space:]]*)/p')"
if [ -n "$SIGN_ARR_BLOCK" ]; then
    ENTRIES="$(echo "$SIGN_ARR_BLOCK" | sed -n 's/.*"\([^"]*\)".*/\1/p' | grep .)"
else
    ENTRIES="$(echo "$SIGN_JOB" | grep -oE 'for f in [^;]+' | head -1 | sed -E 's/for f in //')"
fi
# `path:` names a DIRECTORY that download-artifact populates; each signed
# entry is a FILE inside such a directory. Compare the download directory
# against the distinct set of entry directories: exactly one distinct
# directory, equal to the download path, means the loop signs what the job
# actually downloaded. `%%/*` is wrong here — it truncates at the FIRST
# slash and would reduce everything to 'dist-out'.
DL_DIR="${DL_PATH%/}"
ENTRY_DIRS="$(printf '%s\n' "$ENTRIES" | grep . | xargs -r -n1 dirname | sort -u)"
if [ -z "$SIGN_JOB" ]; then
    fail "could not locate the sign job in release.yml"
elif [ -z "$DL_PATH" ] || [ -z "$ENTRIES" ]; then
    fail "sign job: download path or signing payload list not extractable (dl='$DL_PATH' entries='$ENTRIES')"
elif [ "$(printf '%s\n' "$ENTRY_DIRS" | grep -c .)" -eq 1 ] && [ "$DL_DIR" = "$ENTRY_DIRS" ]; then
    pass "sign job downloads to the directory it signs ($DL_DIR)"
else
    fail "sign job downloads to '$DL_DIR' but signs {$(printf '%s, ' "$ENTRY_DIRS" | sed 's/, $//')} — it would sign nothing"
fi

# ── Case 6: an empty staging dir must fail the sign job, not pass it ───────
# The guard that makes case 5 survivable in production. Without it the job
# is green and unsigned, which is the worst possible combination: the
# signature is a security control, and its absence must be loud.
if echo "$SIGN_JOB" | grep -qE 'refusing to report a successful signing job over an empty staging dir'; then
    pass "sign job fails closed on an empty staging directory"
else
    fail "sign job has no empty-staging guard — a missing download would be reported as signed"
fi

echo
if [ "$FAILURES" -eq 0 ]; then
    echo "RESULT: PASS (6 checks)"
    exit 0
fi
echo "RESULT: FAIL ($FAILURES/6 checks failed)"
exit 1
