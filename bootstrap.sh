#!/usr/bin/env bash
# bootstrap.sh — Install the SDDK framework into detected AI coding editors.
#
# Usage:
#   ./bootstrap.sh                    # auto-detect editors, create symlinks
#   ./bootstrap.sh --zcode            # only ZCode
#   ./bootstrap.sh --opencode         # only OpenCode
#   ./bootstrap.sh --all              # all detected + force re-link
#
# This script symlinks agents/skills/prompts/workflows from the framework root
# (default: the dir containing this script = the CWD repo) into each editor's
# expected directory (~/.config/opencode, ~/.zcode, ...).
#
# For the runtime binary itself, prefer `sddk dev install` (atomic install with
# receipt) over hand-managed binaries. This script only handles content surfaces
# so a fresh checkout becomes immediately usable in any supported editor.

set -euo pipefail

SDDK_FRAMEWORK_ROOT="${SDDK_FRAMEWORK_ROOT:-$(cd "$(dirname "$0")" && pwd)}"
ZCODE_DIR="${ZCODE_DIR:-$HOME/.zcode}"
OPENCODE_DIR="${OPENCODE_DIR:-$HOME/.config/opencode}"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

info()  { echo -e "${GREEN}✅ $1${NC}"; }
warn()  { echo -e "${YELLOW}⚠️  $1${NC}"; }
error() { echo -e "${RED}❌ $1${NC}"; }

# --- Detect editors ---

detect_editors() {
    local editors=()
    [ -d "$ZCODE_DIR/agents" ] && editors+=("zcode")
    [ -d "$OPENCODE_DIR" ] && editors+=("opencode")
    echo "${editors[@]}"
}

# --- ZCode linking ---

link_zcode() {
    info "Linking ZCode agents..."
    mkdir -p "$ZCODE_DIR/agents"
    for f in "$SDDK_FRAMEWORK_ROOT"/agents/*.md; do
        name=$(basename "$f")
        target="$ZCODE_DIR/agents/$name"
        ln -sf "$f" "$target"
    done
    info "Linked $(ls "$ZCODE_DIR/agents"/*.md | wc -l) agents"

    info "Linking ZCode skills..."
    mkdir -p "$ZCODE_DIR/skills"
    for d in "$SDDK_FRAMEWORK_ROOT"/skills/*/; do
        name=$(basename "$d")
        target="$ZCODE_DIR/skills/$name"
        ln -sfn "$d" "$target"
    done
    info "Linked $(ls -d "$ZCODE_DIR/skills"/*/ | wc -l) skills"

    info "Linking ZCode workflows..."
    mkdir -p "$ZCODE_DIR/workflows"
    if [ -d "$SDDK_FRAMEWORK_ROOT/workflows" ]; then
        for f in "$SDDK_FRAMEWORK_ROOT"/workflows/*/; do
            [ -d "$f" ] || continue
            name=$(basename "$f")
            target="$ZCODE_DIR/workflows/$name"
            ln -sfn "$f" "$target"
        done
    fi
    if [ -d "$SDDK_FRAMEWORK_ROOT/prompts/sddk/workflows" ]; then
        for f in "$SDDK_FRAMEWORK_ROOT"/prompts/sddk/workflows/*.yaml; do
            [ -f "$f" ] || continue
            name=$(basename "$f")
            target="$ZCODE_DIR/workflows/$name"
            ln -sf "$f" "$target"
        done
    fi
    info "Linked $(ls "$ZCODE_DIR/workflows" 2>/dev/null | wc -l) workflow files"
}

# --- OpenCode linking ---

link_opencode() {
    info "Linking OpenCode skills..."
    mkdir -p "$OPENCODE_DIR/skills"
    for d in "$SDDK_FRAMEWORK_ROOT"/skills/*/; do
        name=$(basename "$d")
        target="$OPENCODE_DIR/skills/$name"
        ln -sfn "$d" "$target"
    done
    info "Linked $(ls -d "$OPENCODE_DIR/skills"/*/ 2>/dev/null | wc -l) skills"

    # Link BOOK-*.md top-level (where consumers expect them)
    for f in "$SDDK_FRAMEWORK_ROOT"/skills/BOOK-*.md; do
        [ -f "$f" ] || continue
        name=$(basename "$f")
        target="$OPENCODE_DIR/skills/$name"
        ln -sf "$f" "$target"
    done

    info "Linking OpenCode agents..."
    mkdir -p "$OPENCODE_DIR/agents"
    for f in "$SDDK_FRAMEWORK_ROOT"/agents/*.md; do
        name=$(basename "$f")
        target="$OPENCODE_DIR/agents/$name"
        ln -sf "$f" "$target"
    done
    info "Linked $(ls "$OPENCODE_DIR/agents"/*.md 2>/dev/null | wc -l) agents"

    info "Linking OpenCode prompts (sddk)..."
    mkdir -p "$OPENCODE_DIR/prompts/sddk"
    # Link phase specs and docs
    for f in "$SDDK_FRAMEWORK_ROOT"/prompts/sddk/*.md; do
        name=$(basename "$f")
        target="$OPENCODE_DIR/prompts/sddk/$name"
        ln -sf "$f" "$target"
    done
    # Link phase specs subdirectory
    mkdir -p "$OPENCODE_DIR/prompts/sddk/phases"
    for f in "$SDDK_FRAMEWORK_ROOT"/prompts/sddk/phases/*.md; do
        name=$(basename "$f")
        target="$OPENCODE_DIR/prompts/sddk/phases/$name"
        ln -sf "$f" "$target"
    done
    # Link templates subdirectory
    if [ -d "$SDDK_FRAMEWORK_ROOT/prompts/sddk/templates" ]; then
        mkdir -p "$OPENCODE_DIR/prompts/sddk/templates"
        for f in "$SDDK_FRAMEWORK_ROOT"/prompts/sddk/templates/*; do
            name=$(basename "$f")
            target="$OPENCODE_DIR/prompts/sddk/templates/$name"
            ln -sf "$f" "$target"
        done
    fi
    # Link workflows YAML registry (orchestrator reads these for path-specific sequences)
    mkdir -p "$OPENCODE_DIR/prompts/sddk/workflows"
    for f in "$SDDK_FRAMEWORK_ROOT"/prompts/sddk/workflows/*.yaml; do
        [ -f "$f" ] || continue
        name=$(basename "$f")
        target="$OPENCODE_DIR/prompts/sddk/workflows/$name"
        ln -sf "$f" "$target"
    done
    info "Linked sddk prompts + workflow registry"

    # Workflow root (top-level workflows/ tree — used by `sddk dev link`)
    info "Linking OpenCode workflows..."
    mkdir -p "$OPENCODE_DIR/workflows"
    if [ -d "$SDDK_FRAMEWORK_ROOT/workflows" ]; then
        for f in "$SDDK_FRAMEWORK_ROOT"/workflows/*/; do
            [ -d "$f" ] || continue
            name=$(basename "$f")
            target="$OPENCODE_DIR/workflows/$name"
            ln -sfn "$f" "$target"
        done
    fi
    info "Linked $(ls "$OPENCODE_DIR/workflows" 2>/dev/null | wc -l) workflow trees"

    info "OpenCode agents linked to: $OPENCODE_DIR/agents/"
    info "Register agents in opencode.json with: {file: \"$SDDK_FRAMEWORK_ROOT/agents/<name>.md\"}"
}

