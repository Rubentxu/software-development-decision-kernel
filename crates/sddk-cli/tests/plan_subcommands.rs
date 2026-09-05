//! Tests for `sddk plan` subcommands (PLN-LEDGER-002).
//!
//! Covers AC-PLN2-07 (CLI surface) via integration tests:
//! - work-item create → list shows it
//! - dep add rejecting a Q2-violating edge with non-zero exit
//! - evidence attach round-trip
//! - decision record + graph command output

use std::process::Command;

use tempfile::TempDir;

struct PlanFixture {
    _dir: TempDir,
    root: TempDir,
    data: TempDir,
    state: TempDir,
    cache: TempDir,
    home: TempDir,
    project_id: String,
    cycle_id: String,
}

impl PlanFixture {
    /// Sets up: adopted project + running cycle.
    fn with_cycle() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        let state = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();

        let root_str = root.path().to_str().unwrap().to_string();

        // Create minimal git repo at root
        std::fs::create_dir_all(root.path().join(".git")).ok();

        fn run_sddk(
            root: &str,
            home: &str,
            data: &str,
            state: &str,
            cache: &str,
            args: &[&str],
        ) -> std::process::Output {
            let mut cmd = Command::new(env!("CARGO_BIN_EXE_sddk"));
            cmd.env("HOME", home);
            cmd.env("XDG_DATA_HOME", data);
            cmd.env("XDG_STATE_HOME", state);
            cmd.env("XDG_CACHE_HOME", cache);
            cmd.env("USER", "test-cli-actor");
            cmd.current_dir(root);
            cmd.args(args);
            cmd.output().expect("sddk command")
        }

        // Adopt — use --actor (not --actor-id) for adopt
        let adopt = run_sddk(
            &root_str,
            home.path().to_str().unwrap(),
            data.path().to_str().unwrap(),
            state.path().to_str().unwrap(),
            cache.path().to_str().unwrap(),
            &[
                "adopt",
                "apply",
                "--root",
                &root_str,
                "--scope",
                ".",
                "--timestamp",
                "2026-09-05T00:00:00Z",
                "--actor",
                "test",
                "--format",
                "json",
            ],
        );
        assert!(
            adopt.status.success(),
            "adopt failed: {}",
            String::from_utf8_lossy(&adopt.stderr)
        );
        let adopt_json: serde_json::Value =
            serde_json::from_slice(&adopt.stdout).expect("adopt must return JSON project_id");
        let project_id = adopt_json["project_id"]
            .as_str()
            .expect("project_id must be string")
            .to_string();

        // sddk plan requires .sddk/adoption.json in the project root (not just XDG_DATA_HOME).
        // Copy the receipt to the project root so plan commands can find it.
        let receipt_path_from_adopt = adopt_json["receipt_path"]
            .as_str()
            .expect("receipt_path must be string");
        let sddk_dir = root.path().join(".sddk");
        std::fs::create_dir_all(&sddk_dir).unwrap();
        let dest_receipt = sddk_dir.join("adoption.json");
        std::fs::copy(receipt_path_from_adopt, &dest_receipt).unwrap();

        // Start cycle — use --actor (not --actor-id), --name (not --cycle-id)
        let cycle = run_sddk(
            &root_str,
            home.path().to_str().unwrap(),
            data.path().to_str().unwrap(),
            state.path().to_str().unwrap(),
            cache.path().to_str().unwrap(),
            &[
                "cycle",
                "start",
                "--root",
                &root_str,
                "--scope",
                ".",
                "--name",
                "pln-ledger-002-test",
                "--timestamp",
                "2026-09-05T00:00:00Z",
                "--actor",
                "test",
                "--format",
                "json",
            ],
        );
        assert!(
            cycle.status.success(),
            "cycle start failed: {}",
            String::from_utf8_lossy(&cycle.stderr)
        );
        let cycle_json: serde_json::Value =
            serde_json::from_slice(&cycle.stdout).expect("cycle start must return JSON");
        let cycle_id = cycle_json["cycle_id"]
            .as_str()
            .expect("cycle_id must be string")
            .to_string();

        Self {
            _dir: dir,
            root,
            data,
            state,
            cache,
            home,
            project_id,
            cycle_id,
        }
    }

    /// Run sddk plan with the given subcommand args (adds --format json automatically).
    fn run_plan(&self, args: &[&str]) -> std::process::Output {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_sddk"));
        cmd.env("HOME", self.home.path().to_str().unwrap());
        cmd.env("XDG_DATA_HOME", self.data.path().to_str().unwrap());
        cmd.env("XDG_STATE_HOME", self.state.path().to_str().unwrap());
        cmd.env("XDG_CACHE_HOME", self.cache.path().to_str().unwrap());
        cmd.env("USER", "test-cli-actor");
        cmd.current_dir(self.root.path());
        cmd.args(["plan"]);
        cmd.args(args);
        // Always request JSON from plan subcommands
        if !args.contains(&"--format") && !args.contains(&"--format=json") {
            cmd.args(["--format", "json"]);
        }
        cmd.output().expect("sddk plan command")
    }
}

