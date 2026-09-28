#!/usr/bin/env bash
# Shared public-release asset contract.
# Sourced by scripts/release.sh and tests/lib_public_release_gate.sh so the
# tested policy is the same function the production release path executes.
# Callers must define die() and ok() before invoking this function.
# On success, RELEASE_CANONICAL_ASSETS contains the 9 payloads to probe.
validate_release_asset_contract() {
    local release_json="$1"
    local version="$2"

    local canonical_assets=(
        "sddk"
        "sddk.sha256"
        "sddk-v$version-sddk-linux-x86_64-musl.tar.gz"
        "sddk-v$version-sddk-linux-x86_64-musl.tar.gz.sha256"
        "CHECKSUMS"
        "sbom.json"
        "gh-release-receipt.json"
        "software-development-decision-kernel.tar.gz"
        "software-development-decision-kernel.tar.gz.sha256"
    )
    local signed_base_assets=(
        "sddk"
        "sddk-v$version-sddk-linux-x86_64-musl.tar.gz"
        "software-development-decision-kernel.tar.gz"
    )
    local allowed_signature_assets=()
    local signed_base
    local signature_suffix
    for signed_base in "${signed_base_assets[@]}"; do
        for signature_suffix in .sig .pem .bundle.json; do
            allowed_signature_assets+=("$signed_base$signature_suffix")
        done
    done

    local actual_assets
    actual_assets="$(echo "$release_json" | jq -r '.assets[].name' | sort -u)"
    local expected_assets_sorted
    expected_assets_sorted="$(printf '%s\n' "${canonical_assets[@]}" | sort -u)"
    local allowed_assets_sorted
    allowed_assets_sorted="$(printf '%s\n' "${allowed_signature_assets[@]}" | sort -u)"

    local missing_assets
    missing_assets="$(comm -23 <(echo "$expected_assets_sorted") <(echo "$actual_assets"))"
    if [ -n "$missing_assets" ]; then
        die "missing canonical assets: $(echo "$missing_assets" | tr '\n' ' ')"
    fi

    local extra_assets
    extra_assets="$(comm -13 \
        <(printf '%s\n' "$expected_assets_sorted" "$allowed_assets_sorted" | sort -u) \
        <(echo "$actual_assets"))"
    if [ -n "$extra_assets" ]; then
        die "unexpected assets replacing canonical ones: $(echo "$extra_assets" | tr '\n' ' ')"
    fi

    # Consumed by the sourcing gate function after this helper returns.
    # shellcheck disable=SC2034
    RELEASE_CANONICAL_ASSETS=("${canonical_assets[@]}")
    ok "asset set matches 9-asset contract plus allowed signatures"
}
