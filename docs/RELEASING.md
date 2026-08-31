# Release & Distribution

From v1.28.0 SDDK distributes **pre-compiled binaries** via GitHub Releases,
no cloning or compilation required. Users install with a one-liner
(`rustup` / `mise` model):

```bash
curl -fsSL https://raw.githubusercontent.com/Rubentxu/software-development-decision-kernel/main/scripts/install.sh | bash
```

The `scripts/install.sh` script (244 lines):
- Detects platform (`uname -s/m`) → asset `sddk-linux-{x86_64,aarch64}-musl`
  (Linux: **musl static**, runs on any distro regardless of glibc)
- Downloads binary + `sha256` from GitHub Releases
- Verifies SHA256 before installing (fails if mismatch)
- If `cosign` is available, verifies keyless signature (optional)
- Prompts which editor to configure (opencode/zcode/claude/codex or all)
- Downloads `software-development-decision-kernel.tar.gz` (bundle: `agents/`, `skills/`,
  `prompts/sddk/`, `assets/`, `MANIFEST.sha256`) and extracts it to
  `$SDDK_DATA_DIR/framework/<v>/`
- Runs `sddk dev link --editor <X>` (symlinks bundle to editor dir)
- Prints `sddk dev doctor` (final verification)

**Supported platforms in v1.28.0:**
- ✅ Linux x86_64 (musl static)
- ✅ Linux aarch64 (musl static)
- ⏳ macOS x86_64 + arm64 (pending: `cargo-zigbuild` toolchain already installed;
  need to generate binaries and upload to release)
- ⏳ Windows x86_64 (pending: requires `#[cfg(unix)]` carve-out in code using
  `std::os::unix::*` — see `crates/sddk-cli/src/dev_cmd.rs`)

**Local-first release (manual):** tag is pushed first (`git tag vX.Y.Z &&
git push origin vX.Y.Z`), then the binary is uploaded to GitHub Releases.
Workflow `.github/workflows/release.yml` is in `workflow_dispatch` manual mode
since 2026-08-10 (CI exhausted); today's operational path is:

```bash
# 1. Tag + push (local)
cargo build --release --target x86_64-unknown-linux-musl -p sddk-cli --locked
git tag vX.Y.Z && git push origin vX.Y.Z

# 2. Stage assets (Linux x86_64 + aarch64)
./target/x86_64-unknown-linux-musl/release/sddk release dist \
  --prefix dist-amd64 --channel release --commit "$(git rev-parse HEAD)"
cp dist-amd64/dist/sddk sddk-linux-x86_64-musl
cp dist-amd64/dist/{checksums.txt,sbom.json,attestation.json} sddk-linux-x86_64-musl.{CHECKSUMS,sbom.json,attestation.json}
sha256sum sddk-linux-x86_64-musl > sddk-linux-x86_64-musl.sha256
# (repeat for aarch64)

# 3. Framework bundle
tar czf software-development-decision-kernel.tar.gz agents skills prompts/sddk assets MANIFEST.sha256
sha256sum software-development-decision-kernel.tar.gz > software-development-decision-kernel.tar.gz.sha256

# 4. gh release create
gh release create vX.Y.Z --repo Rubentxu/software-development-decision-kernel \
  --target <commit> --title "vX.Y.Z" --notes "..." \
  sddk-linux-x86_64-musl sddk-linux-x86_64-musl.{sha256,CHECKSUMS,sbom.json,attestation.json} \
  sddk-linux-aarch64-musl sddk-linux-aarch64-musl.{sha256,CHECKSUMS,sbom.json,attestation.json} \
  software-development-decision-kernel.tar.gz software-development-decision-kernel.tar.gz.sha256
```

The E2E smoke test lives in `.github/workflows/release.yml:170-217` and runs
automatically when CI is available.

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