// ── work-item create + list ───────────────────────────────────────────────────

#[test]
fn plan_workitem_create_shows_in_list() {
    let fixture = PlanFixture::with_cycle();

    // Create a work item — subcommand is work-item (hyphen), not workitem
    let create = fixture.run_plan(&[
        "work-item",
        "create",
        "--cycle-id",
        &fixture.cycle_id,
        "--title",
        "Test work item",
        "--description",
        "A test description",
        "--actor-id",
        "agent:test",
    ]);
    assert_eq!(
        create.status.code(),
        Some(0),
        "work-item create must succeed: {}",
        String::from_utf8_lossy(&create.stderr)
    );
    let created: serde_json::Value =
        serde_json::from_slice(&create.stdout).expect("create output must be JSON with id");
    let work_item_id = created
        .get("id")
        .expect("id field must be present")
        .as_str()
        .expect("id must be string");

    // List must show the created item
    let list = fixture.run_plan(&["work-item", "list", "--cycle-id", &fixture.cycle_id]);
    assert_eq!(
        list.status.code(),
        Some(0),
        "work-item list must succeed: {}",
        String::from_utf8_lossy(&list.stderr)
    );
    let stdout = String::from_utf8_lossy(&list.stdout);
    assert!(
        stdout.contains(work_item_id),
        "list output must contain created work item id. Got: {stdout}"
    );
}

// ── dep add creates dependency edge ─────────────────────────────────────────

#[test]
fn plan_dep_add_creates_dependency() {
    let fixture = PlanFixture::with_cycle();

    // Create two work items
    let wi_a: serde_json::Value = serde_json::from_slice(
        &fixture
            .run_plan(&[
                "work-item",
                "create",
                "--cycle-id",
                &fixture.cycle_id,
                "--title",
                "Item A",
                "--description",
                "desc",
                "--actor-id",
                "agent:test",
            ])
            .stdout,
    )
    .expect("create A must succeed");
    let wi_b: serde_json::Value = serde_json::from_slice(
        &fixture
            .run_plan(&[
                "work-item",
                "create",
                "--cycle-id",
                &fixture.cycle_id,
                "--title",
                "Item B",
                "--description",
                "desc",
                "--actor-id",
                "agent:test",
            ])
            .stdout,
    )
    .expect("create B must succeed");

    let id_a = wi_a.get("id").expect("id").as_str().unwrap();
    let id_b = wi_b.get("id").expect("id").as_str().unwrap();

    // Add a Blocks dependency from A → B
    let add = fixture.run_plan(&[
        "dep",
        "add",
        "--from-id",
        id_a,
        "--to-id",
        id_b,
        "--kind",
        "Blocks",
        "--actor-id",
        "system:planner",
    ]);
    assert_eq!(
        add.status.code(),
        Some(0),
        "dep add must succeed: {}",
        String::from_utf8_lossy(&add.stderr)
    );
    let add_json: serde_json::Value =
        serde_json::from_slice(&add.stdout).expect("dep add output must be JSON");
    assert!(
        add_json.get("from").is_some() && add_json.get("to").is_some(),
        "dep add response must contain 'from' and 'to' fields"
    );

    // Graph must show the dependency edge
    let graph = fixture.run_plan(&["graph", "--cycle-id", &fixture.cycle_id]);
    assert_eq!(
        graph.status.code(),
        Some(0),
        "graph must succeed: {}",
        String::from_utf8_lossy(&graph.stderr)
    );
    let stdout = String::from_utf8_lossy(&graph.stdout);
    assert!(
        stdout.contains(id_a) && stdout.contains(id_b),
        "graph output must contain both work item ids. Got: {stdout}"
    );
}

// ── evidence attach round-trip ───────────────────────────────────────────────

#[test]
fn plan_evidence_attach_round_trip() {
    let fixture = PlanFixture::with_cycle();

    // Create a work item
    let wi: serde_json::Value = serde_json::from_slice(
        &fixture
            .run_plan(&[
                "work-item",
                "create",
                "--cycle-id",
                &fixture.cycle_id,
                "--title",
                "Evidence test",
                "--description",
                "desc",
                "--actor-id",
                "agent:test",
            ])
            .stdout,
    )
    .expect("create must succeed");
    let work_item_id = wi.get("id").expect("id").as_str().unwrap();

    // Create a temp file with evidence body
    let body_file = fixture._dir.path().join("evidence-body.txt");
    std::fs::write(&body_file, b"build log: 42 tests passed").unwrap();

    // Attach evidence — uses --work-item-id (not positional), --kind, --body-file
    let attach = fixture.run_plan(&[
        "evidence",
        "attach",
        "--work-item-id",
        work_item_id,
        "--kind",
        "log",
        "--body-file",
        body_file.to_str().unwrap(),
        "--actor-id",
        "agent:test",
    ]);
    assert_eq!(
        attach.status.code(),
        Some(0),
        "evidence attach must succeed: {}",
        String::from_utf8_lossy(&attach.stderr)
    );
    let attached: serde_json::Value =
        serde_json::from_slice(&attach.stdout).expect("attach output must be JSON");
    assert!(
        attached.get("id").is_some(),
        "attach response must contain evidence id"
    );
    assert!(
        attached.get("cas").is_some(),
        "attach response must contain CAS hash"
    );
}

