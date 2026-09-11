//! Golden fixtures for the CLI surface — snapshot regression tests.
//!
//! Captures deterministic `--help` output for a representative subset of
//! top-level commands and subcommands against the fixtures in
//! `docs/architecture/tests/fixtures/cli_golden/1.168.8/`.
//!
//! Contract:
//! - The fixtures are **regenerated intentionally** when the CLI surface
//!   legitimately evolves (new commands, new flags, alphabetical reordering).
//! - A diff that removes a command/flag or renames it without crosswalk
//!   documentation is a **regression** and must fail this test until the
//!   fixture is consciously regenerated.
//!
//! Follows the "acceptable diff triggers" policy documented in
//! `docs/architecture/tests/fixtures/cli_golden/1.151.3/README.md`.
//!
//! The fixtures are generated from the **installed** `sddk` binary (version
//! 1.168.8) using the same procedure documented in the fixture README. The
//! test binary here compares against `env!("CARGO_BIN_EXE_sddk")` which is
//! the just-compiled binary of the workspace — so the test enforces that the
//! compiled surface matches the last consciously-blessed snapshot.

use std::process::Command;

/// The fixture directory for the current snapshot.
const FIXTURE_DIR: &str = "docs/architecture/tests/fixtures/cli_golden/1.168.8";

/// The (command args, fixture filename) pairs under test.
const CASES: &[(&[&str], &str)] = &[
    (&[], "sddk-help.txt"),
    (&["cycle"], "sddk-cycle-help.txt"),
    (&["plan"], "sddk-plan-help.txt"),
    (&["run"], "sddk-run-help.txt"),
    (&["dev"], "sddk-dev-help.txt"),
    (&["dev", "graph"], "sddk-dev-graph-help.txt"),
    (&["dev", "cockpit"], "sddk-dev-cockpit-help.txt"),
    (&["dev", "lint"], "sddk-dev-lint-help.txt"),
    (&["ledger"], "sddk-ledger-help.txt"),
    (&["ledger", "watch"], "sddk-ledger-watch-help.txt"),
];

/// Runs `<bin> [args] --help` and normalizes output for comparison.
///
/// Normalizations:
/// - Trailing whitespace stripped per line (clap layout jitter).
/// - No other massaging — the fixture is a raw `--help` capture.
fn help_output(args: &[&str]) -> String {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sddk"));
    cmd.args(args).arg("--help");
    let out = cmd.output().expect("failed to spawn sddk binary");
    let text = if out.stdout.is_empty() {
        String::from_utf8_lossy(&out.stderr).to_string()
    } else {
        String::from_utf8_lossy(&out.stdout).to_string()
    };
    text.lines()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
}

fn fixture_path(name: &str) -> std::path::PathBuf {
    // CARGO_MANIFEST_DIR = crates/sddk-cli → repo root is two up.
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../")
        .join(FIXTURE_DIR)
        .join(name)
}

#[test]
fn cli_golden_surface_matches_blessed_snapshot() {
    let mut failures = Vec::new();

    for (args, name) in CASES {
        let path = fixture_path(name);
        let expected = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            failures.push(format!(
                "{name}: fixture missing ({e}). Regenerate with the procedure in {FIXTURE_DIR}/README.md."
            ));
            String::new()
        });
        if expected.is_empty() {
            continue;
        }

        let actual = help_output(args);
        let expected_norm: String = expected
            .lines()
            .map(|l| l.trim_end())
            .collect::<Vec<_>>()
            .join("\n");

        if actual != expected_norm {
            // Produce a compact diff hint for the failure message.
            let mut hint = String::new();
            let exp_lines: Vec<_> = expected_norm.lines().collect();
            let act_lines: Vec<_> = actual.lines().collect();
            for i in 0..exp_lines.len().max(act_lines.len()) {
                let e = exp_lines.get(i).copied().unwrap_or("<missing>");
                let a = act_lines.get(i).copied().unwrap_or("<extra>");
                if e != a {
                    hint.push_str(&format!(
                        "  line {}:\n    expected: {e}\n    actual:   {a}\n",
                        i + 1
                    ));
                }
            }
            failures.push(format!(
                "{name}: help output diverges from blessed snapshot.\n{hint}\nIf this change is intentional (new command/flag), regenerate the fixture: `sddk {} --help > {}` and commit.",
                args.join(" "),
                path.display()
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "CLI golden fixtures drifted from the blessed snapshot:\n\n{}",
        failures.join("\n\n")
    );
}
