# Release & Distribution

From v1.28.0 SDDK distributes **pre-compiled binaries** via GitHub Releases,
no cloning or compilation required. Users install with a one-liner
(`rustup` / `mise` model):

```bash
curl -fsSL https://raw.githubusercontent.com/Rubentxu/software-development-decision-kernel/main/scripts/install.sh | bash
```

The `scripts/install.sh` script:
- Detects platform (`uname -s/m`) → asset `sddk-linux-{x86_64,aarch64}-musl`
  (Linux: **musl static**, runs on any distro regardless of glibc)
- Prefers the **unified artifact** `sddk-<TAG>-<ASSET>.tar.gz` (binary +
  bundle in one archive, added in v1.63.0); falls back to the legacy
  split-asset path if not present
- Downloads the unified tarball, verifies its sha256, applies `chmod 0755`
  defensively on the extracted binary (defends against CDN-cached releases
  served without the exec bit), then stages an atomic install with rollback
- Generates `BUNDLE.toml` (schema v2) inline if the bundle lacks one, so the
  binary-vs-bundle compatibility check at `sddk dev doctor` succeeds
- Writes `sddk-install.json` with `schema_version=2`, `bundle_version`,
  `bundle_sha256`, `coherence_checked=true`
- Runs `sddk dev link --editor <X>` (symlinks bundle to editor dir)
- Prints `sddk dev doctor` (final verification)

**Supported platforms:**
- ✅ Linux x86_64 (musl static)
- ✅ Linux aarch64 (musl static)
- ⏳ macOS x86_64 + arm64 (pending: `cargo-zigbuild` toolchain already installed;
  need to generate binaries and upload to release)
- ⏳ Windows x86_64 (pending: requires `#[cfg(unix)]` carve-out in code using
  `std::os::unix::*`)

## Canonical release flow (v1.65.0+)

Since cycle-47 every release MUST go through the end-to-end pipeline
documented here, so that local install stays in lockstep with what ships
through GitHub Releases. The canonical entry point is `scripts/release.sh`,
which automates the 13 steps below.

```bash
# 1. Workspace green (gates commit, AGENTS.md §5)
cargo fmt --all -- --check
cargo clippy --workspace --offline --all-targets -- -D errors
cargo test --workspace --offline

# 2. Bump version (creates the chore(release) commit)
bash scripts/release-bump.sh    # or hand-edit Cargo.toml + manifest.toml + CHANGELOG.md

# 3. Two commits on main:
#      feat(uat): <description>
#      chore(release): bump version A.B.C -> X.Y.Z
git push origin main            # pre-push hook requires chore(release) present

# 4. Tag + push
git tag -a vX.Y.Z -m "vX.Y.Z - <title>"
git push origin vX.Y.Z

# 5. End-to-end: build → manifest → bundle → BUNDLE.toml → unified tarball →
#    gh release create → install from real URL → doctor → prune → final state
bash scripts/release.sh        # or --dry-run / --skip-tests / --skip-install / --force
```

`scripts/release.sh` is the single source of truth. If the script cannot
run in your environment (CI constraints, missing `gh` auth, etc.) the
manual equivalent is documented in `AGENTS.md §8` and reproduced below in
the section "Manual fallback". If you ship a release without going through
this pipeline, open a follow-up cycle to retrofit the missing steps.

### Why this shape

- **Single script, single command.** Cycle-46/47 standardized on one entry
  point so a partial release (binary but no bundle, release but no install)
  is impossible to do by accident.
- **`gh release create` is called with all assets in one shot.** The unified
  tarball + BUNDLE.toml pair is the contract that lets `install.sh` run
  with no surprises; legacy split-asset uploads are a fallback for
  back-compat with old `install.sh` versions.
- **Install from the real GitHub URL, never from a local file:// build.**
  This catches CDN-cached assets (where `curl /releases/download/...` may
  serve the previous release for up to ~5 minutes after `--clobber`), as
  well as URL typos and bundle/binary mismatches that a local install
  would silently hide. The script polls the binary sha256 against the CDN
  until it matches what was uploaded before calling `install.sh`.
- **`sddk dev doctor --prefix $PREFIX` is the green light.** The
  `binary.bundle_coherence: present` and `all_present: true` checks
  together prove that the installed binary, the installed bundle, the
  receipt's `bundle_version`, and the active `current` symlink all agree.
