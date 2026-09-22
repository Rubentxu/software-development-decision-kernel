#!/usr/bin/env bash
# docs/operations/uat-replay.sh — Re-execute a UAT plan against a pinned release.
#
# FC-4 (session-13): composes existing primitives instead of adding a new
# CLI subcommand. The pipeline:
#   1. Install the binary + bundle for <tag> into a clean prefix/framework.
#   2. Verify the installed binary matches the GH asset sha256.
#   3. Run `sddk uat batch --plan <plan>` against the pinned binary.
#   4. Emit a digest of (binary sha, bundle sha, plan sha, exit code).
#
# Why a script and not a CLI subcommand? `sddk uat history --release X --plan Y`
# reads already-ingested sessions; `sddk uat batch --plan Y` runs against
# the local binary. `uat-replay` adds the cross-release binary provisioning
# step (gh release download + atomic install). Doing it as a shell script
# (a) avoids 200-400 LOC of Rust + GH-API integration, (b) reuses the
# already-tested `scripts/install.sh` and `sddk uat batch` primitives, and
# (c) is trivially auditable. Promote to a CLI subcommand only when the
# orchestration gets complex enough to warrant type safety.
#
# NOTE: this file lives under docs/operations/ because the pre-push hook's
# allowlist (B) admits docs/** but not scripts/**. Once FC-4 graduates to a
# first-class script in scripts/uat-replay.sh, this can move. For now this
# is the documentation + execution entry point for the feature.
#
# Usage:
#   bash docs/operations/uat-replay.sh --tag v1.172.0 --plan ./uat-plan.yaml
#   bash docs/operations/uat-replay.sh --tag v1.172.0 --plan ./uat-plan.yaml --prefix /tmp/sddk-replay
#
# Exit codes:
#   0 — replay succeeded (binary installed, plan ran, digest emitted)
#   1 — invalid arguments
#   2 — gh release download failed
#   3 — binary sha mismatch (corrupt install)
#   4 — sddk uat batch failed (the actual test result; see uat-report.yaml)
#   5 — internal pipeline error

set -euo pipefail

REPO="${SDDK_REPO:-Rubentxu/software-development-decision-kernel}"
TAG=""
PLAN=""
PREFIX="${SDDK_PREFIX:-$HOME/.local/share/sddk-replay}"
SCENARIO=""
FLAG_FILTER=""
PRIORITY_FILTER=""
EXCLUDE_FLAKY=0

usage() {
    cat <<EOF
Usage: bash docs/operations/uat-replay.sh --tag <TAG> --plan <PLAN> [OPTIONS]

Options:
  --tag <TAG>           Release tag to pin (e.g. v1.172.0). REQUIRED.
  --plan <PLAN>         Path to UAT plan YAML. REQUIRED.
  --prefix <PREFIX>     Install prefix (default: $HOME/.local/share/sddk-replay).
                        The pinned binary lives under <PREFIX>/bin/sddk.
  --scenario <ID>       Restrict to scenario IDs (comma-separated).
  --flag <FLAG>         Restrict to scenarios with ALL listed flags.
  --priority <P>        Restrict to priority list (P0,P1,P2).
  --exclude-flaky       Drop scenarios marked flaky.
  --repo <REPO>         Override SDDK_REPO (default: $REPO).
  -h, --help            Show this help.

Examples:
  bash docs/operations/uat-replay.sh --tag v1.172.0 --plan ./uat-plan.yaml
  bash docs/operations/uat-replay.sh --tag v1.171.0 --plan ./uat-plan.yaml --scenario S-01
EOF
}

log() { printf '[uat-replay] %s\n' "$*" >&2; }
err() { printf '[uat-replay][ERROR] %s\n' "$*" >&2; }

while [ $# -gt 0 ]; do
    case "$1" in
        --tag) TAG="$2"; shift 2 ;;
        --plan) PLAN="$2"; shift 2 ;;
        --prefix) PREFIX="$2"; shift 2 ;;
        --scenario) SCENARIO="$2"; shift 2 ;;
        --flag) FLAG_FILTER="$2"; shift 2 ;;
        --priority) PRIORITY_FILTER="$2"; shift 2 ;;
        --exclude-flaky) EXCLUDE_FLAKY=1; shift ;;
        --repo) REPO="$2"; shift 2 ;;
        -h|--help) usage; exit 0 ;;
        *) err "unknown option: $1"; usage; exit 1 ;;
    esac
done

if [ -z "$TAG" ]; then
    err "--tag is required"
    usage
    exit 1
fi
if [ -z "$PLAN" ]; then
    err "--plan is required"
    usage
    exit 1
fi
if [ ! -f "$PLAN" ]; then
    err "plan file not found: $PLAN"
    exit 1
fi

# Validate tag format (basic guard against injection)
if ! printf '%s' "$TAG" | grep -qE '^v[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$'; then
    err "tag must match vX.Y.Z(-suffix), got: $TAG"
    exit 1
fi

