#!/usr/bin/env bash
# tests-e2e/clean-machine/run.sh — Local shim that delegates to act.
#
# Usage:
#   bash tests-e2e/clean-machine/run.sh               # uses default (latest)
#   bash tests-e2e/clean-machine/run.sh --tag v1.169.86
#
# Requirements:
#   - act must be installed (https://github.com/nektos/act)
#   - podman must be available (used inside the container job)
#   - gh CLI should be available for tag resolution

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
WORKFLOW=".github/workflows/clean-machine-uat.yml"

# Parse --tag if provided
TAG="latest"
REMAINING_ARGS=()

while [ $# -gt 0 ]; do
    case "$1" in
        --tag)
            TAG="${2:-}"
            if [ -z "$TAG" ]; then
                echo "error: --tag requires a value" >&2
                exit 2
            fi
            shift 2
            ;;
        *)
            REMAINING_ARGS+=("$1")
            shift
            ;;
    esac
done

cd "$REPO_ROOT"

if ! command -v act >/dev/null 2>&1; then
    echo "error: act is not installed" >&2
    echo "  Install: https://github.com/nektos/act#installation" >&2
    exit 1
fi

if [ ! -f "$WORKFLOW" ]; then
    echo "error: workflow not found: $WORKFLOW" >&2
    exit 1
fi

echo "Running clean-machine UAT via act..."
echo "  workflow: $WORKFLOW"
echo "  tag:     $TAG"
echo "  repo:    $(pwd)"
echo

if [ "$TAG" = "latest" ]; then
    exec act workflow_dispatch \
        -W "$WORKFLOW" \
        --workflow-dispatch-input tag=latest \
        "${REMAINING_ARGS[@]}"
else
    exec act workflow_dispatch \
        -W "$WORKFLOW" \
        --workflow-dispatch-input "tag=$TAG" \
        "${REMAINING_ARGS[@]}"
fi
