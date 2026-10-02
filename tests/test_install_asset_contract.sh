#!/usr/bin/env bash
# tests/test_install_asset_contract.sh
#
# Characterisation test for the installer <-> release asset-name contract.
#
# Why this exists: the cycle-46 unified-tarball redesign changed which assets
# `scripts/release.sh` publishes, but nothing tied `scripts/install.sh` to those
# names. The installer kept requesting `sddk-<os>-<arch>-musl` as a *bare*
# binary asset, which the release no longer publishes (it publishes a bare
# `sddk` plus a versioned unified tarball). Every user without `gh` on PATH —
# i.e. the default path — therefore hit HTTP 404 on the binary and aborted
# before linking anything. The e2e that should have caught it reported FAIL but
# exited 0, so nothing gated on it.
#
# This test pins the *contract* statically (no network): for every asset name
# the installer can request, assert the release pipeline actually publishes an
# asset matching it. It is a static cross-check of two shell scripts, so it is
# fast and hermetic.
#
# Exit 0 = contract holds. Non-zero = installer and release disagree.

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
INSTALL_SH="$ROOT/scripts/install.sh"
RELEASE_SH="$ROOT/scripts/release.sh"
RELEASE_YML="$ROOT/.github/workflows/release.yml"
# Every source path is anchored to $ROOT. Relative paths made the whole
# suite report "all checks passed" — or fail — depending on the CWD it was
# invoked from: a CI runner that does not cd to the repo would read
# non-existent files, and `grep` on a missing file returns 1, which in an
# `if` reads exactly like "the thing is broken".
UPDATE_RS="$ROOT/crates/sddk-cli/src/dev/update.rs"
DEV_MOD_RS="$ROOT/crates/sddk-cli/src/dev/mod.rs"

failures=0
ok()   { printf '  ok   %s\n' "$1"; }
fail() { printf '  FAIL %s\n' "$1"; failures=$((failures + 1)); }

[ -f "$INSTALL_SH" ] || { echo "missing $INSTALL_SH" >&2; exit 2; }
[ -f "$RELEASE_SH" ] || { echo "missing $RELEASE_SH" >&2; exit 2; }

echo "=== install asset contract ==="

# ── 1. The unified tarball name must be built from the RESOLVED version ──────
# install.sh builds:  UNIFIED_TARBALL="sddk-${VERSION}-${ASSET}.tar.gz"
# release.sh builds:  UNIFIED="$TMP/sddk-${TAG}-sddk-linux-x86_64-musl.tar.gz"
# and publishes:       sddk-${TAG}-sddk-linux-x86_64-musl.tar.gz
#
# The asset name embeds the version. If the installer is asked for `latest`
# and cannot resolve it (no `gh`), it must NOT try to download
# `sddk-latest-...tar.gz`; it must fall back to the pinned assets. Guard the
# resolution: the unified branch must be gated on a resolved version.
if grep -q 'RESOLVED_VERSION" != "latest"' "$INSTALL_SH"; then
    ok "unified download is gated on a resolved (non-latest) version"
else
    fail "unified download is not gated on a resolved version"
fi

# ── 2. The legacy/split path must request an asset the release publishes ────
# The bare binary asset. release.sh publishes `basename $BIN` == `sddk`.
# A bare `sddk-<os>-<arch>-musl` download must therefore not exist.
# shellcheck disable=SC2016  # grep de literales con $ASSET de install.sh (contrato)
if grep -qE 'download "\$\(release_url "\$ASSET"\)"' "$INSTALL_SH"; then
    fail "installer downloads bare \$ASSET (sddk-<os>-<arch>-musl); release publishes bare 'sddk'"
else
    ok "installer does not request a bare \$ASSET binary"
fi

# The bare binary must be fetched under the name release.sh actually publishes.
if grep -q 'release_url "sddk"' "$INSTALL_SH"; then
    ok "installer fetches the binary as 'sddk' (matches published asset)"
else
    fail "installer does not fetch the binary as 'sddk'"
fi

# ── 3. The bundle asset must be a published name ───────────────────────────
if grep -q 'release_url "software-development-decision-kernel.tar.gz"' "$INSTALL_SH"; then
    ok "installer fetches bundle 'software-development-decision-kernel.tar.gz' (published)"
else
    fail "installer does not fetch the published bundle name"
fi

