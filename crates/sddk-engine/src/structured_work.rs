//! Structured Agent Work: AgentWorkRequest → run_structured →
//! ContributionV2 (arch-spec-027, J6).
//!
//! Work orquestado con contratos tipados: sin parsing de markdown
//! como protocolo (SAW-001/002), output inválido distinguible y
//! nunca "success" fabricado (SAW-003), mismo adapter para companion
//! y orchestrated (SAW-004), contribution ≠ authority (SAW-005),
//! receipt de ejecución (SAW-006).
//!
//! Upstream: `docs/architecture/specs/arch-spec-027-structured-agent-work.md`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Typed request for orchestrated work (SAW-001). Host-agnostic:
/// references, no host types embedded.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentWorkRequest {
    pub request_id: String,
    /// Task reference (spec/task id), not inline markdown protocol.
    pub task_ref: String,
    /// Context basis references (e.g. basis revision, deltas).
    pub context_refs: Vec<String>,
    /// Policy references (governance/verification policy ids).
    pub policy_refs: Vec<String>,
    /// Return contract: the schema the host must satisfy (SAW-002).
    pub return_schema: ReturnSchema,
}

/// Explicit schema-constrained return contract (SAW-002).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReturnSchema {
    /// Required fields with a coarse shape descriptor
    /// ("string", "u64", "array<string>", ...).
    pub fields: BTreeMap<String, String>,
}

/// SDDK-owned typed contribution ADT (SAW-002). It is INPUT to
/// Synthesis/Decision/Evidence — never authority (SAW-005).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContributionV2 {
    pub request_id: String,
    /// Fields validated against the ReturnSchema.
    pub fields: BTreeMap<String, serde_json::Value>,
}

/// Terminal outcome of one structured run (SAW-003): failure modes
/// are distinguishable; success is never fabricated.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum StructuredRunOutcome {
    /// Validated ContributionV2.
    Contributed(ContributionV2),
    /// Host output did not match the schema. Carries the violations.
    SchemaViolation(Vec<String>),
    /// Host cancelled the work.
    Cancelled,
    /// Host timed out.
    TimedOut,
    /// Host produced incomplete output.
    Incomplete,
}

/// Execution receipt (SAW-006): task/context/instruction/host
/// compatibility basis + result provenance.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentExecutionReceipt {
    pub request_id: String,
    pub task_ref: String,
    pub context_basis: Vec<String>,
    pub host_compatibility: String,
    /// Provenance of the result: which terminal outcome was produced.
    pub outcome_kind: OutcomeKind,
    /// Number of fields validated (0 for non-contributed outcomes).
    pub validated_fields: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutcomeKind {
    Contributed,
    SchemaViolation,
    Cancelled,
    TimedOut,
    Incomplete,
}

/// Raw host output as returned by the adapter (SAW-004: same
/// adapter for companion and orchestrated usage — this is the
/// shared boundary type).
#[derive(Clone, Debug, PartialEq)]
pub enum RawHostOutput {
    /// A structured payload (e.g. parsed JSON object).
    Fields(BTreeMap<String, serde_json::Value>),
    /// Host signalled cancellation.
    Cancelled,
    /// Host signalled timeout.
    TimedOut,
    /// Host signalled incomplete work.
    Incomplete,
}

/// Closed error surface.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum StructuredWorkError {
    #[error("unknown request {0}")]
    UnknownRequest(String),
}

/// Executor for structured work: validates raw host output against
/// the request's ReturnSchema and produces the typed outcome +
/// receipt. Never fabricates success (SAW-003).
#[derive(Clone, Debug, Default)]
pub struct StructuredWorkExecutor {
    requests: BTreeMap<String, AgentWorkRequest>,
    receipts: Vec<AgentExecutionReceipt>,
}

impl StructuredWorkExecutor {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a request (SAW-001).
    pub fn submit(&mut self, req: AgentWorkRequest) {
        self.requests.insert(req.request_id.clone(), req);
    }

