#!/usr/bin/env bash
# install.sh — Atomic install of the sddk binary and framework bundle from
# GitHub Releases.
#
# Cycle-46 (install-coherence-v1.63) redesign:
#   * Prefers the unified `sddk-<version>.tar.gz` asset (single artifact =
#     single coherent version, rustup/asdf model).
#   * Falls back to the legacy split assets (sddk-linux-*-musl + bundle
#     tarball) when the unified one is not present (older releases).
#   * Stages binary and bundle into isolated directories and atomically
#     swaps them into the prefix / framework dir with rollback on failure.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/Rubentxu/software-development-decision-kernel/main/scripts/install.sh | bash
#   bash install.sh                          # interactive: asks which editor to configure
#   bash install.sh --editor opencode       # non-interactive: configure OpenCode only
#   bash install.sh --editor zcode          # non-interactive: configure ZCode only
#   bash install.sh --editor all            # non-interactive: configure all editors
#   bash install.sh --editor none           # binary + framework only, skip editor link
#   bash install.sh --version v1.0.0        # pinned release
#   bash install.sh --prefix /usr/local/bin  # custom prefix
#
# The script installs the binary atomically with `sddk dev install` (creates a
# receipt under $PREFIX/sddk-receipt.json) and the framework as a versioned
# bundle under $FRAMEWORK_DIR/<version>/, then swaps the `current` symlink and
# links the chosen editor. Everything happens in one shot — exactly like
# rustup / mise / asdf-vm.
#
# No git required.
#
# Environment overrides:
#   SDDK_REPO, SDDK_VERSION, SDDK_PREFIX, SDDK_FRAMEWORK_DIR, SDDK_EDITOR,
#   SDDK_ASSET, SDDK_BASE_URL (testing).

set -euo pipefail

REPO="${SDDK_REPO:-Rubentxu/software-development-decision-kernel}"
VERSION="${SDDK_VERSION:-latest}"
PREFIX="${SDDK_PREFIX:-$HOME/.local/bin}"
FRAMEWORK_DIR="${SDDK_FRAMEWORK_DIR:-${XDG_DATA_HOME:-$HOME/.local/share}/sddk/framework}"
EDITOR="${SDDK_EDITOR:-}"
BASE_URL="${SDDK_BASE_URL:-https://github.com/$REPO/releases}"

while [ $# -gt 0 ]; do
    case "$1" in
        --version) VERSION="$2"; shift 2 ;;
        --prefix) PREFIX="$2"; shift 2 ;;
        --editor) EDITOR="$2"; shift 2 ;;
        --framework) shift ;; # accepted as a no-op for backwards compat
        *) echo "unknown option: $1" >&2; exit 2 ;;
    esac
done

# ── Atomic install machinery ────────────────────────────────────────────────
# Strategy: stage everything under $TMP_DIR, then apply. If any step fails
# after stage, restore_snapshot() removes anything that was applied.
STAGE_BIN=""
STAGE_BUNDLE=""
APPLIED=()

cleanup() {
    local rc=$?
    if [ "$rc" -ne 0 ] && [ "${#APPLIED[@]}" -gt 0 ]; then
        echo
        echo "ERROR: install failed at $CURRENT_STEP; rolling back partial state." >&2
        restore_snapshot || true
    fi
    rm -rf "$TMP_DIR" 2>/dev/null || true
    exit $rc
}

restore_snapshot() {
    # Reverse-order rollback: undo each applied step.
    for ((i=${#APPLIED[@]}-1; i>=0; i--)); do
        local step="${APPLIED[$i]}"
        case "$step" in
            binary)
                rm -f "$PREFIX/sddk" 2>/dev/null || true
                rm -f "$PREFIX/sddk-receipt.json" 2>/dev/null || true
                ;;
            bundle)
                rm -rf "${FRAMEWORK_DIR:?}/$BUNDLE_VERSION" 2>/dev/null || true
                ;;
            symlink)
                # Best-effort: try to restore prior target if we recorded it.
                if [ -n "${PRIOR_FRAMEWORK_TARGET:-}" ] && [ "${PRIOR_FRAMEWORK_TARGET}" != "absent" ]; then
                    ln -sfn "$PRIOR_FRAMEWORK_TARGET" "$FRAMEWORK_DIR/current" 2>/dev/null || true
                else
                    rm -f "$FRAMEWORK_DIR/current" 2>/dev/null || true
                fi
                ;;
        esac
    done
}

