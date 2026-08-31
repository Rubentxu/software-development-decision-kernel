#!/usr/bin/env bash
# install.sh — Install the sddk binary and framework from GitHub Releases.
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

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

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

# --- 1. Binary ---

download "$(release_url "$ASSET")" "$TMP_DIR/sddk"
download "$(release_url "$ASSET.sha256")" "$TMP_DIR/sddk.sha256"
verify_sha256 "$TMP_DIR/sddk" "$TMP_DIR/sddk.sha256"
chmod 0755 "$TMP_DIR/sddk"
TMP_VERSION="$("$TMP_DIR/sddk" --version 2>&1 | awk '{print $NF}')"
echo "  binary reports version: $TMP_VERSION"

# --- 2. Atomic binary install (writes $PREFIX/sddk-receipt.json) ---

mkdir -p "$PREFIX"
"$TMP_DIR/sddk" dev install --prefix "$PREFIX" --channel release --format text
echo "  binary installed: $PREFIX/sddk"
"$PREFIX/sddk" --version

# --- PATH check ---

case ":$PATH:" in
    *":$PREFIX:"*)
        echo "  PATH: ok ($PREFIX already on PATH)"
        ;;
    *)
        echo "  WARNING: $PREFIX is not on your PATH. Add it with:"
        echo "    export PATH=\"$PREFIX:\$PATH\""
        ;;
esac

# --- 3. Ask which editor to configure ---

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

# --- 4. Framework bundle (versioned, asdf-style) ---

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

if [ -d "$BUNDLE_DIR" ] && [ -f "$BUNDLE_DIR/MANIFEST.sha256" ]; then
    echo
    echo "framework: existing $BUNDLE_VERSION bundle detected at $BUNDLE_DIR (using as-is)"
else
    download "$(release_url "software-development-decision-kernel.tar.gz")" "$TMP_DIR/software-development-decision-kernel.tar.gz"
    download "$(release_url "software-development-decision-kernel.tar.gz.sha256")" "$TMP_DIR/sddk-framework.sha256"
    verify_sha256 "$TMP_DIR/software-development-decision-kernel.tar.gz" "$TMP_DIR/sddk-framework.sha256"

    mkdir -p "$BUNDLE_DIR"
    tar xzf "$TMP_DIR/software-development-decision-kernel.tar.gz" -C "$BUNDLE_DIR"
    echo "  framework extracted: $BUNDLE_DIR"
fi

# --- 5. Switch `current` symlink to the freshly installed version ---

# `dev use` resolves the framework dir from SDDK_DATA_DIR / XDG_DATA_HOME /
# HOME — make sure it points at our FRAMEWORK_DIR.
SDDK_DATA_DIR_DATA_ROOT="$(dirname "$FRAMEWORK_DIR")"
SDDK_DATA_DIR="$SDDK_DATA_DIR_DATA_ROOT" "$PREFIX/sddk" dev use --version "$BUNDLE_VERSION" --format text

# --- 6. Link into the chosen editor(s) ---

echo
"$PREFIX/sddk" dev link --root "$FRAMEWORK_DIR/current" --editor "$EDITOR" --format text

# --- 7. Doctor ---

echo
SDDK_DATA_DIR="$SDDK_DATA_DIR_DATA_ROOT" "$PREFIX/sddk" dev doctor --format text || true

# --- 8. Completions hint ---

echo
echo "Shell completions (optional):"
echo "  bash:    source <(sddk completion bash)"
echo "  zsh:     echo 'source <(sddk completion zsh)' >> ~/.zshrc"
echo "  fish:    sddk completion fish > ~/.config/fish/completions/sddk.fish"
echo
echo "Done. Run 'sddk --help' to get started."