#!/usr/bin/env bash
# Compare the CLOUD bundle (release.yml, via act) against the LOCAL one
# (scripts/release.sh step 5).
#
# The two paths state the same fact differently and nothing has ever compared
# their OUTPUT: `release.sh` stages from MANIFEST.sha256, `release.yml` names
# the surfaces explicitly. INC-DEBT-056 was invisible to every list-comparison
# because the defect was in what those lists produce, not in the lists.
#
# This compares the two tarballs member-by-member and digest-by-digest. It is
# the check that would have caught the doubled prefix, the missing
# MANIFEST.sha256, and the gitignored leak — across BOTH routes, not just the
# one that runs in a working tree.
#
# Usage: bash tests/test_release_bundle_parity.sh <local.tar.gz> <cloud.tar.gz>
set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)" || exit 1

LOCAL="${1:-}"
CLOUD="${2:-}"
if [ -z "$LOCAL" ] || [ -z "$CLOUD" ]; then
    echo "usage: $0 <local.tar.gz> <cloud.tar.gz>"
    exit 2
fi
for f in "$LOCAL" "$CLOUD"; do
    [ -f "$f" ] || { echo "missing tarball: $f" >&2; exit 2; }
done

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
FAIL=0
note() { echo "  $1"; }

mkdir -p "$TMP/l" "$TMP/c"
tar xzf "$LOCAL" -C "$TMP/l"
tar xzf "$CLOUD" -C "$TMP/c"

echo "== layout =="
# The two routes deliberately use DIFFERENT layouts and both are correct: the
# cloud tarball is ROOT-LEVEL (release.yml:135-139 documents it and gates it
# with `grep -qx "BUNDLE.toml"`), and the local one is wrapped (release.sh
# uses `--xform`). The consumer, `update_bundle`, handles both via
# `tarball_wraps_all_members_under_one_dir`. So the check is not "same
# layout" — it is "each side ships what its own consumer expects", and then
# "same CONTENT once the wrapper is accounted for".
ROOT_L=""
ROOT_C=""
for side in l c; do
    path="$TMP/$side"
    if [ -d "$path/software-development-decision-kernel" ]; then
        note "ok   — $side: wrapped under software-development-decision-kernel/"
        resolved="$path/software-development-decision-kernel"
    elif [ -f "$path/MANIFEST.sha256" ]; then
        note "ok   — $side: root-level layout (no wrapper), as its producer documents"
        resolved="$path"
    else
        note "FAIL — $side: no MANIFEST.sha256 at the root or under a wrapper"
        FAIL=1
        resolved=""
    fi
    # Plain assignment per side, not `eval`: shellcheck cannot see through eval
    # and reported ROOT_l/ROOT_c as unassigned (SC2154), which is exactly the
    # kind of warning that trains you to ignore the file.
    if [ "$side" = "l" ]; then ROOT_L="$resolved"; else ROOT_C="$resolved"; fi
done
if [ -z "$ROOT_L" ] || [ -z "$ROOT_C" ]; then
    note "FAIL — cannot continue without both roots resolved"
    exit 1
fi

echo
echo "== member parity =="
# Normalise the wrapper prefix away: it is the ONE difference the two routes
# are allowed to have (documented at release.yml:135-139 and consumed via
# `tarball_wraps_all_members_under_one_dir`). Everything else must be
# identical, member for member.
members() {
    tar tzf "$1" \
        | grep -v '/$' \
        | sed 's|^\./||' \
        | sed 's|^software-development-decision-kernel/||' \
        | sort
}
diff <(members "$LOCAL") <(members "$CLOUD") > "$TMP/member.diff"
if [ -s "$TMP/member.diff" ]; then
    note "FAIL — the two routes carry different members (wrapper normalised):"
    head -20 "$TMP/member.diff" | sed 's/^/       /'
    FAIL=1
else
    note "ok   — both routes carry the same $(members "$LOCAL" | wc -l) members"
fi

echo
echo "== content parity =="
# Same paths AND same bytes. A member that exists in both but differs is a
# bundle that would install differently depending on which route shipped it.
MISMATCH=0
while IFS= read -r rel; do
    [ -n "$rel" ] || continue
    a="$ROOT_L/$rel"; b="$ROOT_C/$rel"
    if [ ! -f "$a" ] || [ ! -f "$b" ]; then continue; fi
    if ! cmp -s "$a" "$b"; then
        note "FAIL — content differs: $rel"
        MISMATCH=$((MISMATCH + 1))
    fi
done < <(members "$LOCAL")
if [ "$MISMATCH" -eq 0 ]; then
    note "ok   — every shared member is byte-identical"
else
    FAIL=1
fi

echo
echo "== both routes must satisfy the consumer contract =="
for side in l c; do
    if [ "$side" = "l" ]; then root="$ROOT_L"; else root="$ROOT_C"; fi
    if [ -f "$root/MANIFEST.sha256" ]; then
        note "ok   — $side: MANIFEST.sha256 ships"
    else
        note "FAIL — $side: MANIFEST.sha256 missing (release.yml:230 and update.rs require it)"
        FAIL=1
    fi
    if [ -f "$root/BUNDLE.toml" ]; then
        note "ok   — $side: BUNDLE.toml ships"
    else
        note "FAIL — $side: BUNDLE.toml missing"
        FAIL=1
    fi
    anchor="$(awk -F'"' '/^manifest_sha256 = /{print $2; exit}' "$root/BUNDLE.toml" 2>/dev/null)"
    # Both formats are accepted by verify_manifest_anchor; normalise to compare.
    normalised="${anchor#sha256:}"
    actual="$(sha256sum "$root/MANIFEST.sha256" 2>/dev/null | awk '{print $1}')"
    if [ -n "$anchor" ] && [ "$normalised" = "$actual" ]; then
        note "ok   — $side: anchor matches the manifest that shipped ($anchor)"
    else
        note "FAIL — $side: anchor '$anchor' != manifest digest '$actual'"
        FAIL=1
    fi
done

echo
echo "== no gitignored artefact on either side =="
for side in l c; do
    if [ "$side" = "l" ]; then root="$ROOT_L"; else root="$ROOT_C"; fi
    leak="$(find "$root" \( -name '*.bak' -o -name '.atl' -o -name '*canary*' \) -print 2>/dev/null | head -3)"
    if [ -z "$leak" ]; then
        note "ok   — $side: nothing gitignored rode along"
    else
        note "FAIL — $side: leaked: $leak"
        FAIL=1
    fi
done

echo
if [ "$FAIL" -eq 0 ]; then
    echo "RESULT: PASS — the local and cloud routes publish the same bundle"
    exit 0
fi
echo "RESULT: FAIL"
exit 1
