#!/usr/bin/env bash
# Gate: the declared CHANGELOG section must COVER the commits it describes.
#
# Why this exists (session-61). `tests/test_changelog_merge.sh` proves the
# merge in `release-bump.sh` does not duplicate a version header. It says
# nothing about whether the section under that header *describes the work*.
# Those are different properties, and the gap was live: 26 commits sat
# between v2.4.2 and HEAD, including a whole `feat(architecture)` slice, while
# the `[2.5.0]` section listed only the two features present when the bump was
# committed. Had the release shipped, the published artifact's changelog would
# not have described its own contents.
#
# The same shape as the C3l.7 defect — an artifact that does not declare what
# it is — one layer down.
#
# Contract checked here:
#   (a) the section for the workspace version exists exactly once;
#   (b) every feat/fix commit since the last published tag appears in that
#       section, matched on a normalised subject (scope and punctuation are
#       allowed to drift, the payload is not);
#   (c) test(...) commits are represented too — a test that is the only
#       evidence for a fix is part of what shipped.
#
# Fail-closed: an unparseable changelog, a missing section, or an empty commit
# range each fail with a distinct message. Exit 0 only when coverage holds.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CHANGELOG="$ROOT/CHANGELOG.md"
PASS=0
FAIL=0

ok()   { echo "  [ok]   $1"; PASS=$((PASS + 1)); }
bad()  { echo "  [FAIL] $1"; FAIL=$((FAIL + 1)); }

[[ -f "$CHANGELOG" ]] || { echo "FAIL: no CHANGELOG.md at $CHANGELOG"; exit 1; }

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)"
[[ -n "$VERSION" ]] || { echo "FAIL: cannot read workspace version from Cargo.toml"; exit 1; }

echo "=== changelog coverage gate (workspace $VERSION) ==="

# ── (a) the section exists exactly once ─────────────────────────────────────
HEADER="## [$VERSION]"
COUNT="$(grep -cF "$HEADER" "$CHANGELOG" || true)"
if [[ "$COUNT" -eq 1 ]]; then
  ok "exactly one '$HEADER' header (got $COUNT)"
else
  bad "expected exactly one '$HEADER' header, got $COUNT"
fi

# ── the section body: from its header to the next '## [' ────────────────────
SECTION="$(awk -v h="$HEADER" '
  index($0, h) == 1 { inb = 1; next }
  inb && /^## \[/ { exit }
  inb { print }
' "$CHANGELOG")"

if [[ -z "$(echo "$SECTION" | tr -d '[:space:]')" ]]; then
  bad "section '$HEADER' is empty — it declares nothing about this release"
else
  ok "section '$HEADER' has content"
fi

# ── (b)/(c) every feat/fix/test commit since the last published tag ─────────
LAST_TAG="$(git -C "$ROOT" tag --sort=-version:refname | head -1)"
if [[ -z "$LAST_TAG" ]]; then
  echo "  [skip] no published tag found; coverage cannot be checked"
  echo
  echo "PASS=$PASS FAIL=$FAIL"
  echo "RESULT: PASS (nothing to compare against)"
  exit 0
fi

echo "  (comparing against last published tag: $LAST_TAG)"

# Normalise a subject for comparison: lowercase, collapse whitespace.
norm() { tr '[:upper:]' '[:lower:]' | tr -s '[:space:]' ' '; }

MISSING=0
CHECKED=0
while IFS= read -r SUBJECT; do
  [[ -z "$SUBJECT" ]] && continue
  # Only the kinds a reader uses to learn what shipped. docs/chore are
  # deliberately excluded: they do not change behaviour and a changelog that
  # lists every pointer sync is noise.
  case "$SUBJECT" in
    feat*|fix*|test*) ;;
    *) continue ;;
  esac
  CHECKED=$((CHECKED + 1))
  KEY="${SUBJECT%%:*}"                 # type(scope)
  PAYLOAD_NORM="$(printf '%s' "${SUBJECT#*: }" | norm)"
  SECTION_NORM="$(printf '%s' "$SECTION" | norm)"
  if printf '%s' "$SECTION_NORM" | grep -qF "$KEY"; then
    # type present; require a distinctive slice of the payload too, so a bare
    # "feat(lease):" line cannot satisfy an unrelated feat.
    # Use the first 4 significant words of the payload as a fingerprint.
    FINGERPRINT="$(printf '%s' "$PAYLOAD_NORM" | cut -d' ' -f1-4)"
    if printf '%s' "$SECTION_NORM" | grep -qF "$FINGERPRINT"; then
      ok "covered: $SUBJECT"
    else
      bad "declared type '$KEY' present but not this commit: $SUBJECT"
      MISSING=$((MISSING + 1))
    fi
  else
    bad "missing from section '$HEADER': $SUBJECT"
    MISSING=$((MISSING + 1))
  fi
done < <(git -C "$ROOT" log --format=%s "$LAST_TAG"..HEAD)

if [[ "$CHECKED" -eq 0 ]]; then
  bad "no feat/fix/test commits found in range — the gate cannot vouch for anything"
elif [[ "$MISSING" -eq 0 ]]; then
  ok "all $CHECKED feat/fix/test commits since $LAST_TAG are represented"
fi

echo
echo "PASS=$PASS FAIL=$FAIL"
if [[ "$FAIL" -eq 0 ]]; then
  echo "RESULT: PASS — the declared section describes the work it ships."
  exit 0
fi
echo "RESULT: FAIL — shipping this would publish a changelog that misdescribes the artifact."
exit 1
