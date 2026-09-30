//! E2E: contrato de gates del release (agent-secretless S3.1/S3.2 / W4).
//!
//! El prompt de release (corregido) manda evaluar `no-pending-effects` y
//! `release-uat-approved` con `--evaluator sddk.cli`; ese comando debe emitir
//! receipt real, sin ENGINE_UNREGISTERED_EVALUATOR. Y evaluar
//! `release-receipt` (que es un ARTIFACT del workflow, no un gate) debe
//! fallar con un mensaje que lo explique.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch_dir(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("sddk-gate-contract-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn git_init(root: &Path) {
    let run = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .expect("git")
    };
    run(&["init", "-q"]);
    run(&["config", "user.email", "t@t"]);
    run(&["config", "user.name", "t"]);
    fs::write(root.join("f.txt"), "1\n").unwrap();
    run(&["add", "."]);
    run(&["commit", "-qm", "one"]);
    run(&[
        "remote",
        "add",
        "origin",
        "https://github.com/example/gate-contract.git",
    ]);
}

struct Fixture {
    root: PathBuf,
    remote: String,
    cycle_id: String,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = scratch_dir(name);
        git_init(&root);
        let remote = "https://github.com/example/gate-contract.git".to_string();
        let sddk = |args: &[&str]| {
            Command::new(env!("CARGO_BIN_EXE_sddk"))
                .args(args)
                .current_dir(&root)
                .env("SDDK_STATE_HOME", root.join(".sddk-state"))
                .env("SDDK_DATA_HOME", root.join(".sddk-data"))
                .output()
                .expect("sddk binary")
        };
        let out = sddk(&[
            "adopt",
            "apply",
            "--root",
            root.to_str().unwrap(),
            "--scope",
            ".",
            "--remote",
            &remote,
            "--format",
            "json",
        ]);
        assert!(
            out.status.success(),
            "adopt: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let out = sddk(&[
            "cycle",
            "start",
            "--name",
            name,
            "--root",
            root.to_str().unwrap(),
            "--scope",
            ".",
            "--remote",
            &remote,
            "--format",
            "json",
        ]);
        assert!(
            out.status.success(),
            "start: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        let cycle_id = json["cycle_id"].as_str().expect("cycle_id").to_string();
        Self {
            root,
            remote,
            cycle_id,
        }
    }

    fn evaluate(&self, gate: &str, transition: &str) -> std::process::Output {
        // Evidence REQ-IPV para outcome=passed.
        let evidence = format!(
            r#"{{"argv":["cargo","test"],"exit_code":0,"output_digest":"sha256:{}"}}"#,
            "a".repeat(64)
        );
        Command::new(env!("CARGO_BIN_EXE_sddk"))
            .args([
                "cycle",
                "evaluate-gate",
                "--root",
                self.root.to_str().unwrap(),
                "--scope",
                ".",
                "--remote",
                &self.remote,
                "--cycle",
                &self.cycle_id,
                "--transition",
                transition,
                "--gate",
                gate,
                "--evaluator",
                "sddk.cli",
                "--outcome",
                "passed",
                "--evidence",
                &evidence,
                "--timestamp",
                "2026-08-04T10:00:00Z",
                "--actor",
                "sddk",
                "--format",
                "json",
            ])
            .current_dir(&self.root)
            .env("SDDK_STATE_HOME", self.root.join(".sddk-state"))
            .env("SDDK_DATA_HOME", self.root.join(".sddk-data"))
            .output()
            .expect("sddk binary")
    }
}

fn both(out: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

#[test]
fn release_complete_real_gates_accept_sddk_cli_evaluator() {
    let f = Fixture::new("real-gates");
    // Camino minimo hasta release: walk manual con las transiciones A-min.
    // (Emulamos el contrato del prompt: los dos gates reales de
    // release.complete se evaluan con sddk.cli sin error de registro.)
    for (transition, gate) in [
        ("phase.explore.complete", "exploration-sufficient"),
        ("phase.specify.complete.a-min", "requirements-testable"),
        ("phase.design.complete", "architecture-consistent"),
        ("phase.plan.complete", "plan-executable"),
        ("phase.build.complete", "implementation-complete"),
        ("phase.verify.complete", "tests-pass"),
        ("release.complete", "no-pending-effects"),
        ("release.complete", "release-uat-approved"),
    ] {
        let out = f.evaluate(gate, transition);
        assert!(
            out.status.success(),
            "gate {gate} on {transition} failed: {}",
            both(&out)
        );
        let json: serde_json::Value = serde_json::from_slice(&out.stdout)
            .unwrap_or_else(|e| panic!("receipt json: {e}: {}", both(&out)));
        assert_eq!(json["evaluator"], "sddk.cli");
        assert!(json["receipt_id"].as_str().is_some());
    }
}

#[test]
fn release_receipt_artifact_evaluated_as_gate_fails_with_hint() {
    let f = Fixture::new("artifact-as-gate");
    let out = f.evaluate("release-receipt", "release.complete");
    assert!(!out.status.success(), "artifact must not evaluate as gate");
    let text = both(&out);
    assert!(
        text.contains("not registered for gate"),
        "engine error: {text}"
    );
    assert!(
        text.contains("ARTIFACTS, not gates") || text.contains("gates:"),
        "error must hint the artifact/gate distinction: {text}"
    );
}
