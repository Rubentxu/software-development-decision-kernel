#!/usr/bin/env bash
# Pin the bundle layout contract across producer and consumer (INC-DEBT-034).
#
# The seven releases v2.2.12..v2.2.18 each surfaced a DIFFERENT defect of the
# same path, none of them visible in local tests, because the only reference
# smoke was CI running against a real published release. The last one
# (v2.2.17) was a layout mismatch: the bundle job builds the tarball with no
# wrapper directory, while the consumer applied `--strip-components=1` blindly.
# Blind strip DELETED `MANIFEST.sha256` (a single-component member) and
# flattened `agents/*.md` to the root, so the fail-closed manifest gate
# rejected a correct bundle: "bundle is missing required MANIFEST.sha256".
#
# The root cause was structural, not a typo: the contract was written in one
# place (AGENTS.md §8 step 5, describing a WRAPPED layout) and the bundle was
# produced in another (release.yml, ROOT layout), and NOTHING crossed the two
# fields. Six CI cycles later it was still not pinned.
#
# This test asserts the RELATIONSHIP, not either side restated:
#   1. the producer builds a root-layout bundle, and
#   2. the producer's own consumer extracts it with no strip and gates on the
#      manifest at that exact path.
# If either side moves without the other, the relationship breaks and this
# fails — which is the thing that was never checked.
set -uo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WF="$ROOT/.github/workflows/release.yml"
UPDATE_RS="$ROOT/crates/sddk-cli/src/dev/update.rs"
AGENTS="$ROOT/AGENTS.md"
FAILURES=0
pass() { echo "  ok   — $1"; }
fail() { echo "  FAIL — $1"; FAILURES=$((FAILURES + 1)); }

for f in "$WF" "$UPDATE_RS" "$AGENTS"; do
    if [ ! -f "$f" ]; then
        fail "required file missing: $f"
        echo "RESULT: FAIL"
        exit 1
    fi
done

# ── Case 1: the producer emits a root-layout bundle ───────────────────────
# The exact defect this pins is a `--xform`/`-C`-style wrapper being added to
# the bundle job's tar, which would make the layout diverge from every consumer.
BUNDLE_TAR_LINE="$(grep -nE 'tar czf .*software-development-decision-kernel\.tar\.gz' "$WF" | head -1)"
if [ -z "$BUNDLE_TAR_LINE" ]; then
    fail "could not find the bundle tar command in release.yml"
else
    echo "extracted: $BUNDLE_TAR_LINE"
    # A wrapper transform renames members into a single top-level directory.
    if echo "$BUNDLE_TAR_LINE" | grep -qE -- '--transform|--xform|software-development-decision-kernel/\*|-C software'; then
        fail "the CI bundle job wraps members under a directory; the consumer below extracts with no strip"
    else
        pass "the CI bundle job builds a root-layout bundle (no wrapper transform)"
    fi
fi

# ── Case 2: the producer's consumer extracts with NO strip ───────────────
# release.yml:193 does `tar xzf "$BUNDLE" -C "$WORK/framework"`. If a strip
# were added here it would delete MANIFEST.sha256 for a root-layout bundle.
EXTRACT_LINE="$(grep -nE 'tar xzf "\$BUNDLE"' "$WF" | head -1)"
if [ -z "$EXTRACT_LINE" ]; then
    fail "could not find the bundle extraction in release.yml"
elif echo "$EXTRACT_LINE" | grep -q -- '--strip-components'; then
    fail "the CI consumer strips components on a root-layout bundle; MANIFEST.sha256 would be deleted"
else
    pass "the CI consumer extracts the bundle with no strip ($EXTRACT_LINE)"
fi

# ── Case 3: the manifest gate targets the path the root layout produces ───
# The gate exists and is correct; the v2.2.17 failure was that the manifest
# was looked for in the wrong place. Pin both: the gate must exist AND name
# the root-layout path.
if grep -qE 'test -f "\$WORK/framework/MANIFEST\.sha256"' "$WF"; then
    pass "the CI consumer gates on the manifest at its root-layout path"
else
    fail "the CI consumer does not gate on \$WORK/framework/MANIFEST.sha256 — the v2.2.17 rejection would recur"
fi

# ── Case 4: the Rust consumer detects the layout instead of stripping blind ─
# The fix for the consumer side (2f7d5064). If this regresses to an
# unconditional strip, a root-layout bundle loses its manifest again even
# though the CI job is correct.
if grep -q 'tarball_wraps_all_members_under_one_dir' "$UPDATE_RS"; then
    pass "the Rust consumer detects the archive layout before stripping"
else
    fail "the Rust consumer no longer detects the layout (2f7d5064 regressed) — it would strip blindly again"
