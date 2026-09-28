#!/usr/bin/env bash
# tests/test_release_pipeline_consistency.sh
# shellcheck disable=SC2016  # assertions compare literal workflow source fragments.
#
# The repository has two release execution paths with different roles:
#
#   A. scripts/release.sh — the canonical local release pipeline and public
#      asset gate. It stages the 9 required payloads and signs installable
#      assets before publication.
#   B. .github/workflows/release.yml — the Actions distribution path that
#      release-automation.yml dispatches after pushing a version tag. It builds
#      four targets and must assemble the same canonical payloads plus its
#      explicitly allowlisted non-x86 unified packages.
#
# Both paths use scripts/release-assets-contract.sh. This test pins the
# workflow dispatch, actual musl build, canonical alias/receipt assembly and
# the single-publisher invariant so comments cannot silently drift from the
# executable workflow.
#
# Exit 0 = contract consistent. Non-zero = distribution can diverge or break users.

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
INSTALL_SH="$ROOT/scripts/install.sh"
RELEASE_SH="$ROOT/scripts/release.sh"
WORKFLOW="$ROOT/.github/workflows/release.yml"

failures=0
ok()   { printf '  ok   %s\n' "$1"; }
fail() { printf '  FAIL %s\n' "$1"; failures=$((failures + 1)); }
info() { printf '  ..   %s\n' "$1"; }

for f in "$INSTALL_SH" "$RELEASE_SH"; do
    [ -f "$f" ] || { echo "missing $f" >&2; exit 2; }
done

echo "=== release pipeline consistency ==="

# ── 0. The release robot must actually dispatch the distribution workflow ──
AUTOMATION="$ROOT/.github/workflows/release-automation.yml"
if grep -Fq 'gh workflow run release.yml --ref "$TAG"' "$AUTOMATION"; then
    ok "release-automation dispatches release.yml on the pushed tag"
else
    fail "release-automation no longer dispatches release.yml for tagged releases"
fi

# The asset contract is shared by both publishers, not inferred from a stale
# claim about which workflow has run most often.
if grep -Fq 'source "$ROOT/scripts/release-assets-contract.sh"' "$RELEASE_SH" \
    && grep -Fq 'validate_release_asset_contract "$RELEASE_JSON"' "$RELEASE_SH"; then
    ok "release.sh uses the shared public asset contract"
else
    fail "release.sh does not use scripts/release-assets-contract.sh"
fi

# ── 1. The authoritative pipeline must not build musl by name only ────────
# If release.sh keeps calling the asset "musl" it must actually build musl.
# A single `cargo build --release` cannot produce a musl binary.
#
# Match the BUILD COMMAND, not any mention: prose and comments (including the
# authority note in release.sh's header, which quotes the CI target) must not
# be able to satisfy this check.
#
# RESOLVE the target variable rather than demanding a literal. release.sh
# passes `--target "$BUILD_TARGET"` and defaults that variable to the musl
# triple, which is the correct design: the target is configurable but
# defaults to what the asset name promises. A guard that only matched a
# literal `x86_64-unknown-linux-musl` on the cargo line would fail that
# refactor while the actual behaviour stayed correct — and worse, the
# obvious "fix" would be to inline the literal and lose the override.
#
# So: accept a musl literal on the build line, OR a --target that resolves
# to a variable whose default is musl. Both are real evidence.
# Resolve the default ONCE, before the loop. Reading $RELEASE_SH inside the
# while-loop that already consumes it is the SC2094 hazard, and re-grepping
# per iteration is both wrong and slow.
#
# shellcheck disable=SC2016  # the \${...} MUST stay literal: we are matching
# the source text in release.sh, not expanding a shell variable here.
DEFAULT_BUILD_TARGET="$(sed -n 's/^BUILD_TARGET="\${SDDK_RELEASE_BUILD_TARGET:-\([^}]*\)}".*/\1/p' "$RELEASE_SH" | head -1)"

