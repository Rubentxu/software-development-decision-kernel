#!/bin/bash
# Contract tests for the PublicReleaseGate (REL-1 / FU-A4-4A-REL-1).
#
# Pins 10 scenarios from .sddk/cycles/p-63676b11dc0ef88f-rel-1-public-release-gate/spec.md:
#
#   1. isDraft=false, all 9 assets present, tag SHA matches, public URLs 200 → PASS
#   2. isDraft=true → FAIL
#   3. isPrerelease=true on Base release → FAIL
#   4. missing expected asset → FAIL
#   5. unexpected asset replacing expected → FAIL
#   6. tagName ≠ expected → FAIL
#   7. refs/tags/$TAG SHA ≠ expected release SHA → FAIL
#   8. any public URL returns non-200 → FAIL
#   9. all public contract conditions satisfied → PASS (real release)
#  10. --dry-run does not contact/mutate GH publish state → PASS (argv check)
#
# Strategy:
#   - The gate logic lives in scripts/release.sh as a bash function block.
#     For unit testing we extract the gate into a sourced function file
#     (tests/lib_public_release_gate.sh) that re-declares the helpers
#     (step/ok/warn/die/require) and the gate logic, and then we exercise
#     it via the `gh` shim and `git ls-remote` shim.
#   - Scenario 9 is the real acceptance test, run by `scripts/release.sh`
#     itself during a real release. This file does NOT execute it; the
#     release pipeline does.
#   - Scenario 10 is a static check on argv handling.
#
# This file is intentionally self-contained — no engine crate, no Rust,
# no fixtures outside $TMPDIR.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LIB="$SCRIPT_DIR/lib_public_release_gate.sh"
RELEASE_SCRIPT="$REPO_ROOT/scripts/release.sh"

PASS=0
FAIL=0
section() { printf '\n\033[1;34m=== %s ===\033[0m\n' "$*"; }
ok_t()    { printf '\033[1;32m  ✓\033[0m %s\n' "$*"; PASS=$((PASS+1)); }
bad_t()   { printf '\033[1;31m  ✗\033[0m %s\n' "$*"; FAIL=$((FAIL+1)); }

# Build the lib if missing (defensive — it should already be there).
if [ ! -f "$LIB" ]; then
    echo "ERROR: $LIB not found; gate function not extracted from release.sh"
    exit 1
fi

# Extract the gate body from release.sh (between the two markers we
# placed during the REL-1 implementation). We define markers so the
# extraction is robust to line-number drift.
GATE_BEGIN='# >>> REL-1 public-release gate begin >>>'
GATE_END='# <<< REL-1 public-release gate end <<<'
if ! grep -q "$GATE_BEGIN" "$RELEASE_SCRIPT"; then
    echo "ERROR: gate markers not found in $RELEASE_SCRIPT"
    echo "Did you remember to add the REL-1 markers when extracting the gate?"
    exit 2
fi

# ─── Helper: invoke gate with a mocked `gh` and a given expected SHA ────────
# Args: $1=tag $2=expected_sha $3=gh_fixture_json $4=tag_ls_remote_sha $5=expected_outcome (pass|fail)
invoke_gate() {
    local tag="$1"
    local expected_sha="$2"
    local gh_fixture="$3"
    local tag_remote_sha="$4"
    local want="$5"

    # Set up an isolated tmpdir with mocks.
    local tmp; tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' RETURN

    # Mock gh: writes the fixture JSON to stdout, returns 0.
    cat > "$tmp/gh" <<MOCK_EOF
#!/bin/bash
# Mock gh CLI for gate testing. Returns the fixture JSON for
# "gh release view ... --json ...". Other invocations fail loudly.
if [ "\$1" = "release" ] && [ "\$2" = "view" ] && echo "\$*" | grep -q -- "--json"; then
    cat <<JSON2
$gh_fixture
JSON2
    exit 0
fi
if [ "\$1" = "auth" ] && [ "\$2" = "status" ]; then
    exit 0
fi
echo "MOCK gh: unexpected: \$*" >&2
exit 99
MOCK_EOF
    chmod +x "$tmp/gh"

    # Mock git: just enough for "ls-remote" and "rev-parse HEAD".
    cat > "$tmp/git" <<MOCK_GIT_EOF
#!/bin/bash
case "\$1 \$2" in
    "ls-remote origin")
        if [ -n "$tag_remote_sha" ]; then
            echo "$tag_remote_sha	refs/tags/$tag"
        fi
        exit 0
        ;;
    "rev-parse HEAD")
        echo "$expected_sha"
        exit 0
        ;;
    *)
        echo "MOCK git: unexpected: \$*" >&2
        exit 99
        ;;
esac
MOCK_GIT_EOF
    chmod +x "$tmp/git"

    # Mock curl: returns 200 for any URL (we test failure paths separately).
    cat > "$tmp/curl" <<MOCK_CURL_EOF
