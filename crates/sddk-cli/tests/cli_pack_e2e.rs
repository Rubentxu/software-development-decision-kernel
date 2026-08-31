//! Integration tests for `sddk pack` CLI commands (phase4).
//!
//! Covers: validate (v1 + v2), list, verify, enable/disable idempotency,
//! install from local source with duplicate rejection.
//!
//! Follows the XDG-isolation pattern from `cli_approval_e2e.rs`.

use sha2::Digest;
use sha2::Sha256;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

/// Computes the stable project ID for a fallback seed + scope.
#[allow(dead_code)]
fn fallback_project_id(seed: &str, scope: &str) -> String {
    let hex = {
        let mut hasher = Sha256::new();
        let domain = "sddk.project.fallback.v1";
        hasher.update((domain.len() as u64).to_be_bytes());
        hasher.update(domain.as_bytes());
        hasher.update((seed.len() as u64).to_be_bytes());
        hasher.update(seed.as_bytes());
        hasher.update((scope.len() as u64).to_be_bytes());
        hasher.update(scope.as_bytes());
        format!("{:x}", hasher.finalize())
    };
    format!("p-{}", &hex[..16])
}

struct PackTestEnv {
    root: std::path::PathBuf,
    _dir: TempDir,
}

fn pack_test_setup() -> (PackTestEnv, impl Fn(&[&str]) -> std::process::Output) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("repo");
    let state = tmp.path().join("state");
    let data = tmp.path().join("data");
    let cache = tmp.path().join("cache");
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&state).unwrap();
    std::fs::create_dir_all(&data).unwrap();
    std::fs::create_dir_all(&cache).unwrap();
    std::fs::create_dir_all(&home).unwrap();
    init_git(&root);

    let home_c = home.clone();
    let data_c = data.clone();
    let state_c = state.clone();
    let cache_c = cache.clone();
    let root_c = root.clone();

    let run = move |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_sddk"))
            .args(args)
            .env("HOME", &home_c)
            .env("XDG_DATA_HOME", &data_c)
            .env("XDG_STATE_HOME", &state_c)
            .env("XDG_CACHE_HOME", &cache_c)
            .current_dir(&root_c)
            .output()
            .unwrap()
    };

    (PackTestEnv { root, _dir: tmp }, run)
}

/// Writes a v2 pack manifest at `root/packs/<id>/manifest.toml`.
fn write_v2_pack(root: &Path, id: &str, extra: &str) {
    let dir = root.join("packs").join(id);
    std::fs::create_dir_all(&dir).unwrap();
    let content = format!(
        r#"
[pack]
id = "{id}"
version = "0.1.0"
schema_version = 2
compatibility = ">=1.85"
risk = "low"
consequence = "creates"
category = "domain"

[dependencies]
requires = ["sddk-core"]

[[commands]]
name = "{id}"
surface = ["{id}"]

[fixtures]
paths = ["fixtures/plan.yaml"]
{extra}
"#
    );
    std::fs::write(dir.join("manifest.toml"), content).unwrap();
}

// The fallback seed must be a valid UUID v4; pack subcommands do not accept
// --fallback-seed (they resolve the project identity from the git remote).
// To keep tests hermetic we init a git repo with a fake origin remote.
fn init_git(root: &Path) {
    let _ = Command::new("git")
        .args(["init", "-q"])
        .current_dir(root)
        .output();
    let _ = Command::new("git")
        .args([
            "remote",
            "add",
            "origin",
            "https://github.com/test/pack-test.git",
        ])
        .current_dir(root)
        .output();
}