BUILD_CMD_MUSL=0
while IFS= read -r line; do
    # strip comments, then look for a real cargo build invocation
    code="${line%%#*}"
    case "$code" in
        *cargo*build*)
            if printf '%s' "$code" | grep -q 'x86_64-unknown-linux-musl'; then
                BUILD_CMD_MUSL=1
            elif printf '%s' "$code" | grep -q -- '--target' \
                 && [ "$DEFAULT_BUILD_TARGET" = "x86_64-unknown-linux-musl" ]; then
                BUILD_CMD_MUSL=1
            fi
            ;;
    esac
done < "$RELEASE_SH"

if grep -q 'musl' "$RELEASE_SH"; then
    if [ "$BUILD_CMD_MUSL" = "1" ]; then
        ok "release.sh builds an explicit musl target in a cargo build command"
        BUILD_IS_MUSL=1
    else
        fail "release.sh names a 'musl' asset but no cargo build command uses a musl target"
        BUILD_IS_MUSL=0
    fi
else
    ok "release.sh does not claim a musl asset"
    BUILD_IS_MUSL=0
fi

# ── 2. The workflow must map its real x86_64-musl output to the canonical
# generic binary, and build its canonical receipt and unified tarball names. ─
if grep -Fq 'cp downloads/sddk-linux-x86_64-musl "$STAGE/sddk"' "$WORKFLOW" \
    && grep -Fq 'gh-release-receipt.json' "$WORKFLOW" \
    && grep -Fq 'scripts/release-receipt.sh' "$WORKFLOW" \
    && grep -Fq 'sddk-${TAG}-sddk-linux-x86_64-musl.tar.gz' "$WORKFLOW"; then
    ok "release.yml builds the canonical binary alias, receipt, and unified asset"
else
    fail "release.yml omits one of the canonical installer payloads"
fi

# Only the dedicated publish job may mutate GitHub Release state. Building,
# bundling, and signing must finish before an asset is made public.
PUBLISH_STEPS="$(grep -vE '^[[:space:]]*#' "$WORKFLOW" | grep -cE 'gh release (create|upload)')"
if [ "$PUBLISH_STEPS" -eq 2 ] && grep -Fq 'needs: [sign]' "$WORKFLOW"; then
    ok "release.yml has one post-sign publisher (create + retry upload only)"
else
    fail "release.yml publishes before signing or has multiple competing publishers"
fi

if grep -Fq 'validate_release_asset_contract "$RELEASE_JSON" "${TAG#v}"' "$WORKFLOW"; then
    ok "release.yml executes the shared contract before smoke installation"
else
    fail "release.yml does not validate the published asset set"
fi

# ── 3. The workflow must not be the only place the contract is defined ───
# If the authoritative pipeline and the CI pipeline can disagree, that is a
# structural hazard. At minimum, the release script must reference the CI
# workflow, so a reader knows both exist and which one wins.
if grep -qF 'release.yml' "$RELEASE_SH"; then
    ok "release.sh acknowledges the CI release workflow"
else
    fail "release.sh never mentions .github/workflows/release.yml — two pipelines, no declared authority"
fi

# ── 4. Report, do not enforce, the musl/glibc reality ───────────────────
# This is a fact about the artifact, not a policy. Print it so it is visible.
if [ "$BUILD_IS_MUSL" = "0" ] && grep -q 'musl' "$RELEASE_SH"; then
    info "REALITY: the published 'musl' asset contains a host-glibc binary."
    info "         Users on glibc < build host cannot run it. See INC-DEBT-021."
fi

