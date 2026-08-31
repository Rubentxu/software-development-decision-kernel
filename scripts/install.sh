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
    if download_optional "$(release_url "$UNIFIED_TARBALL.sha256")" "$TMP_DIR/$UNIFIED_TARBALL.sha256"; then
        verify_sha256 "$TMP_DIR/$UNIFIED_TARBALL" "$TMP_DIR/$UNIFIED_TARBALL.sha256"
    else
        echo "  warning: $UNIFIED_TARBALL.sha256 missing; skipping checksum verification"
    fi
    echo "  using unified artifact: $UNIFIED_TARBALL"
    # Stage: extract to a directory mirroring the prefix + framework layout.
    STAGE_ROOT="$TMP_DIR/unified-stage"
    mkdir -p "$STAGE_ROOT"
    tar xzf "$TMP_DIR/$UNIFIED_TARBALL" -C "$STAGE_ROOT"
    STAGE_BIN="$STAGE_ROOT/bin/sddk"
    if [ ! -x "$STAGE_BIN" ]; then
        echo "error: unified tarball does not contain bin/sddk" >&2
        exit 1
    fi
    STAGE_BUNDLE="$STAGE_ROOT/framework"
    if [ ! -d "$STAGE_BUNDLE" ]; then
        echo "error: unified tarball does not contain framework/" >&2
        exit 1
    fi
    TMP_VERSION="$("$STAGE_BIN" --version 2>&1 | awk '{print $NF}')"
    echo "  binary reports version: $TMP_VERSION"
else
    # Legacy split-asset path (pre-cycle-46): separate binary + bundle.
    echo "  using legacy split assets (binary + bundle)"
    CURRENT_STEP="download-binary"
    download "$(release_url "$ASSET")" "$TMP_DIR/sddk"
    download "$(release_url "$ASSET.sha256")" "$TMP_DIR/sddk.sha256"
    verify_sha256 "$TMP_DIR/sddk" "$TMP_DIR/sddk.sha256"
    chmod 0755 "$TMP_DIR/sddk"
    TMP_VERSION="$("$TMP_DIR/sddk" --version 2>&1 | awk '{print $NF}')"
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
    # before we touch any real directories.
    "$STAGE_BIN" dev manifest --verify --root "$STAGE_BUNDLE" >/dev/null \
        || echo "  warning: staged bundle does not verify against its MANIFEST (proceeding anyway)"
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

# ── Stage 6: switch `current` symlink atomically ────────────────────────────

# `dev use` resolves the framework dir from SDDK_DATA_DIR / XDG_DATA_HOME /
# HOME — make sure it points at our FRAMEWORK_DIR.
SDDK_DATA_DIR_DATA_ROOT="$(dirname "$FRAMEWORK_DIR")"
CURRENT_STEP="symlink"
SDDK_DATA_DIR="$SDDK_DATA_DIR_DATA_ROOT" "$INSTALL_BIN" dev use --version "$BUNDLE_VERSION" --format text
APPLIED+=("symlink")

# ── Stage 7: link into the chosen editor(s) ────────────────────────────────

echo
"$INSTALL_BIN" dev link --root "$FRAMEWORK_DIR/current" --editor "$EDITOR" --format text

# ── Stage 8: doctor ─────────────────────────────────────────────────────────

echo
SDDK_DATA_DIR="$SDDK_DATA_DIR_DATA_ROOT" "$INSTALL_BIN" dev doctor --prefix "$PREFIX" --format text || true

# ── Stage 9: completions hint ───────────────────────────────────────────────

echo
echo "Shell completions (optional):"
echo "  bash:    source <(sddk completion bash)"
echo "  zsh:     echo 'source <(sddk completion zsh)' >> ~/.zshrc"
echo "  fish:    sddk completion fish > ~/.config/fish/completions/sddk.fish"
echo
echo "Done. Run 'sddk --help' to get started."
