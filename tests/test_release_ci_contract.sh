#!/usr/bin/env bash
# Pin the release.yml -> install.sh distribution contract (INC-DEBT-034).
# shellcheck disable=SC2016  # source fragments deliberately remain literal.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WF="$ROOT/.github/workflows/release.yml"
HELPER="$ROOT/scripts/release-assets-contract.sh"
PASS=0
FAIL=0
ok() { echo "  ok   $1"; PASS=$((PASS + 1)); }
fail() { echo "  FAIL $1"; FAIL=$((FAIL + 1)); }

job() {
    local name="$1"
    awk -v key="  $name:" '
        $0 == key { emit=1; start=NR }
        emit && NR > start && /^  [a-z][a-z-]*:/ { exit }
        emit { print }
    ' "$WF"
}

BUILD="$(job build)"
FRAMEWORK="$(job framework-bundle)"
UNIFIED="$(job unified-artifact)"
CANONICAL="$(job canonical-assets)"
SIGN="$(job sign)"
PUBLISH="$(job publish)"
SMOKE="$(job smoke-test)"

# Build/bundle/unify stage artifacts, but none publishes partial release assets.
if ! printf '%s\n' "$BUILD$FRAMEWORK$UNIFIED$CANONICAL$SIGN" \
    | grep -vE '^[[:space:]]*#' \
    | grep -qE 'gh release (create|upload)'; then
    ok "release jobs stage only; no public assets before signing"
else
    fail "a pre-publish job mutates the public release"
fi

# The tag workflow assembles every installer-required canonical file and a receipt.
required_patterns=(
    'cp downloads/sddk-linux-x86_64-musl "$STAGE/sddk"'
    'sha256sum sddk > sddk.sha256'
    'sddk-${TAG}-sddk-linux-x86_64-musl.tar.gz'
    'software-development-decision-kernel.tar.gz'
    ' > CHECKSUMS'
    'cp downloads/sddk-linux-x86_64-musl.sbom.json "$STAGE/sbom.json"'
    'scripts/release-receipt.sh'
    '--actor-id github-actions'
    '--tag "$TAG"'
)
missing=0
for pattern in "${required_patterns[@]}"; do
    if ! printf '%s\n' "$CANONICAL" | grep -Fq -- "$pattern"; then
        fail "canonical-assets job missing required producer: $pattern"
        missing=1
    fi
done
if [ "$missing" -eq 0 ]; then
    ok "canonical payload names, checksums, SBOM and System receipt are staged"
fi

# The workflow's four build targets must agree with the contract's three supplemental names.
targets=(sddk-linux-x86_64-musl sddk-linux-aarch64-musl sddk-darwin-arm64 sddk-darwin-x86_64)
for target in "${targets[@]}"; do
    if grep -Fq "asset: $target" "$WF"; then
        ok "build target retained: $target"
    else
        fail "release matrix target disappeared: $target"
    fi
done
for target in sddk-linux-aarch64-musl sddk-darwin-arm64 sddk-darwin-x86_64; do
    if grep -Fq "\"$target\"" "$HELPER"; then
        ok "gate explicitly allows the supplemental unified target: $target"
    else
        fail "gate does not allow the exact supplemental target: $target"
    fi
done

# The signer consumes the canonical artifact, requires each signed payload, and fails closed.
if printf '%s\n' "$SIGN" | grep -Fq 'name: canonical-release-assets' \
    && printf '%s\n' "$SIGN" | grep -Fq 'path: release-assets' \
    && printf '%s\n' "$SIGN" | grep -Fq 'test -s "$f"'; then
    ok "sign job consumes the canonical staging and fails on any missing signed payload"
else
    fail "sign job staging or fail-closed payload check drifted"
fi
for target in sddk-linux-x86_64-musl sddk-linux-aarch64-musl sddk-darwin-arm64 sddk-darwin-x86_64; do
    if printf '%s\n' "$SIGN" | grep -Fq "release-assets/sddk-\${TAG}-$target.tar.gz"; then
        ok "sign job signs unified target: $target"
    else
        fail "sign job omits unified target signature: $target"
    fi
done

# The unified job must NOT execute the target binary to read its version.
# Observed on the v2.2.9 run (36473855952): the three non-x86_64 unified
# jobs ran `"$WORK/bin/sddk" --version` on a Mach-O/ELF aarch64 binary on
# an x86_64 runner — `Exec format error`, exit 126, all three failed, and
# the sign/publish/smoke chain was skipped. The x86_64 binary is already
# downloaded into the same `downloads/` dir, so the version must come from
# the one binary the runner can actually execute, guarded fail-closed.
if printf '%s\n' "$UNIFIED" | grep -qE '\$\{?WORK\}?/bin/sddk"? +--version|bin/sddk" +--version'; then
    fail "unified job executes the target binary on the runner (exit 126 on non-x86_64)"
else
    ok "unified job never executes the foreign-arch target binary"
fi
if printf '%s\n' "$UNIFIED" | grep -Fq 'downloads/sddk-linux-x86_64-musl --version' \
    && printf '%s\n' "$UNIFIED" | grep -Fq 'chmod 0755 downloads/sddk-linux-x86_64-musl' \
    && printf '%s\n' "$UNIFIED" | grep -Fq 'BINARY_VERSION is empty'; then
    ok "unified job derives the version from the runnable x86_64 binary, exec-bit and fail-closed"
else
    fail "unified job lacks the x86_64-derived version with exec-bit and empty-version guards"
fi

# Exactly one publication point, after sign; smoke runs only after publish.
PUBLISH_CALLS="$(grep -vE '^[[:space:]]*#' "$WF" | grep -cE 'gh release (create|upload)')"
if [ "$PUBLISH_CALLS" -eq 2 ] \
    && printf '%s\n' "$PUBLISH" | grep -Fq 'gh release create' \
    && printf '%s\n' "$PUBLISH" | grep -Fq 'gh release upload' \
    && printf '%s\n' "$PUBLISH" | grep -Fq 'release-assets/*'; then
    ok "one publisher creates or retries the fully staged signed release"
else
    fail "release publication is not centralized in the post-sign publish job"
fi
if printf '%s\n' "$PUBLISH" | grep -Fq 'validate_release_asset_contract' \
    && printf '%s\n' "$PUBLISH" | grep -Fq 'curl -fsSL'; then
    ok "post-publication gate checks the shared contract and canonical URLs"
else
    fail "publish job lacks shared contract or public URL probes"
fi
if printf '%s\n' "$SMOKE" | grep -Fq 'needs: [publish]'; then
    ok "installer smoke test waits for complete publication"
else
    fail "installer smoke test can run before publication completes"
fi

echo
if [ "$FAIL" -eq 0 ]; then
    echo "RESULT: PASS ($PASS checks)"
    exit 0
fi
echo "RESULT: FAIL ($FAIL checks failed; $PASS passed)"
exit 1