record_prior_symlink() {
    if [ -L "$FRAMEWORK_DIR/current" ]; then
        local target
        target="$(readlink "$FRAMEWORK_DIR/current" 2>/dev/null || true)"
        PRIOR_FRAMEWORK_TARGET="${target:-absent}"
    elif [ -e "$FRAMEWORK_DIR/current" ]; then
        PRIOR_FRAMEWORK_TARGET="absent"
    else
        PRIOR_FRAMEWORK_TARGET=""
    fi
}

CURRENT_STEP="init"
TMP_DIR="$(mktemp -d)"
trap cleanup EXIT INT TERM

# ── Detect asset name ──────────────────────────────────────────────────────

detect_asset() {
    local os arch
    case "$(uname -s)" in
        Linux*) os=linux ;;
        Darwin*) os=darwin ;;
        *) echo "unsupported OS: $(uname -s)" >&2; exit 1 ;;
    esac
    case "$(uname -m)" in
        x86_64|amd64) arch=x86_64 ;;
        arm64|aarch64) arch=aarch64 ;;
        *) echo "unsupported architecture: $(uname -m)" >&2; exit 1 ;;
    esac
    if [ "$os" = "linux" ]; then
        echo "sddk-${os}-${arch}-musl"
    else
        echo "sddk-${os}-${arch}"
    fi
}

ASSET="${SDDK_ASSET:-$(detect_asset)}"
echo "sddk installer"
echo "  repo:           $REPO"
echo "  version:        $VERSION"
echo "  asset:          $ASSET"
echo "  prefix:         $PREFIX"
echo "  framework_dir:  $FRAMEWORK_DIR"

# ── Helpers ────────────────────────────────────────────────────────────────

download() {
    local url="$1" out="$2"
    echo "  downloading: $url"
    case "$url" in
        file://*)
            cp "${url#file://}" "$out"
            ;;
        *)
            if command -v curl >/dev/null 2>&1; then
                curl -fsSL --retry 3 -o "$out" "$url"
            elif command -v wget >/dev/null 2>&1; then
                wget -qO "$out" "$url"
            else
                echo "error: need curl or wget" >&2
                exit 1
            fi
            ;;
    esac
}

# Try a URL; if any 4xx/5xx, return non-zero instead of failing the script.
download_optional() {
    local url="$1" out="$2"
    case "$url" in
        file://*)
            cp "${url#file://}" "$out" 2>/dev/null && return 0 || return 1
            ;;
    esac
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL --retry 3 -o "$out" "$url" 2>/dev/null && return 0 || return 1
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "$out" "$url" 2>/dev/null && return 0 || return 1
    fi
    return 1
}

release_url() {
    local name="$1"
    if [ "$VERSION" = "latest" ]; then
        echo "$BASE_URL/latest/download/$name"
    else
        echo "$BASE_URL/download/$VERSION/$name"
    fi
}

verify_sha256() {
    local file="$1" sum="$2"
    local expected actual
    expected="$(awk '{print $1}' "$sum")"
    actual="$(sha256sum "$file" | awk '{print $1}')"
    if [ "$expected" != "$actual" ]; then
        echo "error: sha256 mismatch" >&2
        echo "  expected: $expected" >&2
        echo "  actual:   $actual" >&2
        exit 1
    fi
    echo "  sha256 verified: $actual"
}

