//! Adoption-contract integration test — migrated from `tests/test_adoption_contract.sh`.
//!
//! Reproduces the 11 shell assertions using `sddk_testkit::CliSandbox`:
//! - 6 contains-scenarios (required tokens)
//! - 5 absent-scenarios (forbidden tokens)
//!
//! The test reads the installed `agents/sddk-adopt.md` and asserts:
//! - Contains: sddk adopt status, sddk adopt apply, sddk knowledge status,
//!   sddk knowledge path, "Treat the project repository as read-only.",
//!   "Engram is optional."
//! - Absent: PROJECT=$(basename, GITIGNORE=, IGNORE_FILE=,
//!   mkdir -p "$project_path", .gitkeep

use std::fs;

use sddk_testkit::{CliSandbox, TestRepository};

/// Runs `sddk adopt status` against the sandbox and returns stdout.
fn adopt_status(sandbox: &CliSandbox) -> String {
    let output = sandbox
        .sddk_command()
        .args([
            "adopt",
            "status",
            "--root",
            &sandbox.path().to_string_lossy(),
        ])
        .output()
        .expect("sddk adopt status should succeed");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Runs `sddk adopt apply` against the sandbox and returns stdout.
fn adopt_apply(sandbox: &CliSandbox) -> String {
    let output = sandbox
        .sddk_command()
        .args([
            "adopt",
            "apply",
            "--root",
            &sandbox.path().to_string_lossy(),
        ])
        .output()
        .expect("sddk adopt apply should succeed");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Runs `sddk knowledge status` against the sandbox and returns stdout.
fn knowledge_status(sandbox: &CliSandbox) -> String {
    let output = sandbox
        .sddk_command()
        .args([
            "knowledge",
            "status",
            "--root",
            &sandbox.path().to_string_lossy(),
        ])
        .output()
        .expect("sddk knowledge status should succeed");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Runs `sddk knowledge path` against the sandbox and returns stdout.
fn knowledge_path(sandbox: &CliSandbox) -> String {
    let output = sandbox
        .sddk_command()
        .args([
            "knowledge",
            "path",
            "--root",
            &sandbox.path().to_string_lossy(),
        ])
        .output()
        .expect("sddk knowledge path should succeed");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Reads the installed adopt agent file content.
fn read_adopt_file(sandbox: &CliSandbox) -> String {
    let adopt_path = sandbox.path().join("agents/sddk-adopt.md");
    fs::read_to_string(&adopt_path).expect("sddk-adopt.md should exist")
}

// ── Contains scenarios ─────────────────────────────────────────────────────────

#[test]
fn adoption_contains_sddk_adopt_status() {
    let repo = TestRepository::new().unwrap();
    repo.init().unwrap();
    let sandbox = CliSandbox::new(repo, env!("CARGO_BIN_EXE_sddk")).unwrap();
    sandbox
        .init_git("adoption-contract", "adopt@example.com")
        .unwrap();

    // Install the adopt agent file (copy from installed framework)
    let adopt_src =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../agents/sddk-adopt.md");
    if adopt_src.exists() {
        let content = fs::read_to_string(&adopt_src).unwrap();
        fs::create_dir_all(sandbox.path().join("agents")).unwrap();
        sandbox
            .repo()
            .write("agents/sddk-adopt.md", &content)
            .unwrap();
    }

    // Run CLI commands to exercise the sandbox
    let _ = adopt_status(&sandbox);
    let _ = adopt_apply(&sandbox);
    let _ = knowledge_status(&sandbox);
    let _ = knowledge_path(&sandbox);

    // Check that agents/sddk-adopt.md contains the required token
    let content = read_adopt_file(&sandbox);
    assert!(
        content.contains("sddk adopt status"),
        "adopt file should contain 'sddk adopt status': {}",
        content
    );
}

#[test]
fn adoption_contains_sddk_adopt_apply() {
    let repo = TestRepository::new().unwrap();
    repo.init().unwrap();
    let sandbox = CliSandbox::new(repo, env!("CARGO_BIN_EXE_sddk")).unwrap();
    sandbox
        .init_git("adoption-contract", "adopt@example.com")
        .unwrap();

    let adopt_src =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../agents/sddk-adopt.md");
    if adopt_src.exists() {
        let content = fs::read_to_string(&adopt_src).unwrap();
        fs::create_dir_all(sandbox.path().join("agents")).unwrap();
        sandbox
            .repo()
            .write("agents/sddk-adopt.md", &content)
            .unwrap();
    }

    let _ = adopt_status(&sandbox);
    let _ = adopt_apply(&sandbox);
    let _ = knowledge_status(&sandbox);
    let _ = knowledge_path(&sandbox);

    let content = read_adopt_file(&sandbox);
    assert!(
        content.contains("sddk adopt apply"),
        "adopt file should contain 'sddk adopt apply': {}",
        content
    );
}

#[test]
fn adoption_contains_sddk_knowledge_status() {
    let repo = TestRepository::new().unwrap();
    repo.init().unwrap();
    let sandbox = CliSandbox::new(repo, env!("CARGO_BIN_EXE_sddk")).unwrap();
    sandbox
        .init_git("adoption-contract", "adopt@example.com")
        .unwrap();

    let adopt_src =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../agents/sddk-adopt.md");
    if adopt_src.exists() {
        let content = fs::read_to_string(&adopt_src).unwrap();
        fs::create_dir_all(sandbox.path().join("agents")).unwrap();
        sandbox
            .repo()
            .write("agents/sddk-adopt.md", &content)
            .unwrap();
    }

    let _ = adopt_status(&sandbox);
    let _ = adopt_apply(&sandbox);
    let _ = knowledge_status(&sandbox);
    let _ = knowledge_path(&sandbox);

    let content = read_adopt_file(&sandbox);
    assert!(
        content.contains("sddk knowledge status"),
        "adopt file should contain 'sddk knowledge status': {}",
        content
    );
}

#[test]
fn adoption_contains_sddk_knowledge_path() {
    let repo = TestRepository::new().unwrap();
    repo.init().unwrap();
    let sandbox = CliSandbox::new(repo, env!("CARGO_BIN_EXE_sddk")).unwrap();
    sandbox
        .init_git("adoption-contract", "adopt@example.com")
        .unwrap();

    let adopt_src =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../agents/sddk-adopt.md");
    if adopt_src.exists() {
        let content = fs::read_to_string(&adopt_src).unwrap();
        fs::create_dir_all(sandbox.path().join("agents")).unwrap();
        sandbox
            .repo()
            .write("agents/sddk-adopt.md", &content)
            .unwrap();
    }

    let _ = adopt_status(&sandbox);
    let _ = adopt_apply(&sandbox);
    let _ = knowledge_status(&sandbox);
    let _ = knowledge_path(&sandbox);

    let content = read_adopt_file(&sandbox);
    assert!(
        content.contains("sddk knowledge path"),
        "adopt file should contain 'sddk knowledge path': {}",
        content
    );
}

#[test]
fn adoption_contains_readonly_clause() {
    let repo = TestRepository::new().unwrap();
    repo.init().unwrap();
    let sandbox = CliSandbox::new(repo, env!("CARGO_BIN_EXE_sddk")).unwrap();
    sandbox
        .init_git("adoption-contract", "adopt@example.com")
        .unwrap();

    let adopt_src =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../agents/sddk-adopt.md");
    if adopt_src.exists() {
        let content = fs::read_to_string(&adopt_src).unwrap();
        fs::create_dir_all(sandbox.path().join("agents")).unwrap();
        sandbox
            .repo()
            .write("agents/sddk-adopt.md", &content)
            .unwrap();
    }

    let content = read_adopt_file(&sandbox);
    assert!(
        content.contains("Treat the project repository as read-only."),
        "adopt file should contain 'Treat the project repository as read-only.': {}",
        content
    );
}

#[test]
fn adoption_contains_engram_optional_clause() {
    let repo = TestRepository::new().unwrap();
    repo.init().unwrap();
    let sandbox = CliSandbox::new(repo, env!("CARGO_BIN_EXE_sddk")).unwrap();
    sandbox
        .init_git("adoption-contract", "adopt@example.com")
        .unwrap();

    let adopt_src =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../agents/sddk-adopt.md");
    if adopt_src.exists() {
        let content = fs::read_to_string(&adopt_src).unwrap();
        fs::create_dir_all(sandbox.path().join("agents")).unwrap();
        sandbox
            .repo()
            .write("agents/sddk-adopt.md", &content)
            .unwrap();
    }

    let content = read_adopt_file(&sandbox);
    assert!(
        content.contains("Engram is optional."),
        "adopt file should contain 'Engram is optional.': {}",
        content
    );
}

// ── Absent scenarios ────────────────────────────────────────────────────────────

#[test]
fn adoption_absent_basename_identity() {
    let repo = TestRepository::new().unwrap();
    repo.init().unwrap();
    let sandbox = CliSandbox::new(repo, env!("CARGO_BIN_EXE_sddk")).unwrap();
    sandbox
        .init_git("adoption-contract", "adopt@example.com")
        .unwrap();

    let adopt_src =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../agents/sddk-adopt.md");
    if adopt_src.exists() {
        let content = fs::read_to_string(&adopt_src).unwrap();
        fs::create_dir_all(sandbox.path().join("agents")).unwrap();
        sandbox
            .repo()
            .write("agents/sddk-adopt.md", &content)
            .unwrap();
    }

    let content = read_adopt_file(&sandbox);
    assert!(
        !content.contains("PROJECT=$(basename"),
        "adopt file should NOT contain 'PROJECT=$(basename': {}",
        content
    );
}

#[test]
fn adoption_absent_gitignore_assignment() {
    let repo = TestRepository::new().unwrap();
    repo.init().unwrap();
    let sandbox = CliSandbox::new(repo, env!("CARGO_BIN_EXE_sddk")).unwrap();
    sandbox
        .init_git("adoption-contract", "adopt@example.com")
        .unwrap();

    let adopt_src =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../agents/sddk-adopt.md");
    if adopt_src.exists() {
        let content = fs::read_to_string(&adopt_src).unwrap();
        fs::create_dir_all(sandbox.path().join("agents")).unwrap();
        sandbox
            .repo()
            .write("agents/sddk-adopt.md", &content)
            .unwrap();
    }

    let content = read_adopt_file(&sandbox);
    assert!(
        !content.contains("GITIGNORE="),
        "adopt file should NOT contain 'GITIGNORE=': {}",
        content
    );
}

#[test]
fn adoption_absent_ignore_file_assignment() {
    let repo = TestRepository::new().unwrap();
    repo.init().unwrap();
    let sandbox = CliSandbox::new(repo, env!("CARGO_BIN_EXE_sddk")).unwrap();
    sandbox
        .init_git("adoption-contract", "adopt@example.com")
        .unwrap();

    let adopt_src =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../agents/sddk-adopt.md");
    if adopt_src.exists() {
        let content = fs::read_to_string(&adopt_src).unwrap();
        fs::create_dir_all(sandbox.path().join("agents")).unwrap();
        sandbox
            .repo()
            .write("agents/sddk-adopt.md", &content)
            .unwrap();
    }

    let content = read_adopt_file(&sandbox);
    assert!(
        !content.contains("IGNORE_FILE="),
        "adopt file should NOT contain 'IGNORE_FILE=': {}",
        content
    );
}

#[test]
fn adoption_absent_mkdir_project_path() {
    let repo = TestRepository::new().unwrap();
    repo.init().unwrap();
    let sandbox = CliSandbox::new(repo, env!("CARGO_BIN_EXE_sddk")).unwrap();
    sandbox
        .init_git("adoption-contract", "adopt@example.com")
        .unwrap();

    let adopt_src =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../agents/sddk-adopt.md");
    if adopt_src.exists() {
        let content = fs::read_to_string(&adopt_src).unwrap();
        fs::create_dir_all(sandbox.path().join("agents")).unwrap();
        sandbox
            .repo()
            .write("agents/sddk-adopt.md", &content)
            .unwrap();
    }

    let content = read_adopt_file(&sandbox);
    assert!(
        !content.contains("mkdir -p \"$project_path\""),
        "adopt file should NOT contain 'mkdir -p \"$project_path\"': {}",
        content
    );
}

#[test]
fn adoption_absent_gitkeep_placeholder() {
    let repo = TestRepository::new().unwrap();
    repo.init().unwrap();
    let sandbox = CliSandbox::new(repo, env!("CARGO_BIN_EXE_sddk")).unwrap();
    sandbox
        .init_git("adoption-contract", "adopt@example.com")
        .unwrap();

    let adopt_src =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../agents/sddk-adopt.md");
    if adopt_src.exists() {
        let content = fs::read_to_string(&adopt_src).unwrap();
        fs::create_dir_all(sandbox.path().join("agents")).unwrap();
        sandbox
            .repo()
            .write("agents/sddk-adopt.md", &content)
            .unwrap();
    }

    let content = read_adopt_file(&sandbox);
    assert!(
        !content.contains(".gitkeep"),
        "adopt file should NOT contain '.gitkeep': {}",
        content
    );
}
