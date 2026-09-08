//! Cross-build determinism test for DecisionRecord serialization.
use sddk_engine::{
    ActionKind, DecisionReason, DecisionVerdict, ProvenanceLink, ProvenanceStep, DecisionRecord,
};

fn make() -> DecisionRecord {
    DecisionRecord::for_test(
        ActionKind::Resume,
        DecisionVerdict::Allow,
        vec![
            DecisionReason::FrontierEmpty,
            DecisionReason::BlockerUnresolved {
                blocker_id: "b-1".into(),
            },
        ],
        vec![
            ProvenanceLink::new(ProvenanceStep::PolicyAdmission, "default", 100),
            ProvenanceLink::new(ProvenanceStep::FrontierProjection, "T-resume", 100),
        ],
    )
}

#[test]
fn determinism_byte_stable() {
    let d = make();
    let s1 = serde_json::to_string(&d).unwrap();
    let s2 = serde_json::to_string(&d).unwrap();
    assert_eq!(s1, s2, "serde_json output must be deterministic");
    // Also: parse it back, re-serialize, compare.
    let parsed: DecisionRecord = serde_json::from_str(&s1).unwrap();
    let s3 = serde_json::to_string(&parsed).unwrap();
    assert_eq!(s1, s3, "round-trip must be idempotent");
}
