//! `dev use` — asdf-style framework bundle version selector.

use crate::dev::paths::framework_dir;
use crate::{CliEnvironment, CommandOutput, render_result};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "snake_case")]
struct UseOutput {
    version: String,
    current: String,
}

fn use_text(output: &UseOutput) -> String {
    format!("version: {}\ncurrent: {}\n", output.version, output.current)
}

pub(super) fn run_dev_use(args: super::UseArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<UseOutput> {
        let dir = framework_dir(environment)?;
        std::fs::create_dir_all(&dir)?;
        let current = dir.join("current");
        if args.show {
            let active = match std::fs::read_link(&current) {
                Ok(target) => target
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| target.to_string_lossy().into_owned()),
                Err(_) => "none".to_owned(),
            };
            return Ok(UseOutput {
                version: active.clone(),
                current: active,
            });
        }
        // Resolve the target: `path:<dir>` points at a working tree
        // (dogfooding); otherwise a bundle version under framework/<version>/.
        let target = if let Some(path) = args
            .version
            .as_deref()
            .and_then(|version| version.strip_prefix("path:"))
        {
            std::fs::canonicalize(path)?
        } else {
            let version = args
                .version
                .clone()
                .ok_or_else(|| anyhow::anyhow!("--version is required unless --show"))?;
            let version_dir = dir.join(&version);
            if !version_dir.is_dir() {
                anyhow::bail!(
                    "bundle version {version} not installed; run `sddk dev update --root <dir> --version {version}`",
                    version = version
                );
            }
            version_dir
        };
        // Atomically swap the `current` symlink.
        super::swap_current_to(&dir, &target);
        Ok(UseOutput {
            version: args.version.unwrap_or_else(|| "current".to_owned()),
            current: target.to_string_lossy().into_owned(),
        })
    })();
    render_result(result, format, use_text)
}
