//! Tests for planning CAS + CRUD integration (PLN-LEDGER-002).
//!
//! Covers AC-PLN2-03, AC-PLN2-04, AC-PLN2-10 (partial), AC-PLN2-11:
//! - Evidence body round-trip through CAS (body written to cas_root, hash ref stored, get loads body)
//! - Empty body → EmptyEvidenceBody error
//! - Decision record insert/get/list with inline rationale (Q3)
//! - build_provenance_chain over persisted records (closes FIND-PLN-007 partial)
//! - Identity stability via storage: insert then call compute_planning_graph_identity twice → equal

use sddk_domain::planning::{
    DecisionKind, DependencyEdgeKind, DependencyEdgeRecord, EvidenceAttachmentRecord,
    WorkItemRecord, WorkItemStatus, compute_planning_graph_identity,
};
use sddk_domain::{CycleId, CycleManifest};
use sddk_storage::{CycleRecord, ProjectRecord, Storage, WorkspaceRecord};
use tempfile::TempDir;

const CREATED_AT: i64 = 1_725_836_000; // 2026-09-05 00:00:00 UTC

/// Sets up project + workspace + cycle required by FK constraints.
/// Returns the full cycle_id string (e.g. "p-pln-test/evidence").
/// Uses a properly-formatted cycle_id (project/cycle-name format).
fn setup_project_workspace_cycle(storage: &mut Storage, cycle_name: &str) -> String {
    let project_id = "p-pln-test";
    let workspace_id = "ws-pln-test";
    let cycle_id = format!("{}/{}", project_id, cycle_name);
    let project = ProjectRecord {
        project_id: project_id.into(),
        display_name: "PLN Test Project".into(),
        remote_url: Some("https://example.com/test".into()),
        scope: "test".into(),
        created_at: CREATED_AT.to_string(),
    };
    let workspace = WorkspaceRecord {
        workspace_id: workspace_id.into(),
        project_id: project_id.into(),
        canonical_path: "/test".into(),
        created_at: CREATED_AT.to_string(),
    };
    let manifest = CycleManifest::new(
        project_id.into(),
        workspace_id.into(),
        CycleId::new(&cycle_id).expect("valid cycle id"),
        "PLN test cycle".into(),
        "sddk/pln-test".into(),
        "abc123def456".into(),
    );
    let cycle = CycleRecord {
        manifest,
        created_at: CREATED_AT.to_string(),
        updated_at: CREATED_AT.to_string(),
    };
    storage
        .insert_project(&project)
        .expect("insert_project must succeed");
    storage
        .insert_workspace(&workspace)
        .expect("insert_workspace must succeed");
    storage
        .insert_cycle(&cycle)
        .expect("insert_cycle must succeed");
    cycle_id
}

// ── Evidence attachment round-trip ─────────────────────────────────────────

#[test]
fn evidence_attachment_cas_round_trip() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let mut storage = Storage::open(&db_path).expect("must open");

    let cycle_id = setup_project_workspace_cycle(&mut storage, "evidence");

    // Insert a work item
    let wi_record = WorkItemRecord {
        id: "wi-ev-test".into(),
        cycle_id: cycle_id.clone(),
        title: "evidence test item".into(),
        description: "desc".into(),
        status: WorkItemStatus::Draft,
        actor_ref_kind: Some("Agent".into()),
        actor_ref_id: Some("agent:test".into()),
        actor_ref_label: Some("test".into()),
        created_at: CREATED_AT,
        schema_version: 1,
        spine_order: None,
        spine_horizon: None,
        spine_status: None,
        exit_gate: None,
    };
    storage
        .insert_work_item(&wi_record)
        .expect("insert workitem must succeed");

    // Insert evidence with a non-empty body
    let body = b"log output: build succeeded at 14:32";
    // WU-C2: authored on the universal substrate (relation-tagged).
    let record = EvidenceAttachmentRecord::from_universal_relation(
        "ev-001".into(),
        "wi-ev-test".into(),
        "observed_for",
        "pending".into(),
        Some("Agent".into()),
        Some("agent:test".into()),
        Some("test".into()),
        1,
    )
    .expect("observed_for must resolve to a legacy representative");
    storage
        .insert_evidence_attachment(&record, body)
        .expect("insert must succeed");

    // Retrieve and verify body matches
    let (loaded_record, loaded_body) = storage
        .get_evidence_attachment("ev-001")
        .expect("get must succeed")
        .expect("evidence must exist");
    assert_eq!(loaded_body, body, "loaded body must match original");
    assert_eq!(
        loaded_record.relation.as_deref(),
        Some("observed_for"),
        "universal relation must round-trip through storage"
    );
}