# ── 4. release.sh must actually publish the bare binary asset ──────────────
# release.sh uploads `sddk` (basename of the built binary) per the public gate.
# shellcheck disable=SC2016  # grep de literales con $BIN de release.sh (contrato)
if grep -qE 'gh release upload.*"\$\(basename "\$BIN"\)"|\$BIN"|"sddk"' "$RELEASE_SH"; then
    ok "release.sh publishes the bare binary asset"
else
    fail "release.sh does not clearly publish a bare binary asset"
fi

# ── 5. No path may trust a checksum that is optional-with-warning ──────────
# install.sh currently tolerates a missing unified .sha256 ("skipping checksum
# verification"). A payload whose checksum is optional is a soft integrity
# downgrade; it must not silently skip when a checksum asset exists upstream.
if grep -q 'skipping checksum verification' "$INSTALL_SH"; then
    fail "installer silently skips checksum verification when .sha256 is missing"
else
    ok "installer does not silently skip checksum verification"
fi

# ── 6. The binary version probe must fail loudly, not echo its input ───────
# `"$bin" --version | awk '{print $NF}'` silently yields the input unchanged
# when the binary cannot execute (wrong libc/arch), and the installer then
# aborts with a bare `exit 1`. The probe must be validated.
if grep -q 'probe_binary_version' "$INSTALL_SH"; then
    ok "installer uses a validated binary version probe"
else
    fail "installer has no validated binary version probe"
fi
# Only the *call sites* matter here: a `--version | awk` inside
# probe_binary_version is the validated path, not the defect.
if grep -E '^[[:space:]]*TMP_VERSION=' "$INSTALL_SH" | grep -qE -- '--version.*\|.*awk'; then
    fail "installer still parses --version through a bare awk pipe at a call site"
else
    ok "installer has no unvalidated --version | awk call site"
fi

# ── 7. The probe must actually reject a non-semver result ─────────────────
# A behavioural check: a fake "binary" that prints garbage must make the
# probe fail rather than return the garbage.
probe_body="$(sed -n '/^probe_binary_version()/,/^}/p' "$INSTALL_SH")"
if printf '%s' "$probe_body" | grep -q 'grep -qE .*0-9'; then
    ok "probe_binary_version validates the version format"
else
    fail "probe_binary_version does not validate the version format"
fi

# ── 8. release.sh must not label a non-musl build as musl ──────────────────
# release.sh builds the musl asset. The original lie was a single host
# `cargo build --release`; the fix compiles with `--target` from
# BUILD_TARGET (default x86_64-unknown-linux-musl). Extract the actual
# default instead of grepping for a literal: a pin that only recognises one
# spelling of the fix goes stale the moment the variable is introduced.
if grep -q 'musl' "$RELEASE_SH"; then
    # The name still says musl: acceptable ONLY if the build targets musl.
# shellcheck disable=SC2016  # regex literal sobre release.sh: el default es un literal
    BUILD_TARGET_DEFAULT="$(sed -n 's/^BUILD_TARGET="${SDDK_RELEASE_BUILD_TARGET:-\([^}]\+\)}".*/\1/p' "$RELEASE_SH" | head -1)"
    if [ -n "$BUILD_TARGET_DEFAULT" ] && grep -q -- "--target \"\$BUILD_TARGET\"" "$RELEASE_SH" \
        && grep -q -- 'die "cargo build failed para target' "$RELEASE_SH"; then
        ok "release.sh builds a real musl target for the musl asset name ($BUILD_TARGET_DEFAULT)"
    else
        fail "release.sh labels a host-glibc build as 'musl' (name promises a binary it does not ship)"
    fi
else
    ok "release.sh does not claim a musl asset"
fi

# ── signature contract (INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY) ──
#
# Both consumers now REFUSE an unsigned artifact. That is only safe if the
# release actually signs, so the three sides have to agree: release.sh
# signs, install.sh verifies, dev update verifies. A change on one side
# without the others turns every install into a hard failure.
# Read the file ONCE with awk instead of piping greps. `grep -q` exits as
# soon as it matches and closes the pipe, so the upstream `grep -v` can be
# killed by SIGPIPE; under `set -o pipefail` that turns into a non-zero
# result. The failure was INTERMITTENT (2 of 5 runs failed here), which is
# worse than a hard failure: a guard that is green half the time trains
# everyone to ignore it. awk has no early exit, so the result is stable.
if awk '
    /^[[:space:]]*#/                 { next }
    /^[[:space:]]*(die|warn|ok|echo) / { next }
    /cosign sign-blob/ && !/^[[:space:]]*#/ { found = 1 }
    END { exit(found ? 0 : 1) }