fi

# ── Case 5: the strip stays conditional on detection ─────────────────────
# The detection function existing is not enough; the strip must be GATED on
# its verdict. This check was written wrong first and the mutation proved it:
# asserting "the detector appears somewhere before the strip push" passes even
# when the `if` reads `if true`, because the assignment that computes
# `strip_components` is 20 lines earlier either way.
#
# That is also why the Rust unit test did not catch it:
# `root_level_ci_layout_is_not_detected_as_wrapped` proves the DETECTOR is
# correct, while the defect lives at the CALL SITE. A correct function behind
# a wrong `if` is the exact shape this INC had. So: parse the block the push
# sits in and require its condition to name the flag the detector assigns.
STRIP_LINE="$(grep -nE 'tar_args\.push\("--strip-components=1"' "$UPDATE_RS" | head -1)"
if [ -z "$STRIP_LINE" ]; then
    fail "no --strip-components push found in update.rs; expected the conditional strip"
else
    STRIP_N="${STRIP_LINE%%:*}"
    # The contract is a two-line relationship, and both lines must be read
    # together:
    #     strip_components = tarball_wraps_all_members_under_one_dir(&listing);
    #     ...
    #     if strip_components {
    #         tar_args.push("--strip-components=1"...)
    #
    # Four wrong approaches came first, all disproven by the mutation:
    #   (a) "the detector appears anywhere above" — passes on `if true`.
    #   (b) "least-indented `if` above" — escapes the function (line 85).
    #   (c) "nearest `if` at indent-4" — still escapes: the enclosing fn body
    #       is indent 4, so an unrelated `if expected != actual` matches.
    #   (d) "nearest `if` at all within the fn" — same, an earlier unrelated
    #       `if` wins.
    # Searching for the nearest `if` is the wrong primitive. What identifies
    # the guard is that the strip push is the FIRST statement of the block
    # the detector's flag guards — so locate the block by its closing brace
    # and take the `if` that opens it.
    PUSH_INDENT="$(awk -v n="$STRIP_N" 'NR==n { match($0, /[^ \t]/); print RSTART-1 }' "$UPDATE_RS")"
    BLOCK_INDENT=$((PUSH_INDENT - 4))
    GUARD="$(awk -v n="$STRIP_N" -v bind="$BLOCK_INDENT" '
        NR < n {
            if ($0 ~ /^[ \t]*if[ \t(]/) {
                this_indent = match($0, /[^ \t]/) - 1
                if (this_indent == bind) { found = NR; text = $0 }
            }
        }
        # The LAST such `if` at exactly the block indent, scanning forward
        # from the function start, is not necessarily right either; instead
        # keep overwriting so the NEAREST one wins.
        END { if (found) print found ":" text }
    ' "$UPDATE_RS")"
    if [ -z "${GUARD:-}" ]; then
        fail "the strip at line $STRIP_N is not inside an if-block — blind strip is back"
    else
        GUARD_TEXT="${GUARD#*:}"
        GUARD_N="${GUARD%%:*}"
        echo "guarding condition at line $GUARD_N: $GUARD_TEXT"
        # `if strip_components` (or `if !strip_components`, or a comparison
        # against it) is the contract. `if true` / `if wrapped` computed
        # elsewhere would be exactly the undetected regression.
        if echo "$GUARD_TEXT" | grep -qE 'strip_components'; then
            pass "the strip is gated on the detector's verdict (if at line $GUARD_N)"
        else
            fail "the strip at line $STRIP_N is gated on '$GUARD_TEXT', not on the detector's verdict — blind strip is back"
        fi
    fi
fi

# ── Case 6: the documented contract matches the produced layout ──────────
# The other half of the gap. AGENTS.md §8 step 5 described a wrapped layout;
# the producer emits a root layout; the two were never cross-checked. A doc
# that says "wrapped" while CI ships "root" is the same divergence one layer
# down, and it is what misleads the next person changing the pipeline.
STEP5="$(grep -nE '^\| 5 \| Bundle tarball' "$AGENTS" | head -1)"
if [ -z "$STEP5" ]; then
    fail "could not find the step-5 bundle row in AGENTS.md"
elif echo "$STEP5" | grep -qi 'layout raíz\|layout root'; then
    pass "AGENTS.md step 5 documents the root layout that CI actually produces"
else
    fail "AGENTS.md step 5 does not document the root layout (it may still claim a wrapper directory)"
fi

echo
if [ "$FAILURES" -eq 0 ]; then
    echo "RESULT: PASS (6 checks)"
    exit 0
fi
echo "RESULT: FAIL ($FAILURES/6 checks failed)"
exit 1