# ─────────────────────────────────────────────────────────────────────────
# session-29 — step 2.5 BEHAVIOURAL contract.
#
# The checks above are static. They did not catch two real mutations:
#   M4  short-circuit exit 0 -> exit 1  (aborts a legitimate release)
#   M5  release.sh "keep TAG" warn -> die (changes the outcome)
# A grep cannot see either, so this section EXECUTES the real decision.
#
# What is actually under test: release.sh step 2.5 runs
# `release-bump.sh --dry-run`, parses `new tag:`, and if empty KEEPS the
# workspace version as the release tag. After session-29's short-circuit
# there is a THIRD way to produce no tag — the workspace already declares
# the release. These must not be confused: the tag published has to be the
# workspace version, and the pipeline must not die.
#
# The decision is reproduced verbatim from release.sh lines 402-421 against
# the real release-bump.sh in an isolated fixture. Both scripts are copied
# in, exactly as CI does (CI checks out the repo, so its ROOT is the repo).
#
# KNOWN LIMIT, stated rather than hidden: release.sh cannot be invoked here.
# It requires `gh auth status`, a verified remote and a clean main, and past
# step 8 it publishes irreversibly. So the behavioural harness re-implements
# step 2.5's decision instead of executing release.sh. A mutation applied ONLY
# to release.sh's copy of that logic (M8: turning the "keep TAG" warn into a
# die) is therefore not visible to the behavioural cases — which is why the
# two static checks above exist and why the limit is written down instead of
# being discovered later as a false guarantee.
# ─────────────────────────────────────────────────────────────────────────

# Static: release.sh must KEEP the workspace tag (not die) when
# release-bump.sh derives none. This is the branch the session-29
# short-circuit now reaches on the declared-release path.
if grep -q 'release-bump.sh did not produce a tag; keeping workspace-derived TAG' "$ROOT/scripts/release.sh"; then
    ok "release.sh keeps the workspace TAG when release-bump derives none"
else
    fail "release.sh no longer keeps the workspace tag; M8-shaped mutation"
fi

# Static: the short-circuit must exit 0. release.sh line 408 turns a
# non-zero exit into `die "cannot compute SemVer tag"`, which would abort a
# legitimate release. The behavioural CASE C below proves the same thing by
# execution; this pins it without running the fixture.
if grep -A2 'no bump to derive: the workspace already declares' "$ROOT/scripts/release-bump.sh" \
   | grep -qE '^\s*exit 0$'; then
    ok "the declared-release short-circuit exits 0, so release.sh does not die"
else
    fail "the declared-release short-circuit must exit 0, not error out"
fi

step25_tag() {
    # Reproduce release.sh step 2.5. Echoes the final TAG it would release.
    local dir="$1" workspace_ver="$2" tag_ver="$3" commit_subject="$4"
    # NOTE: each case needs a FRESH directory. A leftover fixture makes
    # `git tag` fail ("already exists"), the subshell aborts before the
    # script is copied, and the harness then measures whatever was left
    # behind — a green check that tested nothing.
    chmod -R u+rw "$dir" 2>/dev/null || true
    rm -rf "$dir"
    mkdir -p "$dir/scripts"
    (
        cd "$dir" || exit 2
        git init -q .
        git config user.email t@example.com
        git config user.name t
        cp "$ROOT/scripts/release-bump.sh" scripts/release-bump.sh
        mkdir -p crates/fake
        cat > Cargo.toml <<EOF
[workspace]
members = ["crates/fake"]

[workspace.package]
version = "$workspace_ver"
edition = "2021"
EOF
        cat > crates/fake/Cargo.toml <<'EOF'
[package]
name = "fake"
version.workspace = true
EOF
        git add -A
        git commit -qm "chore: base"
        git tag "v$tag_ver"
        git commit -q --allow-empty -m "$commit_subject"
    ) >/dev/null 2>&1

    # ---- verbatim from scripts/release.sh step 2.5 ----
    local VERSION TAG STEP2P5_OUTPUT SEMVER_TAG
    VERSION="$workspace_ver"
    TAG="v$VERSION"
    if STEP2P5_OUTPUT="$(cd "$dir" && bash scripts/release-bump.sh --dry-run 2>&1)"; then
        :
    else
        # release.sh: die "cannot compute SemVer tag"
        echo "DIE"
        return 0
    fi
    SEMVER_TAG="$(echo "$STEP2P5_OUTPUT" | awk '/^new tag: / {print $3; exit}')"
    if [ -z "$SEMVER_TAG" ]; then
        :                  # release.sh warns and KEEPS TAG
    else
        [ "$SEMVER_TAG" != "$TAG" ] && TAG="$SEMVER_TAG"
    fi
    echo "$TAG"
}

TMPD="$(mktemp -d)"
trap 'chmod -R u+rw "$TMPD" 2>/dev/null; rm -rf "$TMPD"' EXIT