' "$RELEASE_SH"; then
    ok "release.sh signs its artifacts with cosign (signing loop, not a comment)"
else
    fail "release.sh has no active cosign sign-blob call; installers now require signatures"
fi

# Count CALLS, not occurrences. `grep -q verify_signature` is satisfied by
# the function DEFINITION alone, so deleting every call site leaves the
# check green. That is precisely the failure mode this test exists to
# catch, reproduced in its own body — so the assertion has to be stricter
# than the thing it is asserting about.
#
# A call site is `verify_signature "$..."` with a quoted argument on the
# same line; the definition ends with `) {` and takes no such argument.
_verify_calls() {
    grep -cE '^[[:space:]]*verify_signature[[:space:]]+"' "$1" || true
}
if [ "$(_verify_calls "$INSTALL_SH")" -ge 2 ]; then
    ok "install.sh verifies a signature on BOTH download paths (unified + legacy)"
else
    fail "install.sh has only $(( $(_verify_calls "$INSTALL_SH") )) of 2 required verify_signature call sites — an artifact path is unprotected"
fi

if grep -q 'SDDK_ALLOW_UNSIGNED' "$INSTALL_SH"; then
    ok "install.sh has a named, explicit opt-out (SDDK_ALLOW_UNSIGNED)"
else
    fail "install.sh has no opt-out: unsigned releases become uninstallable with no escape"
fi

# A call site ends in `?;`, a definition ends in `{`. Matching the bare
# name is satisfied by the function definition on its own, which means the
# check would pass with every call site deleted.
if grep -qF 'verify_bundle_signature(&bundle, &url, args.allow_unsigned)?;' "$UPDATE_RS" 2>/dev/null; then
    ok "sddk dev update verifies a signature before installing (call site, not definition)"
else
    fail "sddk dev update has no verify_bundle_signature call site (authenticity gap reopened)"
fi

# A present-but-invalid signature must be a hard failure in BOTH paths.
# If either degrades to a warning, an attacker who controls the download
# origin can serve a bad signature and downgrade the check themselves.
# The bad-signature branch must abort, not warn. Checking only that the
# string "exit 1" appears anywhere in the file is worthless — install.sh
# has many exit 1 sites, so downgrading THIS branch to a warning leaves
# the check green. Pin the branch: the FAILED message must be followed by
# an abort in its own block.
if awk '
    /cosign verification FAILED/ { inbranch=1; next }
    inbranch && /^[[:space:]]*exit 1/ { found=1; inbranch=0 }
    inbranch && /^[[:space:]]*(fi|})/ { inbranch=0 }
    END { exit(found ? 0 : 1) }
' "$INSTALL_SH"; then
    ok "install.sh aborts on a failed signature verification (not a warning)"
else
    fail "install.sh does not exit on a bad signature — an attacker can downgrade this to a warning"
fi

# Signing must be all-or-nothing. A release where the binary has a
# signature and the bundle tarball does not passes an `-eq 0` guard, then
# fails at install time for the user. The count is compared against the
# size of the list, so adding an artifact cannot silently weaken the gate.
# shellcheck disable=SC2016  # literal productivo grep -F (contrato)
if grep -qF '[ "$SIGNED_COUNT" -ne "${#SIGN_ARTIFACTS[@]}" ]' "$RELEASE_SH"; then
    ok "release.sh requires ALL artifacts signed, not just one"
else
    fail "release.sh allows a partially signed release (SIGNED_COUNT -eq 0)"
fi

# release.sh must publish the certificate, not only the signature. The
# CI signs detached, and both consumers now REFUSE a detached signature
# that has no .pem. If release.sh ships only .sig + .bundle.json, a release
# signed locally is verifiable only through the bundle path — and a
# consumer that fetched the .sig gets a refusal instead of a verdict.
if grep -qF 'for ext in .sig .bundle.json .pem; do' "$RELEASE_SH"; then
    ok "release.sh publishes .sig, .bundle.json and .pem for every artifact"
else
    fail "release.sh does not publish the .pem certificate; detached verification is impossible"
fi

