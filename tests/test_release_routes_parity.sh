#!/usr/bin/env bash
# Reproduce BOTH production bundle routes and compares their output.
#
# The two routes state the same fact differently and, until session-65i,
# nothing compared their OUTPUT:
#
#   - scripts/release.sh (local)  stages FROM MANIFEST.sha256, wraps the tarball
#   - .github/workflows/release.yml (cloud) names the surfaces, root-level tarball
#
# INC-DEBT-056 was invisible to every list-comparison because the defect was in
# what those lists PRODUCE. This runs both and compares the tarballs
# member-by-member and byte-by-byte.
#
# The cloud route is reproduced from a CLEAN CHECKOUT (git archive), not from
# the working tree, because that is what actions/checkout gives it — and the
# difference is the whole reason the cloud route is safe. Mounting the working
# tree instead produced a FAIL that was an artefact of the harness: it shipped
# `agents/.atl/` and `assets/*.bak`, which a real checkout cannot contain.
#
# It runs the job body in a container rather than shelling out to `act`,
# because `act`'s upload-artifact step fails on this host
# (`path escapes from parent`) AFTER the tar is built, and its container is
# destroyed on exit, taking the artifact with it. The `run:` body is copied
# verbatim from release.yml, so a change there must be mirrored here — that
# duplication is the price, and the parity test is what makes it visible.
set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)" || exit 1

# The scratch tree lives INSIDE the repo, not in `mktemp -d`. Reason measured,
# not guessed: `mktemp -d` creates the directory with mode 700, and the
# container reaches it as a different uid mapping, so every read of the mounted
# tree fails with `Permission denied` — while the host shows `drwxr-xr-x` and
# the ids match. A repo-local path is mounted fine.
WORK="$PWD/.parity-work"
rm -rf "$WORK" 2>/dev/null
mkdir -p "$WORK"
OUT="$WORK/out"
mkdir -p "$OUT"
cleanup() { rm -rf "$WORK" 2>/dev/null; }
trap cleanup EXIT

echo "== building the cloud route from a clean checkout =="
git archive --format=tar HEAD | tar x -C "$WORK"

# The body below is release.yml:103-141 (the `framework-bundle` job's `run:`),
# with the two changes that only exist to make it observable here: the version
# comes from the environment instead of ${GITHUB_REF_NAME#v}, and the tarball
# is left in $CWD instead of bundle/ so it can be shipped out.
podman run --rm \
    -v "$WORK:/w:ro" \
    -w /tmp \
    -e GITHUB_REF_NAME=v2.5.3 \
    docker.io/library/bash:latest \
    bash -c '
        set -euo pipefail
        mkdir -p /tmp/r && cd /tmp/r
        tar cf - -C /w agents skills prompts assets specs docs MANIFEST.sha256 Cargo.toml | tar xf -
        test -f MANIFEST.sha256
        FW_VERSION="$(awk "/^\\[workspace\\.package\\]/{flag=1; next} flag && /^version = /{gsub(/\"/, \"\", \$3); print \$3; exit}" Cargo.toml)"
        test -n "$FW_VERSION"
        [ "$FW_VERSION" = "${GITHUB_REF_NAME#v}" ] || { echo "workspace version $FW_VERSION != tag ${GITHUB_REF_NAME#v}" >&2; exit 1; }
        MANIFEST_SHA="$(sha256sum MANIFEST.sha256 | cut -d" " -f1)"
        printf "%s\n" \
          "[bundle]" "schema_version = 2" \
          "version = \"$FW_VERSION\"" \
          "binary_min_version = \"$FW_VERSION\"" \
          "binary_max_version = \"$FW_VERSION\"" \
          "" "[contents]" \
          "manifest_sha256 = \"sha256:$MANIFEST_SHA\"" \
          > BUNDLE.toml
        tar czf cloud.tar.gz agents skills prompts/sddk assets specs docs/impeccable-reference MANIFEST.sha256 BUNDLE.toml
        tar tzf cloud.tar.gz | grep -qx "BUNDLE.toml" || { echo "missing root-level BUNDLE.toml" >&2; exit 1; }
        cat cloud.tar.gz
    ' > "$OUT/cloud.tar.gz" 2>"$OUT/cloud.log"
RC=$?
if [ "$RC" -ne 0 ] || [ ! -s "$OUT/cloud.tar.gz" ]; then
    echo "  FAIL — could not build the cloud bundle:"
    sed 's/^/       /' "$OUT/cloud.log"
    exit 1
fi
echo "  ok   — cloud bundle built ($(stat -c%s "$OUT/cloud.tar.gz") bytes)"

echo
echo "== building the local route (release.sh step 5) =="
STAGE="$WORK/s/software-development-decision-kernel"
mkdir -p "$STAGE"
awk '{print $2}' MANIFEST.sha256 | xargs -d '\n' cp --parents -t "$STAGE"
cp MANIFEST.sha256 "$STAGE/"
VERSION="$(awk '/^\[workspace\.package\]/{flag=1; next} flag && /^version = /{gsub(/"/, "", $3); print $3; exit}' Cargo.toml)"
MANIFEST_SHA="$(sha256sum MANIFEST.sha256 | awk '{print $1}')"
printf '%s\n' \
    '[bundle]' 'schema_version = 2' \
    "version = \"$VERSION\"" \
    "binary_min_version = \"$VERSION\"" \
    "binary_max_version = \"$VERSION\"" \
    '' '[contents]' "manifest_sha256 = \"sha256:$MANIFEST_SHA\"" \
    > "$STAGE/BUNDLE.toml"
tar czf "$OUT/local.tar.gz" --xform "s|^\./|software-development-decision-kernel/|" \
    -C "$WORK/s" software-development-decision-kernel
echo "  ok   — local bundle built ($(stat -c%s "$OUT/local.tar.gz") bytes)"

echo
bash "$(dirname "${BASH_SOURCE[0]}")/test_release_bundle_parity.sh" \
    "$OUT/local.tar.gz" "$OUT/cloud.tar.gz"