    /// run_structured: execute one request against raw host output
    /// (SAW-002/003/006). The adapter already delivered the output;
    /// this validates and types it.
    pub fn run_structured(
        &mut self,
        request_id: &str,
        raw: RawHostOutput,
    ) -> Result<(StructuredRunOutcome, AgentExecutionReceipt), StructuredWorkError> {
        let req = self
            .requests
            .get(request_id)
            .ok_or_else(|| StructuredWorkError::UnknownRequest(request_id.to_string()))?;
        let (outcome, kind, validated) = match raw {
            RawHostOutput::Cancelled => {
                (StructuredRunOutcome::Cancelled, OutcomeKind::Cancelled, 0)
            }
            RawHostOutput::TimedOut => (StructuredRunOutcome::TimedOut, OutcomeKind::TimedOut, 0),
            RawHostOutput::Incomplete => {
                (StructuredRunOutcome::Incomplete, OutcomeKind::Incomplete, 0)
            }
            RawHostOutput::Fields(fields) => {
                let mut violations = Vec::new();
                for (name, shape) in &req.return_schema.fields {
                    match fields.get(name) {
                        None => violations.push(format!("missing field {name}")),
                        Some(v) => {
                            if !shape_matches(v, shape) {
                                violations.push(format!("field {name} expected {shape}, got {v}"));
                            }
                        }
                    }
                }
                if violations.is_empty() {
                    let n = fields.len();
                    (
                        StructuredRunOutcome::Contributed(ContributionV2 {
                            request_id: req.request_id.clone(),
                            fields,
                        }),
                        OutcomeKind::Contributed,
                        n,
                    )
                } else {
                    (
                        StructuredRunOutcome::SchemaViolation(violations),
                        OutcomeKind::SchemaViolation,
                        0,
                    )
                }
            }
        };
        let receipt = AgentExecutionReceipt {
            request_id: req.request_id.clone(),
            task_ref: req.task_ref.clone(),
            context_basis: req.context_refs.clone(),
            host_compatibility: "structured-v1".to_string(),
            outcome_kind: kind,
            validated_fields: validated,
        };
        self.receipts.push(receipt.clone());
        Ok((outcome, receipt))
    }

    /// All receipts so far (SAW-006 provenance trail).
    #[must_use]
    pub fn receipts(&self) -> &[AgentExecutionReceipt] {
        &self.receipts
    }
}

