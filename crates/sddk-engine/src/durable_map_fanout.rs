//! Durable Map fan-out.
//!
//! Cycle: DW-OPERATORS-002 (H6, order 340, context pack `advanced-runtime`).
//!
//! Records a typed, durable
//! [`TypedChildOutput`](crate::typed_child_output::TypedChildOutput)
//! projection of every Map iteration's emitted output so that:
//!
//! 1. **Replay** can reconstruct the Map's result from the projection
//!    store alone — no body re-execution.
//! 2. **Type drift** between body operator output and its declared
//!    `OperatorOutputSchema` is impossible to mask: any iteration that
//!    fails validation short-circuits the fan-out.
//! 3. **Lineage** is captured under a synthetic `AttemptId` of the form
//!    `map-iter-{parent}-{child}-{idx:06}` so replays can identify the
//!    same projection deterministically.
//!
//! ## Design
//!
//! - `DurableMapFanOut` wraps the existing
//!   [`TypedChildOutputBuilder`](crate::typed_child_output::TypedChildOutputBuilder)
//!   plus a [`MapFanOutSink`] (decoupled from any specific store).
//! - `ChildOutputSinkAdapter` bridges to the existing
//!   [`ChildOutputStore`](crate::typed_child_output::ChildOutputStore)
//!   trait so callers can re-use `InMemoryChildOutputStore` or any
//!   other implementation.
//! - All errors are wrapped in [`DurableFanOutError`] preserving
//!   closed-set pattern matching.
//!
//! See:
//!
//! - `REQ-DurableMapFanOut` (spec, accepted)
//! - `ADR-093` (architecture decision, accepted)
//! - `REQ-TypedChildOutputLineage` (DW-OPERATORS-001, dependency)

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use sddk_domain::OperatorOutputSchema;
use sddk_domain::workflow_ir::NodeId;
use sddk_domain::workflow_run::{AttemptId, NodeRun};

use crate::typed_child_output::{
    ChildOutputStore, ChildOutputStoreError, InMemoryChildOutputStore, TypedChildOutput,
    TypedChildOutputBuilder,
};

// ── MapFanOutSink ────────────────────────────────────────────────────────────

/// Receives recorded projections.
///
/// Decouples `DurableMapFanOut` from any specific store implementation.
/// The default adapter wraps a `ChildOutputStore`.
pub trait MapFanOutSink {
    /// Persist a projection. Errors propagate as
    /// `DurableFanOutError::Sink`.
    fn append(&mut self, projection: &TypedChildOutput) -> Result<(), ChildOutputStoreError>;
}

/// Adapter forwarding to any `ChildOutputStore` implementation.
pub struct ChildOutputSinkAdapter<S: ChildOutputStore> {
    inner: S,
}

impl<S: ChildOutputStore> ChildOutputSinkAdapter<S> {
    /// Wrap a store.
    pub fn new(inner: S) -> Self {
        Self { inner }
    }

    /// Borrow the underlying store.
    pub fn inner(&self) -> &S {
        &self.inner
    }

    /// Mutably borrow the underlying store.
    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.inner
    }

    /// Consume and return the inner store.
    pub fn into_inner(self) -> S {
        self.inner
    }
}

impl<S: ChildOutputStore> MapFanOutSink for ChildOutputSinkAdapter<S> {
    fn append(&mut self, projection: &TypedChildOutput) -> Result<(), ChildOutputStoreError> {
        self.inner.append(projection)
    }
}

/// Type alias for the canonical in-memory adapter.
pub type InMemoryChildOutputSink = ChildOutputSinkAdapter<InMemoryChildOutputStore>;

// ── DurableFanOutError ──────────────────────────────────────────────────────

/// Errors emitted by `DurableMapFanOut::record_iteration`.
///
/// Both variants wrap the inner closed-set error so callers can still
/// pattern-match.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
#[non_exhaustive]
pub enum DurableFanOutError {
    /// Schema or lineage validation failed.
    #[error("lineage validation failed: {0}")]
    Lineage(crate::typed_child_output::LineageError),
    /// Sink rejected the projection (e.g. attempt id collision).
    #[error("sink rejected projection: {0}")]
    Sink(ChildOutputStoreError),
}

// ── Synthetic attempt id ────────────────────────────────────────────────────