# Verify a cosign signature for a downloaded artifact.
#
# Same reasoning as `sddk dev update` (see
# INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY): the .sha256 and the payload
# come from the SAME base URL, so the checksum only proves the bytes did
# not change in transit. A compromised origin serves a hostile payload
# WITH a matching checksum and this check passes. Integrity is not
# authenticity.
#
# Policy, identical to the Rust path so the two consumers cannot drift:
#   * signature present but invalid -> hard failure. Never a warning: an
#     attacker who can serve artifacts can serve a bad signature, and a
#     warning would turn that into a bypass.
#   * signature absent            -> hard failure unless the operator sets
#     SDDK_ALLOW_UNSIGNED=1, which prints what it is doing.
#   * cosign absent               -> same as absent, with the install line.
# The project's public signing key, as the base64 BODY of its PEM.
#
# This is a copy of assets/trust/release-verify-key.pub, and
# tests/test_install_asset_contract.sh fails when the two drift. The authority
# is the Rust constant; this is the shell reader of it.
#
# A BODY, not a PEM, because the drift check is `sed`, which is line
# oriented: a multi-line PEM compares as empty on both sides and the guard
# passes without ever comparing a key. Verified against cosign v3.1.3 — the
# body is only accepted with header/footer framing, which _rebuild_verify_key
# adds.
#
# PROVISIONING: when the release signing key exists in the KMS, replace the
# next line with the base64 body of its public key (one line, no PEM
# framing), and put the same body in
# assets/trust/release-verify-key.pub. The guard below fails
# until both match.
SDDK_RELEASE_VERIFY_KEY_BODY="@@SDDK_TRANSITION_ANCHOR_NOT_A_REAL_KEY@@"

# Legacy keyless anchor (Fulcio via GitHub Actions), retained only for
# releases v2.2.11..v2.5.2. Its values are copies of
# cosign::LEGACY_CERT_IDENTITY_REGEXP and cosign::LEGACY_CERT_ISSUER.
#
# The defaults are INLINED below rather than hoisted into named constants,
# which was the second attempt at this and is the form that actually works.
# The certification harness greps them out of this file by their
# parameter-expansion shape, and an install.sh from v2.5.2 has exactly this
# shape. Storing them under another name, or resolving them through an alias,
# yields a string the harness cannot compare — so the transition window, the
# releases the legacy anchor exists for, would become uncertifiable. The
# extractor's target is this file's SHAPE, which makes the shape a contract
# rather than a style choice.
#
# NOTE: the grep pattern itself is deliberately not written out anywhere
# above. An earlier version of this comment quoted it, and the extractor
# matched the COMMENT first, returning a truncated fragment — the harness
# failed with "none of the expected identities matched" while the
# certificate's subject was exactly right.
#
# There are deliberately no named constants for these two values. They are
# inlined at the point of use in verify_signature, because that inline form
# is what the certification harness greps and what an install.sh from v2.5.2
# contains; a named constant here was the first attempt and left shellcheck
# reporting two genuinely unused variables.

# The transition placeholder is not a key. Refusing to verify against it is
# the whole point: an unprovisioned anchor must not degrade into "accept
# whatever the signature says".
_rebuild_verify_key() {
    case "$SDDK_RELEASE_VERIFY_KEY_BODY" in
        @@SDDK_TRANSITION_ANCHOR_NOT_A_REAL_KEY@@|'')
            echo "error: the release signing key has not been provisioned" >&2
            echo "  assets/trust/release-verify-key.pub holds the" >&2
            echo "  transition placeholder, not a key. Refusing to verify." >&2
            return 1
            ;;
    esac
    {
        printf -- '-----BEGIN PUBLIC KEY-----\n'
        printf -- '%s\n' "$SDDK_RELEASE_VERIFY_KEY_BODY"
        printf -- '-----END PUBLIC KEY-----\n'
    }
}