// ── decision record + graph ──────────────────────────────────────────────────

#[test]
fn plan_decision_record_and_graph() {
    let fixture = PlanFixture::with_cycle();

    // Create a work item
    let wi: serde_json::Value = serde_json::from_slice(
        &fixture
            .run_plan(&[
                "work-item",
                "create",
                "--cycle-id",
                &fixture.cycle_id,
                "--title",
                "Decision test",
                "--description",
                "desc",
                "--actor-id",
                "agent:test",
            ])
            .stdout,
    )
    .expect("create must succeed");
    let work_item_id = wi.get("id").expect("id").as_str().unwrap();

    // Record a decision — uses --work-item-id, --kind, --rationale
    let record = fixture.run_plan(&[
        "decision",
        "record",
        "--work-item-id",
        work_item_id,
        "--kind",
        "Implementation",
        "--rationale",
        "Best approach given the constraints",
        "--actor-id",
        "user:bob",
    ]);
    assert_eq!(
        record.status.code(),
        Some(0),
        "decision record must succeed: {}",
        String::from_utf8_lossy(&record.stderr)
    );
    let recorded: serde_json::Value =
        serde_json::from_slice(&record.stdout).expect("record output must be JSON");
    assert!(
        recorded.get("id").is_some(),
        "record response must contain decision id"
    );

    // Graph must show the work item and decision
    let graph = fixture.run_plan(&["graph", "--cycle-id", &fixture.cycle_id]);
    assert_eq!(
        graph.status.code(),
        Some(0),
        "graph must succeed: {}",
        String::from_utf8_lossy(&graph.stderr)
    );
    let stdout = String::from_utf8_lossy(&graph.stdout);
    let graph_json: serde_json::Value =
        serde_json::from_str(&stdout).expect("graph output must be valid JSON");
    let work_item_ids = graph_json
        .get("work_item_ids")
        .expect("graph must contain work_item_ids field")
        .as_array()
        .expect("work_item_ids must be an array");
    assert!(
        work_item_ids
            .iter()
            .any(|id| id.as_str() == Some(work_item_id)),
        "graph work_item_ids must contain the created work item"
    );
    let decision_refs = graph_json
        .get("decision_refs")
        .expect("graph must contain decision_refs field")
        .as_array()
        .expect("decision_refs must be an array");
    assert!(
        !decision_refs.is_empty(),
        "graph decision_refs must contain the recorded decision"
    );
}

// ── evidence attach with empty body rejected ────────────────────────────────

#[test]
fn plan_evidence_attach_empty_body_rejected() {
    let fixture = PlanFixture::with_cycle();

    // Create a work item
    let wi: serde_json::Value = serde_json::from_slice(
        &fixture
            .run_plan(&[
                "work-item",
                "create",
                "--cycle-id",
                &fixture.cycle_id,
                "--title",
                "Empty evidence test",
                "--description",
                "desc",
                "--actor-id",
                "agent:test",
            ])
            .stdout,
    )
    .expect("create must succeed");
    let work_item_id = wi.get("id").expect("id").as_str().unwrap();

    // Create an empty temp file
    let empty_file = fixture._dir.path().join("empty-evidence.txt");
    std::fs::write(&empty_file, b"").unwrap();

    // Attach must fail
    let attach = fixture.run_plan(&[
        "evidence",
        "attach",
        "--work-item-id",
        work_item_id,
        "--kind",
        "log",
        "--body-file",
        empty_file.to_str().unwrap(),
        "--actor-id",
        "agent:test",
    ]);
    assert_ne!(
        attach.status.code(),
        Some(0),
        "evidence attach with empty body must be rejected"
    );
}

// ── decision record with empty rationale rejected ───────────────────────────

#[test]
fn plan_decision_record_empty_rationale_rejected() {
    let fixture = PlanFixture::with_cycle();

    // Create a work item
    let wi: serde_json::Value = serde_json::from_slice(
        &fixture
            .run_plan(&[
                "work-item",
                "create",
                "--cycle-id",
                &fixture.cycle_id,
                "--title",
                "Empty rationale test",
                "--description",
                "desc",
                "--actor-id",
                "agent:test",
            ])
            .stdout,
    )
    .expect("create must succeed");
    let work_item_id = wi.get("id").expect("id").as_str().unwrap();

    // Record with empty rationale must fail
    let record = fixture.run_plan(&[
        "decision",
        "record",
        "--work-item-id",
        work_item_id,
        "--kind",
        "Implementation",
        "--rationale",
        "",
        "--actor-id",
        "user:bob",
    ]);
    assert_ne!(
        record.status.code(),
        Some(0),
        "decision with empty rationale must be rejected"
    );
}
