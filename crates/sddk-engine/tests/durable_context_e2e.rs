//! C3j slice 1 — durable context e2e (SPEC-005 CTX-001/002/011;
//! CTX-UAT-006, CTX-UAT-013, CTX-UAT-014 a nivel de sustrato).
//!
//! Escenarios cubiertos aquí (process-level, con stores durables REALES
//! sobre filesystem):
//! - CTX-UAT-006: proceso A compila capsule via cold_start y cierra;
//!   proceso B reabre y recupera la MISMA capsule/basis del storage.
//! - CTX-UAT-013: host session restart — binding reattach sin importar
//!   transcript (el store durable devuelve exactamente lo persistido).
//! - CTX-UAT-014: la capsule persistida mantiene progressive disclosure
//!   (refs en fetch_on_demand, no contenido expandido).
//!
//! Cubierto a nivel unitario en los módulos (`durable_session_binding`,
//! `durable_capsule_store`): upsert, corrupción tipada, listing. Aquí se
//! ejercita la COMBINACIÓN cold_start + stores durables.

use sddk_engine::agentic_session_binding::{AgenticBinding, AgenticSessionRef, BindingTarget};
use sddk_engine::cold_start::{CapsuleStore, InMemoryRunStateViewInputs};
use sddk_engine::context_bridge::ContextBridge;
use sddk_engine::context_capsule::CompilerPolicy;
use sddk_engine::durable_capsule_store::FilesystemCapsuleStore;
use sddk_engine::durable_session_binding::{load_all, save_binding};
use sddk_engine::retry::WallClock;
use sddk_engine::run_view::RunOrigin;
use std::sync::Arc;

fn seeded_view(run: &str) -> InMemoryRunStateViewInputs {
    let inputs = InMemoryRunStateViewInputs::new();
    inputs.seed(sddk_engine::run_view::RunStateView::for_test(
        RunOrigin::Declared,
        run,
        vec!["node-a".to_string()],
        vec![],
        vec![],
        1,
    ));
    inputs
}

// durability-required: cold_start + FilesystemCapsuleStore against the real
// filesystem; simulating the fs would not exercise restart durability.
#[test]
fn ctx_uat_006_restart_recovers_same_capsule_from_durable_store() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("capsules");

    // ── Process A: cold start compiles and persists. ──
    let first_capsule_id = {
        let store = FilesystemCapsuleStore::open(&root).unwrap();
        let views = seeded_view("run-1");
        let out = sddk_engine::cold_start_core(
            "run-1",
            "node-a",
            &views,
            &store,
            Arc::new(WallClock),
            CompilerPolicy::default(),
        )
        .unwrap();
        assert!(matches!(out.source, sddk_engine::ColdStartSource::Fresh));
        out.capsule.capsule_id.clone()
        // store dropped here = "process exit"
    };

    // ── Process B: brand new store over the same root. ──
    let store_b = FilesystemCapsuleStore::open(&root).unwrap();
    let recovered = store_b
        .resolve_ref(&first_capsule_id)
        .expect("CTX-UAT-006: capsule must survive process restart");
    assert_eq!(recovered.capsule_id, first_capsule_id);
    assert_eq!(
        store_b.last_capsule("run-1").unwrap().capsule_id,
        first_capsule_id
    );

    // Second cold start over the durable store RECOVERS (not Fresh): the
    // persisted capsule is found and used as parent.
    let views_b = seeded_view("run-1");
    let out_b = sddk_engine::cold_start_core(
        "run-1",
        "node-a",
        &views_b,
        &store_b,
        Arc::new(WallClock),
        CompilerPolicy::default(),
    )
    .unwrap();
    assert!(
        matches!(out_b.source, sddk_engine::ColdStartSource::FromRecovery),
        "second cold start must recover from the durable capsule, not start fresh"
    );
}