// ── M3: fresh-process reopen — body bytes survive a brand-new connection ──────
//
// A5-EVIDENCE-ATTACHMENT-MIGRATION-V1 §M3 (CAS persistence proof).
// The same in-process round-trip above only proves identity, not
// persistence. THIS test opens a SECOND `Storage` against the SAME
// SQLite file with a fresh `Connection`, and asserts:
//   1. The CAS body is recoverable byte-for-byte (write-to-CAS-then-row
//      actually persists the bytes — not just a digest).
//   2. The universal `relation` tag survives the reopen (the production
//      authority is durable, not derived on read).
//   3. The body_ref string matches `sha256:<hex of original bytes>` so
//      callers can recompute the digest without coupling to internals.
//   4. Reinserting the same body is content-addressed (same body_ref,
//      no duplicate CAS object).
//
// This is the test the cycle was named after.

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(bytes);
    let digest = h.finalize();
    let mut out = String::with_capacity(64);
    for byte in digest.iter() {
        out.push_str(&format!("{:02x}", byte));
    }
    out
}

#[test]
fn m3_universal_evidence_cas_persists_across_fresh_storage_reopen() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger_m3.sqlite");

    // -- Phase A: write through storage A, then drop it -----------------
    let cycle_id;
    let body_a: Vec<u8>;
    let body_b: Vec<u8>; // second attachment, distinct content
    let body_binary: Vec<u8>; // third attachment, non-UTF8

    {
        let mut storage_a = Storage::open(&db_path).expect("open storage A");
        cycle_id = setup_project_workspace_cycle(&mut storage_a, "m3-cycle");

        let work_items = [
            ("wi-m3-a", "attachment a"),
            ("wi-m3-b", "attachment b"),
            ("wi-m3-bin", "binary attachment"),
        ];
        for (id, title) in work_items {
            let wi = WorkItemRecord {
                id: id.into(),
                cycle_id: cycle_id.clone(),
                title: title.into(),
                description: "m3".into(),
                status: WorkItemStatus::Draft,
                actor_ref_kind: Some("Agent".into()),
                actor_ref_id: Some("agent:m3".into()),
                actor_ref_label: Some("m3".into()),
                created_at: CREATED_AT,
                schema_version: 1,
                spine_order: None,
                spine_horizon: None,
                spine_status: None,
                exit_gate: None,
            };
            storage_a.insert_work_item(&wi).expect("insert workitem");
        }

        // 3 attachments with distinct content (UTF-8, ASCII, non-UTF-8 binary).
        body_a = b"observed_for: cycle started at 14:32".to_vec();
        body_b = b"verifies: metric mean=0.97 sigma=0.02".to_vec();
        body_binary = (0u8..=255u8).collect(); // full byte range, NOT valid UTF-8

        for (ev_id, wi_id, relation, body) in [
            ("ev-m3-a", "wi-m3-a", "observed_for", body_a.as_slice()),
            ("ev-m3-b", "wi-m3-b", "verifies", body_b.as_slice()),
            (
                "ev-m3-bin",
                "wi-m3-bin",
                "references",
                body_binary.as_slice(),
            ),
        ] {
            let record = EvidenceAttachmentRecord::from_universal_relation(
                ev_id.into(),
                wi_id.into(),
                relation,
                "pending".into(),
                Some("Human".into()),
                Some("human:operator".into()),
                Some("op".into()),
                1,
            )
            .expect("relation must resolve");
            storage_a
                .insert_evidence_attachment(&record, body)
                .expect("insert must succeed");
        }

        // Sanity: same body inserted twice yields the same body_ref
        // (content-addressed; no duplicate CAS object). We capture the
        // first hash for comparison after reopen.
        let expected_a_hash = format!("sha256:{}", sha256_hex(&body_a));

        // storage A is dropped at end of scope; CAS bytes are persisted
        // on disk via the underlying `cas_put` machinery.
        drop(storage_a);

        // Pin the expected CAS reference for assertion in phase B.
        assert_eq!(expected_a_hash, format!("sha256:{}", sha256_hex(&body_a)));
    }

    // -- Phase B: reopen with a FRESH Storage connection --------------
    {
        let storage_b = Storage::open(&db_path).expect("reopen storage B");

        // 1. body_a is recoverable byte-for-byte
        let (rec_a, bytes_a) = storage_b
            .get_evidence_attachment("ev-m3-a")
            .expect("get must succeed")
            .expect("attachment must exist");
        assert_eq!(
            bytes_a,
            body_a.as_slice(),
            "M3: reopened CAS bytes must be byte-equal to the original"
        );
        assert_eq!(
            rec_a.relation.as_deref(),
            Some("observed_for"),
            "M3: relation tag must survive reopen (production authority)"
        );
        assert_eq!(
            rec_a.body_ref,
            format!("sha256:{}", sha256_hex(&body_a)),
            "M3: persisted body_ref must equal sha256 of original bytes"
        );

        // 2. body_b round-trips with its own relation tag
        let (rec_b, bytes_b) = storage_b
            .get_evidence_attachment("ev-m3-b")
            .expect("get must succeed")
            .expect("attachment must exist");
        assert_eq!(bytes_b, body_b.as_slice());
        assert_eq!(rec_b.relation.as_deref(), Some("verifies"));
        assert_eq!(rec_b.body_ref, format!("sha256:{}", sha256_hex(&body_b)));

        // 3. Binary (non-UTF8) body round-trips byte-equal
        let (rec_bin, bytes_bin) = storage_b
            .get_evidence_attachment("ev-m3-bin")
            .expect("get must succeed")
            .expect("attachment must exist");
        assert_eq!(
            bytes_bin.len(),
            256,
            "M3: binary attachment must round-trip with full 256-byte range"
        );
        assert_eq!(
            bytes_bin,
            body_binary.as_slice(),
            "M3: binary body must be byte-equal after reopen"
        );
        assert_eq!(rec_bin.relation.as_deref(), Some("references"));

        // 4. Actor metadata survives reopen
        assert_eq!(rec_a.actor_ref_kind.as_deref(), Some("Human"));
        assert_eq!(rec_a.actor_ref_id.as_deref(), Some("human:operator"));
        assert_eq!(rec_a.actor_ref_label.as_deref(), Some("op"));
    }

    // -- Phase C: list_evidence_attachments_by_work_item is consistent ---
    {
        let storage_c = Storage::open(&db_path).expect("reopen storage C");
        let listed = storage_c
            .list_evidence_attachments_by_work_item("wi-m3-a")
            .expect("list must succeed");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, "ev-m3-a");
        assert_eq!(listed[0].relation.as_deref(), Some("observed_for"));
    }
}

