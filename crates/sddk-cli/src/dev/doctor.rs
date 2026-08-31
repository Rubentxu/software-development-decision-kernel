//! `dev doctor` — toolchain and environment prerequisite checker.

use crate::dev::common::{read_receipt, tool_version};
use crate::dev::manifest::verify_manifest;
use crate::dev::paths::resolve_active_framework_root;
use crate::{CliEnvironment, CommandOutput, render_result};
use std::path::{Path, PathBuf};

// ── Private helpers (used only by doctor) ───────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "snake_case")]
struct DoctorOutput {
    checks: Vec<DoctorCheck>,
    all_present: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "snake_case")]
struct DoctorCheck {
    tool: String,
    present: bool,
}

// `dead_code` allow: retained as API surface for future detailed checks;
/// tracked for cleanup in phase2-hygiene-baseline.
#[allow(dead_code)]
struct FrameworkCheck {
    name: String,
    status: String,
    detail: String,
}

fn check_framework(root: &Path, editor_dir: &Path) -> Vec<FrameworkCheck> {
    let mut checks = Vec::new();

    // Broken symlinks in editor agents.
    let agents_dir = editor_dir.join("agents");
    if let Ok(entries) = std::fs::read_dir(&agents_dir) {
        let broken: Vec<String> = entries
            .flatten()
            .filter(|entry| {
                entry
                    .path()
                    .symlink_metadata()
                    .map(|m| m.file_type().is_symlink())
                    .unwrap_or(false)
                    && !entry.path().exists()
            })
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        checks.push(FrameworkCheck {
            name: "broken_agent_links".into(),
            status: if broken.is_empty() { "PASS" } else { "WARN" }.into(),
            detail: if broken.is_empty() {
                "no broken agent symlinks".into()
            } else {
                format!("broken: {}", broken.join(", "))
            },
        });
    }

    // Stale copies: regular files where a symlink is expected AND the repo has
    // a matching asset (local-only agents are legitimate, not stale).
    let mut stale: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&agents_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(metadata) = std::fs::symlink_metadata(&path)
                && metadata.file_type().is_file()
                && path.extension().and_then(|e| e.to_str()) == Some("md")
                && root.join("agents").join(entry.file_name()).exists()
            {
                stale.push(entry.file_name().to_string_lossy().into_owned());
            }
        }
    }
    checks.push(FrameworkCheck {
        name: "stale_agent_copies".into(),
        status: if stale.is_empty() { "PASS" } else { "WARN" }.into(),
        detail: if stale.is_empty() {
            "all agents are symlinks".into()
        } else {
            format!("stale copies (run dev link): {}", stale.join(", "))
        },
    });

    // Workflow origin: editor workflows must be symlinks to repo.
    let workflows_dir = editor_dir.join("workflows");
    let mut orphan_workflows = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&workflows_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e == "yaml")
                && path
                    .symlink_metadata()
                    .map(|m| m.file_type().is_file())
                    .unwrap_or(false)
            {
                orphan_workflows.push(entry.file_name().to_string_lossy().into_owned());
            }
        }
    }
    checks.push(FrameworkCheck {
        name: "workflow_origin".into(),
        status: if orphan_workflows.is_empty() {
            "PASS"
        } else {
            "WARN"
        }
        .into(),
        detail: if orphan_workflows.is_empty() {
            "workflows are linked from repo".into()
        } else {
            format!(
                "orphan copies (run dev link): {}",
                orphan_workflows.join(", ")
            )
        },
    });

    let _ = root;
    checks
}

fn doctor_text(output: &DoctorOutput) -> String {
    let mut text = String::new();
    for check in &output.checks {
        text.push_str(&format!(
            "{}: {}\n",
            check.tool,
            if check.present { "present" } else { "missing" }
        ));
    }
    text.push_str(&format!("all_present: {}\n", output.all_present));
    text
}

// ── Public subcommand ─────────────────────────────────────────────────────────