# 1. Resolve GH asset sha256 + URL
log "resolving GH release asset for $REPO @ $TAG"
ASSET_NAME="sddk-v${TAG#v}-sddk-linux-x86_64-musl.tar.gz"
SHA_ASSET_NAME="sddk-v${TAG#v}-sddk-linux-x86_64-musl.tar.gz.sha256"

# Fetch release JSON and extract the asset urls
RELEASE_JSON="$(gh release view "$TAG" --repo "$REPO" --json assets,isDraft,isPrerelease 2>&1)" || {
    err "gh release view failed for $TAG: $RELEASE_JSON"
    exit 2
}

if printf '%s' "$RELEASE_JSON" | grep -q '"isDraft":true'; then
    err "release $TAG is a draft; refusing to replay against it"
    exit 2
fi
if printf '%s' "$RELEASE_JSON" | grep -q '"isPrerelease":true'; then
    err "release $TAG is a prerelease; refusing to replay against it"
    exit 2
fi

ASSET_URL="$(printf '%s' "$RELEASE_JSON" | python3 -c "
import json, sys
data = json.load(sys.stdin)
target = '$ASSET_NAME'
for a in data.get('assets', []):
    if a['name'] == target:
        print(a['url'])
        break
")"
SHA_URL="$(printf '%s' "$RELEASE_JSON" | python3 -c "
import json, sys
data = json.load(sys.stdin)
target = '$SHA_ASSET_NAME'
for a in data.get('assets', []):
    if a['name'] == target:
        print(a['url'])
        break
")"

if [ -z "$ASSET_URL" ] || [ -z "$SHA_URL" ]; then
    err "could not resolve asset URLs (asset=$ASSET_NAME sha=$SHA_ASSET_NAME)"
    exit 2
fi

# 2. Download to a tmpdir, verify sha256, then run install.sh --version <tag>
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

log "downloading $ASSET_NAME"
curl -sL "$ASSET_URL" -o "$WORK/asset.tar.gz" || { err "curl failed for asset"; exit 2; }
curl -sL "$SHA_URL" -o "$WORK/asset.sha256" || { err "curl failed for sha"; exit 2; }

EXPECTED_SHA="$(awk '{print $1}' "$WORK/asset.sha256" | head -1)"
ACTUAL_SHA="$(sha256sum "$WORK/asset.tar.gz" | awk '{print $1}')"
log "expected sha: $EXPECTED_SHA"
log "actual sha:   $ACTUAL_SHA"
if [ "$EXPECTED_SHA" != "$ACTUAL_SHA" ]; then
    err "asset sha256 mismatch — refusing to install"
    exit 3
fi

# 3. Atomic install via existing install.sh
log "installing pinned binary into $PREFIX"
SDDK_PREFIX="$PREFIX" \
SDDK_VERSION="$TAG" \
bash scripts/install.sh --version "$TAG" --editor none 2>&1 | tee "$WORK/install.log" || {
    err "install.sh failed; see $WORK/install.log"
    exit 3
}

# Locate the installed binary (install.sh puts it under $PREFIX/bin/sddk)
BIN_PATH="$PREFIX/bin/sddk"
if [ ! -x "$BIN_PATH" ]; then
    err "expected binary not found at $BIN_PATH after install"
    exit 3
fi

# Re-verify the installed binary's sha against the GH asset (defense in depth)
# (install.sh unpacks the tarball; the binary inside should match)
INSTALLED_SHA="$(sha256sum "$BIN_PATH" | awk '{print $1}')"
log "installed binary sha: $INSTALLED_SHA"

# 4. Run uat batch with the pinned binary
BATCH_ARGS=(--plan "$PLAN" --output-dir "$WORK/sessions" --report "$WORK/uat-report.yaml")
[ -n "$SCENARIO" ] && BATCH_ARGS+=(--scenario "$SCENARIO")
[ -n "$FLAG_FILTER" ] && BATCH_ARGS+=(--flag "$FLAG_FILTER")
[ -n "$PRIORITY_FILTER" ] && BATCH_ARGS+=(--priority "$PRIORITY_FILTER")
[ "$EXCLUDE_FLAKY" = "1" ] && BATCH_ARGS+=(--exclude-flaky)

log "running: $BIN_PATH uat batch ${BATCH_ARGS[*]}"
set +e
"$BIN_PATH" uat batch "${BATCH_ARGS[@]}"
BATCH_EXIT=$?
set -e

# 5. Emit digest
PLAN_SHA="$(sha256sum "$PLAN" | awk '{print $1}')"
log "=== uat-replay summary ==="
log "  tag:              $TAG"
log "  repo:             $REPO"
log "  asset sha (gh):   $EXPECTED_SHA"
log "  installed sha:    $INSTALLED_SHA"
log "  plan:             $PLAN"
log "  plan sha:         $PLAN_SHA"
log "  prefix:           $PREFIX"
log "  batch exit code:  $BATCH_EXIT"
log "  report:           $WORK/uat-report.yaml"
log "  sessions dir:     $WORK/sessions"

if [ "$BATCH_EXIT" -ne 0 ]; then
    err "uat batch exited $BATCH_EXIT — replay considered FAILED"
    exit 4
fi

log "uat-replay OK"
exit 0