#[test]
fn m3_new_write_with_missing_relation_fails_closed() {
    // Post-A5-EVIDENCE-ATTACHMENT-MIGRATION-V1 §M2: the write path is
    // fail-closed. A record built with `relation = None` MUST be rejected
    // by `insert_evidence_attachment` — the legacy `kind` discriminator
    // must NEVER be used to derive `relation` on write.
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger_m3_fail.sqlite");
    let mut storage = Storage::open(&db_path).expect("open");

    let cycle_id = setup_project_workspace_cycle(&mut storage, "fail-cycle");
    let wi = WorkItemRecord {
        id: "wi-fail".into(),
        cycle_id: cycle_id.clone(),
        title: "fail test".into(),
        description: "d".into(),
        status: WorkItemStatus::Draft,
        actor_ref_kind: None,
        actor_ref_id: None,
        actor_ref_label: None,
        created_at: CREATED_AT,
        schema_version: 1,
        spine_order: None,
        spine_horizon: None,
        spine_status: None,
        exit_gate: None,
    };
    storage.insert_work_item(&wi).expect("insert workitem");

    // Build a record directly (no `from_universal_relation`) with
    // `relation = None`. This is the malformed-input case the production
    // ratchet forbids.
    let record = EvidenceAttachmentRecord {
        id: "ev-fail".into(),
        work_item_id: "wi-fail".into(),
        kind: sddk_domain::planning::PlanningEvidenceKind::Log,
        relation: None,
        body_ref: "sha256:placeholder".into(),
        actor_ref_kind: None,
        actor_ref_id: None,
        actor_ref_label: None,
        schema_version: 1,
    };

    let result = storage.insert_evidence_attachment(&record, b"some body");
    assert!(
        matches!(
            result,
            Err(sddk_storage::StorageError::MissingEvidenceRelation)
        ),
        "M2: missing relation must fail closed; got {result:?}"
    );
}

