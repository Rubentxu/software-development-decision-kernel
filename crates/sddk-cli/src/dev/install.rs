//! `dev install` — atomic binary prefix installation with receipt.
//!
//! When `--source` is provided, the source is treated as a full framework
//! bundle: `MANIFEST.sha256` is verified, `BUNDLE.toml` is parsed and its
//! binary compatibility range is checked against CARGO_PKG_VERSION, the
//! surfaces are copied, and the receipt records bundle metadata (v2 schema)
//! so `sddk dev doctor --coherence` can later validate binary↔bundle coherence
//! without re-parsing the source.

use crate::dev::bundle_manifest::{
    BUNDLE_MANIFEST_FILE, parse_bundle_manifest, verify_bundle_compat,
};
use crate::dev::common::{
    CopyMode, MANIFEST_SURFACES, RECEIPT_FILE, atomic_write, copy_tree, receipt_text,
};
use crate::dev::manifest::{MANIFEST_FILE, verify_manifest};
use crate::git_cmd::default_timestamp;
use crate::{CommandOutput, render_result};
use sha2::Digest;

pub(super) fn run_dev_install(args: super::InstallArgs) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<super::InstallReceipt> {
        // FAIL-CLOSED: when --source is provided, verify the source MANIFEST
        // BEFORE any writes to the prefix. A tampered source cannot corrupt an
        // existing installation.
        if let Some(source) = &args.source {
            let source = std::fs::canonicalize(source)?;
            let mismatches = verify_manifest(&source)?;
            if !mismatches.is_empty() {
                anyhow::bail!(
                    "manifest verification FAILED ({} mismatch(es)):\n  {}",
                    mismatches.len(),
                    mismatches.join("\n  ")
                );
            }
            // Bundle compatibility: refuse to install if the source's BUNDLE.toml
            // declares a binary range that excludes CARGO_PKG_VERSION. This is
            // the pre-write half of the binary↔bundle coherence contract.
            let bundle_toml = source.join(BUNDLE_MANIFEST_FILE);
            let manifest = parse_bundle_manifest(&bundle_toml).map_err(|e| {
                anyhow::anyhow!(
                    "BUNDLE.toml missing or invalid at {}: {}. \
                     A valid BUNDLE.toml is required for coherent installs (v2 schema). \
                     Run `sddk dev manifest --bundle` to (re)generate one.",
                    bundle_toml.display(),
                    e
                )
            })?;
            verify_bundle_compat(&manifest, env!("CARGO_PKG_VERSION")).map_err(|e| {
                anyhow::anyhow!(
                    "binary {} is not compatible with bundle {}: {}",
                    env!("CARGO_PKG_VERSION"),
                    manifest.bundle.version,
                    e
                )
            })?;
        }

        // NOW safe to write: compute binary digest after manifest verified.
        let binary = std::env::current_exe()?;
        let bytes = std::fs::read(&binary)?;
        let digest = format!("sha256:{:x}", sha2::Sha256::digest(&bytes));

        // Routing: if the prefix already terminates in `/bin`, install the
        // binary directly under the prefix (no extra `bin/` segment); this
        // matches the GNU autoconf/CMake convention of `--prefix=/opt/sdk/bin`
        // meaning "the binary directory". Otherwise nest under `bin/`.
        let ends_with_bin = args.prefix.file_name().and_then(|name| name.to_str()) == Some("bin");
        let target_dir = if ends_with_bin {
            args.prefix.clone()
        } else {
            args.prefix.join("bin")
        };
        std::fs::create_dir_all(&target_dir)?;
        let destination = target_dir.join("sddk");
        // Mode 0o755 BEFORE rename so the binary is born executable — fixes
        // the chmod-less atomic write that left ELF files at 0644.
        atomic_write(&destination, &bytes, Some(0o755))?;
        let binary_path = if ends_with_bin {
            "sddk".to_owned()
        } else {
            "bin/sddk".to_owned()
        };

        // Bundle surface copy: when --source is provided, copy surfaces AFTER
        // manifest verified. Binary-only (no --source) skips this block.
        let bundle_metadata: Option<(String, String, String)> = if let Some(source) = &args.source {
            let source = std::fs::canonicalize(source)?;
            for surface in MANIFEST_SURFACES {
                let src_dir = source.join(surface);
                if !src_dir.is_dir() {
                    continue;
                }
                copy_tree(&src_dir, &args.prefix.join(surface), CopyMode::IfChanged)?;
            }
            // Also copy the MANIFEST.sha256 and BUNDLE.toml themselves to the
            // prefix so `dev verify` and `dev doctor` can re-check installed
            // surfaces and bundle compatibility without network access.
            for rel in [MANIFEST_FILE, BUNDLE_MANIFEST_FILE] {
                let src = source.join(rel);
                if src.is_file() {
                    let dest = args.prefix.join(rel);
                    std::fs::copy(&src, &dest)?;
                }
            }
            // Compute bundle manifest hash for receipt (covers BUNDLE.toml).
            let bundle_toml = source.join(BUNDLE_MANIFEST_FILE);
            let bundle_hash = if bundle_toml.is_file() {
                let bundle_bytes = std::fs::read(&bundle_toml)?;
                Some(format!("sha256:{:x}", sha2::Sha256::digest(&bundle_bytes)))
            } else {
                None
            };
            // Parse bundle version from BUNDLE.toml (already validated above).
            let bundle_version = if bundle_toml.is_file() {
                parse_bundle_manifest(&bundle_toml)
                    .ok()
                    .map(|m| m.bundle.version)
            } else {
                None
            };
            // Bundle path relative to the framework root.
            let bundle_path = args
                .prefix
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_owned());
            match (bundle_version, bundle_hash, bundle_path) {
                (Some(v), Some(h), Some(p)) => Some((v, h, p)),
                _ => None,
            }
        } else {
            None
        };

        // Parse tag from --release-receipt JSON when provided.
        // This is the mechanism by which `release plan` propagates the planned
        // tag into the install receipt for downstream byte-stability verification.
        let tag_from_receipt = args.release_receipt.as_ref().and_then(|path| {
            let bytes = std::fs::read(path).ok()?;
            let json: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
            json.get("tag")?.as_str().map(String::from)
        });

        let (bundle_version, bundle_sha256, bundle_path) = match bundle_metadata {
            Some((v, h, p)) => (Some(v), Some(h), Some(p)),
            None => (None, None, None),
        };
        let has_bundle = args.source.is_some();

        let receipt = super::InstallReceipt {
            schema_version: if has_bundle { 2 } else { 1 },
            version: env!("CARGO_PKG_VERSION").to_owned(),
            commit: args
                .commit
                .or_else(|| std::env::var("GITHUB_SHA").ok())
                .unwrap_or_else(default_timestamp),
            binary_sha256: digest,
            channel: args.channel.clone(),
            installed_at: args.timestamp.unwrap_or_else(default_timestamp),
            binary_path,
            bundle: has_bundle,
            tag: tag_from_receipt,
            bundle_version,
            bundle_sha256,
            bundle_path,
            coherence_checked: if has_bundle { Some(true) } else { None },
        };
        let receipt_path = args.prefix.join(RECEIPT_FILE);
        atomic_write(
            &receipt_path,
            serde_json::to_string_pretty(&receipt)?.as_bytes(),
            None,
        )?;
        Ok(receipt)
    })();
    render_result(result, format, receipt_text)
}
