//! Property tests for gateway redaction idempotence.

use proptest::prelude::*;
use proptest::strategy::{Just, Strategy};
use sddk_gateway::redact;

fn arbitrary_json() -> impl Strategy<Value = serde_json::Value> {
    let leaf = prop_oneof![
        Just(serde_json::Value::Null),
        any::<bool>().prop_map(serde_json::Value::Bool),
        any::<i64>().prop_map(serde_json::Value::from),
        "[a-zA-Z0-9_]{0,12}".prop_map(serde_json::Value::String),
    ];
    leaf.prop_recursive(3, 12, 3, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..4).prop_map(serde_json::Value::Array),
            prop::collection::hash_map("[a-z_]{1,12}", inner, 0..4).prop_map(|map| {
                serde_json::Value::Object(map.into_iter().collect::<serde_json::Map<_, _>>())
            }),
        ]
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn redaction_is_idempotent(value in arbitrary_json()) {
        let once = redact(value.clone());
        let twice = redact(once.clone());
        prop_assert_eq!(once, twice);
    }
}