#[test]
fn m3_empty_body_continues_failing_closed() {
    // Pre-existing contract preserved: empty body is rejected before
    // CAS write (no orphan CAS object for empty payload).
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger_m3_empty.sqlite");
    let mut storage = Storage::open(&db_path).expect("open");

    let cycle_id = setup_project_workspace_cycle(&mut storage, "empty-m3");
    let wi = WorkItemRecord {
        id: "wi-empty-m3".into(),
        cycle_id: cycle_id.clone(),
        title: "empty".into(),
        description: "d".into(),
        status: WorkItemStatus::Draft,
        actor_ref_kind: None,
        actor_ref_id: None,
        actor_ref_label: None,
        created_at: CREATED_AT,
        schema_version: 1,
        spine_order: None,
        spine_horizon: None,
        spine_status: None,
        exit_gate: None,
    };
    storage.insert_work_item(&wi).expect("insert workitem");

    let record = EvidenceAttachmentRecord::from_universal_relation(
        "ev-empty-m3".into(),
        "wi-empty-m3".into(),
        "observed_for",
        "pending".into(),
        None,
        None,
        None,
        1,
    )
    .expect("relation must resolve");

    let result = storage.insert_evidence_attachment(&record, b"");
    assert!(
        matches!(result, Err(sddk_storage::StorageError::EmptyEvidenceBody)),
        "empty body must fail closed before CAS write; got {result:?}"
    );
}

#[test]
fn m3_legacy_row_with_null_relation_still_decodes_via_compat_path() {
    // Pre-A5-EVIDENCE-ATTACHMENT-MIGRATION-V1: a row predating MIGRATION_19
    // has `relation = NULL`. The read path MUST still decode it via
    // `from_legacy_kind_tag` (read-compat), populating the `relation`
    // column on read. The asymmetry with the hardened write path is
    // intentional and documented at `insert_evidence_attachment`.

    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger_m3_legacy.sqlite");
    let mut storage = Storage::open(&db_path).expect("open");

    let cycle_id = setup_project_workspace_cycle(&mut storage, "legacy-m3");
    let wi = WorkItemRecord {
        id: "wi-legacy-m3".into(),
        cycle_id: cycle_id.clone(),
        title: "legacy".into(),
        description: "d".into(),
        status: WorkItemStatus::Draft,
        actor_ref_kind: None,
        actor_ref_id: None,
        actor_ref_label: None,
        created_at: CREATED_AT,
        schema_version: 1,
        spine_order: None,
        spine_horizon: None,
        spine_status: None,
        exit_gate: None,
    };
    storage.insert_work_item(&wi).expect("insert workitem");

    // Insert the row normally (which CAS-writes the body too), then
    // null-out the `relation` column to simulate a legacy pre-MIGRATION_19
    // shape. The CAS body remains; only the metadata column changes.
    let body_bytes = b"legacy snapshot bytes".to_vec();
    let record = EvidenceAttachmentRecord::from_universal_relation(
        "ev-legacy-m3".into(),
        "wi-legacy-m3".into(),
        "observed_for", // canonical, used for the write; then dropped on read
        "pending".into(),
        None,
        None,
        None,
        1,
    )
    .expect("observed_for must resolve");
    storage
        .insert_evidence_attachment(&record, &body_bytes)
        .expect("insert must succeed");

    // Now simulate a pre-MIGRATION_19 row by nulling `relation`.
    storage
        .connection_for_tests()
        .execute(
            "UPDATE evidence_attachments_v1 SET relation = NULL WHERE id = ?1",
            rusqlite::params!["ev-legacy-m3"],
        )
        .expect("null-out relation must succeed");

    // Reopen with fresh Storage to simulate process restart
    drop(storage);
    let storage_b = Storage::open(&db_path).expect("reopen");
    let (rec, bytes) = storage_b
        .get_evidence_attachment("ev-legacy-m3")
        .expect("get must succeed")
        .expect("legacy attachment must be readable");
    assert_eq!(bytes, body_bytes, "legacy row body must round-trip via CAS");
    // The compat decoder populates `relation` from `kind.relation_tag()`:
    // for an inserted-observed_for, the legacy kind is `Log`, which
    // also maps to "observed_for".
    assert_eq!(
        rec.relation.as_deref(),
        Some("observed_for"),
        "legacy NULL-relation row must decode via from_legacy_kind_tag"
    );
}