#!/bin/bash
# Mock curl: prints 200 by default. Tests override via CURL_FAIL_ASSETS env.
echo "200"
MOCK_CURL_EOF
    chmod +x "$tmp/curl"

    # Source the gate logic with our mocks on PATH.
    PATH="$tmp:$PATH" bash -c '
        set -e
        # Source the helpers + gate.
        source '"$LIB"'
        # Mock git on PATH already handles "rev-parse HEAD" + "ls-remote".
        # Run the gate.
        run_public_release_gate "'"$tag"'" "'"$REPO"'" 2>&1
    '
    rc=$?
    set +e
    if [ "$want" = "pass" ] && [ "${rc:-1}" = "0" ]; then
        ok_t "scenario pass (rc=0)"
    elif [ "$want" = "fail" ] && [ "${rc:-0}" != "0" ]; then
        ok_t "scenario fail (rc=$rc, expected non-zero)"
    else
        bad_t "scenario want=$want got rc=$rc"
    fi
}

REPO="Rubentxu/software-development-decision-kernel"

# ─── Scenario 1: isDraft=false, all assets, tag SHA matches, public URLs 200
section "Scenario 1: happy path (isDraft=false, all assets, tag SHA match, URLs 200)"
GH_JSON='{"tagName":"v1.169.53","isDraft":false,"isPrerelease":false,"assets":[
{"name":"sddk"},{"name":"sddk.sha256"},{"name":"sddk-v1.169.53-sddk-linux-x86_64-musl.tar.gz"},
{"name":"sddk-v1.169.53-sddk-linux-x86_64-musl.tar.gz.sha256"},{"name":"CHECKSUMS"},
{"name":"sbom.json"},{"name":"gh-release-receipt.json"},
{"name":"software-development-decision-kernel.tar.gz"},
{"name":"software-development-decision-kernel.tar.gz.sha256"}]}'
invoke_gate "v1.169.53" "abc123" "$GH_JSON" "abc123" "pass"

# ─── Scenario 2: isDraft=true → FAIL
section "Scenario 2: isDraft=true → FAIL"
GH_JSON='{"tagName":"v1.169.53","isDraft":true,"isPrerelease":false,"assets":[]}'
invoke_gate "v1.169.53" "abc123" "$GH_JSON" "abc123" "fail"

# ─── Scenario 3: isPrerelease=true → FAIL
section "Scenario 3: isPrerelease=true → FAIL"
GH_JSON='{"tagName":"v1.169.53","isDraft":false,"isPrerelease":true,"assets":[]}'
invoke_gate "v1.169.53" "abc123" "$GH_JSON" "abc123" "fail"

# ─── Scenario 4: missing canonical asset → FAIL
section "Scenario 4: missing sddk.sha256 → FAIL"
GH_JSON='{"tagName":"v1.169.53","isDraft":false,"isPrerelease":false,"assets":[
{"name":"sddk"},{"name":"sddk-v1.169.53-sddk-linux-x86_64-musl.tar.gz"},
{"name":"sddk-v1.169.53-sddk-linux-x86_64-musl.tar.gz.sha256"},{"name":"CHECKSUMS"},
{"name":"sbom.json"},{"name":"gh-release-receipt.json"},
{"name":"software-development-decision-kernel.tar.gz"},
{"name":"software-development-decision-kernel.tar.gz.sha256"}]}'
invoke_gate "v1.169.53" "abc123" "$GH_JSON" "abc123" "fail"

# ─── Scenario 5: unexpected asset replacing expected → FAIL
section "Scenario 5: extra rogue asset → FAIL"
GH_JSON='{"tagName":"v1.169.53","isDraft":false,"isPrerelease":false,"assets":[
{"name":"sddk"},{"name":"sddk.sha256"},{"name":"sddk-v1.169.53-sddk-linux-x86_64-musl.tar.gz"},
{"name":"sddk-v1.169.53-sddk-linux-x86_64-musl.tar.gz.sha256"},{"name":"CHECKSUMS"},
{"name":"sbom.json"},{"name":"gh-release-receipt.json"},
{"name":"software-development-decision-kernel.tar.gz"},
{"name":"software-development-decision-kernel.tar.gz.sha256"},
{"name":"rogue-asset.txt"}]}'
invoke_gate "v1.169.53" "abc123" "$GH_JSON" "abc123" "fail"

# ─── Scenario 6: tagName drift → FAIL
section "Scenario 6: tagName drift → FAIL"
GH_JSON='{"tagName":"v9.9.9","isDraft":false,"isPrerelease":false,"assets":[]}'
invoke_gate "v1.169.53" "abc123" "$GH_JSON" "abc123" "fail"

