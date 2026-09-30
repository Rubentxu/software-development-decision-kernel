//! E2E: validación de baseline y anclaje de rutas en `sddk uat` (report D4/D5).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sddk-uat-paths-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn git_init_with_tag(root: &Path) {
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
    run(&["tag", "v1.0.0"]);
}

fn sddk_in(cwd: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_sddk"))
        .args(args)
        .current_dir(cwd)
        .env("SDDK_STATE_HOME", cwd.join(".sddk-state"))
        .env("SDDK_DATA_HOME", cwd.join(".sddk-data"))
        .output()
        .expect("sddk binary")
}

fn both(out: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

#[test]
fn plan_from_nonexistent_tag_is_rejected() {
    let root = scratch_dir("bad-from");
    git_init_with_tag(&root);

    let out = sddk_in(
        &root,
        &[
            "uat",
            "plan",
            "--release",
            "v2.0.0",
            "--from",
            "v9.9.9-does-not-exist",
        ],
    );
    assert!(!out.status.success(), "typo'd --from must fail");
    assert!(
        both(&out).contains("no such git tag"),
        "error must explain: {}",
        both(&out)
    );

    // El tag real sí pasa y escribe el plan.
    let ok = sddk_in(
        &root,
        &["uat", "plan", "--release", "v2.0.0", "--from", "v1.0.0"],
    );
    assert!(ok.status.success(), "valid --from: {}", both(&ok));
    assert!(root.join("uat-plan-v2.0.0.yaml").exists());
}

#[test]
fn status_respects_root_instead_of_cwd() {
    let root = scratch_dir("status-root");
    git_init_with_tag(&root);

    // Artifacts en el checkout; el plan se genera ahí.
    let plan = sddk_in(&root, &["uat", "plan", "--release", "v2.0.0"]);
    assert!(plan.status.success());
    assert!(root.join("uat-plan-v2.0.0.yaml").exists());

    // Sin --root y desde fuera: no ve nada (correcto, no hay artefactos ahí).
    let elsewhere = scratch_dir("status-elsewhere");
    let out = sddk_in(&elsewhere, &["uat", "status", "--release", "v2.0.0"]);
    assert!(out.status.success());
    let text = both(&out);
    assert!(text.contains("plan: missing"), "elsewhere: {text}");

    // Con --root hacia el checkout: ve el plan aunque el cwd sea otro.
    let out = sddk_in(
        &elsewhere,
        &[
            "uat",
            "status",
            "--release",
            "v2.0.0",
            "--root",
            root.to_str().unwrap(),
        ],
    );
    assert!(out.status.success(), "{}", both(&out));
    let text = both(&out);
    assert!(text.contains("plan: generated"), "with --root: {text}");
}