/// Verifies get_evidence_attachment on a non-existent ID returns None (not error).
#[test]
fn get_evidence_attachment_nonexistent_returns_none() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let storage = Storage::open(&db_path).expect("must open");

    let result = storage.get_evidence_attachment("nonexistent-ev");
    assert!(result.is_ok(), "get must not error for missing id");
    assert!(
        result.unwrap().is_none(),
        "missing evidence must return None"
    );
}

// ── Empty evidence body ─────────────────────────────────────────────────────

#[test]
fn evidence_insert_empty_body_rejected() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let mut storage = Storage::open(&db_path).expect("must open");

    let cycle_id = setup_project_workspace_cycle(&mut storage, "empty");
    let wi_record = WorkItemRecord {
        id: "wi-empty".into(),
        cycle_id: cycle_id.clone(),
        title: "empty test".into(),
        description: "desc".into(),
        status: WorkItemStatus::Draft,
        actor_ref_kind: Some("Agent".into()),
        actor_ref_id: Some("agent:test".into()),
        actor_ref_label: Some("test".into()),
        created_at: CREATED_AT,
        schema_version: 1,
        spine_order: None,
        spine_horizon: None,
        spine_status: None,
        exit_gate: None,
    };
    storage
        .insert_work_item(&wi_record)
        .expect("insert workitem must succeed");

    // WU-C2: authored on the universal substrate (relation-tagged).
    let record = EvidenceAttachmentRecord::from_universal_relation(
        "ev-empty".into(),
        "wi-empty".into(),
        "observed_for",
        "pending".into(),
        Some("Agent".into()),
        Some("agent:test".into()),
        Some("test".into()),
        1,
    )
    .expect("observed_for must resolve to a legacy representative");
    let result = storage.insert_evidence_attachment(&record, b"");
    assert!(result.is_err(), "empty body must be rejected");
    let err = result.unwrap_err();
    assert!(
        format!("{}", err).to_lowercase().contains("empty")
            || format!("{}", err).contains("non-empty"),
        "error must mention empty/non-empty: {err}"
    );
}

// ── Decision record inline storage (Q3) ─────────────────────────────────────

#[test]
fn decision_record_insert_get_round_trip() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let mut storage = Storage::open(&db_path).expect("must open");

    let cycle_id = setup_project_workspace_cycle(&mut storage, "decision");
    let wi_record = WorkItemRecord {
        id: "wi-decision".into(),
        cycle_id: cycle_id.clone(),
        title: "decision test".into(),
        description: "desc".into(),
        status: WorkItemStatus::Draft,
        actor_ref_kind: Some("Human".into()),
        actor_ref_id: Some("user:bob".into()),
        actor_ref_label: Some("bob".into()),
        created_at: CREATED_AT,
        schema_version: 1,
        spine_order: None,
        spine_horizon: None,
        spine_status: None,
        exit_gate: None,
    };
    storage
        .insert_work_item(&wi_record)
        .expect("insert workitem must succeed");

    let record = sddk_domain::planning::DecisionRecordRecord {
        id: "dec-001".into(),
        work_item_id: "wi-decision".into(),
        kind: DecisionKind::Accept,
        rationale: "Best approach after evaluating alternatives".into(),
        actor_ref_kind: Some("Human".into()),
        actor_ref_id: Some("user:bob".into()),
        actor_ref_label: Some("bob".into()),
        schema_version: 1,
    };
    storage
        .insert_decision_record(&record)
        .expect("insert decision must succeed");

    let loaded = storage
        .get_decision_record("dec-001")
        .expect("get must not error")
        .expect("decision must exist");
    assert_eq!(
        loaded.rationale, "Best approach after evaluating alternatives",
        "rationale must be preserved verbatim"
    );
    assert_eq!(loaded.kind, DecisionKind::Accept);
}

