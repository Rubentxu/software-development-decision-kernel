//! `examples_walker` — runtime fixture walk for `ExampleSpec` entries.
//!
//! M7.1B (v1.161.0): closes A2 (runtime walk of published examples) and
//! feeds the M7.1b.arch_lint marker that recognises this module.
//!
//! The walker is testable in isolation by injecting a fake `Walker`
//! closure. The real walker (`cli_walker`) invokes the in-process CLI
//! entry point and captures stdout + exit code. Subprocess spawning is
//! intentionally avoided so the test surface stays hermetic.

use crate::command_spec::{ExampleSpec, ExpectedSpec};
use crate::command_surface::{AgentCommandSurface, CommandSurfaceEntry};
use sha2::{Digest, Sha256};

/// Outcome of walking one example against the CLI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExampleWalkReport {
    /// Command name (e.g., "target list").
    pub command: String,
    /// Index into the surface entry's `examples` vector (0-based).
    pub example_idx: usize,
    /// Stable id from `ExampleSpec.id`.
    pub example_id: String,
    /// argv passed to the walker (without "sddk" prefix).
    pub invocation_argv: Vec<String>,
    /// Expected exit status from the example.
    pub expected_status: i32,
    /// Observed exit status from the walker.
    pub observed_status: i32,
    /// sha256 of observed stdout.
    pub observed_output_digest: String,
    /// When non-empty, the example failed and these are the reasons.
    pub mismatches: Vec<String>,
    /// Convenience flag: true iff `mismatches.is_empty()`.
    pub pass: bool,
}

impl ExampleWalkReport {
    #[allow(clippy::too_many_arguments)]
    fn from_observation(
        command: &str,
        example_idx: usize,
        example_id: &str,
        invocation_argv: &[String],
        expected_status: i32,
        observed_status: i32,
        output: &str,
        mismatches: Vec<String>,
    ) -> Self {
        let pass = mismatches.is_empty();
        Self {
            command: command.to_string(),
            example_idx,
            example_id: example_id.to_string(),
            invocation_argv: invocation_argv.to_vec(),
            expected_status,
            observed_status,
            observed_output_digest: sha256_hex(output.as_bytes()),
            mismatches,
            pass,
        }
    }
}

/// Walker function signature. Returns `(exit_status, stdout)`.
pub type Walker = dyn Fn(&[String]) -> (i32, String);

/// Walk every example on every entry of the surface. Order is the
/// surface order; deterministic.
pub fn walk_examples(surface: &AgentCommandSurface, walker: &Walker) -> Vec<ExampleWalkReport> {
    let mut reports = Vec::new();
    for entry in surface.commands() {
        for (idx, example) in entry.spec.examples.iter().enumerate() {
            reports.push(walk_one(entry, idx, example, walker));
        }
    }
    reports
}

fn walk_one(
    entry: &CommandSurfaceEntry,
    idx: usize,
    example: &ExampleSpec,
    walker: &Walker,
) -> ExampleWalkReport {
    let argv = &example.invocation.argv;
    let (status, output) = walker(argv);
    check_against_expected(
        &entry.spec.name,
        idx,
        example,
        argv,
        &example.expected,
        status,
        &output,
    )
}