fn shape_matches(v: &serde_json::Value, shape: &str) -> bool {
    match shape {
        "string" => v.is_string(),
        "u64" => v.is_u64(),
        "bool" => v.is_boolean(),
        "array<string>" => v
            .as_array()
            .is_some_and(|a| a.iter().all(|x| x.is_string())),
        _ => true, // unknown descriptors are pass-through (coarse schema)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn request() -> AgentWorkRequest {
        AgentWorkRequest {
            request_id: "req-1".into(),
            task_ref: "task:t-42".into(),
            context_refs: vec!["basis:rev-7".into()],
            policy_refs: vec!["policy:default".into()],
            return_schema: ReturnSchema {
                fields: BTreeMap::from([
                    ("summary".to_string(), "string".into()),
                    ("confidence".to_string(), "u64".into()),
                ]),
            },
        }
    }

    /// SAW-001/002: typed request with explicit return schema; valid
    /// output yields a typed ContributionV2.
    #[test]
    fn saw001_002_typed_request_schema_result() {
        let mut ex = StructuredWorkExecutor::new();
        ex.submit(request());
        let (outcome, receipt) = ex
            .run_structured(
                "req-1",
                RawHostOutput::Fields(BTreeMap::from([
                    ("summary".to_string(), json!("done")),
                    ("confidence".to_string(), json!(3)),
                ])),
            )
            .unwrap();
        match outcome {
            StructuredRunOutcome::Contributed(c) => {
                assert_eq!(c.request_id, "req-1");
                assert_eq!(c.fields.get("confidence"), Some(&json!(3)));
            }
            other => panic!("expected Contributed, got {other:?}"),
        }
        assert_eq!(receipt.outcome_kind, OutcomeKind::Contributed);
        assert_eq!(receipt.validated_fields, 2);
    }

    /// SAW-003: schema failure, cancellation, timeout and incomplete
    /// are all distinguishable; no fabricated success.
    #[test]
    fn saw003_invalid_output_visible_not_fabricated() {
        let mut ex = StructuredWorkExecutor::new();
        ex.submit(request());
        let (o, _) = ex
            .run_structured(
                "req-1",
                RawHostOutput::Fields(BTreeMap::from([("summary".to_string(), json!("x"))])),
            )
            .unwrap();
        match o {
            StructuredRunOutcome::SchemaViolation(v) => {
                assert!(v.iter().any(|s| s.contains("confidence")))
            }
            other => panic!("expected SchemaViolation, got {other:?}"),
        }
        for (raw, kind) in [
            (RawHostOutput::Cancelled, OutcomeKind::Cancelled),
            (RawHostOutput::TimedOut, OutcomeKind::TimedOut),
            (RawHostOutput::Incomplete, OutcomeKind::Incomplete),
        ] {
            let (o, r) = ex.run_structured("req-1", raw).unwrap();
            assert_ne!(
                o,
                StructuredRunOutcome::Contributed(ContributionV2 {
                    request_id: "req-1".into(),
                    fields: BTreeMap::new()
                })
            );
            assert_eq!(r.outcome_kind, kind);
            assert_eq!(r.validated_fields, 0);
        }
    }

    /// SAW-004: same executor/adapter boundary serves companion
    /// (ad-hoc) and orchestrated (registered) usage.
    #[test]
    fn saw004_same_adapter_two_modes() {
        let mut ex = StructuredWorkExecutor::new();
        // Orchestrated: submitted request.
        ex.submit(request());
        let (o1, _) = ex
            .run_structured(
                "req-1",
                RawHostOutput::Fields(BTreeMap::from([
                    ("summary".to_string(), json!("a")),
                    ("confidence".to_string(), json!(1)),
                ])),
            )
            .unwrap();
        // Companion: another request through the SAME boundary type.
        let mut r2 = request();
        r2.request_id = "req-adhoc".into();
        ex.submit(r2);
        let (o2, _) = ex
            .run_structured(
                "req-adhoc",
                RawHostOutput::Fields(BTreeMap::from([
                    ("summary".to_string(), json!("b")),
                    ("confidence".to_string(), json!(2)),
                ])),
            )
            .unwrap();
        assert!(matches!(o1, StructuredRunOutcome::Contributed(_)));
        assert!(matches!(o2, StructuredRunOutcome::Contributed(_)));
    }

    /// SAW-005: ContributionV2 is data, not authority — the type has
    /// no mutation surface into Decision/Verification/Governance
    /// (structural pin: read-only fields, no effectful API).
    #[test]
    fn saw005_contribution_not_authority() {
        let mut ex = StructuredWorkExecutor::new();
        ex.submit(request());
        let (o, _) = ex
            .run_structured(
                "req-1",
                RawHostOutput::Fields(BTreeMap::from([
                    ("summary".to_string(), json!("s")),
                    ("confidence".to_string(), json!(9)),
                ])),
            )
            .unwrap();
        if let StructuredRunOutcome::Contributed(c) = o {
            // It is plain serializable data. Nothing here mutates any
            // authority store: the executor only records receipts.
            let _serialized = serde_json::to_string(&c).unwrap();
        } else {
            panic!("expected contribution");
        }
        // Receipts are the only side effect: an append-only trail.
        assert_eq!(ex.receipts().len(), 1);
    }

    /// SAW-006: receipt captures task/context/host basis and result
    /// provenance for every terminal outcome.
    #[test]
    fn saw006_receipt_provenance() {
        let mut ex = StructuredWorkExecutor::new();
        ex.submit(request());
        let _ = ex.run_structured("req-1", RawHostOutput::TimedOut).unwrap();
        let r = &ex.receipts()[0];
        assert_eq!(r.task_ref, "task:t-42");
        assert_eq!(r.context_basis, vec!["basis:rev-7".to_string()]);
        assert_eq!(r.host_compatibility, "structured-v1");
        assert_eq!(r.outcome_kind, OutcomeKind::TimedOut);
    }
}