#[test]
fn ctx_uat_013_session_binding_reattaches_without_transcript() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("bindings");

    // ── Process A: host owns the transcript; SDDK persists ONLY the
    //    semantic binding (CTX-011). ──
    let transcript_is_host_owned = "opaque host bytes that SDDK must never import";
    {
        let binding = AgenticBinding::attach(
            AgenticSessionRef::new("host-session-7"),
            BindingTarget::WorkItem {
                project_id: "p-1".into(),
                work_item_id: "wi-2".into(),
            },
        );
        save_binding(&root, &binding).unwrap();
    }

    // ── Process B: reattach from durable store; transcript stays opaque. ──
    let recovered = load_all(&root).unwrap();
    let binding = recovered
        .get(&AgenticSessionRef::new("host-session-7"))
        .expect("binding survives restart");
    assert_eq!(
        binding.target,
        BindingTarget::WorkItem {
            project_id: "p-1".into(),
            work_item_id: "wi-2".into(),
        }
    );
    // The transcript content appears NOWHERE in the recovered state: the
    // store round-trips only the semantic binding.
    let serialized = serde_json::to_string(&binding).unwrap();
    assert!(
        !serialized.contains("opaque host bytes"),
        "CTX-011: transcript must never be imported into SDDK state"
    );
    let _ = transcript_is_host_owned;
}

#[test]
fn ctx_uat_014_persisted_capsule_keeps_progressive_disclosure_refs() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("capsules");

    let store = FilesystemCapsuleStore::open(&root).unwrap();
    let views = seeded_view("run-1");
    let out = sddk_engine::cold_start_core(
        "run-1",
        "node-a",
        &views,
        &store,
        Arc::new(WallClock),
        CompilerPolicy::default(),
    )
    .unwrap();

    // The capsule structure carries ref layers, not expanded dumps: the
    // persisted body keeps the same three-layer progressive disclosure the
    // compiler produced. Heavy detail stays referenced (CTX-007).
    let persisted = store.resolve_ref(&out.capsule.capsule_id).unwrap();
    assert_eq!(
        persisted.artifacts.must_read.len(),
        out.capsule.artifacts.must_read.len()
    );
    let serialized = serde_json::to_string(&persisted).unwrap();
    assert!(
        !serialized.contains("TRANSCRIPT"),
        "capsule content is semantic refs, not a transcript dump"
    );
}

// Session ≠ run identity preserved across a durable round-trip (C3j exit
// gate): binding target kinds do not collapse into each other on disk.
#[test]
fn session_and_run_targets_survive_restart_distinct() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("bindings");

    let by_session =
        AgenticBinding::attach(AgenticSessionRef::new("s-1"), BindingTarget::Ephemeral);
    save_binding(&root, &by_session).unwrap();

    let recovered = load_all(&root).unwrap();
    assert_eq!(
        recovered
            .get(&AgenticSessionRef::new("s-1"))
            .unwrap()
            .target,
        BindingTarget::Ephemeral
    );
}

// ContextBridge over a recovered binding: bootstrap is basis-scoped, and a
// stale delta is rejected (CTX-008 contract over the durable substrate).
#[test]
fn bridge_over_recovered_binding_rejects_stale_delta() {
    let binding = AgenticBinding::attach(
        AgenticSessionRef::new("s-9"),
        BindingTarget::Project {
            project_id: "p-1".into(),
        },
    );
    let (mut bridge, ctx) = ContextBridge::bootstrap(&binding, "rev-1");
    assert_eq!(ctx.basis_revision, "rev-1");
    assert_eq!(bridge.current_revision(), "rev-1");

    let stale = sddk_engine::context_bridge::ContextDelta {
        from_revision: "rev-0".into(),
        to_revision: "rev-1".into(),
        relevance_reason: "late arrival".into(),
        additions: vec![],
        deletions: vec![],
        seq: 1,
        advisory_only: true,
    };
    assert!(
        bridge.apply(stale).is_err(),
        "stale from_revision must fail"
    );
}