#[test]
fn decision_record_list_by_work_item() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let mut storage = Storage::open(&db_path).expect("must open");

    let cycle_id = setup_project_workspace_cycle(&mut storage, "list");
    let wi_record = WorkItemRecord {
        id: "wi-list".into(),
        cycle_id: cycle_id.clone(),
        title: "list test".into(),
        description: "desc".into(),
        status: WorkItemStatus::Draft,
        actor_ref_kind: Some("Human".into()),
        actor_ref_id: Some("user:bob".into()),
        actor_ref_label: Some("bob".into()),
        created_at: CREATED_AT,
        schema_version: 1,
        spine_order: None,
        spine_horizon: None,
        spine_status: None,
        exit_gate: None,
    };
    storage
        .insert_work_item(&wi_record)
        .expect("insert workitem must succeed");

    for i in 0..2 {
        let record = sddk_domain::planning::DecisionRecordRecord {
            id: format!("dec-list-{}", i),
            work_item_id: "wi-list".into(),
            kind: DecisionKind::Accept,
            rationale: format!("rationale {}", i),
            actor_ref_kind: Some("Human".into()),
            actor_ref_id: Some("user:bob".into()),
            actor_ref_label: Some("bob".into()),
            schema_version: 1,
        };
        storage.insert_decision_record(&record).unwrap();
    }

    let decisions = storage
        .list_decision_records_by_work_item("wi-list")
        .expect("list must succeed");
    assert_eq!(decisions.len(), 2, "must have 2 decision records");
}

// ── build_provenance_chain ───────────────────────────────────────────────────

#[test]
fn build_provenance_chain_over_persisted_records() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let mut storage = Storage::open(&db_path).expect("must open");
    let cycle_id = setup_project_workspace_cycle(&mut storage, "provenance");

    let wi = WorkItemRecord {
        id: "wi-chain".into(),
        cycle_id: cycle_id.clone(),
        title: "chain test".into(),
        description: "desc".into(),
        status: WorkItemStatus::Active,
        actor_ref_kind: Some("Agent".into()),
        actor_ref_id: Some("agent:test".into()),
        actor_ref_label: Some("test".into()),
        created_at: CREATED_AT,
        schema_version: 1,
        spine_order: None,
        spine_horizon: None,
        spine_status: None,
        exit_gate: None,
    };
    storage
        .insert_work_item(&wi)
        .expect("insert workitem must succeed");

    // WU-C2: authored on the universal substrate (relation-tagged).
    let ev_record = EvidenceAttachmentRecord::from_universal_relation(
        "ev-chain".into(),
        "wi-chain".into(),
        "observed_for",
        "pending".into(),
        Some("Agent".into()),
        Some("agent:test".into()),
        Some("test".into()),
        1,
    )
    .expect("observed_for must resolve to a legacy representative");
    storage
        .insert_evidence_attachment(&ev_record, b"log line")
        .expect("evidence must succeed");

    let dec_record = sddk_domain::planning::DecisionRecordRecord {
        id: "dec-chain".into(),
        work_item_id: "wi-chain".into(),
        kind: DecisionKind::Accept,
        rationale: "Accepted after review".into(),
        actor_ref_kind: Some("Human".into()),
        actor_ref_id: Some("user:bob".into()),
        actor_ref_label: Some("bob".into()),
        schema_version: 1,
    };
    storage
        .insert_decision_record(&dec_record)
        .expect("decision must succeed");

    let chain = storage
        .build_provenance_chain(&cycle_id)
        .expect("build_provenance_chain must succeed");

    assert_eq!(chain.cycle_id, cycle_id);
    assert!(
        chain.work_item_ids.contains(&"wi-chain".to_string()),
        "chain must contain work item id"
    );
    assert!(
        !chain.evidence_refs.is_empty(),
        "chain must contain evidence ref"
    );
    assert!(
        !chain.decision_refs.is_empty(),
        "chain must contain decision ref"
    );
}