# The CI smoke test greps install.sh output for a success string. If the
# two drift, the release FAILS at the smoke-test step on the first real tag
# — and that string was already wrong once ("cosign keyless" vs "cosign,
# sigstore trust root"). Extract the exact string each side uses and
# compare, so the divergence fails here instead of at publish time.
_install_msg=$(sed -n 's/.*echo "  \(signature verified[^"]*\)".*/\1/p' "$INSTALL_SH" | head -1)
_ci_msg=$(sed -n 's/.*grep -Fq "\(signature verified[^"]*\)".*/\1/p' "$RELEASE_YML" | head -1)

if [ -n "$_install_msg" ] && [ "$_install_msg" = "$_ci_msg" ]; then
    ok "release.yml smoke test greps the exact string install.sh prints"
else
    fail "smoke test and installer disagree on the success string: ci='$_ci_msg' installer='$_install_msg'"
fi

# The CI signs DETACHED (.sig + .pem). A verifier that only reads --bundle
# falls through to --signature with no certificate, which cannot pin an
# identity. Both consumers must fetch the .pem.
# shellcheck disable=SC2016  # literal productivo grep -F (contrato)
if grep -qF -- '--certificate $cert_file' "$INSTALL_SH"; then
    ok "install.sh passes the leaf --certificate on the detached path"
else
    fail "install.sh does not pass the leaf certificate on the detached path (verification dies: chain-only)"
fi

if grep -qF '"--certificate"]' "$UPDATE_RS"; then
    ok "sddk dev update passes the leaf --certificate on the detached path"
else
    fail "sddk dev update does not pass the leaf certificate on the detached path"
fi

# The pinning constants live in crates/sddk-cli/src/cosign.rs and are
# copied into install.sh. A silent divergence there means the CLI trusts a
# signer the shell installer rejects, or worse both accept something nobody
# intended. Extract both and compare, so drift is a test failure.
COSIGN_RS="$ROOT/crates/sddk-cli/src/cosign.rs"
# rustfmt may keep the raw string on one line or wrap it, so match the
# declaration by name and take everything between r" and "; regardless of
# layout. A layout-specific extraction silently returns "" after a fmt run,
# and then "cosign.rs == install.sh" compares empty to empty and passes.
_rust_identity=$(sed -n 's/^pub const LEGACY_CERT_IDENTITY_REGEXP: &str = r"\(.*\)";$/\1/p' "$COSIGN_RS" | head -1)
if [ -z "$_rust_identity" ]; then
    _rust_identity=$(tr '\n' ' ' < "$COSIGN_RS" \
        | sed -n 's/.*LEGACY_CERT_IDENTITY_REGEXP: &str = r"\(.*\)";.*/\1/p')