# CASE A: workspace declares the release (2.2.0 ahead of v2.0.1).
# The published tag MUST be the workspace version, and step 2.5 must not die.
got="$(step25_tag "$TMPD/a" 2.2.0 2.0.1 'feat: something')"
if [ "$got" = "v2.2.0" ]; then
    ok "step 2.5 publishes the declared workspace version (v2.2.0), no double bump"
else
    fail "step 2.5 published '$got'; the declared release v2.2.0 must win"
fi

# CASE B: workspace == tag (normal flow). release-bump.sh derives a bump and
# release.sh adopts it. This guards the fix from silencing NORMAL releases.
got="$(step25_tag "$TMPD/b" 2.0.1 2.0.1 'feat: something')"
if [ "$got" = "v2.1.0" ]; then
    ok "step 2.5 still adopts the derived semver tag on a normal release"
else
    fail "step 2.5 produced '$got'; a normal release must still bump to v2.1.0"
fi

# CASE C: the short-circuit must not error out. A non-zero exit from
# release-bump.sh makes release.sh `die` and abort a legitimate release.
got="$(step25_tag "$TMPD/c" 2.2.0 2.0.1 'feat: something')"
if [ "$got" != "DIE" ]; then
    ok "step 2.5 does not die on the declared-release short-circuit"
else
    fail "release-bump.sh exited non-zero; release.sh would die on a valid release"
fi
# ─────────────────────────────────────────────────────────────────────────
# session-29 — BUNDLE.toml manifest_sha256 must be the MANIFEST's own digest.
#
# release.sh computed it as `awk 'NR==1 {print $1}' MANIFEST.sha256`, which is
# the sha256 of the first FILE listed in the manifest (agents/analytics-judge.md).
# The field claimed to bind the bundle to its manifest and instead named a
# bundled file — while nothing validated it, so it was both wrong and inert
# (INC-DEBT-025-MANIFEST-SHA-FROM-FIRST-LINE).
#
# Behavioural, not a grep: it writes a real MANIFEST.sha256, SOURCES the real
# assignment line out of release.sh, and compares the result against the
# file's true digest. Sourcing (rather than grep-and-hope) is what makes this
# falsifiable — mutating the line changes the outcome.
# ─────────────────────────────────────────────────────────────────────────

SHIM="$TMPD/manifest-sha"
mkdir -p "$SHIM"
# Two entries with DIFFERENT digests, so choosing the wrong one is
# observable rather than coincidental.
cat > "$SHIM/MANIFEST.sha256" <<'MANIFEST'
aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa  agents/analytics-judge.md
bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb  agents/orchestrator.md
MANIFEST

REAL_SHA="$(cd "$SHIM" && sha256sum MANIFEST.sha256 | awk '{print $1}')"
FIRST_LINE_SHA="$(awk 'NR==1 {print $1}' "$SHIM/MANIFEST.sha256")"

# Pull the real assignment out of release.sh and run it against the shim.
EXTRACTED=""
if grep -E '^MANIFEST_SHA=' "$ROOT/scripts/release.sh" > "$SHIM/assign.sh"; then
    # shellcheck source=/dev/null   # runtime-generated, not a fixed file
    EXTRACTED="$( cd "$SHIM" && MANIFEST_SHA='' && . ./assign.sh && printf '%s' "$MANIFEST_SHA" )"
fi

if [ -z "$EXTRACTED" ]; then
    fail "release.sh has no MANIFEST_SHA= assignment line to check"
elif [ "$EXTRACTED" = "$REAL_SHA" ]; then
    ok "manifest_sha256 is the manifest's own sha256"
else
    fail "manifest_sha256 is $EXTRACTED; expected the manifest digest $REAL_SHA (the first-line value would be $FIRST_LINE_SHA)"
fi

