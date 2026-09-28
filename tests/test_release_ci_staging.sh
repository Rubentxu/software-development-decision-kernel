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

echo
if [ "$FAILURES" -eq 0 ]; then
    echo "RESULT: PASS (4 checks)"
    exit 0
fi
echo "RESULT: FAIL ($FAILURES/4 checks failed)"
exit 1
