//! `dev install` — atomic binary prefix installation with receipt.

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
        if let Some(source) = &args.source {
            let source = std::fs::canonicalize(source)?;
            for surface in MANIFEST_SURFACES {
                let src_dir = source.join(surface);
                if !src_dir.is_dir() {
                    continue;
                }
                copy_tree(&src_dir, &args.prefix.join(surface), CopyMode::IfChanged)?;
            }
            // Also copy the MANIFEST.sha256 itself to the prefix so `dev verify`
            // can re-check installed surfaces against it.
            let manifest_src = source.join(MANIFEST_FILE);
            if manifest_src.is_file() {
                let manifest_dest = args.prefix.join(MANIFEST_FILE);
                std::fs::copy(&manifest_src, &manifest_dest)?;
            }
        }

        // Parse tag from --release-receipt JSON when provided.
        // This is the mechanism by which `release plan` propagates the planned
        // tag into the install receipt for downstream byte-stability verification.
        let tag_from_receipt = args.release_receipt.as_ref().and_then(|path| {
            let bytes = std::fs::read(path).ok()?;
            let json: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
            json.get("tag")?.as_str().map(String::from)
        });

        let receipt = super::InstallReceipt {
            version: env!("CARGO_PKG_VERSION").to_owned(),
            commit: args
                .commit
                .or_else(|| std::env::var("GITHUB_SHA").ok())
                .unwrap_or_else(default_timestamp),
            binary_sha256: digest,
            channel: args.channel.clone(),
            installed_at: args.timestamp.unwrap_or_else(default_timestamp),
            binary_path,
            bundle: args.source.is_some(),
            tag: tag_from_receipt,
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
