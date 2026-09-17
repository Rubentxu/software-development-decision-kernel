#!/bin/bash
# Release admission — A5-1 §6.
#
# The release contract is SEMANTIC: a release HEAD must carry a REAL,
# monotonically increasing `[workspace.package] version` change relative to
# its first parent. A commit subject (e.g. `chore(release): bump version`) is
# a human convention, never authority.
#
# Single machine-readable source for the invariant: `githooks/pre-push` uses
# the same "real version change" notion for push admission (a weaker check: it
# does not require monotonicity), while release admission (this file) is
# strictly stronger.
#
# Sourceable: defines functions only, never calls `exit`. Callers decide how to
# fail closed.

# cargo_ws_version_at <rev> → prints the [workspace.package] version at <rev>,
# or nothing if the revision/file/key is absent.
cargo_ws_version_at() {
    local rev="$1" content
    content="$(git show "$rev:Cargo.toml" 2>/dev/null || true)"
    [[ -z "$content" ]] && return 0
    echo "$content" | awk '
        /^\[workspace\.package\]/ { in_block=1; next }
        /^\[/ { in_block=0 }
        in_block && /^version[[:space:]]*=/ {
            match($0, /"[^"]*"/); print substr($0, RSTART+1, RLENGTH-2); exit
        }
    '
}

# semver_gt <a> <b> → 0 iff a > b under dotted numeric comparison.
semver_gt() {
    local a="$1" b="$2"
    [[ "$a" == "$b" ]] && return 1
    local IFS=. av bv i ai bi
    # shellcheck disable=SC2206
    av=($a)
    # shellcheck disable=SC2206
    bv=($b)
    for i in 0 1 2; do
        ai="${av[$i]:-0}"
        bi="${bv[$i]:-0}"
        [[ "$ai" =~ ^[0-9]+$ ]] || ai=0
        [[ "$bi" =~ ^[0-9]+$ ]] || bi=0
        if ((10#$ai > 10#$bi)); then return 0; fi
        if ((10#$ai < 10#$bi)); then return 1; fi
    done
    return 1
}

# release_admission_check [HEAD-rev]
# Prints `ACCEPT <prev> -> <head>` and returns 0, or `REJECT <reason>` and
# returns 1. Never exits.
release_admission_check() {
    local head_rev="${1:-HEAD}" head_version prev_version
    head_version="$(cargo_ws_version_at "$head_rev")"
    prev_version="$(cargo_ws_version_at "$head_rev^")"
    if [[ -z "$head_version" ]]; then
        echo "REJECT missing-head-version"
        return 1
    fi
    if [[ -z "$prev_version" ]]; then
        echo "REJECT missing-parent-version"
        return 1
    fi
    if ! semver_gt "$head_version" "$prev_version"; then
        echo "REJECT non-monotonic $prev_version -> $head_version"
        return 1
    fi
    echo "ACCEPT $prev_version -> $head_version"
    return 0
}