- **`sddk dev update --prune-only --keep 1` clears stale version dirs.**
  Post-install, the framework directory may carry 15+ old `1.X.Y/` dirs
  from prior installs; the prune keeps current + top-1 most recent and
  drops the rest.
- **Two commits: `feat` + `chore(rerelease)`.** The pre-push hook
  (`githooks/pre-push`) refuses any push to `main` without a
  `^chore\(release\): bump version` commit in the range. Merging the bump
  into the feature commit breaks the hook.

## Manual fallback

When `scripts/release.sh` cannot be used, run the steps inline. This is
the operational reference for the script.

```bash
REPO=Rubentxu/software-development-decision-kernel
TAG=v$(awk '/^\[workspace\.package\]/{f=1; next} f && /^version = /{gsub("\"",""); print $2; exit}' Cargo.toml)
TMP=$(mktemp -d)

# 1. Build
cargo build --release --offline --bin sddk
BIN=$(cargo metadata --format-version 1 --offline \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])' \
)/release/sddk
chmod 0755 "$BIN"

# 2. Manifest
$BIN dev manifest --root . --format text
$BIN dev manifest --verify --root . --format text

# 3. Bundle tarball (legacy split-asset path)
tar czf $TMP/software-development-decision-kernel.tar.gz \
    --xform 's|^|software-development-decision-kernel/|' \
    -C . agents skills prompts/sddk assets MANIFEST.sha256
sha256sum $TMP/software-development-decision-kernel.tar.gz \
    > $TMP/software-development-decision-kernel.tar.gz.sha256

# 4. BUNDLE.toml (schema v2) — extracted into the bundle
mkdir -p $TMP/bundle && tar xzf $TMP/software-development-decision-kernel.tar.gz -C $TMP/bundle
MANIFEST_SHA=$(awk 'NR==1 {print $1}' MANIFEST.sha256)
printf '%s\n' '[bundle]' 'schema_version = 2' \
    "version = \"${TAG#v}\"" \
    "binary_min_version = \"${TAG#v}\"" \
    "binary_max_version = \"${TAG#v}\"" \
    '' '[contents]' "manifest_sha256 = \"$MANIFEST_SHA\"" \
    > $TMP/bundle/software-development-decision-kernel/BUNDLE.toml

# 5. Unified tarball (preferred path since v1.63.0)
mkdir -p $TMP/pack/bin $TMP/pack/framework
cp "$BIN" $TMP/pack/bin/sddk
cp -r $TMP/bundle/software-development-decision-kernel $TMP/pack/framework
tar -C $TMP/pack -czf $TMP/sddk-${TAG}-sddk-linux-x86_64-musl.tar.gz bin framework
sha256sum $TMP/sddk-${TAG}-sddk-linux-x86_64-musl.tar.gz \
    > $TMP/sddk-${TAG}-sddk-linux-x86_64-musl.tar.gz.sha256

# 6. Checksums + sbom
echo "$(sha256sum $BIN | awk '{print $1}')  $(basename $BIN)" > $TMP/$(basename $BIN).sha256
( cd $TMP && sha256sum sddk-${TAG}-sddk-linux-x86_64-musl.tar.gz software-development-decision-kernel.tar.gz ) > $TMP/CHECKSUMS
cat > $TMP/sbom.json <<EOF
{"bomFormat":"CycloneDX","specVersion":"1.5","version":1,"components":[{"type":"application","name":"sddk","version":"${TAG#v}","purl":"pkg:generic/sddk@${TAG#v}"}]}
EOF

# 7. Publish
gh release create "$TAG" --repo $REPO --title "sddk $TAG" \
    --notes "Release $TAG" \
    "$BIN" "$BIN.sha256" \
    $TMP/CHECKSUMS $TMP/sbom.json \
    $TMP/sddk-${TAG}-sddk-linux-x86_64-musl.tar.gz \
    $TMP/sddk-${TAG}-sddk-linux-x86_64-musl.tar.gz.sha256 \
    $TMP/software-development-decision-kernel.tar.gz \
    $TMP/software-development-decision-kernel.tar.gz.sha256

# 8. Install from real URL (wait out CDN cache)
sleep 60   # empirical: CDN caches for up to ~5 min; --clobber uploads don't invalidate
unset SDDK_BASE_URL SDDK_VERSION
SDDK_PREFIX=/home/rubentxu/.local/bin
SDDK_FRAMEWORK_DIR=/home/rubentxu/.local/share/sddk/framework
bash scripts/install.sh --version "$TAG" --editor all

# 9. Verify
$SDDK_PREFIX/sddk dev doctor --prefix $SDDK_PREFIX
$SDDK_PREFIX/sddk dev update --prune-only --keep 1 --root $SDDK_FRAMEWORK_DIR
```