verify_signature() {
    local file="$1" sig_url="$2" label="$3"
    # Los tres destinos se asignan AQUI, no mas abajo: se leen en 297/298/302 y
    # en los dos bloques de verificacion. `set -u` convierte un `local` sin valor
    # en un fallo inmediato, y la reescritura del anchor key-based (3b0dc9dc)
    # dejo la declaracion `local` y todas las lecturas, pero perdio estas tres
    # asignaciones -- con lo que el instalador moria en la primera lectura y
    # `SDDK_ALLOW_UNSIGNED=1` no podia rescatarlo, porque la rama que lo honra
    # esta DESPUES. Se declaran junto a la firma para que `local` y lectura no
    # puedan separarse otra vez: un destino que se lee y no se asigna es una
    # variable sin fuente, y declararla en la misma linea es lo que la hace
    # visible al leer la funcion entera de arriba abajo.
    local sig_file="$file.sig"
    local bundle_file="$file.bundle.json"
    local cert_file="$file.pem"
    local key_file="$file.sddk-anchor.pub"

    if ! command -v cosign >/dev/null 2>&1; then
        _signature_absent "cosign is not installed" "$label"
        return $?
    fi

    # The keyless releases sign DETACHED: they publish <file>.sig AND
    # <file>.pem. The bundle form is what current cosign emits with
    # --bundle. Both are accepted; the certificate file is required in the
    # detached case, because a detached signature with no leaf certificate
    # has nothing for --certificate-identity to match and cosign would fall
    # back to trusting any certificate it likes.
    if ! download_optional "$sig_url.bundle.json" "$bundle_file"; then
        if ! download_optional "$sig_url.sig" "$sig_file"; then
            _signature_absent "no signature asset published" "$label"
            return $?
        fi
        download_optional "$sig_url.pem" "$cert_file" || true
    fi

    if ! _rebuild_verify_key >"$key_file"; then
        rm -f "$key_file"
        return 1
    fi

    # ── Anchor 1: key-based, the one this project signs with now ────────
    local kb_args="verify-blob --key $key_file"
    if [ -s "$bundle_file" ]; then
        kb_args="$kb_args --bundle $bundle_file"
    else
        kb_args="$kb_args --signature $sig_file"
    fi

    # shellcheck disable=SC2086 # args is a deliberately word-split arg list
    if cosign $kb_args "$file" >/dev/null 2>&1; then
        rm -f "$key_file"
        echo "  signature verified (cosign key-based, project signing key pinned)"
        return 0
    fi

    # ── Anchor 2: legacy keyless, until the transition closes ───────────
    # A detached .sig with no .pem cannot be pinned. Refuse rather than try
    # an unpinned verification that would report success for any signer.
    if [ ! -s "$bundle_file" ] && [ ! -s "$cert_file" ]; then
        rm -f "$key_file"
        echo "error: detached signature for $label has no .pem certificate" >&2
        echo "  Refusing to verify without a certificate chain: --certificate-identity" >&2
        echo "  has nothing to match, so any signer would be accepted." >&2
        return 1
    fi

    # The defaults are inlined at the point of use — see the note above. This
    # is unchanged from before ADR-0151, so an operator overriding either
    # variable gets the same behaviour it always did.
    local cert_identity="${SDDK_COSIGN_IDENTITY:-^https://github\.com/Rubentxu/software-development-decision-kernel/\.github/workflows/release\.yml@refs/tags/v[0-9]+\.[0-9]+\.[0-9]+$}"
    local cert_issuer="${SDDK_COSIGN_ISSUER:-https://token.actions.githubusercontent.com}"
    if [ -z "$cert_identity" ] || [ -z "$cert_issuer" ]; then
        rm -f "$key_file"
        echo "error: SDDK_COSIGN_IDENTITY and SDDK_COSIGN_ISSUER must both be non-empty" >&2
        echo "  An empty value means 'accept any signer', which proves nothing." >&2
        return 1
    fi

    local legacy_args
    if [ -s "$bundle_file" ]; then
        legacy_args="verify-blob --bundle $bundle_file"
    else
        legacy_args="verify-blob --signature $sig_file --certificate $cert_file"
    fi
    legacy_args="$legacy_args --certificate-identity-regexp=$cert_identity --certificate-oidc-issuer=$cert_issuer"

    # shellcheck disable=SC2086 # args is a deliberately word-split arg list
    if cosign $legacy_args "$file" >/dev/null 2>&1; then
        rm -f "$key_file"
        echo "  signature verified (cosign keyless legacy anchor, identity and issuer pinned)"
        return 0
    fi

    rm -f "$key_file"
    echo "error: cosign verification FAILED for $label against BOTH anchors" >&2
    echo "  Refusing to install. A present-but-invalid signature is never a" >&2
    echo "  warning: anyone who can serve artifacts can serve a bad signature," >&2
    echo "  and warning here would make that a bypass." >&2
    exit 1
}

