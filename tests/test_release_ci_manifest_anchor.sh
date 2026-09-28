#!/usr/bin/env bash
# Pin the `manifest_sha256` computation performed by the release workflow.
#
# Session-31 (OBSERVED). `release.yml` built BUNDLE.toml with
#
#     MANIFEST_SHA="$(awk 'NR==1 {print $1}' "$WORK/framework/MANIFEST.sha256")"
#
# `MANIFEST.sha256` is a *listing*: line 1 is `<hash>  agents/<file>`, so NR==1
# takes the digest of the first listed FILE, not of MANIFEST.sha256 itself. The
# field therefore claimed an anchor it did not carry, in the exact job that
# publishes. `verify_manifest_anchor` (bundle_manifest.rs) would reject such a
# bundle on any non-exempt version, so publishing via CI produced bundles that
# their own installer refuses.
#
# This test extracts the REAL pattern from the workflow instead of restating it
# (lesson from tests/test_release_static_guard.sh, whose first version hardcoded
# the pattern and so survived mutating it). A mutation in release.yml must make
# this test fail.
set -uo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WF="$ROOT/.github/workflows/release.yml"
FAILURES=0
pass() { echo "  ok   — $1"; }
fail() { echo "  FAIL — $1"; FAILURES=$((FAILURES + 1)); }

# The line that assigns MANIFEST_SHA. Extracted, not restated.
PATTERN="$(grep -nE '^\s*MANIFEST_SHA=' "$WF" | head -1 | sed -E 's/^[0-9]+://')"
if [ -z "$PATTERN" ]; then
    fail "release.yml has no MANIFEST_SHA assignment — the anchor producer disappeared"
    echo "RESULT: FAIL"
    exit 1
fi
echo "extracted pattern: $PATTERN"

# ── Case 1: the digest must come from hashing the MANIFEST FILE ────────────
# `sha256sum <file>` is the shape. `awk NR==1` (or any other first-line read)
# reads a listed file's digest, which is a different value.
if echo "$PATTERN" | grep -qE 'sha256sum[[:space:]]+"?\$?\{?WORK' ; then
    pass "digests MANIFEST.sha256 itself via sha256sum"
else
    fail "does not hash MANIFEST.sha256 (sha256sum on the file is required)"
fi
if echo "$PATTERN" | grep -qE "awk[^|]*NR==1" ; then
    fail "reads the first LINE of the manifest (digest of a listed file, not of the manifest)"
else
    pass "does not read a listed file's digest"
fi

# ── Case 2: the emitted field carries the sha256: prefix ──────────────────
# dev manifest (manifest.rs) writes "sha256:<hex>". release.yml and release.sh
# historically wrote bare hex. verify_manifest_anchor normalises both, so this
# is a format-convergence check, not a gate: the point is that the two
# producers stop diverging, so a future release cannot reproduce the split.
PREFIX_LINE="$(grep -nE 'manifest_sha256[[:space:]]*=' "$WF" | head -1)"
if echo "$PREFIX_LINE" | grep -qE 'sha256:'; then
    pass "writes the manifest_sha256 value with the sha256: prefix"
else
    fail "writes manifest_sha256 without the canonical sha256: prefix (producer divergence)"
fi

# ── Case 3: the value hashed must be the manifest that ships ───────────────
# Guards against hashing some other file in $WORK.
if echo "$PATTERN" | grep -qE 'MANIFEST\.sha256'; then
    pass "hashes the MANIFEST.sha256 that is packaged into the tarball"
else
    fail "does not name MANIFEST.sha256 as the hashed input"
fi

echo
if [ "$FAILURES" -eq 0 ]; then
    echo "RESULT: PASS (3 checks)"
    exit 0
fi
echo "RESULT: FAIL ($FAILURES/3 checks failed)"
exit 1
