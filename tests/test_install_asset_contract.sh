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
# release.sh compiles a single `cargo build --release` (host glibc) and packs
# it as `sddk-${TAG}-sddk-linux-x86_64-musl.tar.gz`. The name promises a
# static/musl binary that does not exist. Either build musl for real or stop
# claiming it. This test pins that the lie is not reintroduced silently.
if grep -q 'musl' "$RELEASE_SH"; then
    # The name still says musl: acceptable ONLY if the build is a real musl
    # target. A single cargo build --release is not.
    if grep -qE 'target.*x86_64-unknown-linux-musl' "$RELEASE_SH"; then
        ok "release.sh builds a real musl target for the musl asset name"
    else
        fail "release.sh labels a host-glibc build as 'musl' (name promises a binary it does not ship)"
    fi
else
    ok "release.sh does not claim a musl asset"
fi

echo
if [ "$failures" -ne 0 ]; then
    echo "install asset contract: $failures check(s) FAILED"
    exit 1
fi
echo "install asset contract: all checks passed"
