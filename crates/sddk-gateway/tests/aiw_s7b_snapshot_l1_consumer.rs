//! AIW-S7b integration tests: a durable `StorageSnapshot` flows through
//! the gateway consumer into the real Secretary L1 engine (G01 + G03),
//! with no live `Storage` handle anywhere in the loop.

use sddk_engine::context_compiler::storage_adapter::StorageSnapshot;
use sddk_engine::{ClosedSetKind, SecretaryId};
use sddk_gateway::storage_snapshot_l1_consumer::{SnapshotConsumerError, SnapshotL1Consumer};

#[test]
fn empty_adapter_id_e2e() {
    let snapshot = StorageSnapshot::from_bytes("", 7, vec![1]);
    let err = SnapshotL1Consumer::new(1000)
        .consume(&snapshot)
        .expect_err("empty adapter_id must reject");
    assert_eq!(
        err,
        SnapshotConsumerError::InvalidSnapshot("empty adapter_id".into())
    );
}

#[test]
fn no_template_e2e() {
    // An engine with no registered template must surface UnknownTemplate
    // as ProposalRejected — a real SecretaryL1Engine rejection, not a
    // gateway-side shortcut.
    let snapshot = StorageSnapshot::from_bytes("storage.ledger_head", 3, vec![]);
    let err = SnapshotL1Consumer::unregistered(1000)
        .consume(&snapshot)
        .expect_err("unregistered template must reject");
    match err {
        SnapshotConsumerError::ProposalRejected(e) => {
            assert!(e.to_string().contains("snapshot-rehydrate"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn durable_snapshot_e2e() {
    let snapshot = StorageSnapshot::from_ledger_events("storage.ledger_head", &[]);
    let proposal = SnapshotL1Consumer::new(1000)
        .with_secretary(SecretaryId::new("sec-e2e"))
        .consume(&snapshot)
        .expect("consume ok");
    assert_eq!(proposal.template_id, "snapshot-rehydrate");
    assert_eq!(proposal.kind, ClosedSetKind::SuggestRehydrationStep);
    assert_eq!(
        proposal.evidence_refs,
        vec!["storage:storage.ledger_head:0".to_string()]
    );
    assert_eq!(proposal.coverage_satisfied, vec!["durable-snapshot"]);
    assert!(proposal.coverage_missing.is_empty());
    assert_eq!(proposal.issued_at_ms, 1000);
}

#[test]
fn empty_log_head_e2e() {
    // Real engine: non-zero head → full confidence and reconciled
    // evidence ref; zero head → halved confidence (G01 observability).
    let durable = StorageSnapshot::from_bytes("storage.ledger_head", 99, vec![7]);
    let p_hi = SnapshotL1Consumer::new(2000)
        .consume(&durable)
        .expect("consume ok");
    assert_eq!(p_hi.confidence, 0.95);
    assert_eq!(
        p_hi.evidence_refs,
        vec!["storage:storage.ledger_head:99".to_string()]
    );

    let empty = StorageSnapshot::from_bytes("storage.ledger_head", 0, vec![]);
    let p_lo = SnapshotL1Consumer::new(2000)
        .consume(&empty)
        .expect("consume ok");
    assert_eq!(p_lo.confidence, 0.5);
    assert_eq!(
        p_lo.evidence_refs,
        vec!["storage:storage.ledger_head:0".to_string()]
    );
}