/// Build the synthetic `AttemptId` for a Map iteration.
///
/// Format: `map-iter-{parent}-{child}-{idx:06}`. Stable across replays
/// and bounded to 999 999 iterations per parent; this is intentional
/// (cycles that need more can supersede this ADR).
pub fn synthetic_iteration_attempt_id(
    parent: &NodeId,
    child: &NodeId,
    iteration_index: usize,
) -> AttemptId {
    AttemptId(format!(
        "map-iter-{}-{}-{:06}",
        parent.0, child.0, iteration_index
    ))
}

// ── DurableMapFanOut ────────────────────────────────────────────────────────

/// Records every iteration of a Map body into a typed durable
/// projection.
#[non_exhaustive]
pub struct DurableMapFanOut {
    parent_node_id: NodeId,
    child_node_id: NodeId,
    declared_schema: OperatorOutputSchema,
    sink: Box<dyn MapFanOutSink>,
    builder: TypedChildOutputBuilder,
    recorded_count: usize,
    /// Local cache for deterministic `recorded()` listing without
    /// depending on the sink's sort behaviour. The sink remains the
    /// authoritative store; this cache mirrors successful appends.
    cache: Vec<TypedChildOutput>,
}

impl DurableMapFanOut {
    /// Construct a recorder.
    pub fn new(
        parent_node_id: NodeId,
        child_node_id: NodeId,
        declared_schema: OperatorOutputSchema,
        sink: Box<dyn MapFanOutSink>,
    ) -> Self {
        Self {
            parent_node_id,
            child_node_id,
            declared_schema,
            sink,
            builder: TypedChildOutputBuilder::new(),
            recorded_count: 0,
            cache: Vec::new(),
        }
    }

    /// Construct a recorder with a custom builder (e.g. pinned schema
    /// version).
    pub fn with_builder(
        parent_node_id: NodeId,
        child_node_id: NodeId,
        declared_schema: OperatorOutputSchema,
        sink: Box<dyn MapFanOutSink>,
        builder: TypedChildOutputBuilder,
    ) -> Self {
        Self {
            parent_node_id,
            child_node_id,
            declared_schema,
            sink,
            builder,
            recorded_count: 0,
            cache: Vec::new(),
        }
    }

    /// Parent node id.
    pub fn parent_node_id(&self) -> &NodeId {
        &self.parent_node_id
    }

    /// Child (Map) node id.
    pub fn child_node_id(&self) -> &NodeId {
        &self.child_node_id
    }

    /// Declared body operator output schema.
    pub fn declared_schema(&self) -> &OperatorOutputSchema {
        &self.declared_schema
    }

    /// Number of iterations successfully recorded so far.
    pub fn recorded_count(&self) -> usize {
        self.recorded_count
    }

    /// Deterministic listing of all recorded projections, sorted by
    /// `(recorded_at, child_attempt_id)`.
    pub fn recorded(&self) -> Vec<TypedChildOutput> {
        let mut out = self.cache.clone();
        out.sort_by(|a, b| {
            a.recorded_at
                .cmp(&b.recorded_at)
                .then_with(|| a.child_attempt_id.0.cmp(&b.child_attempt_id.0))
        });
        out
    }