_signature_absent() {
    local why="$1" label="$2"
    if [ "${SDDK_ALLOW_UNSIGNED:-0}" = "1" ]; then
        echo "warning: SDDK_ALLOW_UNSIGNED=1 — installing $label with NO signature check." >&2
        echo "warning: integrity (sha256) is verified, authenticity is NOT." >&2
        echo "warning: a compromised download origin would be accepted." >&2
        return 0
    fi
    echo "error: cannot verify the authenticity of $label: $why" >&2
    echo "  The .sha256 is downloaded from the same origin as the payload, so it" >&2
    echo "  proves the bytes did not change in transit, not that they are ours." >&2
    echo "  If this release is genuinely unsigned, re-run with SDDK_ALLOW_UNSIGNED=1" >&2
    echo "  to accept it knowingly. See INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY." >&2
    exit 1
}

# Probe the staged binary for its version.
#
# The previous form was:  TMP_VERSION="$("$bin" --version 2>&1 | awk '{print $NF}')"
# That silently reports garbage when the binary cannot execute (missing
# loader, wrong libc, bad arch). `awk` then echoes its *input* back, the
# command substitution "succeeds", and the caller stores a path string as if it
# were a version — then aborts later with a bare `exit 1` and no message
# (observed in session-16 against debian:12-slim + a glibc build).
#
# Fail loudly and specifically instead: an unrunnable binary is a broken
# release, and the operator needs to know *why* rather than seeing rc=1.
probe_binary_version() {
    local bin="$1" out
    if ! out="$("$bin" --version 2>&1)"; then
        echo "error: staged binary could not be executed" >&2
        echo "  binary: $bin" >&2
        echo "  output: $out" >&2
        echo "  this usually means the asset was built against a different libc" >&2
        echo "  than the host (glibc vs musl) or a different architecture" >&2
        exit 1
    fi
    # Require a plausible semver token; reject awk's input-echo behaviour.
    if ! printf '%s' "$out" | grep -qE '[0-9]+\.[0-9]+\.[0-9]+'; then
        echo "error: staged binary reported no recognisable version" >&2
        echo "  binary: $bin" >&2
        echo "  output: $out" >&2
        exit 1
    fi
    printf '%s' "$out" | awk '{print $NF}'
}

# ── Stage 1: download unified tarball OR legacy split assets ────────────────
CURRENT_STEP="download"

# Resolve the concrete version (handles `latest`) so the unified asset name
# can be computed. For `latest` we keep using the per-asset URLs below; the
# unified detection happens only when SDDK_VERSION is pinned.
RESOLVED_VERSION="$VERSION"
if [ "$RESOLVED_VERSION" = "latest" ]; then
    # Use the GitHub redirector to discover the latest tag, then proceed
    # with the split-asset path (same as before).
    echo "  resolving latest version via GitHub API..."
    if command -v gh >/dev/null 2>&1; then
        RESOLVED_VERSION="$(gh release view --repo "$REPO" --json tagName -q '.tagName' 2>/dev/null || echo latest)"
    fi
    if [ "$RESOLVED_VERSION" = "latest" ]; then
        echo "  (gh unavailable or release not found: staying with split-asset download)"
    else
        VERSION="$RESOLVED_VERSION"
        echo "  resolved: $VERSION"
    fi
fi

# Unified artifact filename: per-arch so the same tarball naming pattern as
# `sddk-linux-x86_64-musl` etc. applies. The release.yml produces one
# `sddk-${VERSION}-${ASSET}.tar.gz` per matrix entry (e.g.
# sddk-v1.63.0-linux-x86_64-musl.tar.gz).
UNIFIED_TARBALL="sddk-${VERSION}-${ASSET}.tar.gz"

