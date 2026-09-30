//! E2E: guardas fail-closed de `sddk backlog` (agent-secretless report D3/D7).
//!
//! D3: `backlog discard --reason superseded` sin `--superseded-by` (o con un
//! sucesor inexistente) debe fallar sin emitir evento. D7: `backlog render
//! --check` debe detectar drift entre el archivo y la proyección del ledger.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch_dir(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("sddk-backlog-guard-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn git_init(root: &Path) {
    let ok = Command::new("git")
        .args(["init", "-q"])
        .current_dir(root)
        .status()
        .expect("git")
        .success();
    assert!(ok, "git init failed");
    Command::new("git")
        .args([
            "-C",
            root.to_str().unwrap(),
            "remote",
            "add",
            "origin",
            "https://github.com/example/backlog-guard.git",
        ])
        .status()
        .unwrap();
}

fn sddk(root: &Path, args: &[&str]) -> std::process::Output {
    let mut full: Vec<&str> = args.to_vec();
    full.push("--root");
    full.push(root.to_str().unwrap());
    // El ledger es global por project_id en $XDG_STATE_HOME/sddk/projects:
    // cada test aísla state/data/cache para no leer ledgers ajenos. El cwd
    // también se fija al root: `backlog render` escribe BACKLOG.md relativo
    // al cwd del proceso, no a --root.
    let state = root.join(".sddk-state");
    Command::new(env!("CARGO_BIN_EXE_sddk"))
        .args(&full)
        .current_dir(root)
        .env("SDDK_STATE_HOME", &state)
        .env("SDDK_DATA_HOME", root.join(".sddk-data"))
        .env("XDG_CACHE_HOME", root.join(".sddk-cache"))
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

fn capture(root: &Path, summary: &str) -> String {
    let out = sddk(
        root,
        &[
            "backlog",
            "capture",
            "--origin-cycle-id",
            "p-x/c-guard",
            "--origin-phase",
            "explore",
            "--actor-ref",
            "agent:test",
            "--summary",
            summary,
        ],
    );
    assert!(out.status.success(), "capture failed: {}", both(&out));
    // capture emite YAML: "item_id: bl-..."
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .find_map(|l| l.strip_prefix("item_id:"))
        .map(str::trim)
        .expect("item_id in capture output")
        .to_string()
}

fn live_count(root: &Path) -> usize {
    let list = sddk(root, &["backlog", "list", "--format", "json"]);
    assert!(list.status.success(), "list failed: {}", both(&list));
    let json: serde_json::Value = serde_json::from_slice(&list.stdout).expect("list json");
    json["items"].as_array().expect("items array").len()
}

#[test]
fn superseded_without_successor_is_rejected() {
    let root = scratch_dir("no-successor");
    git_init(&root);
    let id = capture(&root, "finding A");

    let out = sddk(
        &root,
        &[
            "backlog",
            "discard",
            "--item-id",
            &id,
            "--reason",
            "superseded",
            "--actor-ref",
            "agent:test",
        ],
    );
    assert!(!out.status.success(), "must fail without --superseded-by");
    let text = both(&out);
    assert!(
        text.contains("superseded-by"),
        "error must name the flag: {text}"
    );

    // El item sigue vivo: no se emitió evento de descarte.
    assert_eq!(
        live_count(&root),
        1,
        "item must remain live after rejected discard"
    );
}

#[test]
fn superseded_with_missing_successor_is_rejected() {
    let root = scratch_dir("missing-successor");
    git_init(&root);
    let id = capture(&root, "finding B");

    let out = sddk(
        &root,
        &[
            "backlog",
            "discard",
            "--item-id",
            &id,
            "--reason",
            "superseded",
            "--superseded-by",
            "B-999",
            "--actor-ref",
            "agent:test",
        ],
    );
    assert!(
        !out.status.success(),
        "must fail for a nonexistent successor"
    );
    assert!(
        both(&out).contains("B-999"),
        "error must name the missing successor"
    );

    assert_eq!(live_count(&root), 1, "still live");
}

#[test]
fn superseded_with_existing_successor_succeeds_and_self_reference_fails() {
    let root = scratch_dir("ok-successor");
    git_init(&root);
    let old = capture(&root, "old finding");
    let new = capture(&root, "replacement finding");

    // Auto-referencia prohibida.
    let out = sddk(
        &root,
        &[
            "backlog",
            "discard",
            "--item-id",
            &old,
            "--reason",
            "superseded",
            "--superseded-by",
            &old,
            "--actor-ref",
            "agent:test",
        ],
    );
    assert!(!out.status.success(), "self-reference must fail");
    assert!(both(&out).contains("DIFFERENT"), "must say why");

    // Sucesor real: éxito.
    let out = sddk(
        &root,
        &[
            "backlog",
            "discard",
            "--item-id",
            &old,
            "--reason",
            "superseded",
            "--superseded-by",
            &new,
            "--actor-ref",
            "agent:test",
        ],
    );
    assert!(out.status.success(), "valid supersede: {}", both(&out));

    assert_eq!(live_count(&root), 1, "only successor live");
}

#[test]
fn wontfix_does_not_require_successor() {
    let root = scratch_dir("wontfix");
    git_init(&root);
    let id = capture(&root, "not worth it");

    let out = sddk(
        &root,
        &[
            "backlog",
            "discard",
            "--item-id",
            &id,
            "--reason",
            "wontfix",
            "--actor-ref",
            "agent:test",
        ],
    );
    assert!(out.status.success(), "wontfix: {}", both(&out));
}

#[test]
fn render_check_detects_drift_and_passes_when_clean() {
    let root = scratch_dir("render-check");
    git_init(&root);
    capture(&root, "drift probe");

    // Sin archivo: --check falla.
    let out = sddk(&root, &["backlog", "render", "--check"]);
    assert!(!out.status.success(), "missing file must fail check");

    // Regenerar y volver a chequear: limpio.
    let generated = sddk(&root, &["backlog", "render"]);
    assert!(generated.status.success(), "render: {}", both(&generated));
    let clean = sddk(&root, &["backlog", "render", "--check"]);
    assert!(
        clean.status.success(),
        "clean file must pass: {}",
        both(&clean)
    );

    // Drift manual: --check falla sin tocar el archivo.
    let backlog_path = root.join("BACKLOG.md");
    fs::write(&backlog_path, "# tampered\n").unwrap();
    let dirty = sddk(&root, &["backlog", "render", "--check"]);
    assert!(!dirty.status.success(), "drift must fail check");
    assert!(
        both(&dirty).contains("diverges"),
        "error must describe drift: {}",
        both(&dirty)
    );
    assert_eq!(
        fs::read_to_string(&backlog_path).unwrap(),
        "# tampered\n",
        "--check must not rewrite the file"
    );
}