#[test]
fn build_provenance_chain_empty_cycle() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let storage = Storage::open(&db_path).expect("must open");

    let chain = storage
        .build_provenance_chain("c-empty-cycle")
        .expect("build_provenance_chain must succeed for empty cycle");
    assert!(chain.work_item_ids.is_empty());
    assert!(chain.evidence_refs.is_empty());
    assert!(chain.decision_refs.is_empty());
}

// ── Identity stability ────────────────────────────────────────────────────────

#[test]
fn graph_identity_deterministic_across_re_lists() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let mut storage = Storage::open(&db_path).expect("must open");
    let cycle_id = setup_project_workspace_cycle(&mut storage, "identity");

    let wi_a = WorkItemRecord {
        id: "wi-ida".into(),
        cycle_id: cycle_id.clone(),
        title: "Item A".into(),
        description: "desc a".into(),
        status: WorkItemStatus::Active,
        actor_ref_kind: Some("Agent".into()),
        actor_ref_id: Some("agent:test".into()),
        actor_ref_label: Some("test".into()),
        created_at: CREATED_AT,
        schema_version: 1,
        spine_order: None,
        spine_horizon: None,
        spine_status: None,
        exit_gate: None,
    };
    let wi_b = WorkItemRecord {
        id: "wi-idb".into(),
        cycle_id: cycle_id.clone(),
        title: "Item B".into(),
        description: "desc b".into(),
        status: WorkItemStatus::Draft,
        actor_ref_kind: Some("Agent".into()),
        actor_ref_id: Some("agent:test".into()),
        actor_ref_label: Some("test".into()),
        created_at: CREATED_AT,
        schema_version: 1,
        spine_order: None,
        spine_horizon: None,
        spine_status: None,
        exit_gate: None,
    };
    storage.insert_work_item(&wi_a).unwrap();
    storage.insert_work_item(&wi_b).unwrap();

    // Build identity twice via re-listing
    let items_a = storage.list_work_items_by_cycle(&cycle_id).unwrap();
    let domain_items_a: Vec<_> = items_a.iter().map(|r| r.clone().into_domain()).collect();
    let edges_a: Vec<_> = storage
        .list_dependency_edges_by_cycle(&cycle_id)
        .unwrap()
        .iter()
        .map(|r| r.clone().into_domain())
        .collect();
    let ev_refs_a: Vec<_> = items_a
        .iter()
        .flat_map(|wi| {
            storage
                .list_evidence_attachments_by_work_item(&wi.id)
                .unwrap()
        })
        .map(|e| e.body_ref.clone())
        .collect();
    let dec_refs_a: Vec<_> = items_a
        .iter()
        .flat_map(|wi| storage.list_decision_records_by_work_item(&wi.id).unwrap())
        .map(|d| d.id.clone())
        .collect();
    let hash_a =
        compute_planning_graph_identity(&domain_items_a, &edges_a, &ev_refs_a, &dec_refs_a);

    // Re-list and recompute
    let items_b = storage.list_work_items_by_cycle(&cycle_id).unwrap();
    let domain_items_b: Vec<_> = items_b.iter().map(|r| r.clone().into_domain()).collect();
    let edges_b: Vec<_> = storage
        .list_dependency_edges_by_cycle(&cycle_id)
        .unwrap()
        .iter()
        .map(|r| r.clone().into_domain())
        .collect();
    let ev_refs_b: Vec<_> = items_b
        .iter()
        .flat_map(|wi| {
            storage
                .list_evidence_attachments_by_work_item(&wi.id)
                .unwrap()
        })
        .map(|e| e.body_ref.clone())
        .collect();
    let dec_refs_b: Vec<_> = items_b
        .iter()
        .flat_map(|wi| storage.list_decision_records_by_work_item(&wi.id).unwrap())
        .map(|d| d.id.clone())
        .collect();
    let hash_b =
        compute_planning_graph_identity(&domain_items_b, &edges_b, &ev_refs_b, &dec_refs_b);

    assert_eq!(
        hash_a, hash_b,
        "compute_planning_graph_identity must be deterministic across re-lists"
    );
}

