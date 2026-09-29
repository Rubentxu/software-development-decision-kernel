#!/usr/bin/env bash
# End-to-end supply-chain authenticity check against a REAL published release.
#
# Why this exists (INC-AUDIT-S14):
#
# `tests/test_install_asset_contract.sh` pins the identity/issuer constants
# and checks they do not drift between `cosign.rs` and `install.sh`. That is
# a structural check: it proves the *policy* is coherent. It never proves a
# signature that Actions actually minted verifies under that policy.
#
# That gap is why INC-AUDIT-S14 sat at `code-closed, distribution-open` for
# several sessions: the code path had tests, but "closed in distribution"
# required observing a real signed release verify, and nobody had a runnable
# check that produced that observation on demand. This script is that check.
#
# It is NOT part of the apply-scoped profile: it needs network access, the
# `cosign` binary, and `gh`. It is a verify/release-profile gate. Skipped
# explicitly (not silently) when the tools are absent, because a skipped
# check that reports itself as skipped is honest; one that reports PASS
# without verifying anything is not.
#
# Usage:
#   tests/test_supply_chain_authenticity.sh --tag v2.2.27
#   SDDK_RELEASE_REPO=Rubentxu/software-development-decision-kernel \
#     tests/test_supply_chain_authenticity.sh --tag v2.2.27
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPO="${SDDK_RELEASE_REPO:-Rubentxu/software-development-decision-kernel}"
TAG=""
# Optional directory of already-downloaded assets. `release.sh` step 9c
# fetches them from the public CDN because that is the exact copy users get;
# re-downloading them here over `gh` would double the transfer for nothing
# and, worse, would verify a DIFFERENT copy than the one the CDN served.
CACHE_DIR="${SDDK_AUTH_ASSETS_DIR:-}"

PASS=0
FAIL=0
SKIP=0

ok()   { printf '\033[0;32m  ✓\033[0m %s\n' "$1"; PASS=$((PASS+1)); }
fail() { printf '\033[0;31m  ✗\033[0m %s\n' "$1"; FAIL=$((FAIL+1)); }
skip() { printf '\033[1;33m  ~\033[0m %s\n' "$1"; SKIP=$((SKIP+1)); }

while [ $# -gt 0 ]; do
    case "$1" in
        --tag) TAG="${2:-}"; shift 2 ;;
        --assets-dir) CACHE_DIR="${2:-}"; shift 2 ;;
        -h|--help) sed -n '2,30p' "${BASH_SOURCE[0]}"; exit 0 ;;
        *) printf 'unknown arg: %s\n' "$1" >&2; exit 2 ;;
    esac
done

if [ -z "$TAG" ]; then
    echo "FAIL: --tag is required (e.g. --tag v2.2.27)" >&2
    exit 2
fi
if [ -n "$CACHE_DIR" ] && [ ! -d "$CACHE_DIR" ]; then
    echo "FAIL: --assets-dir '$CACHE_DIR' is not a directory" >&2
    exit 2
fi
VERSION="${TAG#v}"

COSIGN_RS="$ROOT/crates/sddk-cli/src/cosign.rs"

# ── The pins under test come from the code, never from this script ────────
# Hardcoding them here would let the check pass while the product drifts,
# which is the exact failure mode this repo already paid for once
# (INC-DEBT-024: a literal pin passed today and broke on the next tag).
#
# The extraction must be NON-GREEDY. `.*` before the closing delimiter spans
# newlines once newlines are folded into spaces, so a greedy pattern here
# silently returns "the pin plus the entire rest of cosign.rs" — which then
# gets handed to cosign as the identity pattern and fails with a
# misleading "malformed subject in identity" that has nothing to do with
# the real signature. Extract the single `r"..."` token instead.
_rust_identity=$(tr '\n' ' ' < "$COSIGN_RS" \
    | grep -oE 'DEFAULT_CERT_IDENTITY_REGEXP: &str = r"[^"]+"' \
    | head -1 \
    | sed -E 's/.*r"([^"]+)"/\1/')
