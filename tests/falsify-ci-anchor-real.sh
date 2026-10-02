#!/usr/bin/env bash
# End-to-end falsification of the release-workflow's manifest anchor.
#
# The pattern test (tests/test_release_ci_manifest_anchor.sh) pins the *shape* of
# the computation in `.github/workflows/release.yml`. This one proves the
# consequence: that the BUNDLE.toml the workflow produces is accepted by the
# installer, and that the one it used to produce is rejected.
#
# That distinction matters, because `verify_manifest_anchor` is only called
# from the install path (install.rs:121). `sddk dev manifest --verify` checks
# files against the manifest but never compares the declared anchor — so it
# cannot detect this class of defect at all. Only a real `dev install` can.
#
# Observed session-31, both directions, with the release binary:
#   - bundle declaring sha256:<digest of MANIFEST.sha256>  -> installs, exit 0
#   - bundle declaring <digest of the first listed file>  -> refused, exit != 0
#
# Portable by design: resolves the repo root and the target directory from the
# script location, and skips (exit 0) when no release binary has been built, so
# it is safe to call from release.sh without forcing a release build.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# Resolve the release binary the same way cargo would.
TARGET_DIR="${CARGO_TARGET_DIR:-$(cd "$ROOT" && cargo metadata --no-deps --format-version 1 2>/dev/null | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')}"
BIN="$TARGET_DIR/release/sddk"

if [ ! -x "$BIN" ]; then
    echo "SKIP: no release binary at $BIN (run: cargo build --release --bin sddk)"
    exit 0
fi

# Binary version drives the declared compatibility range; using the real one
# keeps the anchor the only variable under test (a version mismatch is
# reported by verify_bundle_compat and would mask the anchor verdict).
BINVER="$("$BIN" --version 2>&1 | awk '{print $NF}')"
if [ -z "$BINVER" ] || [ "$BINVER" = "$BIN" ]; then
    echo "SKIP: could not read the binary version"
    exit 0
fi

build_bundle() {
    # $1 = destination dir, $2 = declared manifest_sha256 value
    #
    # Staged from MANIFEST.sha256, NOT from a hand-written surface list. This
    # function WAS a sixth copy of that list and it went stale when `specs` and
    # `docs/impeccable-reference` became surfaces: it staged only the original
    # four, then pasted the full manifest on top, so `dev install` refused BOTH
    # bundles because 16 listed files were missing — and refused them
    # *identically*. That is the property this file exists to distinguish: a
    # bundle with the correct anchor must install while a bundle with a wrong
    # anchor must be refused *for the anchor reason*. Once both fail for an
    # incidental reason the negative check passes for the wrong reason and the
    # whole falsification is vacuous. The third assertion below ("refusal names
    # the manifest anchor") is what caught it; nothing else would have.
    mkdir -p "$1"
    while read -r _digest path; do
        [ -n "${path:-}" ] || continue
        mkdir -p "$1/$(dirname "$path")"
        cp "$ROOT/$path" "$1/$path"
    done < "$ROOT/MANIFEST.sha256"
    # The manifest cannot list itself, so the loop above never stages it — and
    # `dev install` needs it to verify against. Same reason `release.sh` copies
    # it explicitly after deriving the staging from the manifest.
    cp "$ROOT/MANIFEST.sha256" "$1/MANIFEST.sha256"
    printf '[bundle]\nschema_version = 2\nversion = "%s"\nbinary_min_version = "%s"\nbinary_max_version = "%s"\n\n[contents]\nmanifest_sha256 = "%s"\n' \
        "$BINVER" "$BINVER" "$BINVER" "$2" > "$1/BUNDLE.toml"
}

REAL_DIGEST="sha256:$(sha256sum "$ROOT/MANIFEST.sha256" | cut -d' ' -f1)"
# What the buggy `awk 'NR==1 {print $1}'` produced: the digest of the first
# *listed file*, not of the manifest.
FIRST_FILE_DIGEST="$(head -1 "$ROOT/MANIFEST.sha256" | awk '{print $1}')"

GOOD="$WORK/good"
BAD="$WORK/bad"
build_bundle "$GOOD" "$REAL_DIGEST"
build_bundle "$BAD" "$FIRST_FILE_DIGEST"

echo "binary version : $BINVER"
echo "real manifest  : $REAL_DIGEST"
echo "first-file bug : $FIRST_FILE_DIGEST"
echo

echo "=== A) bundle the FIXED workflow produces (expect accept) ==="
"$BIN" dev install --prefix "$WORK/prefix-good" --source "$GOOD" >"$WORK/good.log" 2>&1
GOOD_RC=$?
tail -3 "$WORK/good.log"
echo "exit=$GOOD_RC"
echo
echo "=== B) bundle the BUGGY workflow produced (expect reject) ==="
"$BIN" dev install --prefix "$WORK/prefix-bad" --source "$BAD" >"$WORK/bad.log" 2>&1
BAD_RC=$?
tail -3 "$WORK/bad.log"
echo "exit=$BAD_RC"
echo

FAILED=0
if [ "$GOOD_RC" -eq 0 ]; then
    echo "  ok   — the fixed producer's bundle installs"
else
    echo "  FAIL — the fixed producer's bundle was refused"
    FAILED=$((FAILED + 1))
fi
if [ "$BAD_RC" -ne 0 ]; then
    echo "  ok   — the buggy producer's bundle is refused (anchor mismatch)"
else
    echo "  FAIL — a bundle with a wrong manifest anchor was accepted"
    FAILED=$((FAILED + 1))
fi
# The refusal must be the anchor check specifically, not some unrelated error.
if grep -q 'manifest anchor check' "$WORK/bad.log"; then
    echo "  ok   — refusal names the manifest anchor, not an incidental error"
else
    echo "  FAIL — refusal did not come from the anchor check"
    FAILED=$((FAILED + 1))
fi

echo
if [ "$FAILED" -eq 0 ]; then
    echo "RESULT: PASS (3 checks)"
    exit 0
fi
echo "RESULT: FAIL ($FAILED/3 checks failed)"
exit 1