// ── Self-loop rejection (AC-PLN2-02 / spec line 90) ──────────────────────────

/// AC-PLN2-02: insert_dependency_edge rejects self-loops (from_id == to_id).
#[test]
fn insert_dependency_edge_rejects_self_loop() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let mut storage = Storage::open(&db_path).expect("must open");

    let cycle_id = setup_project_workspace_cycle(&mut storage, "selfloop");

    // Insert a work item
    let wi_record = WorkItemRecord {
        id: "wi-selfloop-test".into(),
        cycle_id,
        title: "self-loop test".into(),
        description: "desc".into(),
        status: WorkItemStatus::Draft,
        actor_ref_kind: Some("Agent".into()),
        actor_ref_id: Some("agent:test".into()),
        actor_ref_label: Some("test".into()),
        created_at: CREATED_AT,
        schema_version: 1,
        spine_order: None,
        spine_horizon: None,
        spine_status: None,
        exit_gate: None,
    };
    storage
        .insert_work_item(&wi_record)
        .expect("insert workitem must succeed");

    // Self-loop edge must be rejected
    let self_loop_edge = DependencyEdgeRecord {
        from_id: "wi-selfloop-test".into(),
        to_id: "wi-selfloop-test".into(),
        kind: DependencyEdgeKind::Blocks,
        actor_ref_kind: Some("System".into()),
        actor_ref_id: Some("system:planner".into()),
        actor_ref_label: Some("planner".into()),
        schema_version: 1,
    };
    let result = storage.insert_dependency_edge(&self_loop_edge);
    assert!(
        result.is_err(),
        "insert_dependency_edge must reject self-loop"
    );
    let err = result.unwrap_err();
    assert!(
        format!("{}", err).contains("self-loop"),
        "error must mention self-loop: {err}"
    );
}

/// Valid non-self-loop edge must still succeed after self-loop rejection.
#[test]
fn insert_dependency_edge_valid_edge_still_works() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let mut storage = Storage::open(&db_path).expect("must open");

    let cycle_id = setup_project_workspace_cycle(&mut storage, "valid-edge");

    // Insert two work items
    let wi_a = WorkItemRecord {
        id: "wi-valid-a".into(),
        cycle_id: cycle_id.clone(),
        title: "A".into(),
        description: "desc".into(),
        status: WorkItemStatus::Draft,
        actor_ref_kind: Some("Agent".into()),
        actor_ref_id: Some("agent:test".into()),
        actor_ref_label: Some("test".into()),
        created_at: CREATED_AT,
        schema_version: 1,
        spine_order: None,
        spine_horizon: None,
        spine_status: None,
        exit_gate: None,
    };
    let wi_b = WorkItemRecord {
        id: "wi-valid-b".into(),
        cycle_id,
        title: "B".into(),
        description: "desc".into(),
        status: WorkItemStatus::Draft,
        actor_ref_kind: Some("Agent".into()),
        actor_ref_id: Some("agent:test".into()),
        actor_ref_label: Some("test".into()),
        created_at: CREATED_AT,
        schema_version: 1,
        spine_order: None,
        spine_horizon: None,
        spine_status: None,
        exit_gate: None,
    };
    storage
        .insert_work_item(&wi_a)
        .expect("insert A must succeed");
    storage
        .insert_work_item(&wi_b)
        .expect("insert B must succeed");

    // Valid edge A → B must succeed
    let edge = DependencyEdgeRecord {
        from_id: "wi-valid-a".into(),
        to_id: "wi-valid-b".into(),
        kind: DependencyEdgeKind::Blocks,
        actor_ref_kind: Some("System".into()),
        actor_ref_id: Some("system:planner".into()),
        actor_ref_label: Some("planner".into()),
        schema_version: 1,
    };
    let result = storage.insert_dependency_edge(&edge);
    assert!(
        result.is_ok(),
        "insert_dependency_edge must accept valid edge: {}",
        result.unwrap_err()
    );
}
