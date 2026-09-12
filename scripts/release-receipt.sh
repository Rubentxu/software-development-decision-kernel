#!/usr/bin/env bash
# release-receipt.sh — Emit an actor-kind authority receipt for GitHub Releases.
#
# The "GitHub Releases" writable surface (ADR-069 §3, row 7) is admitted only
# for the `System` ActorKind — the CLI (or its driving script) publishing on
# behalf of a human releaser. This helper validates the actor against the
# same v1.81.x prefix heuristic locked in
# `crates/sddk-engine/src/authority.rs::infer_actor_kind` (and documented in
# ADR-069 §5), then writes a JSON receipt that downstream tooling
# (`gh release create --notes`, receipt queries, audit) can consume.
#
# Fail-closed: if the actor_kind is not `System`, the helper exits non-zero
# and writes nothing. Same shape as the engine-side `AuthorityContext::validate`
# used in `sddk dev install` (v1.168.24) and `apply_cycle_start` (v1.168.22).
#
# Usage:
#   bash scripts/release-receipt.sh --actor-id <id> --tag <tag> --out <path>
#   bash scripts/release-receipt.sh --actor-id <id> --tag <tag> --out -   # stdout
#
# Actor resolution:
#   --actor-id <id>          explicit (default: SDDK_ACTOR env, fallback "system")
#
# Exit codes:
#   0  receipt written; actor_kind admitted as System
#   2  invalid arguments
#   3  actor_kind rejected (Human or Agent) — fail-closed
#   4  failed to write receipt (permission denied, missing dir)
#
# Receipt schema (v1):
#   {
#     "schema_version": 1,
#     "surface": "github_releases",
#     "actor_kind": "System",
#     "actor_id": "system",
#     "tag": "v1.168.27",
#     "timestamp": "2026-09-12T06:22:00Z",
#     "script": "scripts/release-receipt.sh"
#   }

set -euo pipefail

ACTOR_ID=""
TAG=""
OUT=""
LEASE_OWNER=""

usage() {
    sed -n '2,/^[^#]/p' "$0" | head -40
}

while [ $# -gt 0 ]; do
    case "$1" in
        --actor-id)   ACTOR_ID="$2"; shift 2 ;;
        --tag)        TAG="$2"; shift 2 ;;
        --out)        OUT="$2"; shift 2 ;;
        --lease-owner) LEASE_OWNER="$2"; shift 2 ;;
        -h|--help)
            usage
            exit 0
            ;;
        *) echo "unknown option: $1" >&2; exit 2 ;;
    esac
done

# Resolve actor (same shape as dev/install.rs)
if [ -z "$ACTOR_ID" ]; then
    ACTOR_ID="${SDDK_ACTOR:-system}"
fi

if [ -z "$TAG" ]; then
    echo "error: --tag is required" >&2
    exit 2
fi

# Apply the v1.81.x prefix heuristic from
# `crates/sddk-engine/src/authority.rs::infer_actor_kind`. This must stay in
# lockstep with that function; if the engine side changes, this script must
# change too. The two helpers are documented in ADR-069 §5 as a locked
# contract.
infer_actor_kind() {
    local actor="$1"
    case "$actor" in
        user:*)  echo "Human" ;;
        agent:*) echo "Agent" ;;
        *)       echo "System" ;;
    esac
}

ACTOR_KIND="$(infer_actor_kind "$ACTOR_ID")"

# Authority check: GitHub Releases surface admits System, per
# `WRITABLE_SURFACE_MATRIX[GithubReleases] = &[ActorKind::System]`.
case "$ACTOR_KIND" in
    System) ;;
    *)
        echo "actor_kind rejected on github_releases surface: $ACTOR_KIND" >&2
        echo "  actor_id: $ACTOR_ID" >&2
        echo "  admitted: [System]" >&2
        echo "  hint: pass a plain id (no user:/agent: prefix) or SDDK_ACTOR=system" >&2
        exit 3
        ;;
esac

# Build receipt. ISO 8601 UTC; no external deps.
TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

# JSON-escape actor_id (defensive; receipt is not user-supplied payload).
escape_json() {
    local s="$1"
    s="${s//\\/\\\\}"
    s="${s//\"/\\\"}"
    printf '%s' "$s"
}

ACTOR_ID_JSON="$(escape_json "$ACTOR_ID")"
TAG_JSON="$(escape_json "$TAG")"
LEASE_OWNER_JSON="$(escape_json "$LEASE_OWNER")"
TIMESTAMP_JSON="$(escape_json "$TIMESTAMP")"

LEASE_FIELD=""
if [ -n "$LEASE_OWNER" ]; then
    LEASE_FIELD=",\"lease_owner\":\"${LEASE_OWNER_JSON}\""
fi

RECEIPT=$(cat <<EOF
{
  "schema_version": 1,
  "surface": "github_releases",
  "actor_kind": "System",
  "actor_id": "${ACTOR_ID_JSON}",
  "tag": "${TAG_JSON}",
  "timestamp": "${TIMESTAMP_JSON}",
  "script": "scripts/release-receipt.sh"${LEASE_FIELD}
}
EOF
)

if [ "$OUT" = "-" ]; then
    printf '%s\n' "$RECEIPT"
elif [ -n "$OUT" ]; then
    mkdir -p "$(dirname "$OUT")"
    printf '%s\n' "$RECEIPT" > "$OUT" \
        || { echo "failed to write receipt to $OUT" >&2; exit 4; }
else
    # Default: emit next to the bundle tarball in the release scratch.
    # release.sh step 9 will provide --out explicitly.
    printf '%s\n' "$RECEIPT"
fi