#!/usr/bin/env bash
# release-bump.sh — Compute and apply the next semver release bump.
#
# Usage:
#   bash scripts/release-bump.sh --dry-run   # print what would change (no writes)
#   bash scripts/release-bump.sh             # apply bump: versions + lock + CHANGELOG
#   bash scripts/release-bump.sh --force-version 1.1.0   # explicit version
#
# Bump rules from conventional commits since the last tag:
#   BREAKING CHANGE / <type>!  -> major
#   feat                       -> minor
#   fix|refactor|perf|docs|ci|chore|style|test|build -> patch
#   otherwise                  -> no release
#
# Updates: workspace Cargo.toml, 7 crate Cargo.tomls, manifest.toml,
# Cargo.lock (via cargo check), and CHANGELOG.md.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DRY_RUN=0
FORCE_VERSION=""

while [ $# -gt 0 ]; do
    case "$1" in
        --dry-run) DRY_RUN=1; shift ;;
        --force-version) FORCE_VERSION="$2"; shift 2 ;;
        *) echo "unknown option: $1" >&2; exit 2 ;;
    esac
done

cd "$ROOT"

# --- Git state ---

LAST_TAG="$(git tag --sort=-v:refname | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' | head -1 || true)"
if [ -z "$LAST_TAG" ]; then
    echo "error: no semver tag found" >&2
    exit 1
fi
CURRENT="${LAST_TAG#v}"