## CI / local-first

`scripts/release.sh` runs locally. `.github/workflows/release.yml` is in
`workflow_dispatch` mode (manual only since 2026-08-10; see AGENTS.md §2.5).
The release gate IS the local pipeline — there is no CI gating step. The
pre-push hook (`githooks/pre-push`) enforces the `chore(release)` commit
rule; everything else is convention.

## MANIFEST regeneration

When `prompts/sddk/`, `skills/_shared/`, `agents/`, or `docs/` change, regenerate `MANIFEST.sha256` in the same commit:

```bash
sddk dev manifest
```

This prevents the "forced hygiene commit" pattern where manifest refresh becomes a separate release commit (cycle-9 had this: `ab54b8e chore(release): refresh MANIFEST.sha256`).

Before creating a release tag, verify the committed manifest from the repository root:

```bash
sddk dev manifest --verify --root .
```

## Preflight ordering (RDI)

Release distribution integrity (RDI) enforces that the MANIFEST exact-set is
verified **before** any push, tag, or GitHub Release creation. The ordering:

```
release plan
  └─ preflight_manifest(git.root(), false, "production release always verifies")
       └─ verify_manifest(&workspace_root)  ← bail if mismatches
  └─ version_lockstep_check(tag vs Cargo.toml)
  └─ write_release_receipt → {cycle_artifacts_dir}/release-receipt.json

release dist
  └─ preflight_manifest(staging_dir, skip, ...)
       └─ verify_manifest(&staging_dir)  ← bail before attestation write
  └─ embed {manifest_sha256, manifest_count, manifest_surfaces} in sbom + attestation
```

The `--skip-manifest-preflight` flag is an **escape hatch for dirty dev
workspaces**. When used, it writes an audit entry and bypasses verification:

```
release dist --skip-manifest-preflight
  → echo "[{RFC3339}] staged manifest verification SKIPPED: --skip-manifest-preflight"
```

Use this only in development, never in CI or production releases.

## Release receipt

After `release plan` completes, a signed receipt is written to:

```
{cycle_artifacts_dir}/release-receipt.json
```

The receipt is a JSON object signed with the local gate-signing key
(`$SDDK_DATA_DIR/keys/gate-signing.key`). The HMAC payload binds all receipt
fields:

```
receipt_id|gate|transition|plan_hash|head_sha|tag|binary_sha256|manifest_sha256|manifest_count|bundle_roundtrip_verified
```

Verify a receipt via:

```bash
# With JSON file (widened 10-field payload):
sddk release verify --prefix <dist-prefix> --receipt <path-to-release-receipt.json>

# With pipe-separated legacy format (preserved for backward compatibility):
sddk release verify --prefix <dist-prefix> --receipt "receipt_id|gate|transition|plan_hash|signature"
```

The JSON file path is detected when the `--receipt` argument contains `.json` or
path separators; otherwise it is parsed as a pipe-separated string.

## Version lockstep rule (workspace ↔ git tag)

Normative since the publication gap v1.36.1 → v1.49.0 (13 tags shipped as
tag-only — no GitHub Release artifacts — while `workspace.package.version`
lagged at 1.42.5, freezing `install.sh` and `sddk dev update` at v1.36.1):

- `workspace.package.version` (root `Cargo.toml`) MUST equal the released git
  tag `vX.Y.Z` at release time.
- The release-prep commit `chore(release): bump version A.B.C -> X.Y.Z` MUST be
  the last commit before the annotated tag, and the tag MUST point to that
  commit.
- Between releases the workspace version MAY drift ahead; it MUST NOT be
  tagged. Tags MUST NOT exist without a matching GitHub Release publication:
  tag-only releases are forbidden because `install.sh` and `sddk dev update`
  cannot see them (they silently rot — see ROADMAP §Publication release plan).
- `sddk dev doctor` MUST report `binary.bundle_coherence: present` before
  publishing.
- Automated enforcement in `sddk release plan/apply` (refuse to package on
  workspace/tag mismatch) is a tracked work item: ROADMAP §Publication release
  plan. Until it lands, this rule is enforced by release-phase review.