# --- Knowledge vault setup ---

setup_knowledge_base() {
    info "Knowledge graph template is at: $SDDK_FRAMEWORK_ROOT/knowledge-template/"
    info "Per-project vaults will be created at: \$HOME/.sddk-knowledge/{project}/ (in user home, outside repo)"
    info "  (auto-created on first SDDK cycle per project)"
}

# --- Main ---

main() {
    echo "🔍 SDDK Framework Bootstrap"
    echo "   Framework root: $SDDK_FRAMEWORK_ROOT"
    echo ""

    local editors
    if [ "${1:-}" = "--all" ]; then
        editors="zcode opencode"
    elif [ "${1:-}" = "--zcode" ]; then
        editors="zcode"
    elif [ "${1:-}" = "--opencode" ]; then
        editors="opencode"
    else
        editors=$(detect_editors)
    fi

    if [ -z "$editors" ]; then
        error "No editors detected. Install ZCode (~/.zcode/) or OpenCode (~/.config/opencode/) first."
        exit 1
    fi

    info "Detected editors: $editors"
    echo ""

    for editor in $editors; do
        case "$editor" in
            zcode)    link_zcode ;;
            opencode) link_opencode ;;
        esac
        echo ""
    done

    setup_knowledge_base
    echo ""

    info "Bootstrap complete!"
    echo ""
    echo "Next steps:"
    echo "  1. Install the runtime binary (recommended, atomic + receipt):"
    echo "       sddk dev install --prefix ~/.local --source ."
    echo "  2. Verify the install:"
    echo "       sddk dev verify --prefix ~/.local"
    echo "  3. Diagnose environment:"
    echo "       sddk dev doctor"
    echo "  4. Adopt a project (in that project's dir):"
    echo "       sddk adopt"
    echo "  5. Start a cycle:"
    echo "       sddk cycle start --root . --scope . --change <change-name>"
    echo ""
    echo "To verify symlinks:"
    echo "  ls -la ~/.zcode/agents/"
    echo "  ls -la ~/.config/opencode/agents/"
    echo "  ls -la ~/.config/opencode/skills/knowledge-graph/"
    echo "  ls -la ~/.config/opencode/workflows/"
}

main "$@"