pub(super) fn run_dev_doctor(
    args: super::DoctorArgs,
    environment: &CliEnvironment,
) -> CommandOutput {
    let format = args.format;
    let mut checks = Vec::new();
    for tool in ["cargo", "rustc", "git", "gh"] {
        let present = tool_version(tool).is_ok();
        checks.push(DoctorCheck {
            tool: tool.to_owned(),
            present,
        });
    }
    // Framework asset integrity checks for detected editors.
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let opencode_dir = home.join(".config/opencode");
    let zcode_dir = home.join(".zcode");
    let mut framework_warnings = 0usize;
    for (label, editor_dir) in [("opencode", opencode_dir), ("zcode", zcode_dir)] {
        if !editor_dir.is_dir() {
            continue;
        }
        for check in check_framework(&root, &editor_dir) {
            if check.status != "PASS" {
                framework_warnings += 1;
            }
            checks.push(DoctorCheck {
                tool: format!("{label}.{}", check.name),
                present: check.status == "PASS",
            });
        }
    }
    // Advisory, non-blocking: report presence of the claude/codex editor dirs
    // (native agent registration targets). Absence is informational — it does
    // not affect `all_present`.
    for (label, editor_dir) in [
        ("claude", home.join(".claude")),
        ("codex", home.join(".codex")),
    ] {
        checks.push(DoctorCheck {
            tool: format!("editor.{label}_dir"),
            present: editor_dir.is_dir(),
        });
    }
    // Runtime assets integrity: the CLI resolves dashboard kit + UAT drivers
    // from the active framework bundle (ADR-013). A dev update without asset
    // sync leaves stale/missing assets that break `uat dashboard` and
    // `uat run --executor playwright|computer_use` at runtime.
    if let Ok(framework_root) = resolve_active_framework_root(environment) {
        let assets = framework_root.join("assets");
        let driver_ok = assets.join("uat-driver/driver.mjs").is_file()
            && assets.join("uat-driver/computer_use.mjs").is_file()
            && assets.join("uat-driver/assess.mjs").is_file();
        let kit_ok = assets.join("uat-dashboard/kit/components.js").is_file()
            && assets.join("uat-dashboard/views/guided.html").is_file();
        checks.push(DoctorCheck {
            tool: "assets.uat-driver".into(),
            present: driver_ok,
        });
        checks.push(DoctorCheck {
            tool: "assets.uat-dashboard-kit".into(),
            present: kit_ok,
        });
        if !driver_ok || !kit_ok {
            framework_warnings += 1;
        }
        // Content integrity: verify the active framework root against its
        // MANIFEST.sha256 (per-file hashes of agents/skills/prompts/
        // workflows/assets — the same manifest shipped with the release).
        // A missing manifest is informational (pre-manifest bundles), not a
        // failure; a present-but-mismatched manifest is a real problem.
        let manifest_status = verify_manifest(&framework_root);
        let (manifest_present, manifest_ok) = match &manifest_status {
            Ok(mismatches) => (true, mismatches.is_empty()),
            Err(_) => (false, true),
        };
        checks.push(DoctorCheck {
            tool: "content.manifest".into(),
            present: manifest_ok,
        });
        if manifest_present && !manifest_ok {
            framework_warnings += 1;
        }
        // Binary/bundle version coherence (INC-DEBT-005): a stale `sddk`
        // binary on PATH running against a newer (or older) bundle produced
        // mixed-version sessions and legacy receipt rows. Compare this
        // binary's compile-time version with the active bundle's receipt.
        // No receipt (dogfooding `path:` override, pre-receipt bundles) is
        // informational — no check is emitted.
        if let Ok(receipt) = read_receipt(&framework_root) {
            let coherent = receipt.version == env!("CARGO_PKG_VERSION");
            checks.push(DoctorCheck {
                tool: "binary.bundle_coherence".into(),
                present: coherent,
            });
            if !coherent {
                framework_warnings += 1;
            }
        }
    }

    // Surface brevity checks (ADR-016): agent ≤ 300, skill ≤ 150, prompt ≤ 200.
    let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut brevity_violations = 0usize;

    // Agents: agents/*.md
    if let Ok(entries) = std::fs::read_dir(root.join("agents")) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("md")
                && let Ok(content) = std::fs::read_to_string(&path)
            {
                let line_count = content.lines().count();
                let rel = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");
                let present = line_count <= 300;
                if !present {
                    brevity_violations += 1;
                }
                checks.push(DoctorCheck {
                    tool: format!("surface.briefness.{rel}"),
                    present,
                });
            }
        }
    }

    // Skills: skills/*/SKILL.md
    if let Ok(entries) = std::fs::read_dir(root.join("skills")) {
        for entry in entries.flatten() {
            let skill_dir = entry.path();
            if skill_dir.is_dir() {
                let skill_name = skill_dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");
                let skl_path = skill_dir.join("SKILL.md");
                if skl_path.is_file()
                    && let Ok(content) = std::fs::read_to_string(&skl_path)
                {
                    let line_count = content.lines().count();
                    let present = line_count <= 150;
                    if !present {
                        brevity_violations += 1;
                    }
                    checks.push(DoctorCheck {
                        tool: format!("surface.briefness.{skill_name}/SKILL.md"),
                        present,
                    });
                }
            }
        }
    }

    // Prompts: prompts/sddk/*.md
    let prompts_dir = root.join("prompts/sddk");
    if prompts_dir.is_dir()
        && let Ok(entries) = std::fs::read_dir(&prompts_dir)
    {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && path.extension().and_then(|e| e.to_str()) == Some("md")
                && let Ok(content) = std::fs::read_to_string(&path)
            {
                let line_count = content.lines().count();
                let rel = path
                    .strip_prefix(&prompts_dir)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                let present = line_count <= 200;
                if !present {
                    brevity_violations += 1;
                }
                checks.push(DoctorCheck {
                    tool: format!("surface.briefness.{rel}"),
                    present,
                });
            }
        }
    }

    // Surface empty-dirs check (ADR-016): no empty subdirectories in surfaces.
    for surface_dir in ["agents", "skills", "prompts/sddk"] {
        let dir_path = root.join(surface_dir);
        if dir_path.is_dir()
            && let Ok(entries) = std::fs::read_dir(&dir_path)
        {
            for entry in entries.flatten() {
                let path = entry.path();
                // Only check directories (not files).
                if path.is_dir() {
                    let rel = path
                        .strip_prefix(&root)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .replace('\\', "/");
                    // Empty if no files (recursive check).
                    let is_empty = path
                        .read_dir()
                        .map(|mut i| i.next().is_none())
                        .unwrap_or(false);
                    let present = !is_empty;
                    checks.push(DoctorCheck {
                        tool: format!("surface.empty_dirs.{rel}"),
                        present,
                    });
                }
            }
        }
    }

    // `all_present` reflects only non-brevity checks (framework layout).
    // Brevity violations are tracked separately via `brevity_violations` and
    // only affect the exit code in strict mode (ADR-016 §4).
    let all_present = framework_warnings == 0;
    let result = Ok::<_, anyhow::Error>(DoctorOutput {
        all_present,
        checks,
    });
    match result {
        Ok(output) => {
            let cloned = DoctorOutput {
                all_present: output.all_present,
                checks: output.checks.clone(),
            };
            let mut command = render_result(Ok(cloned), format, doctor_text);
            // Strict mode: only brevity violations trigger non-zero exit (ADR-016 §4).
            // surface.empty_dirs is detect-only advisory — never promoted by --strict.
            if args.strict && brevity_violations > 0 {
                command.status = 1;
            } else if !output.all_present {
                // Advisory: non-brevity layout issues are fatal.
                command.status = 1;
            }
            command
        }
        Err(error) => crate::failure_envelope(&error),
    }
}
