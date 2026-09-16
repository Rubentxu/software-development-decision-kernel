#!/bin/bash
# tests/lib_public_release_gate.sh
# Extracted gate logic from scripts/release.sh step 9b.
# Re-declares the helpers used by the gate and exposes run_public_release_gate.
# This file MUST be kept in sync with the gate block in scripts/release.sh.

set -e

# Helpers (mirrored from scripts/release.sh).
step() { printf '\n\033[1;34m==>\033[0m %s\n' "$*"; }
ok()   { printf '\033[1;32m  ✓\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m  !\033[0m %s\n' "$*" >&2; }
die()  { printf '\033[1;31m  ✗\033[0m %s\n' "$*" >&2; exit 1; }
require() {
    command -v "$1" >/dev/null 2>&1 \
        || die "required command not found: $1"
}

# Public gate function. Mirrors scripts/release.sh step 9b exactly.
# Args: $1=tag, $2=repo. Reads VERSION from $SDDK_VERSION (or default 1.169.53).
run_public_release_gate() {
    local TAG="$1"
    local REPO="$2"
    local VERSION="${SDDK_VERSION:-1.169.53}"

    step "9b/14 — public-release gate for $TAG"
    require jq

    # 1. Tag SHA anchoring.
    local EXPECTED_RELEASE_SHA
    EXPECTED_RELEASE_SHA="$(git rev-parse HEAD)"
    local ACTUAL_TAG_SHA
    ACTUAL_TAG_SHA="$(git ls-remote origin "$TAG" | awk '{print $1}')"
    if [ -z "$ACTUAL_TAG_SHA" ]; then
        die "tag $TAG not found on origin — refusing to install"
    fi
    if [ "$EXPECTED_RELEASE_SHA" != "$ACTUAL_TAG_SHA" ]; then
        die "tag $TAG SHA drift: HEAD=$EXPECTED_RELEASE_SHA tag=$ACTUAL_TAG_SHA — refusing to install"
    fi
    ok "tag SHA anchored: $ACTUAL_TAG_SHA"

    # 2. Release metadata.
    local RELEASE_JSON
    RELEASE_JSON="$(gh release view "$TAG" --repo "$REPO" --json tagName,isDraft,isPrerelease,assets 2>/dev/null)" \
        || die "gh release view $TAG failed — refusing to install"
    local GOTTEN_TAG
    GOTTEN_TAG="$(echo "$RELEASE_JSON" | jq -r '.tagName')"
    if [ "$GOTTEN_TAG" != "$TAG" ]; then
        die "tagName drift: expected $TAG got $GOTTEN_TAG"
    fi
    ok "tagName match: $TAG"

    local IS_DRAFT
    IS_DRAFT="$(echo "$RELEASE_JSON" | jq -r '.isDraft')"
    if [ "$IS_DRAFT" != "false" ]; then
        die "release $TAG is in draft state (isDraft=$IS_DRAFT) — run: gh release edit $TAG --draft=false"
    fi
    ok "isDraft=false"

    local IS_PRERELEASE
    IS_PRERELEASE="$(echo "$RELEASE_JSON" | jq -r '.isPrerelease')"
    if [ "$IS_PRERELEASE" != "false" ]; then
        die "release $TAG is a prerelease (isPrerelease=$IS_PRERELEASE) — Base releases must be non-prerelease"
    fi
    ok "isPrerelease=false"

    # 3. Asset contract.
    local CANONICAL_ASSETS=(
        "sddk"
        "sddk.sha256"
        "sddk-v$VERSION-sddk-linux-x86_64-musl.tar.gz"
        "sddk-v$VERSION-sddk-linux-x86_64-musl.tar.gz.sha256"
        "CHECKSUMS"
        "sbom.json"
        "gh-release-receipt.json"
        "software-development-decision-kernel.tar.gz"
        "software-development-decision-kernel.tar.gz.sha256"
    )
    local ACTUAL_ASSETS
    ACTUAL_ASSETS="$(echo "$RELEASE_JSON" | jq -r '.assets[].name' | sort -u)"
    local EXPECTED_ASSETS_SORTED
    EXPECTED_ASSETS_SORTED="$(printf '%s\n' "${CANONICAL_ASSETS[@]}" | sort -u)"
    local MISSING_ASSETS
    MISSING_ASSETS="$(comm -23 <(echo "$EXPECTED_ASSETS_SORTED") <(echo "$ACTUAL_ASSETS"))"
    local EXTRA_ASSETS
    EXTRA_ASSETS="$(comm -13 <(echo "$EXPECTED_ASSETS_SORTED") <(echo "$ACTUAL_ASSETS"))"
    if [ -n "$MISSING_ASSETS" ]; then
        die "missing canonical assets: $(echo "$MISSING_ASSETS" | tr '\n' ' ')"
    fi
    if [ -n "$EXTRA_ASSETS" ]; then
        die "unexpected assets replacing canonical ones: $(echo "$EXTRA_ASSETS" | tr '\n' ' ')"
    fi
    ok "asset set matches 9-asset contract"

    # 4. Public URL HTTP probes.
    local CURL_MAX_ATTEMPTS="${CURL_MAX_ATTEMPTS:-6}"
    local CURL_SLEEP="${CURL_SLEEP:-10}"
    local URL_FAILS=""
    for asset in "${CANONICAL_ASSETS[@]}"; do
        local URL="https://github.com/$REPO/releases/download/$TAG/$asset"
        local ok_remote=0
        local last_rc="000"
        for i in $(seq 1 "$CURL_MAX_ATTEMPTS"); do
            last_rc="$(curl -fsSL -o /dev/null -w '%{http_code}' "$URL" 2>/dev/null || echo "000")"
            if [ "$last_rc" = "200" ]; then
                ok_remote=1
                break
            fi
            # In test mode, short-circuit to keep scenarios fast.
            if [ -n "${CURL_FAIL_FAST:-}" ]; then
                break
            fi
            sleep "$CURL_SLEEP"
        done
        if [ "$ok_remote" = "0" ]; then
            URL_FAILS="$URL_FAILS $asset(HTTP $last_rc)"
        fi
    done
    if [ -n "$URL_FAILS" ]; then
        die "public URL probes failed after 60s-per-asset budget:$URL_FAILS"
    fi
    ok "9/9 canonical assets reachable from public CDN (HTTP 200)"
    ok "public-release gate PASS"
}
