#!/usr/bin/env bash
# Run directly: `bash tests/test_release_bundle_step5.sh`. Not wired into the hook or CI (the cloud path cannot run here); run it before trusting any change to release.sh step 5.
# End-to-end falsation of release.sh step 5, executed rather than read.
#
# Reproduces the staging the script performs, packs it, extracts it, and asks
# the questions a consumer asks: is MANIFEST.sha256 present, does the
# BUNDLE.toml anchor match the manifest that actually shipped, and did any
# gitignored artefact ride along.
set -uo pipefail
# cd to the REPO ROOT, not to this script's directory: every path below
# (MANIFEST.sha256, agents/, …) is repo-relative. Getting this wrong makes
# the test fail with "No existe el fichero o el directorio" on MANIFEST.sha256
# — which is indistinguishable from a real defect until you read the cd line.
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)" || exit 1

VERSION=2.5.3
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
STAGE="$TMP/bundle-stage/software-development-decision-kernel"
mkdir -p "$STAGE"

# --- exactly what release.sh:554-570 does -------------------------------
awk '{print $2}' MANIFEST.sha256 | xargs -d '\n' cp --parents -t "$STAGE"
cp MANIFEST.sha256 "$STAGE/"
:
MANIFEST_SHA="$(sha256sum MANIFEST.sha256 | awk '{print $1}')"
printf '%s\n' \
    '[bundle]' 'schema_version = 2' \
    "version = \"$VERSION\"" \
    "binary_min_version = \"$VERSION\"" \
    "binary_max_version = \"$VERSION\"" \
    '' '[contents]' "manifest_sha256 = \"sha256:$MANIFEST_SHA\"" \
    > "$STAGE/BUNDLE.toml"
# --- the fail-closed check the script runs ------------------------------
DIFF_OUT="$(diff <(awk '{print $2}' MANIFEST.sha256 | sort) \
                 <(cd "$STAGE" && find . -type f -printf '%P\n' \
                    | grep -vx -e '^BUNDLE.toml$' -e '^MANIFEST\.sha256$' | sort) || true)"
if [ -n "$DIFF_OUT" ]; then
    echo "FATAL: staged bundle does not match the manifest:"
    echo "$DIFF_OUT"
    exit 1
fi
for REQUIRED in MANIFEST.sha256 BUNDLE.toml; do
    [ -f "$STAGE/$REQUIRED" ] || { echo "FATAL: missing $REQUIRED"; exit 1; }
done
echo "  ok   — staging matches the manifest; both required files present"

tar czf "$TMP/b.tar.gz" --xform "s|^\./|software-development-decision-kernel/|" \
    -C "$TMP/bundle-stage" software-development-decision-kernel
mkdir -p "$TMP/x" && tar xzf "$TMP/b.tar.gz" -C "$TMP/x"
FW="$TMP/x/software-development-decision-kernel"

# --- what a consumer asks, against the EXTRACTED bundle ------------------
echo
echo "== consumer checks, on the extracted bundle =="
FAIL=0
note() { echo "  $1"; }

if [ -f "$FW/MANIFEST.sha256" ]; then
    note "ok   — MANIFEST.sha256 ships (release.yml:230 and update.rs require it)"
else
    note "FAIL — MANIFEST.sha256 does not ship"
    FAIL=1
fi
if [ -f "$FW/BUNDLE.toml" ]; then
    note "ok   — BUNDLE.toml ships at the wrapper root"
else
    note "FAIL — BUNDLE.toml missing"
    FAIL=1
fi

# Exactly one wrapper level. release.yml:135 documents the local path as the
# wrapped one; a second wrapper is what a mis-specified -C produces.
DEPTH="$(tar tzf "$TMP/b.tar.gz" | grep -c '^software-development-decision-kernel/software-development-decision-kernel/')"
if [ "$DEPTH" -eq 0 ]; then
    note "ok   — single wrapper directory (no doubled prefix)"
else
    note "FAIL — doubled wrapper prefix on $DEPTH members"
    FAIL=1
fi

SHIPPED="$(awk -F'"' '/^manifest_sha256 = /{print $2; exit}' "$FW/BUNDLE.toml")"
ACTUAL="sha256:$(sha256sum "$FW/MANIFEST.sha256" | awk '{print $1}')"
if [ "$SHIPPED" = "$ACTUAL" ]; then
    note "ok   — anchor matches the manifest that shipped ($SHIPPED)"
else
    note "FAIL — anchor $SHIPPED != $ACTUAL"
    FAIL=1
fi

# Every manifest entry must exist in the extracted bundle with its digest.
MISSING=0; MISMATCH=0
while read -r want path; do
    [ -n "$path" ] || continue
    if [ ! -f "$FW/$path" ]; then MISSING=$((MISSING + 1)); continue; fi
    got="$(sha256sum "$FW/$path" | awk '{print $1}')"
    [ "$got" = "$want" ] || MISMATCH=$((MISMATCH + 1))
done < <(awk '{print $1, $2}' "$FW/MANIFEST.sha256")
if [ "$MISSING" -eq 0 ]; then
    note "ok   — all 394 manifest entries present in the bundle"
else
    note "FAIL — $MISSING manifest entries missing from the bundle"
    FAIL=1
fi
if [ "$MISMATCH" -eq 0 ]; then
    note "ok   — all digests verify"
else
    note "FAIL — $MISMATCH digests do not verify"
    FAIL=1
fi

LEAK="$(find "$FW" \( -name '*.bak' -o -name '.atl' -o -name '*canary*' \) -print | head -3)"
if [ -z "$LEAK" ]; then
    note "ok   — no gitignored artefact rode along"
else
    note "FAIL — leaked: $LEAK"
    FAIL=1
fi

COUNT="$(find "$FW" -type f | wc -l)"
note "     bundle carries $COUNT files (394 manifest + MANIFEST.sha256 + BUNDLE.toml = 396 expected)"

echo
if [ "$FAIL" -eq 0 ]; then
    echo "RESULT: PASS"
    exit 0
fi
echo "RESULT: FAIL"
exit 1