fi
_rust_issuer=$(sed -n 's/^pub const LEGACY_CERT_ISSUER: &str = "\(.*\)";$/\1/p' "$COSIGN_RS" | head -1)
# Read the shell copy the way the certification harness reads it: from the
# parameter-expansion default. install.sh has no named constant for these
# (shellcheck reported them unused when they existed), and the inline form is
# what a pre-migration install.sh contains — so reading the same way keeps
# this drift check meaningful for both shapes.
_shell_identity=$(grep -o 'SDDK_COSIGN_IDENTITY:-[^}]*' "$INSTALL_SH" | head -1)
_shell_identity=${_shell_identity#SDDK_COSIGN_IDENTITY:-}
_shell_issuer=$(grep -o 'SDDK_COSIGN_ISSUER:-[^}]*' "$INSTALL_SH" | head -1)
_shell_issuer=${_shell_issuer#SDDK_COSIGN_ISSUER:-}

if [ -n "$_rust_identity" ] && [ "$_rust_identity" = "$_shell_identity" ]; then
    ok "install.sh and cosign.rs pin the same certificate identity"
else
    fail "certificate identity drift: cosign.rs='$_rust_identity' install.sh='$_shell_identity'"
fi

# ── The pinned identity must match the subject Fulcio ACTUALLY mints ────────
# Observed on the first signed release (v2.2.11, Actions run 36477625442,
# 2026-09-28): cosign printed the real certificate subject when rejecting the
# old pattern:
#
#   got subjects [https://github.com/Rubentxu/software-development-decision-kernel/
#   .github/workflows/release.yml@refs/tags/v2.2.11]
#   with issuer https://token.actions.githubusercontent.com
#
# The previously shipped pattern assumed `owner/repo:workflow@ref` (colon
# form, no scheme/host). It never matched a real certificate because there
# had never been a signed release to check against, and the Rust tests built
# their expectation with the same wrong shape (self-referential). Embed the
# OBSERVED subject literally so the pin is anchored to reality, not to a
# helper that can share the pattern's blind spot.
_OBSERVED_SUBJECT="https://github.com/Rubentxu/software-development-decision-kernel/.github/workflows/release.yml@refs/tags/v2.2.11"
_OBSERVED_REJECT="Rubentxu/software-development-decision-kernel:.github/workflows/release.yml@refs/tags/v2.2.11"
if [[ "$_OBSERVED_SUBJECT" =~ $_shell_identity ]]; then
    ok "pinned identity matches the subject observed on the first signed release"
else
    fail "pinned identity NEVER matched the real Fulcio subject (install.sh copy)"
fi
if [[ ! "$_OBSERVED_REJECT" =~ ^https:// ]]; then
    : # the colon form is what the old wrong pattern expected; informational
fi

# verify_signature must only call functions that exist. The signature probe
# used to call `fetch`, a function install.sh does not define: every probe
# returned 127 inside `2>/dev/null`, so a FULLY SIGNED release was reported
# as "no signature asset published" and the smoke test failed while the
# release was actually complete (v2.2.11, run 36477625442). bash -n cannot
# catch undefined functions, so pin the calls by name.
if grep -qE '(^|[^_a-zA-Z])fetch ' "$INSTALL_SH"; then
    fail "install.sh still calls fetch(), which it never defines (probe always fails)"
else
    ok "install.sh signature probe calls only defined helpers"
fi

if [ -n "$_rust_issuer" ] && [ "$_rust_issuer" = "$_shell_issuer" ]; then
    ok "install.sh and cosign.rs pin the same OIDC issuer"
else
    fail "OIDC issuer drift: cosign.rs='$_rust_issuer' install.sh='$_shell_issuer'"
fi

# Both halves must always travel together. A script that passes only one
# of the two flags is the original bug in a new location.
_verify_flags() {
    grep -cE -- '--certificate-identity-regexp=[^ ]+ --certificate-oidc-issuer=' "$1" || true
}
if [ "$(_verify_flags "$INSTALL_SH")" -ge 1 ]; then
    ok "install.sh pins identity and issuer as a pair (regexp form)"
else
    fail "install.sh does not pass --certificate-identity-regexp and --certificate-oidc-issuer together"
fi

# Fixed-string grep, not -E: the format! macro is full of regex metacharacters
# and the escaping needed to survive them is exactly where this went wrong
# before. -F cannot misread the pattern.
# Match the inner format! argument only, not the whole statement. The
# real line is `cmd.arg(format!("--certificate-identity={identity}"));` —
# including `cmd.arg(` or the trailing `))` makes the pattern miss on a
# correct implementation, which is how this check produced a false FAIL.
if grep -qF 'format!("--certificate-identity-regexp={identity}")' "$UPDATE_RS" \
   && grep -qF 'format!("--certificate-oidc-issuer={issuer}")' "$UPDATE_RS"; then
    ok "sddk dev update pins identity and issuer as a pair (regexp form)"
else
    fail "sddk dev update does not pin both certificate halves"
fi

# The unsigned opt-in must be reachable from the CLI, not only from an
# environment variable, so an operator can see it in --help. A var-only
# opt-in is invisible in the interface and gets "fixed" by exporting the
# var in a shell profile.
if grep -qF 'pub(super) allow_unsigned: bool' "$DEV_MOD_RS"; then
    ok "sddk dev update exposes an explicit --allow-unsigned flag"
else
    fail "the unsigned opt-in is env-var only; there is no discoverable flag"
fi

# Both rejection AND acceptance need coverage. Testing only the rejection
# leaves the accepting branch unexercised, which is exactly where a typo
# silently ships a dead branch.
if grep -qF 'fn explicit_opt_in_accepts_an_unsigned_bundle()' "$UPDATE_RS"; then
    ok "the accepting branch of the unsigned policy is covered by a test"
else
    fail "only the rejection path is tested; the opt-in branch is unexercised"
fi

# Same for the Rust path: cosign failure must be a bail, never a warning.
# `bail!` opens BEFORE the message it prints, so anchor on the status
# check and require a bail inside the same brace-delimited block. Anchoring
# on the message text instead would miss the bail that guards it.
if awk '
    # The two-anchor flow replaced `if !output.status.success() { bail! }`
    # with: try anchor 1, try anchor 2, and bail only when BOTH fail. The
    # control to pin is therefore that specific bail, not "a bail somewhere
    # nearby".
    #
    # Three earlier attempts at this check all passed for the wrong reason,
    # and each failure is worth naming because they are the same mistake:
    #
    #   1. Anchored on `if !output.status.success() {` — a shape that no
    #      longer exists, so the guard was green because it never fired.
    #   2. Unscoped `/anyhow::bail!/` — matched 17 bails in the file, so
    #      deleting the guarded one left it green.
    #   3. Scoped to the function — but the function has three bails, so
    #      deleting the guarded one still left it green.
    #
    # So: scoped to the function AND matched on the message the bail prints.
    # The bail is multi-line, so the message lives on the following lines.
    /fn verify_bundle_signature\(/ { infn=1; next }
    infn && /^}/ { infn=0 }
    infn && /anyhow::bail!\(/ {
        # Read the continuation lines of this macro call.
        if ((getline m1) > 0 && (getline m2) > 0 && (getline m3) > 0) {
            if (m1 ~ /cosign verification FAILED/ ||
                m2 ~ /cosign verification FAILED/ ||
                m3 ~ /cosign verification FAILED/) { found = 1 }
        }
    }
    END { exit(found ? 0 : 1) }
' "$UPDATE_RS" 2>/dev/null; then
    ok "sddk dev update bails on a failed signature verification"
else
    fail "sddk dev update does not bail on a bad signature"
fi

# --- Key-based anchor: the three copies must agree -----------------------
#
# The anchor is the project's public signing key. It exists in three places:
# the Rust constant (via include_str!), the shell constant in install.sh, and
# the file on disk. A divergence means the CLI trusts one signer and the
# installer another, which is a hole neither would report.

ANCHOR_FILE="$ROOT/assets/trust/release-verify-key.pub"
if [ ! -f "$ANCHOR_FILE" ]; then
    fail "the key-based anchor file is missing: $ANCHOR_FILE"
else
    _anchor_file_body=$(tr -d '\n' < "$ANCHOR_FILE" | sed 's/[[:space:]]*$//')
    _anchor_shell_body=$(sed -n 's/^SDDK_RELEASE_VERIFY_KEY_BODY="\(.*\)"$/\1/p' "$INSTALL_SH" | head -1)

    if [ -z "$_anchor_file_body" ]; then
        # The empty-equals-empty case: a missing or blank anchor yields "" on
        # both sides and a naive comparison PASSES. This is the failure the
        # flat-body design exists to prevent, so it is checked explicitly.
        fail "the anchor file is empty; an empty anchor makes --key point at nothing"
    elif [ -n "$_anchor_shell_body" ] && [ "$_anchor_file_body" = "$_anchor_shell_body" ]; then
        ok "install.sh and the anchor file carry the same key body"
    else
        fail "key-based anchor drift: file='$_anchor_file_body' install.sh='$_anchor_shell_body'"
    fi

    if grep -qF 'assets/trust/release-verify-key.pub' "$COSIGN_RS"; then
        ok "cosign.rs embeds the anchor from the same file"
    else
        fail "cosign.rs no longer reads the anchor file; the constant may have been inlined"
    fi

    # Placeholder state must be COHERENT, not permanently a placeholder: the
    # anchor is legitimately unprovisioned today and legitimately provisioned
    # after the KMS key exists. What is never acceptable is one copy carrying
    # the placeholder while the other carries a real key — that is the state
    # where install.sh refuses and the CLI does not, or vice versa, and
    # neither reports it.
    #
    # Only the ASSIGNMENT line is read. The marker also appears in
    # install.sh's runtime `case` guard, which must keep matching the
    # placeholder forever — that is what refuses a regressed anchor — so
    # grepping the whole file would report "still unprovisioned" after the key
    # was provisioned, and this check would fail the legitimate state.
    _install_sh_has_marker=0
    _anchor_file_has_marker=0
    grep -m1 '^SDDK_RELEASE_VERIFY_KEY_BODY="' "$INSTALL_SH" \
        | grep -qF '@@SDDK_TRANSITION_ANCHOR_NOT_A_REAL_KEY@@' && _install_sh_has_marker=1
    grep -qF '@@SDDK_TRANSITION_ANCHOR_NOT_A_REAL_KEY@@' "$ANCHOR_FILE" && _anchor_file_has_marker=1

    if [ "$_install_sh_has_marker" -eq "$_anchor_file_has_marker" ]; then
        if [ "$_install_sh_has_marker" -eq 1 ]; then
            ok "the unprovisioned-anchor placeholder is present in both copies and refused"
        else
            ok "the anchor is provisioned in both copies (post-provisioning state)"
        fi
    else
        fail "placeholder state is incoherent: install.sh carries the marker=$_install_sh_has_marker, anchor file carries it=$_anchor_file_has_marker"
    fi

    # Each consumer must REBUILD the PEM. cosign rejects a bare body with
    # "PEM decoding failed" — the same error a malformed key gives, which is
    # what once made a negative control pass for the wrong reason.
    for _consumer in "$INSTALL_SH" "$COSIGN_RS"; do
        if grep -q 'BEGIN PUBLIC KEY' "$_consumer"; then
            ok "$(basename "$_consumer") rebuilds the PEM framing cosign requires"
        else
            fail "$(basename "$_consumer") does not rebuild the PEM framing"
        fi
    done
fi

# Both anchors must be present in both consumers. Dropping the legacy branch
# breaks installing v2.5.2; dropping the key-based branch silently reopens the
# path the migration exists to close.
for _consumer in "$INSTALL_SH" "$UPDATE_RS"; do
    if grep -q -- '--key' "$_consumer" && grep -q -- '--certificate-identity-regexp' "$_consumer"; then
        ok "$(basename "$_consumer") verifies against both anchors"
    else
        fail "$(basename "$_consumer") is missing one of the two anchors"
    fi
done

# --- Signing identity: key-based anchor (ADR-0151) ---------------------
#
# The assertions removed from here pinned the GitHub Actions OIDC issuer and
# checked that release.sh read the issuer back out of the certificate it
# minted. Both described the keyless anchor, which is no longer the signing
# path — so they were replaced rather than dropped, because the invariant
# they protected is still the one that matters: never publish a signature
# this project's own installers reject.
#
# A control that is not asserted here is a control that quietly stops
# controlling the first time someone refactors the block.

# 1. release.sh must VERIFY against the pinned anchor before publishing.
#    Reading a certificate issuer proved "signed by something"; verifying the
#    artifact against the same key the installers pin proves "signed by us".
#    Asserted as the call PLUS the die on its failure branch: a verify with
#    no refusal is a verify nobody reads.
if awk '
    /cosign verify-blob --key/ { sawverify=1 }
    sawverify && /does NOT verify against the pinned public key/ { diefound=1 }
    END { exit((sawverify && diefound) ? 0 : 1) }
' "$RELEASE_SH" 2>/dev/null; then
    ok "release.sh verifies the signed artifact against the pinned anchor and dies if it fails"
else
    fail "release.sh does not verify against the pinned anchor before publishing"
fi

# 2. The signer must be selectable without a hosted OIDC runner. Requiring
#    GITHUB_ACTIONS is exactly what made the v2.5.3 release impossible (it
#    aborted at step 8c). If this gate returns, the project is back to not
#    being able to ship — so its absence is asserted, not merely tolerated.
if grep -q 'GITHUB_ACTIONS' "$RELEASE_SH"; then
    fail "release.sh gates signing on GITHUB_ACTIONS again; no release can be produced without an Actions runner"
else
    ok "release.sh no longer requires a GitHub Actions runner to sign"
fi

# 3. SDDK_RELEASE_SIGNING_KEY selects the signer. With it unset the script
#    must die, not fall back to something that merely looks signed.
if awk '
    /signing_ref="\$\{SDDK_RELEASE_SIGNING_KEY:-0?\}"?|"\$\{SDDK_RELEASE_SIGNING_KEY:-\}"/ { sawgate=1 }
    sawgate && /die "no signing key configured/ { diefound=1 }
    END { exit(diefound ? 0 : 1) }
' "$RELEASE_SH" 2>/dev/null; then
    ok "release.sh dies when no signing key is configured"
else
    fail "release.sh does not fail closed when SDDK_RELEASE_SIGNING_KEY is unset"
fi

# 4. The unprovisioned-anchor guard. This is the one that stops a release
#    nobody can install from being published, and the anchor file IS still
#    the transition marker today, so this is live, not hypothetical.
if grep -q 'SDDK_TRANSITION_ANCHOR_NOT_A_REAL_KEY' "$RELEASE_SH"; then
    ok "release.sh refuses to publish while the anchor is the transition placeholder"
else
    fail "release.sh no longer refuses an unprovisioned anchor; a release nobody can install would ship"
fi

# 5. A private key FILE is the shape that breaks the premise of ADR-0151, so
#    it is refused by default. Asserted because the cheap path is the one
#    that gets taken by accident, and because "it works on my machine" is how
#    a long-lived signing key ends up in a backup.
if grep -q 'SDDK_ALLOW_FILE_SIGNER' "$RELEASE_SH"; then
    ok "release.sh refuses a private key file signer by default (ADR-0151)"
else
    fail "release.sh accepts a private key file signer without an explicit acknowledgement"
fi

# 6. Partial signing must stay fatal. A release whose bundle tarball has no
#    signature is one install.sh refuses, and the user finds out.
# shellcheck disable=SC2016  # literal de contrato en release.sh
if grep -qF 'SIGNED_COUNT" -ne "${#SIGN_ARTIFACTS[@]}"' "$RELEASE_SH"; then
    ok "release.sh refuses a partially signed release"
else
    fail "release.sh can publish a partially signed release"
fi

# The cosign that signs a release must be a declared decision, not a third
# party's default. `cosign-installer` pins the ACTION by SHA but leaves
# `cosign-release` at its own default (v2.4.3), so without this the version
# that produces a security control can change with nothing in the repo to
# show it. Every installer step must state the version.
missing_pins=$(python3 - "$RELEASE_YML" <<'PY'
import sys, yaml
try:
    doc = yaml.safe_load(open(sys.argv[1]))
except Exception as exc:
    print(f"unreadable workflow: {exc}")
    raise SystemExit(0)
unpinned = []
for jname, job in (doc.get("jobs") or {}).items():
    for step in job.get("steps") or []:
        uses = str(step.get("uses", ""))
        if "cosign-installer" in uses:
            ver = (step.get("with") or {}).get("cosign-release")
            if not ver:
                unpinned.append(f"{jname}/{step.get('name')}")
print(",".join(unpinned))
PY
)
if [ -z "$missing_pins" ]; then
    ok "every cosign-installer step pins cosign-release explicitly"
else
    fail "cosign version left to the action default in: $missing_pins"
fi

# --- Framework dir contract (INC-A5-FWDIR) -----------------------------
#
# install.sh documents SDDK_FRAMEWORK_DIR as the override (line 40) and the
# CI smoke job sets it to a non-default path. The installer must pass that
# dir to the CLI dev subcommands (dev use / dev doctor), and the CLI must
# honor it. The v2.2.12 smoke failure (run 36482350538) was exactly this
# passthrough missing: dev use resolved $SDDK_DATA_DIR/framework (= /tmp
# + "framework") and the install rolled back.
# shellcheck disable=SC2016  # literal del default en install.sh (contrato)
if grep -q 'FRAMEWORK_DIR="${SDDK_FRAMEWORK_DIR:-' "$INSTALL_SH"; then
    ok "install.sh keeps SDDK_FRAMEWORK_DIR as the documented override"
else
    fail "install.sh no longer honors the SDDK_FRAMEWORK_DIR override"
fi

# shellcheck disable=SC2016  # literales $FRAMEWORK_DIR de install.sh (contrato)
if grep -qE '^SDDK_FRAMEWORK_DIR="\$FRAMEWORK_DIR" .*dev use ' "$INSTALL_SH"; then
    ok "dev use receives the installer's framework dir through SDDK_FRAMEWORK_DIR"
else
    fail "dev use is not passed the installer framework dir (INC-A5-FWDIR regression)"
fi

# shellcheck disable=SC2016  # literales $FRAMEWORK_DIR de install.sh (contrato)
if grep -qE '^SDDK_FRAMEWORK_DIR="\$FRAMEWORK_DIR" .*dev doctor ' "$INSTALL_SH"; then
    ok "dev doctor receives the installer's framework dir through SDDK_FRAMEWORK_DIR"
else
    fail "dev doctor is not passed the installer framework dir"
fi

# The old derivation ($(dirname FRAMEWORK_DIR) as SDDK_DATA_DIR) only worked
# when the framework dir was literally <data-root>/framework; pin its removal
# so the broken shape cannot come back silently.
if grep -q 'SDDK_DATA_DIR_DATA_ROOT' "$INSTALL_SH"; then
    fail "stale data-root derivation is back in install.sh (INC-A5-FWDIR)"
else
    ok "no stale data-root derivation in the symlink/doctor stages"
fi

echo
if [ "$failures" -ne 0 ]; then
    echo "install asset contract: $failures check(s) FAILED"
    exit 1
fi
echo "install asset contract: all checks passed"