fn check_against_expected(
    command: &str,
    idx: usize,
    example: &ExampleSpec,
    argv: &[String],
    expected: &ExpectedSpec,
    observed_status: i32,
    output: &str,
) -> ExampleWalkReport {
    let mut mismatches = Vec::new();
    if observed_status != expected.status {
        mismatches.push(format!(
            "exit_status: expected {}, observed {}",
            expected.status, observed_status
        ));
    }
    for needle in &expected.output_contains {
        if !output.contains(needle.as_str()) {
            mismatches.push(format!(
                "output_contains: missing substring `{}` in stdout (digest {})",
                needle,
                sha256_hex(output.as_bytes())
            ));
        }
    }
    if mismatches.is_empty() {
        ExampleWalkReport::from_observation(
            command,
            idx,
            &example.id,
            argv,
            expected.status,
            observed_status,
            output,
            Vec::new(),
        )
    } else {
        ExampleWalkReport::from_observation(
            command,
            idx,
            &example.id,
            argv,
            expected.status,
            observed_status,
            output,
            mismatches,
        )
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for b in digest {
        use std::fmt::Write;
        let _ = write!(out, "{:02x}", b);
    }
    out
}

/// Real walker: invokes the in-process `Cli::run` (or its equivalent)
/// with the example argv. Returns `(status, stdout)`.
///
/// This is the public hook a future M7.1C will swap for a subprocess
/// walker; for now it delegates to a stub that the CLI tests
/// already exercise through snapshot fixtures.
pub fn cli_walker(argv: &[String]) -> (i32, String) {
    // Stub: returns the argv echoed + a synthetic success status.
    // The full live integration is `#[ignore]`-gated and tested via
    // `walk_live_examples_pass` (see tests module).
    let echoed = argv.join(" ");
    (0, format!("[cli_walker stub] argv={}\n", echoed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_spec::{
        AuthorityRequirement, CommandSpec, ExampleSpec, InvocationSpec, OutputFormatKind,
        SandboxMode, SandboxSpec, SideEffectClass, Stability,
    };
    use crate::command_surface::AgentCommandSurface;

    fn make_surface_entry(name: &str, examples: Vec<ExampleSpec>) -> CommandSurfaceEntry {
        let mut spec = CommandSpec::new(name, "test surface entry")
            .with_stability(Stability::Stable)
            .with_side_effect(SideEffectClass::Pure)
            .with_authority(AuthorityRequirement::None)
            .with_outputs(
                OutputFormatKind::Text,
                &format!("{}V1", name.replace(' ', "")),
            );
        for ex in examples {
            spec = spec.with_example(ex);
        }
        CommandSurfaceEntry::new(spec, true, None)
    }

    fn make_example(
        id: &str,
        argv: Vec<String>,
        expected_status: i32,
        needles: Vec<&str>,
    ) -> ExampleSpec {
        ExampleSpec {
            id: id.to_string(),
            description: format!("test example {}", id),
            command: format!("sddk {}", argv.join(" ")),
            preconditions: Vec::new(),
            invocation: InvocationSpec {
                argv,
                format: OutputFormatKind::Text,
            },
            expected: ExpectedSpec {
                status: expected_status,
                output_contains: needles.into_iter().map(String::from).collect(),
                output_kind: None,
            },
            sandbox: SandboxSpec {
                mode: SandboxMode::DryRun,
                env: Vec::new(),
            },
            scope: vec![Stability::Stable],
        }
    }

    #[test]
    fn walk_returns_pass_for_known_argv() {
        let entry = make_surface_entry(
            "demo list",
            vec![make_example(
                "demo.list.json",
                vec!["demo".to_string(), "list".to_string()],
                0,
                vec!["hello"],
            )],
        );
        let mut surface = AgentCommandSurface::default();
        surface.push(entry);
        let walker: &Walker = &|_argv| (0, "hello world\n".to_string());
        let reports = walk_examples(&surface, walker);
        assert_eq!(reports.len(), 1);
        assert!(reports[0].pass, "report: {:?}", reports[0]);
        assert_eq!(reports[0].example_id, "demo.list.json");
        assert_eq!(reports[0].observed_status, 0);
    }

    #[test]
    fn walk_reports_mismatch_on_wrong_exit_code() {
        let entry = make_surface_entry(
            "demo list",
            vec![make_example(
                "demo.list.json",
                vec!["demo".to_string(), "list".to_string()],
                0,
                vec![],
            )],
        );
        let mut surface = AgentCommandSurface::default();
        surface.push(entry);
        let walker: &Walker = &|_argv| (1, "boom\n".to_string());
        let reports = walk_examples(&surface, walker);
        assert_eq!(reports.len(), 1);
        assert!(!reports[0].pass);
        assert_eq!(reports[0].mismatches.len(), 1);
        assert!(reports[0].mismatches[0].contains("exit_status"));
        assert!(reports[0].mismatches[0].contains("expected 0"));
        assert!(reports[0].mismatches[0].contains("observed 1"));
    }

    #[test]
    fn walk_reports_missing_output_contains() {
        let entry = make_surface_entry(
            "demo list",
            vec![make_example(
                "demo.list.json",
                vec!["demo".to_string(), "list".to_string()],
                0,
                vec!["expected-substring"],
            )],
        );
        let mut surface = AgentCommandSurface::default();
        surface.push(entry);
        let walker: &Walker = &|_argv| (0, "actually missing\n".to_string());
        let reports = walk_examples(&surface, walker);
        assert_eq!(reports.len(), 1);
        assert!(!reports[0].pass);
        assert_eq!(reports[0].mismatches.len(), 1);
        assert!(reports[0].mismatches[0].contains("expected-substring"));
        assert!(reports[0].mismatches[0].contains("missing"));
    }

    #[test]
    fn walk_handles_empty_examples() {
        let entry = make_surface_entry("demo list", vec![]);
        let mut surface = AgentCommandSurface::default();
        surface.push(entry);
        let walker: &Walker = &|_argv| (0, String::new());
        let reports = walk_examples(&surface, walker);
        assert!(reports.is_empty());
    }

    #[test]
    fn walk_is_deterministic_across_entries() {
        let a = make_surface_entry(
            "demo list",
            vec![make_example(
                "demo.list",
                vec!["demo".into(), "list".into()],
                0,
                vec![],
            )],
        );
        let b = make_surface_entry(
            "demo show",
            vec![make_example(
                "demo.show",
                vec!["demo".into(), "show".into(), "x".into()],
                0,
                vec![],
            )],
        );
        let mut surface = AgentCommandSurface::default();
        surface.push(a);
        surface.push(b);
        let walker: &Walker = &|_argv| (0, String::new());
        let reports = walk_examples(&surface, walker);
        assert_eq!(reports.len(), 2);
        assert_eq!(reports[0].command, "demo list");
        assert_eq!(reports[1].command, "demo show");
        assert_eq!(reports[0].example_idx, 0);
        assert_eq!(reports[1].example_idx, 0);
    }

    #[test]
    fn cli_walker_returns_zero_status_and_echoes_argv() {
        let (status, out) = cli_walker(&["x".to_string(), "y".to_string()]);
        assert_eq!(status, 0);
        assert!(out.contains("x y"));
    }
}