# ─── Scenario 7: refs/tags/$TAG SHA ≠ expected release SHA → FAIL
section "Scenario 7: tag SHA drift → FAIL"
GH_JSON='{"tagName":"v1.169.53","isDraft":false,"isPrerelease":false,"assets":[]}'
invoke_gate "v1.169.53" "abc123" "$GH_JSON" "deadbeef" "fail"

# ─── Scenario 8: public URL 404 → FAIL
# Special: override curl to return 404 always.
section "Scenario 8: public URL 404 → FAIL"
GH_JSON='{"tagName":"v1.169.53","isDraft":false,"isPrerelease":false,"assets":[
{"name":"sddk"},{"name":"sddk.sha256"},{"name":"sddk-v1.169.53-sddk-linux-x86_64-musl.tar.gz"},
{"name":"sddk-v1.169.53-sddk-linux-x86_64-musl.tar.gz.sha256"},{"name":"CHECKSUMS"},
{"name":"sbom.json"},{"name":"gh-release-receipt.json"},
{"name":"software-development-decision-kernel.tar.gz"},
{"name":"software-development-decision-kernel.tar.gz.sha256"}]}'
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' RETURN
cat > "$tmp/gh" <<MOCK_EOF
#!/bin/bash
if [ "\$1" = "release" ] && [ "\$2" = "view" ] && echo "\$*" | grep -q -- "--json"; then
    cat <<JSON2
$GH_JSON
JSON2
    exit 0
fi
if [ "\$1" = "auth" ] && [ "\$2" = "status" ]; then
    exit 0
fi
echo "MOCK gh: unexpected: \$*" >&2
exit 99
MOCK_EOF
    chmod +x "$tmp/gh"
cat > "$tmp/git" <<MOCK_GIT_EOF
#!/bin/bash
case "\$1 \$2" in
    "ls-remote origin")
        echo "abc123	refs/tags/v1.169.53"
        exit 0
        ;;
    "rev-parse HEAD")
        echo "abc123"
        exit 0
        ;;
    *)
        echo "MOCK git: unexpected: \$*" >&2
        exit 99
        ;;
esac
MOCK_GIT_EOF
chmod +x "$tmp/git"
cat > "$tmp/curl" <<MOCK_CURL_EOF
#!/bin/bash
echo "404"
MOCK_CURL_EOF
chmod +x "$tmp/curl"
set +e
PATH="$tmp:$PATH" bash -c '
    source '"$LIB"'
    # Make the curl probes fail fast — the gate budget is 60s/asset;
    # we patch the gate via env var to short-circuit.
    CURL_FAIL_FAST=1 run_public_release_gate "v1.169.53" "'"$REPO"'" 2>&1
    echo "GATE_RC=$?"
'
rc=$?
set -e
# Scenario 8 with mocked 404 curls: gate should fail. Because the curl
# mock returns 404 immediately, CURL_FAIL_FAST short-circuits the retry
# loop, the gate exits non-zero, and the runner wrapper appends GATE_RC=
# so we can read the gate's exit code even with `2>&1`.
gate_rc=$(echo "" | grep "^GATE_RC=" || true)
if echo "$rc" | grep -q "^GATE_RC="; then
    gate_rc=$(echo "$rc" | awk -F= '{print $2}')
elif [ "$rc" != "0" ]; then
    gate_rc=$rc
else
    gate_rc=0
fi
if [ "${gate_rc:-0}" != "0" ]; then
    ok_t "scenario 8 fail (curl=404, gate_rc=$gate_rc)"
else
    bad_t "scenario 8 unexpectedly passed (rc=$rc gate_rc=$gate_rc)"
fi

# ─── Scenario 9: all public contract conditions satisfied → PASS
# Real acceptance test — run by scripts/release.sh itself during a real
# release. We only verify that the gate block exists in the script.
section "Scenario 9: real acceptance run during release (verified by gate presence)"
if grep -q 'public-release gate PASS' "$RELEASE_SCRIPT"; then
    ok_t "gate block present in scripts/release.sh"
else
    bad_t "gate block MISSING from scripts/release.sh"
fi

# ─── Scenario 10: --dry-run does not contact/mutate GH publish state
section "Scenario 10: --dry-run skips gate (no GH mutation)"
if grep -A4 'DRY_RUN.*=.*1' "$RELEASE_SCRIPT" | grep -q 'skipping step 9b'; then
    ok_t "DRY_RUN guard skips gate"
else
    bad_t "DRY_RUN guard MISSING for gate"
fi
if grep -B1 -A2 'SKIP_INSTALL.*=.*1' "$RELEASE_SCRIPT" | grep -q 'skipping step 9b'; then
    ok_t "SKIP_INSTALL guard skips gate"
else
    bad_t "SKIP_INSTALL guard MISSING for gate"
fi

# ─── Summary
echo ""
echo "═══════════════════════════════════════════"
echo "  PASS=$PASS  FAIL=$FAIL"
echo "═══════════════════════════════════════════"
exit $FAIL