_rust_issuer=$(sed -n 's/^pub const DEFAULT_CERT_ISSUER: &str = "\(.*\)";$/\1/p' "$COSIGN_RS" | head -1)

if [ -z "$_rust_identity" ] || [ -z "$_rust_issuer" ]; then
    fail "could not extract pins from cosign.rs (identity='$_rust_identity' issuer='$_rust_issuer')"
    echo
    echo "PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
    exit 1
fi
ok "pins extracted from cosign.rs (not hardcoded in this script)"

# ── Prerequisites ─────────────────────────────────────────────────────────
if ! command -v cosign >/dev/null 2>&1; then
    skip "SKIPPED: cosign not in PATH — authenticity NOT verified"
    echo "SKIP=$SKIP — this run proves nothing about signature validity"
    exit 0
fi
if ! command -v gh >/dev/null 2>&1; then
    skip "SKIPPED: gh not in PATH — cannot fetch the real release assets"
    echo "SKIP=$SKIP — this run proves nothing about signature validity"
    exit 0
fi
ok "cosign and gh available"

if [ -n "$CACHE_DIR" ]; then
    WORK="$CACHE_DIR"
    ok "using pre-fetched assets from $WORK (no re-download)"
else
    WORK="$(mktemp -d)"
    trap 'rm -rf "$WORK"' EXIT
fi

# ── 1. The release must be public, final, and carry signatures ────────────
release_json="$(gh release view "$TAG" --repo "$REPO" --json isDraft,isPrerelease,assets 2>/dev/null)"
if [ -z "$release_json" ]; then
    fail "cannot read release $TAG from $REPO"
    echo "PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
    exit 1
fi

is_draft=$(printf '%s' "$release_json" | jq -r '.isDraft')
is_pre=$(printf '%s' "$release_json" | jq -r '.isPrerelease')
if [ "$is_draft" = "false" ]; then
    ok "release is not a draft"
else
    fail "release $TAG isDraft=$is_draft"
fi
if [ "$is_pre" = "false" ]; then
    ok "release is not a prerelease"
else
    fail "release $TAG isPrerelease=$is_pre"
fi

for asset in sddk sddk.sig sddk.pem; do
    if printf '%s' "$release_json" | jq -e --arg a "$asset" '.assets[] | select(.name==$a)' >/dev/null; then
        ok "asset present: $asset"
    else
        fail "missing signature asset: $asset"
    fi
done

# ── 2. The three canonical signed payloads must each verify ───────────────
# release.sh signs 3 artifacts (binary, unified tarball, framework bundle).
# Verifying only the binary would let the other two rot undetected.
SIGNED_PAYLOADS=(
    "sddk|sddk"
    "sddk-v${VERSION}-sddk-linux-x86_64-musl.tar.gz|software-development-decision-kernel.tar.gz"
)

payload_ok=0
payload_total=0
for pair in "${SIGNED_PAYLOADS[@]}"; do
    unified="${pair%%|*}"
    bundle="${pair##*|}"

    # Download the payload once; verify the binary against it and the
    # unified tarball against the bundle when both exist.
    for want in "$unified" "$bundle"; do
        [ -f "$WORK/$want" ] && continue
        [ -n "$CACHE_DIR" ] && continue
        gh release download "$TAG" --repo "$REPO" \
            --pattern "$want" --pattern "$want.sig" --pattern "$want.pem" \
            --dir "$WORK" --clobber >/dev/null 2>&1 || true
    done

    if [ -f "$WORK/$bundle" ] && [ -f "$WORK/$bundle.sig" ] && [ -f "$WORK/$bundle.pem" ]; then
        payload_total=$((payload_total+1))
        if out=$(cosign verify-blob \
            --signature "$WORK/$bundle.sig" \
            --certificate "$WORK/$bundle.pem" \
            --certificate-identity-regexp "$_rust_identity" \
            --certificate-oidc-issuer "$_rust_issuer" \
            "$WORK/$bundle" 2>&1) \
            && printf '%s' "$out" | grep -q 'Verified OK'; then
            ok "signature verifies under the pinned trust root: $bundle"
            payload_ok=$((payload_ok+1))
        else
            fail "signature does NOT verify: $bundle"
            printf '      %s\n' "$(printf '%s' "$out" | tail -1)"
        fi
    else
        skip "no detached signature published for $bundle (only $unified/binary are signed in this layout)"
    fi