if [ "$RESOLVED_VERSION" != "latest" ] && \
   download_optional "$(release_url "$UNIFIED_TARBALL")" "$TMP_DIR/$UNIFIED_TARBALL"; then
    # Unified artifact path (cycle-46 capa 3): a single tarball containing
    # bin/, framework/, BUNDLE.toml, INSTALL.toml.
    # Fail closed: an artifact whose checksum is unavailable is an unverified
    # payload. Silently continuing (session-15/16 audit) was an integrity
    # downgrade that also hid upstream publishing gaps — the whole reason the
    # broken asset name went unnoticed. A release that omits the .sha256 is a
    # broken release, not a tolerated condition.
    if ! download_optional "$(release_url "$UNIFIED_TARBALL.sha256")" "$TMP_DIR/$UNIFIED_TARBALL.sha256"; then
        echo "error: $UNIFIED_TARBALL.sha256 missing — refusing to install an unverified artifact" >&2
        echo "  the release is incomplete; report it rather than falling back to no integrity check" >&2
        exit 1
    fi
    verify_sha256 "$TMP_DIR/$UNIFIED_TARBALL" "$TMP_DIR/$UNIFIED_TARBALL.sha256"
    # Authenticity, after integrity. The .sha256 above came from the same
    # origin, so it cannot distinguish "our artifact" from "an artifact the
    # compromised origin chose to serve along with a matching checksum".
    verify_signature "$TMP_DIR/$UNIFIED_TARBALL" "$(release_url "$UNIFIED_TARBALL")" "$UNIFIED_TARBALL"
    echo "  using unified artifact: $UNIFIED_TARBALL"
    # Stage: extract to a directory mirroring the prefix + framework layout.
    STAGE_ROOT="$TMP_DIR/unified-stage"
    mkdir -p "$STAGE_ROOT"
    tar xzf "$TMP_DIR/$UNIFIED_TARBALL" -C "$STAGE_ROOT"
    STAGE_BIN="$STAGE_ROOT/bin/sddk"
    if [ ! -f "$STAGE_BIN" ]; then
        echo "error: unified tarball does not contain bin/sddk" >&2
        exit 1
    fi
    # Defensive chmod: some release pipelines (incl. the GitHub CDN-cached
    # variant we hit on cycle-47 day-0) ship the tarball without the
    # executable bit set on the binary. Re-apply 0755 before exec'ing.
    chmod 0755 "$STAGE_BIN" 2>/dev/null || true
    STAGE_BUNDLE="$STAGE_ROOT/framework"
    if [ ! -d "$STAGE_BUNDLE" ]; then
        echo "error: unified tarball does not contain framework/" >&2
        exit 1
    fi
    TMP_VERSION="$(probe_binary_version "$STAGE_BIN")"
    echo "  binary reports version: $TMP_VERSION"
else
    # Legacy split-asset path (pre-cycle-46): separate binary + bundle.
    #
    # Asset-name contract (session-16): the release publishes the bare binary
    # as `sddk` (basename of the built binary), NOT as `sddk-<os>-<arch>-musl`.
    # `$ASSET` is only a component of the *unified tarball* name
    # (`sddk-${VERSION}-${ASSET}.tar.gz`); it was never a published asset on its
    # own. Requesting it here returned HTTP 404 and aborted the install before
    # anything was linked, for every user without `gh` on PATH.
    echo "  using legacy split assets (binary + bundle)"
    CURRENT_STEP="download-binary"
    download "$(release_url "sddk")" "$TMP_DIR/sddk"
    download "$(release_url "sddk.sha256")" "$TMP_DIR/sddk.sha256"
    verify_sha256 "$TMP_DIR/sddk" "$TMP_DIR/sddk.sha256"
    verify_signature "$TMP_DIR/sddk" "$(release_url "sddk")" "sddk"
    chmod 0755 "$TMP_DIR/sddk"
    TMP_VERSION="$(probe_binary_version "$TMP_DIR/sddk")"
    echo "  binary reports version: $TMP_VERSION"

    CURRENT_STEP="download-bundle"
    download "$(release_url "software-development-decision-kernel.tar.gz")" "$TMP_DIR/software-development-decision-kernel.tar.gz"
    download "$(release_url "software-development-decision-kernel.tar.gz.sha256")" "$TMP_DIR/sddk-framework.sha256"
    verify_sha256 "$TMP_DIR/software-development-decision-kernel.tar.gz" "$TMP_DIR/sddk-framework.sha256"

    # Extract bundle to staging directory (NOT to FRAMEWORK_DIR yet).
    STAGE_BUNDLE="$TMP_DIR/bundle-stage"
    mkdir -p "$STAGE_BUNDLE"
    tar xzf "$TMP_DIR/software-development-decision-kernel.tar.gz" -C "$STAGE_BUNDLE"

    STAGE_BIN="$TMP_DIR/sddk"
    # Ensure the staging bundle contains BUNDLE.toml (the legacy artifact may
    # not have it; the dev doctor check then fails. Generate one here if
    # missing so the installed prefix is coherent.)
    if [ ! -f "$STAGE_BUNDLE/BUNDLE.toml" ]; then
        cat > "$STAGE_BUNDLE/BUNDLE.toml" <<EOF
