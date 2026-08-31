//! `dev verify` — verify an installed prefix against its receipt.

use crate::dev::common::{failure_status, read_receipt, receipt_text};
use crate::dev::manifest::verify_manifest;
use crate::{CommandOutput, OutputFormat};
use sha2::{Digest, Sha256};

pub(super) fn run_dev_verify(args: super::VerifyArgs) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<super::InstallReceipt> {
        let receipt = read_receipt(&args.prefix)?;
        let binary_path = args.prefix.join(&receipt.binary_path);
        let bytes = std::fs::read(&binary_path)?;
        let digest = format!("sha256:{:x}", Sha256::digest(&bytes));
        if digest != receipt.binary_sha256 {
            anyhow::bail!(
                "binary digest mismatch: receipt {}, found {}",
                receipt.binary_sha256,
                digest
            );
        }
        // When the receipt indicates a full bundle install, verify the installed
        // surfaces against the manifest (fail-closed on any mismatch).
        if receipt.bundle {
            let mismatches = verify_manifest(&args.prefix)?;
            if !mismatches.is_empty() {
                anyhow::bail!(
                    "manifest verification FAILED ({} mismatch(es)):\n  {}",
                    mismatches.len(),
                    mismatches.join("\n  ")
                );
            }
        }
        Ok(receipt)
    })();
    match result {
        Ok(receipt) => match format {
            OutputFormat::Json => {
                let mut value = serde_json::to_value(&receipt).unwrap_or(serde_json::Value::Null);
                if let serde_json::Value::Object(map) = &mut value {
                    map.insert("valid".into(), serde_json::Value::Bool(true));
                }
                CommandOutput {
                    stdout: format!(
                        "{}\n",
                        serde_json::to_string_pretty(&value).unwrap_or_default()
                    ),
                    ..CommandOutput::default()
                }
            }
            OutputFormat::Text => CommandOutput {
                stdout: format!("valid: true\n{}", receipt_text(&receipt)),
                ..CommandOutput::default()
            },
        },
        Err(error) => failure_status(error.to_string()),
    }
}