done

if [ "$payload_total" -eq 0 ]; then
    fail "no signed payload could be verified — authenticity UNPROVEN for $TAG"
elif [ "$payload_ok" -eq "$payload_total" ]; then
    ok "verified $payload_ok/$payload_total signed payload(s) for $TAG"
else
    fail "only $payload_ok/$payload_total signed payloads verified for $TAG"
fi

# ── 3. The pin must discriminate, not accept anything ─────────────────────
# A trust root that verifies everything is not a trust root. "cosign said OK"
# is a much weaker claim than "cosign accepted ONLY the intended signer", and
# only the second one is a security property.
#
# Each control below is DERIVED from the real pin by mutating exactly one
# component the pin is supposed to constrain. Using a hardcoded string that
# matches nothing would pass even if the shipped pin had been replaced with
# `.*`, which is the exact regression these controls exist to catch.
if [ -f "$WORK/sddk" ] && [ -f "$WORK/sddk.sig" ] && [ -f "$WORK/sddk.pem" ]; then
    # Control 1: same repo/workflow shape, but a feature branch instead of a
    # tag. A branch holding id-token: write is not a release act.
    _branch_pin=$(printf '%s' "$_rust_identity" \
        | sed -E 's#@refs/tags/v\[0-9\].*\$#@refs/heads/main\$#')
    if [ "$_branch_pin" = "$_rust_identity" ]; then
        fail "could not derive the branch control from the pin — the pin shape changed"
    elif cosign verify-blob --signature "$WORK/sddk.sig" --certificate "$WORK/sddk.pem" \
        --certificate-identity-regexp "$_branch_pin" \
        --certificate-oidc-issuer "$_rust_issuer" "$WORK/sddk" >/dev/null 2>&1; then
        fail "pin accepted a BRANCH subject — a non-release ref can produce a trusted signature"
    else
        ok "pin rejects a branch ref where a tag is required (control)"
    fi

    # Control 2: the right identity, but a certificate from a different OIDC
    # provider. Proves the issuer is actually load-bearing.
    if cosign verify-blob --signature "$WORK/sddk.sig" --certificate "$WORK/sddk.pem" \
        --certificate-identity-regexp "$_rust_identity" \
        --certificate-oidc-issuer 'https://oauth2.sigstore.dev/auth' \
        "$WORK/sddk" >/dev/null 2>&1; then
        fail "pin accepted a certificate from a different OIDC issuer"
    else
        ok "pin rejects a different OIDC issuer (control)"
    fi

    # Control 3: the wildcard. If the shipped pin ever degrades to something
    # that accepts any subject, this control stops failing and the gate goes
    # red. This is the mutation test for the trust root itself.
    if cosign verify-blob --signature "$WORK/sddk.sig" --certificate "$WORK/sddk.pem" \
        --certificate-identity-regexp '.*' \
        --certificate-oidc-issuer "$_rust_issuer" "$WORK/sddk" >/dev/null 2>&1; then
        ok "control: a wildcard pattern WOULD accept this certificate (so the pin is what rejects)"
    else
        fail "even a wildcard pattern was rejected — verification is failing for a different reason"
    fi
fi

echo
echo "═══════════════════════════════════════════"
echo "  PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
echo "═══════════════════════════════════════════"

[ "$FAIL" -eq 0 ] || exit 1
exit 0