# The fixture must actually be able to catch the BUG, not merely have two
# unequal hashes. The previous version of this guard compared
# REAL_SHA != FIRST_LINE_SHA, which stays true even for a one-line manifest:
# it proved the two values differ, but never proved the buggy extraction
# would FAIL against this fixture. Mutation M11 (collapse the fixture to one
# line) slipped through it.
#
# So assert the discrimination the gate actually depends on: feeding the
# buggy first-line extraction must NOT equal the correct answer.
#
# Note on a mutation that was tried and correctly does NOT fail here (M11,
# collapsing the fixture to a single line): a one-line manifest still has a
# file digest different from that line's literal value, so the gate keeps
# discriminating. The guard is right to stay green — M11 is not a defect in
# the code under test, only in the fixture's tidiness.
BUGGY_EXTRACT="$(awk 'NR==1 {print $1}' "$SHIM/MANIFEST.sha256")"
if [ "$BUGGY_EXTRACT" != "$REAL_SHA" ]; then
    ok "fixture discriminates: the buggy first-line extraction fails against it"
else
    fail "fixture is vacuous: the buggy extraction would pass, so this gate proves nothing"
fi

# ─────────────────────────────────────────────────────────────────────────
# session-30 retrospective — the musl staticness guard must accept BOTH
# static spellings and reject dynamic ones.
#
# INC-021-FALSE-NEGATIVE: the guard only looked for the literal "statically
# linked", but `file` 5.46 describes a real static-pie musl binary as
#
#   ELF 64-bit LSB pie executable, x86-64, static-pie linked, stripped
#
# So the guard REJECTED the correct musl binary. It was fixed to accept both
# spellings, but until now only by hand-verified cases, with no permanent
# test. This is that test, and it is behavioural in the same sense as the
# manifest-sha gate above: it sources the REAL pattern out of release.sh
# rather than grep-and-hope, so mutating the pattern changes the outcome.
# ─────────────────────────────────────────────────────────────────────────

if grep -E "grep -qE '" scripts/release.sh | grep -E 'statically linked|static-pie linked' > "$TMPD/static-pattern.txt" 2>/dev/null; then
    ok "release.sh has a staticness pattern to check"
else
    fail "release.sh has no staticness pattern line to check"
fi

# The real descriptions, verbatim from `file` on this host and from the
# upstream musl toolchain. Both static spellings must pass; both dynamic ones
# must fail.
# The pattern is SOURCED from release.sh, never hardcoded here. Hardcoding it
# makes the test vacuous: it would keep passing even after release.sh
# regressed to the old single-spelling pattern, which is exactly the bug this
# test exists to catch. (Observed while writing it: the first version
# hardcoded the pattern and a mutation of release.sh survived it.)
STATIC_PATTERN=""
if grep -E "grep -qE '" scripts/release.sh | grep -E 'statically linked|static-pie linked' > "$TMPD/static-line.txt" 2>/dev/null; then
    # Pull the -E pattern argument out of the real line.
    STATIC_PATTERN="$( sed -n "s/.*grep -qE '\([^']*\)'.*/\1/p" "$TMPD/static-line.txt" | head -1 )"
fi

check_static() {
    local desc="$1" want="$2" label="$3"
    if [ -z "$STATIC_PATTERN" ]; then
        fail "staticness guard: $label -> could not extract the pattern from release.sh"
        return
    fi
    if printf '%s' "$desc" | grep -qE "$STATIC_PATTERN"; then
        got=accept
    else
        got=reject
    fi
    if [ "$got" = "$want" ]; then
        ok "staticness guard: $label -> $got"
    else
        fail "staticness guard: $label -> $got, expected $want"
    fi
}

check_static "ELF 64-bit LSB pie executable, x86-64, static-pie linked, stripped" accept \
    "static-pie (the real musl binary, the regression case)"
check_static "ELF 64-bit LSB executable, x86-64, statically linked, stripped" accept \
    "statically linked (plain static)"
check_static "ELF 64-bit LSB pie executable, x86-64, dynamically linked, interpreter /lib64/ld-linux-x86-64.so.2" reject \
    "dynamically linked (INC-021 must not recur)"
check_static "ELF 64-bit LSB shared object, x86-64, dynamically linked" reject \
    "shared object"

echo
if [ "$failures" -ne 0 ]; then
    echo "release pipeline consistency: $failures check(s) FAILED"
    exit 1
fi
echo "release pipeline consistency: all checks passed"