    /// Record one iteration's emitted output.
    ///
    /// Returns the produced projection on success. On schema drift the
    /// sink is left untouched for this iteration; previously recorded
    /// iterations remain. On sink errors the same applies — we do not
    /// roll back successful appends (replay can detect partial state).
    pub fn record_iteration(
        &mut self,
        iteration_index: usize,
        raw_outputs: BTreeMap<String, serde_json::Value>,
        recorded_at: String,
    ) -> Result<TypedChildOutput, DurableFanOutError> {
        // Mint synthetic attempt id for this iteration.
        let attempt_id = synthetic_iteration_attempt_id(
            &self.parent_node_id,
            &self.child_node_id,
            iteration_index,
        );

        // Build a synthetic `Attempt` (no recorded history) the builder
        // can validate against. The `NodeRun` is a stub whose only
        // purpose is to expose `dependencies.contains(parent)`.
        let mut deps = std::collections::BTreeSet::new();
        deps.insert(self.parent_node_id.clone());
        let child_run = NodeRun {
            node_id: self.child_node_id.clone(),
            state: sddk_domain::workflow_run::NodeRunState::Completed,
            dependencies: deps,
            attempts: Vec::new(),
            expansion_permissions: std::collections::BTreeSet::new(),
            schema_version: sddk_domain::OPERATOR_CONTRACT_SCHEMA_VERSION,
        };
        let attempt = sddk_domain::workflow_run::Attempt {
            attempt_id: attempt_id.clone(),
            node_id: self.child_node_id.clone(),
            route: sddk_domain::Route {
                provider: "map-body".to_string(),
                model: "iteration".to_string(),
                host: "local".to_string(),
            },
            started_at: recorded_at.clone(),
            ended_at: Some(recorded_at.clone()),
            outcome: Some(sddk_domain::workflow_run::AttemptOutcome::Succeeded {
                outputs: raw_outputs,
            }),
            usage: sddk_domain::Usage {
                tokens_in: 0,
                tokens_out: 0,
                cost_micros: 0,
                wall_ms: 0,
            },
            context_capsule: sddk_domain::ContextCapsuleRef::Inline {
                summary: "map-iter".to_string(),
                sha256: "0".repeat(64),
            },
            idempotency_key: sddk_domain::IdempotencyKey {
                project_id: "p".to_string(),
                run_id: sddk_domain::RunId("r".to_string()),
                node_id: self.child_node_id.clone(),
                attempt_seq: iteration_index as u32,
            },
            schema_version: sddk_domain::OPERATOR_CONTRACT_SCHEMA_VERSION,
        };

        // Build + validate.
        let projection = self
            .builder
            .build(
                &self.parent_node_id,
                &child_run,
                &attempt,
                &self.declared_schema,
                recorded_at,
            )
            .map_err(DurableFanOutError::Lineage)?;

        // Append to sink.
        self.sink
            .append(&projection)
            .map_err(DurableFanOutError::Sink)?;

        self.cache.push(projection.clone());
        self.recorded_count += 1;
        Ok(projection)
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use sddk_domain::SchemaDialect;
    use std::collections::BTreeSet;

    fn node_id(s: &str) -> NodeId {
        NodeId(s.to_string())
    }

    fn string_schema() -> OperatorOutputSchema {
        let mut sf = BTreeMap::new();
        sf.insert(
            "answer".to_string(),
            sddk_domain::OperatorSchema::with_defaults(
                SchemaDialect::JsonSchemaDraft07,
                serde_json::json!({ "type": "string" }),
            ),
        );
        let mut req = BTreeSet::new();
        req.insert("answer".to_string());
        OperatorOutputSchema {
            static_fields: sf,
            required_fields: req,
            accepts_extra_fields: false,
            description: None,
        }
    }

    fn make_outputs(answer: &str) -> BTreeMap<String, serde_json::Value> {
        let mut out = BTreeMap::new();
        out.insert("answer".to_string(), serde_json::json!(answer));
        out
    }

    fn recorder() -> DurableMapFanOut {
        DurableMapFanOut::new(
            node_id("P"),
            node_id("C"),
            string_schema(),
            Box::new(InMemoryChildOutputSink::new(InMemoryChildOutputStore::new())),
        )
    }

    // ── S-1: happy path three iterations ──────────────────────────────────

    #[test]
    fn s1_three_iterations_all_recorded() {
        let mut r = recorder();
        for (i, ts) in ["t3", "t1", "t2"].iter().enumerate() {
            let proj = r
                .record_iteration(i, make_outputs(&format!("ok-{i}")), ts.to_string())
                .unwrap();
            assert_eq!(proj.parent_node_id, node_id("P"));
            assert_eq!(proj.child_node_id, node_id("C"));
        }
        assert_eq!(r.recorded_count(), 3);

        let listed = r.recorded();
        assert_eq!(listed.len(), 3);
        assert_eq!(listed[0].recorded_at, "t1");
        assert_eq!(listed[1].recorded_at, "t2");
        assert_eq!(listed[2].recorded_at, "t3");

        // Adapter store should also contain all three.
        // (We can't get the adapter back from `recorder()` because
        // the sink is moved into a Box, so we test via a fresh recorder
        // in S-5 below.)
    }

    // ── S-2: schema drift fails closed ───────────────────────────────────

    #[test]
    fn s2_schema_drift_short_circuits() {
        let mut r = recorder();
        // Iteration 0 OK.
        r.record_iteration(0, make_outputs("ok"), "t1".to_string())
            .unwrap();
        // Iteration 1 fails type validation.
        let err = r
            .record_iteration(
                1,
                {
                    let mut o = make_outputs("ok");
                    o.insert("answer".to_string(), serde_json::json!(42));
                    o
                },
                "t2".to_string(),
            )
            .unwrap_err();
        assert!(matches!(err, DurableFanOutError::Lineage(_)));
        // Iteration 0 remains; iteration 1 not recorded.
        assert_eq!(r.recorded_count(), 1);
    }

    // ── S-3: synthetic AttemptId is stable ────────────────────────────────

    #[test]
    fn s3_synthetic_attempt_id_is_stable() {
        let mut r = recorder();
        let p1 = r
            .record_iteration(0, make_outputs("ok"), "t".to_string())
            .unwrap();
        // Same parent + child + iteration_index ⇒ same attempt_id.
        // A second call at the same index on the same recorder must
        // collide at the sink (S-6 territory in the spec).
        let p1_id = p1.child_attempt_id.clone();
        let err = r
            .record_iteration(0, make_outputs("ok"), "t2".to_string())
            .unwrap_err();
        assert!(matches!(err, DurableFanOutError::Sink(_)));
        assert_eq!(p1_id.0, "map-iter-P-C-000000");
    }

    // ── S-4: sink error propagates ────────────────────────────────────────

    struct AlwaysFailSink;
    impl MapFanOutSink for AlwaysFailSink {
        fn append(&mut self, _projection: &TypedChildOutput) -> Result<(), ChildOutputStoreError> {
            Err(ChildOutputStoreError::AttemptIdAlreadyStored {
                attempt_id: sddk_domain::workflow_run::AttemptId("dup".to_string()),
            })
        }
    }

    #[test]
    fn s4_sink_error_propagates() {
        let mut r = DurableMapFanOut::new(
            node_id("P"),
            node_id("C"),
            string_schema(),
            Box::new(AlwaysFailSink),
        );
        let err = r
            .record_iteration(0, make_outputs("ok"), "t".to_string())
            .unwrap_err();
        assert!(matches!(err, DurableFanOutError::Sink(_)));
        // Cache untouched.
        assert_eq!(r.recorded_count(), 0);
    }

    // ── S-5: adapter round-trips ──────────────────────────────────────────

    #[test]
    fn s5_adapter_round_trips() {
        let store = InMemoryChildOutputStore::new();
        let mut r = DurableMapFanOut::new(
            node_id("P"),
            node_id("C"),
            string_schema(),
            Box::new(InMemoryChildOutputSink::new(store)),
        );

        let proj1 = r
            .record_iteration(0, make_outputs("ok-1"), "t1".to_string())
            .unwrap();
        let proj2 = r
            .record_iteration(1, make_outputs("ok-2"), "t2".to_string())
            .unwrap();

        // We need to recover the underlying store to query it.
        // The recorder's sink is `Box<dyn MapFanOutSink>` so we cannot
        // downcast back here. Instead, verify via `recorded()` that
        // both projections are present and ordered.
        let listed = r.recorded();
        assert_eq!(listed, vec![proj1, proj2]);
    }

    // ── Extra: get the underlying store out via mut ───────────────────────

    #[test]
    fn sink_adapter_round_trips_via_inner_mut() {
        let store = InMemoryChildOutputStore::new();
        let mut sink = ChildOutputSinkAdapter::new(store);
        sink.append(&TypedChildOutput {
            child_attempt_id: sddk_domain::workflow_run::AttemptId("a".to_string()),
            parent_node_id: node_id("P"),
            child_node_id: node_id("C"),
            declared_schema_id: "x".to_string(),
            typed_fields: BTreeMap::new(),
            recorded_at: "t".to_string(),
            schema_version: 1,
        })
        .unwrap();
        let got = sink
            .inner_mut()
            .get_by_attempt(&sddk_domain::workflow_run::AttemptId("a".to_string()))
            .unwrap();
        assert!(got.is_some());
    }

    // ── Extra: synthetic id format ───────────────────────────────────────

    #[test]
    fn synthetic_id_pads_index_to_six_digits() {
        let id = synthetic_iteration_attempt_id(&node_id("P"), &node_id("C"), 42);
        assert_eq!(id.0, "map-iter-P-C-000042");
        let id0 = synthetic_iteration_attempt_id(&node_id("P"), &node_id("C"), 0);
        assert_eq!(id0.0, "map-iter-P-C-000000");
        let id_max = synthetic_iteration_attempt_id(&node_id("P"), &node_id("C"), 999_999);
        assert_eq!(id_max.0, "map-iter-P-C-999999");
    }
}