#[test]
fn pack_validate_accepts_v1_and_v2() {
    let (env, run) = pack_test_setup();
    // v2 manifest
    write_v2_pack(&env.root, "sddk-pack-uat", "");
    let out = run(&[
        "pack",
        "validate",
        "--manifest",
        "packs/sddk-pack-uat/manifest.toml",
    ]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("schema_version: 2"), "got: {stdout}");

    // v1 manifest (required/optional only)
    let v1 = env.root.join("v1.toml");
    std::fs::write(
        &v1,
        r#"
[pack]
id = "legacy-pack"
version = "0.1.0"
schema_version = 1
compatibility = ">=1.85"
risk = "low"
consequence = "creates"

[dependencies]
required = ["sddk-core"]
optional = ["sddk-core"]

[[commands]]
name = "legacy"
surface = ["legacy"]

[fixtures]
paths = ["t.sh"]
"#,
    )
    .unwrap();
    let out = run(&["pack", "validate", "--manifest", "v1.toml"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("schema_version: 1"), "got: {stdout}");
}

#[test]
fn pack_list_shows_discovered_packs() {
    let (env, run) = pack_test_setup();
    write_v2_pack(&env.root, "sddk-pack-uat", "");
    let out = run(&["pack", "list"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("sddk-pack-uat"), "got: {stdout}");
    assert!(stdout.contains("domain"), "got: {stdout}");
}

#[test]
fn pack_list_empty_when_no_packs() {
    let (_env, run) = pack_test_setup();
    let out = run(&["pack", "list"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("no packs found"), "got: {stdout}");
}

#[test]
fn pack_verify_reports_unsatisfied_requirement() {
    let (env, run) = pack_test_setup();
    write_v2_pack(&env.root, "sddk-pack-uat", "");
    let out = run(&["pack", "verify", "--id", "sddk-pack-uat"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_ne!(
        out.status.code(),
        Some(0),
        "expected non-zero for unsatisfied dep"
    );
    assert!(stdout.contains("unsatisfied"), "got: {stdout}");
}

#[test]
fn pack_verify_passes_with_provider() {
    let (env, run) = pack_test_setup();
    // provider sddk-core provides sddk-core capability
    let core = env.root.join("packs/sddk-core");
    std::fs::create_dir_all(&core).unwrap();
    std::fs::write(
        core.join("manifest.toml"),
        r#"
[pack]
id = "sddk-core"
version = "0.1.0"
schema_version = 2
compatibility = ">=1.85"
risk = "low"
consequence = "creates"
category = "core"

[provides]
capabilities = ["sddk-core"]

[[commands]]
name = "sddk-core"
surface = ["core"]

[fixtures]
paths = ["fixtures/plan.yaml"]
"#,
    )
    .unwrap();
    write_v2_pack(&env.root, "sddk-pack-uat", "");
    let out = run(&["pack", "verify", "--id", "sddk-pack-uat"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {} / stdout: {stdout}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("valid: true"), "got: {stdout}");
}

#[test]
fn pack_enable_disable_is_idempotent() {
    let (env, run) = pack_test_setup();
    write_v2_pack(&env.root, "sddk-pack-uat", "");
    // disable twice — both succeed
    for _ in 0..2 {
        let out = run(&["pack", "disable", "--id", "sddk-pack-uat"]);
        assert_eq!(
            out.status.code(),
            Some(0),
            "stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let out = run(&["pack", "list"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("false"),
        "expected disabled state, got: {stdout}"
    );
    // enable twice — both succeed
    for _ in 0..2 {
        let out = run(&["pack", "enable", "--id", "sddk-pack-uat"]);
        assert_eq!(
            out.status.code(),
            Some(0),
            "stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let out = run(&["pack", "list"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("true"),
        "expected enabled state, got: {stdout}"
    );
}

#[test]
fn pack_install_from_local_and_rejects_duplicate() {
    let (env, run) = pack_test_setup();
    // source pack outside repo (write_v2_pack creates src/packs/<id>/)
    let source = env.root.join("src");
    write_v2_pack(&source, "sddk-pack-cognicode", "");
    let out = run(&[
        "pack",
        "install",
        "--source",
        "src/packs/sddk-pack-cognicode",
    ]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("sddk-pack-cognicode"), "got: {stdout}");
    assert!(
        env.root
            .join("packs/sddk-pack-cognicode/manifest.toml")
            .exists()
    );

    // duplicate install rejected
    let out = run(&[
        "pack",
        "install",
        "--source",
        "src/packs/sddk-pack-cognicode",
    ]);
    assert_ne!(out.status.code(), Some(0), "expected duplicate rejection");
}

#[test]
fn pack_inspect_shows_entry() {
    let (env, run) = pack_test_setup();
    write_v2_pack(&env.root, "sddk-pack-uat", "");
    let out = run(&["pack", "inspect", "--id", "sddk-pack-uat"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("id: sddk-pack-uat"), "got: {stdout}");
}

/// Conformance fixtures (SPEC-017): valid fixtures exit 0, invalid fixtures
/// exit non-zero with their stable diagnostic code.
#[test]
fn pack_conformance_fixtures() {
    let (env, run) = pack_test_setup();
    let fixtures = env.root.join("fixtures/packs");
    std::fs::create_dir_all(&fixtures).unwrap();
    // Copy fixture manifests from the repo into the test root.
    for (name, _expected_code) in [
        ("valid-v2.toml", None),
        ("valid-v1.toml", None),
        ("invalid-conflict.toml", Some("PACK009")),
        ("invalid-dup-provides.toml", Some("PACK010")),
        ("invalid-empty-req.toml", Some("PACK008")),
    ] {
        let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/packs")
            .join(name);
        std::fs::copy(&source, fixtures.join(name)).unwrap();
    }

    for (name, expected_code) in [
        ("valid-v2.toml", None),
        ("valid-v1.toml", None),
        ("invalid-conflict.toml", Some("PACK009")),
        ("invalid-dup-provides.toml", Some("PACK010")),
        ("invalid-empty-req.toml", Some("PACK008")),
    ] {
        let out = run(&[
            "pack",
            "validate",
            "--manifest",
            &format!("fixtures/packs/{name}"),
        ]);
        let stdout = String::from_utf8_lossy(&out.stdout);
        match expected_code {
            None => {
                assert_eq!(
                    out.status.code(),
                    Some(0),
                    "{name} should be valid, got stdout: {stdout}"
                );
            }
            Some(code) => {
                assert_ne!(
                    out.status.code(),
                    Some(0),
                    "{name} should be invalid, got stdout: {stdout}"
                );
                assert!(
                    stdout.contains(code),
                    "{name} should report {code}, got: {stdout}"
                );
            }
        }
    }
}