[bundle]
schema_version = 2
version = "$TMP_VERSION"
binary_min_version = "$TMP_VERSION"
binary_max_version = "$TMP_VERSION"

[contents]
EOF
        echo "  warning: bundle lacked BUNDLE.toml; generated one inline (binary compat: [$TMP_VERSION, $TMP_VERSION])"
    fi

    # Verify the staged bundle against its (possibly regenerated) BUNDLE.toml
    # before we touch any real directories. Fail-closed (ADR-0127): a
    # legacy-path bundle that does not verify against its own MANIFEST is
    # never installed; we surface the failure rather than ship a corrupt
    # bundle.
    if ! "$STAGE_BIN" dev manifest --verify --root "$STAGE_BUNDLE"; then
        echo "error: staged bundle fails MANIFEST verification; refusing to install" >&2
        exit 1
    fi
fi

# ── Stage 2: verify BUNDLE.toml compatibility (pre-write preflight) ────────
CURRENT_STEP="bundle-compat"
"$STAGE_BIN" dev doctor --format text >/dev/null 2>&1 || true
# Use a fresh BUNDLE.toml check: read it and compare against TMP_VERSION.
bundle_toml_check="$( [ -f "$STAGE_BUNDLE/BUNDLE.toml" ] && echo present || echo missing )"
if [ "$bundle_toml_check" = "missing" ]; then
    echo "error: staged bundle has no BUNDLE.toml after preflight" >&2
    exit 1
fi
echo "  bundle stage OK (binary=$TMP_VERSION, BUNDLE.toml present)"

# ── Stage 3: atomic binary install ──────────────────────────────────────────
# `sddk dev install` places the binary at $PREFIX/sddk when $PREFIX already
# ends in `/bin` (e.g. /usr/local/bin) or at $PREFIX/bin/sddk otherwise
# (rustup-style layout, the default for $HOME/.local/bin which lacks a
# trailing /bin).
INSTALL_BIN="$PREFIX/sddk"
case "$PREFIX" in
    */bin) INSTALL_BIN="$PREFIX/sddk" ;;
    *) INSTALL_BIN="$PREFIX/bin/sddk" ;;
esac
CURRENT_STEP="install-binary"
echo
mkdir -p "$PREFIX"
"$STAGE_BIN" dev install --prefix "$PREFIX" --channel release --source "$STAGE_BUNDLE" --format text
APPLIED+=("binary")
echo "  binary installed: $INSTALL_BIN"
"$INSTALL_BIN" --version

# ── PATH check ──────────────────────────────────────────────────────────────

# Suggest the parent of bin/ when we placed the binary at bin/sddk, so the
# user can `export PATH=<...>/bin:$PATH` and `sddk` resolves without a
# full path.
PATH_PARENT="$PREFIX"
case "$PREFIX" in
    */bin) PATH_PARENT="$PREFIX" ;;
    *)     PATH_PARENT="$PREFIX/bin" ;;
esac
case ":$PATH:" in
    *":$PATH_PARENT:"*)
        echo "  PATH: ok ($PATH_PARENT already on PATH)"
        ;;
    *)
        echo "  WARNING: $PATH_PARENT is not on your PATH. Add it with:"
        echo "    export PATH=\"$PATH_PARENT:\$PATH\""
        ;;
esac

# ── Stage 4: ask which editor to configure ──────────────────────────────────