# The version the workspace is actually AT, which may be ahead of the last
# tag (a manual `--force-version` bump, or the ceremonial-release-pending
# state described in AGENTS.md §2.3).
#
# Deriving the next version from the TAG alone is wrong: it cannot see a
# manual bump, so a workspace at 2.1.1 with last tag v2.0.1 derived 2.1.0
# and the CI release branch would have REGRESSED the workspace to 2.1.0
# (session-28, reproduced in an isolated clone).
#
# But deriving max(workspace, tag) and then BUMPING it is also wrong, in the
# opposite direction: if the workspace is already ahead of the tag, the
# operator has declared the release version (AGENTS.md §2.3 — "el workspace
# version es puntero ceremonial del release ... declara la versión que va a
# aparecer como tag"). With workspace 2.2.0 and last tag v2.0.1, a minor
# commit derived v2.3.0, and CI's "Open release PR when a bump is pending"
# would have opened and auto-merged a PR taking 2.2.0 -> 2.3.0 — publishing
# a version the operator never asked for, and desynchronising workspace,
# tag and CHANGELOG (session-29).
#
# So: ahead of the tag means the workspace IS the pending release, and there
# is nothing to derive. Equal means derive from the tag as usual.
WORKSPACE_VERSION="$(grep -A1 '^\[workspace\.package\]' Cargo.toml | grep '^version' | sed -E 's/.*"([^"]+)".*/\1/')"

# semver_gt <a> <b> → 0 when a > b
semver_gt() {
    local a="$1" b="$2" i
    local -a av bv
    IFS='.' read -r -a av <<<"$a"
    IFS='.' read -r -a bv <<<"$b"
    for i in 0 1 2; do
        local ai bi
        ai="${av[$i]:-0}"
        bi="${bv[$i]:-0}"
        [[ "$ai" =~ ^[0-9]+$ ]] || ai=0
        [[ "$bi" =~ ^[0-9]+$ ]] || bi=0
        if ((10#$ai > 10#$bi)); then return 0; fi
        if ((10#$ai < 10#$bi)); then return 1; fi
    done
    return 1
}

if [ -n "$WORKSPACE_VERSION" ] && semver_gt "$WORKSPACE_VERSION" "$CURRENT"; then
    # The operator has already declared the release version by hand
    # (`--force-version`, or the committed ceremonial bump). The release IS
    # that version — re-deriving one on top would skip the declared release.
    BASE_VERSION="$WORKSPACE_VERSION"
    PENDING_RELEASE_IS_WORKSPACE=1
    echo "workspace ($WORKSPACE_VERSION) is ahead of last tag ($CURRENT); the workspace declares the pending release ($WORKSPACE_VERSION), no bump to derive"
else
    BASE_VERSION="$CURRENT"
    PENDING_RELEASE_IS_WORKSPACE=0
fi

COMMITS="$(git log --oneline --no-merges "${LAST_TAG}..HEAD" 2>/dev/null | grep -vE 'chore\(release\)' || true)"
if [ -z "$COMMITS" ]; then
    echo "no commits since $LAST_TAG — nothing to release"
    exit 0
fi

# --- Bump level ---

LEVEL="none"
if [ -n "$FORCE_VERSION" ]; then
    LEVEL="forced"
elif echo "$COMMITS" | grep -qiE 'breaking change|^[a-z]+!:' || git log --format=%B "${LAST_TAG}..HEAD" | grep -qiE 'breaking change'; then
    LEVEL="major"
elif echo "$COMMITS" | grep -qE '^[a-f0-9]+ feat'; then
    LEVEL="minor"
elif echo "$COMMITS" | grep -qE '^[a-f0-9]+ (fix|refactor|perf|docs|ci|chore|style|test|build)'; then
    LEVEL="patch"
fi

if [ -z "$FORCE_VERSION" ] && [ "$PENDING_RELEASE_IS_WORKSPACE" = "1" ]; then
    # The workspace already declares the release version. There is nothing to
    # derive — emitting a bump here would make CI publish a version the
    # operator never declared. `--force-version` bypasses this on purpose:
    # it is the explicit way to say "and actually make it this".
    echo "no bump to derive: the workspace already declares the pending release ($WORKSPACE_VERSION)"
    exit 0
fi

if [ "$LEVEL" = "none" ]; then
    echo "no release-worthy commits since $LAST_TAG"
    exit 0
fi

next_version() {
    local cur="$1" level="$2"
    local major minor patch
    IFS='.' read -r major minor patch <<<"$cur"
    case "$level" in
        major) echo "$((major + 1)).0.0" ;;
        minor) echo "$major.$((minor + 1)).0" ;;
        patch) echo "$major.$minor.$((patch + 1))" ;;
    esac
}

if [ "$LEVEL" = "forced" ]; then
    NEXT="$FORCE_VERSION"
else
    NEXT="$(next_version "$BASE_VERSION" "$LEVEL")"
fi
NEW_TAG="v$NEXT"

echo "release bump: $LAST_TAG -> $NEW_TAG ($LEVEL)"
echo "new tag: $NEW_TAG"
if [ "$DRY_RUN" = "1" ]; then
    echo "--- commits ---"
    echo "$COMMITS"
    echo "--- files to update ---"
    echo "  Cargo.toml (workspace) + crates/*/Cargo.toml + manifest.toml + Cargo.lock + CHANGELOG.md"
    exit 0
fi

# --- Apply version bump ---

# Anchor the sed to the version currently in workspace Cargo.toml, not the
# last tag — workspace version may be ahead of the tag between releases
# (AGENTS.md §2.3 makes workspace == release tag MANDATORY, but historical
# commits kept the workspace 1+ patch ahead for testing). Without this,
# `s/^version = "$CURRENT"/.../` silently misses when workspace ≠ last tag.
# Reuse the WORKSPACE_VERSION already read above so the anchor and the
# derivation can never disagree.
for f in Cargo.toml crates/*/Cargo.toml; do
    sed -i "s/^version = \"$WORKSPACE_VERSION\"/version = \"$NEXT\"/" "$f"
done
# manifest.toml can drift from the tag version across manual bumps; set its
# single top-level `version = "…"` line unconditionally (`schema_version`
# starts with a different anchor and is never touched).
sed -i "s/^version = \"[^\"]*\"/version = \"$NEXT\"/" manifest.toml

# Regenerate Cargo.lock from the bumped manifests.
cargo check --workspace --quiet 2>/dev/null || cargo check --workspace

# --- CHANGELOG ---

if [ ! -f CHANGELOG.md ]; then
    cat > CHANGELOG.md <<'EOF'
# Changelog

All notable changes to this project are documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

EOF
fi

TODAY="$(date -u +%Y-%m-%d)"
ENTRY_FILE="$(mktemp)"
trap 'rm -f "$ENTRY_FILE"' EXIT
{
    echo "## [$NEXT] - $TODAY"
    echo
    features="$(echo "$COMMITS" | grep -E '^[a-f0-9]+ feat' || true)"
    fixes="$(echo "$COMMITS" | grep -E '^[a-f0-9]+ fix' || true)"
    other="$(echo "$COMMITS" | grep -vE '^[a-f0-9]+ (feat|fix)' || true)"
    if [ -n "$features" ]; then
        echo "### Features"
        echo "$features" | sed -E 's/^[a-f0-9]+ (feat)(\([^)]*\))?: /\1\2: /' | sed -E 's/^/  - /'
        echo
    fi
    if [ -n "$fixes" ]; then
        echo "### Fixes"
        echo "$fixes" | sed -E 's/^[a-f0-9]+ (fix)(\([^)]*\))?: /\1\2: /' | sed -E 's/^/  - /'
        echo
    fi
    if [ -n "$other" ]; then
        echo "### Other"
        echo "$other" | sed -E 's/^[a-f0-9]+ //' | sed -E 's/^/  - /'
        echo
    fi
} > "$ENTRY_FILE"

# Keep-a-Changelog ordering: newest first. Insert the new entry right after
# the header, before the first existing `## [` section.
if grep -qE '^## \[' CHANGELOG.md; then
    FIRST="$(grep -n -m1 '^## \[' CHANGELOG.md | cut -d: -f1)"
    {
        head -n "$((FIRST - 1))" CHANGELOG.md
        cat "$ENTRY_FILE"
        tail -n "+$FIRST" CHANGELOG.md
    } > CHANGELOG.md.new && mv CHANGELOG.md.new CHANGELOG.md
else
    cat "$ENTRY_FILE" >> CHANGELOG.md
fi

echo "applied: $CURRENT -> $NEXT"
echo "changed files:"
git status --short | head -20
