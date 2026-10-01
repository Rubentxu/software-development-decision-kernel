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
# This test extracts the REAL pattern from each producer instead of restating
# it (lesson from tests/test_release_static_guard.sh, whose first version
# hardcoded the pattern and so survived mutating it). A mutation in any
# producer must make this test fail.
#
# BOTH producers are checked (session-65h). The first version of this gate took
# `WF=release.yml` and stopped there, with `release.sh` appearing only in a
# comment — so the two producers it exists to converge could drift freely, and
# HAD: `release.sh` still emits bare hex while `release.yml` emits `sha256:`
# prefixed. The check named the divergence and then was structurally unable to
# see it. Same genre as the bundle staging (INC-DEBT-056): a contract stated
# once and observed in one place.
set -uo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REL_SH="$ROOT/scripts/release.sh"
WF="$ROOT/.github/workflows/release.yml"
FAILURES=0
CHECKS=0
pass() { echo "  ok   — $1"; }
fail() { echo "  FAIL — $1"; FAILURES=$((FAILURES + 1)); }

check_producer() {
    # $1 = human label, $2 = file. Runs the three cases against one producer.
    local label="$1" file="$2"

    echo
    echo "-- $label --"

    # The line that assigns MANIFEST_SHA. Extracted, not restated.
    local pattern
    pattern="$(grep -nE '^[[:space:]]*MANIFEST_SHA=' "$file" | head -1 | sed -E 's/^[0-9]+://')"
    if [ -z "$pattern" ]; then
        fail "$label: no MANIFEST_SHA assignment — the anchor producer disappeared"
        return
    fi
    CHECKS=$((CHECKS + 1))
    echo "   extracted pattern: $pattern"

    # ── Case 1: the digest must come from hashing the MANIFEST FILE ────────
    # `sha256sum <file>` is the shape. `awk NR==1` (or any other first-line
    # read) reads a listed file's digest, which is a different value.
    CHECKS=$((CHECKS + 1))
    if echo "$pattern" | grep -qE 'sha256sum[[:space:]]+("?\$?\{?WORK|[A-Za-z_./]*MANIFEST\.sha256)' ; then
        pass "$label: digests MANIFEST.sha256 itself via sha256sum"
    else
        fail "$label: does not hash MANIFEST.sha256 (sha256sum on the file is required)"
    fi
    CHECKS=$((CHECKS + 1))
    if echo "$pattern" | grep -qE "awk[^|]*NR==1" ; then
        fail "$label: reads the first LINE of the manifest (digest of a listed file, not of the manifest)"
    else
        pass "$label: does not read a listed file's digest"
    fi

    # ── Case 2: the emitted field carries the sha256: prefix ───────────────
    # dev manifest (manifest.rs) writes "sha256:<hex>". `verify_manifest_anchor`
    # normalises both, so this is a format-convergence check, not a
    # correctness gate: the point is that the producers stop diverging, so a
    # future release cannot reproduce the split. All occurrences are checked,
    # not just the first — a file can carry the right line and a wrong one.
    local prefix_lines
    prefix_lines="$(grep -nE 'manifest_sha256[[:space:]]*=' "$file" | grep -vE '^\s*[0-9]+:\s*#' || true)"
    if [ -z "$prefix_lines" ]; then
        fail "$label: no manifest_sha256 assignment found"
    else
        local missing=""
        while IFS= read -r line; do
            [ -z "$line" ] && continue
            echo "$line" | grep -qE 'sha256:' || missing="$missing
      $line"
            CHECKS=$((CHECKS + 1))
        done <<< "$prefix_lines"
        if [ -n "$missing" ]; then
            fail "$label: manifest_sha256 without the canonical sha256: prefix (producer divergence):$missing"
        else
            pass "$label: every manifest_sha256 carries the sha256: prefix"
        fi
    fi

    # ── Case 3: the value hashed must be the manifest that ships ───────────
    # Guards against hashing some other file in $WORK.
    CHECKS=$((CHECKS + 1))
    if echo "$pattern" | grep -qE 'MANIFEST\.sha256'; then
        pass "$label: hashes the MANIFEST.sha256 that is packaged into the tarball"
    else
        fail "$label: does not name MANIFEST.sha256 as the hashed input"
    fi
}

# Negative control: the checks must be able to fail. A gate that only ever
# observes green has not been shown to examine anything (INC-DEBT-053/054/055,
# the same genre four times over). Mutate each producer in a scratch copy and
# require the corresponding case to reject it.
echo "== negative control: a bare-hex producer must be rejected =="
CTRL="$(mktemp -d)"
trap 'rm -rf "$CTRL"' EXIT
sed -E 's#sha256:\$\{?MANIFEST_SHA\}?#${MANIFEST_SHA}#; s#"sha256:\$MANIFEST_SHA"#"$MANIFEST_SHA"#' \
    "$REL_SH" > "$CTRL/release.sh" 2>/dev/null || true
if diff -q "$REL_SH" "$CTRL/release.sh" >/dev/null 2>&1; then
    echo "  ok   — control is a no-op on the real file (nothing to falsify here)"
else
    grep -nE 'manifest_sha256[[:space:]]*=' "$CTRL/release.sh" | grep -vE '^\s*[0-9]+:\s*#' \
        | grep -qE 'sha256:' \
        && { fail "control: the bare-hex mutation was NOT detected"; } \
        || { CHECKS=$((CHECKS + 1)); pass "control: a bare-hex producer is detected by Case 2"; }
fi

check_producer "release.sh (local path)" "$REL_SH"
check_producer "release.yml (cloud path)" "$WF"

echo
echo "RESULT: $([ "$FAILURES" -eq 0 ] && echo "PASS" || echo "FAIL") ($CHECKS checks, $FAILURES failed)"
[ "$FAILURES" -eq 0 ] || exit 1
exit 0