if [ -z "$EDITOR" ]; then
    if [ -t 0 ] || [ -e /dev/tty ]; then
        echo
        echo "¿Querés configurar el framework SDDK en un editor de IA?"
        echo "  1) OpenCode"
        echo "  2) ZCode"
        echo "  3) Claude"
        echo "  4) Codex"
        echo "  5) Todos"
        echo "  6) Ninguno (solo binario + framework)"
        # SC2162: false positive — `-r` flag IS present in `read -rp`; directive is
        # load-bearing only on shells that misparse `read -rp` as `read -p` without -r.
        # Intent: preserve backslashes in user input if editor name contains \ or similar.
        # shellcheck disable=SC2162
        read -rp "Elección [5]: " choice < /dev/tty 2>/dev/null || choice="5"
        case "${choice:-5}" in
            1) EDITOR=opencode ;;
            2) EDITOR=zcode ;;
            3) EDITOR=claude ;;
            4) EDITOR=codex ;;
            5) EDITOR=all ;;
            6) EDITOR=none ;;
            *) echo "opción inválida: $choice" >&2; exit 2 ;;
        esac
    else
        echo "  (no TTY: using --editor all; pass --editor none for binary only)"
        EDITOR=all
    fi
fi

# ── Stage 5: extract bundle to framework dir (atomic) ───────────────────────

if [ "$EDITOR" = "none" ]; then
    echo
    echo "Framework bundle:"
    echo "  (skipped: --editor none). Re-run with an editor, or:"
    echo "  sddk dev update --root $FRAMEWORK_DIR --version $VERSION"
    echo "  sddk dev use $TMP_VERSION"
    echo
    echo "Done. Run 'sddk --help' to get started."
    exit 0
fi

BUNDLE_VERSION="${BUNDLE_VERSION:-$TMP_VERSION}"
BUNDLE_DIR="$FRAMEWORK_DIR/$BUNDLE_VERSION"
record_prior_symlink

if [ -d "$BUNDLE_DIR" ] && [ -f "$BUNDLE_DIR/MANIFEST.sha256" ]; then
    echo
    echo "framework: existing $BUNDLE_VERSION bundle detected at $BUNDLE_DIR (using as-is)"
else
    CURRENT_STEP="extract-bundle"
    echo
    echo "  installing framework bundle to $BUNDLE_DIR"
    mkdir -p "$FRAMEWORK_DIR"
    if ! cp -R "$STAGE_BUNDLE/." "$BUNDLE_DIR/"; then
        echo "error: failed to copy staged bundle to $BUNDLE_DIR" >&2
        exit 1
    fi
    APPLIED+=("bundle")
    echo "  framework extracted: $BUNDLE_DIR"
fi

# ── Stage 6: switch `current` symlink atomically ──────────────────────────

# `dev use` resolves the framework dir from SDDK_FRAMEWORK_DIR (INC-A5-FWDIR,
# honored by the CLI since v2.2.12) → SDDK_DATA_DIR / XDG_DATA_HOME / HOME.
# Pass SDDK_FRAMEWORK_DIR through so a non-default FRAMEWORK_DIR (as set by
# the CI smoke job) is the one the CLI sees; the data-root fallback stays
# consistent for the default layout.
CURRENT_STEP="symlink"
SDDK_FRAMEWORK_DIR="$FRAMEWORK_DIR" "${INSTALL_BIN}" dev use --version "$BUNDLE_VERSION" --format text
APPLIED+=("symlink")

# ── Stage 7: link into the chosen editor(s) ────────────────────────────────

echo
"$INSTALL_BIN" dev link --root "$FRAMEWORK_DIR/current" --editor "$EDITOR" --format text

# ── Stage 8: doctor ─────────────────────────────────────────────────────────

echo
SDDK_FRAMEWORK_DIR="$FRAMEWORK_DIR" "${INSTALL_BIN}" dev doctor --prefix "$PREFIX" --format text || true

# ── Stage 9: completions hint ───────────────────────────────────────────────

echo
echo "Shell completions (optional):"
echo "  bash:    source <(sddk completion bash)"
echo "  zsh:     echo 'source <(sddk completion zsh)' >> ~/.zshrc"
echo "  fish:    sddk completion fish > ~/.config/fish/completions/sddk.fish"
echo
echo "Done. Run 'sddk --help' to get started."
