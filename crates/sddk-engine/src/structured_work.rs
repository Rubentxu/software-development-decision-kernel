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
    /// H02 of ROADMAP §2 C1: a request_id already exists and the new
    /// request is not structurally identical. The existing entry is
    /// preserved (no silent replace); the caller must use a fresh
    /// request_id or `submit_idempotent` to retry.
    #[error(
        "duplicate request_id {id}: existing task_ref={existing_task_ref}, \
         new task_ref={new_task_ref} (use a fresh request_id or submit_idempotent)"
    )]
    DuplicateRequest {
        id: String,
        existing_task_ref: String,
        new_task_ref: String,
    },
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
    ///
    /// H02 of ROADMAP §2 C1: strict mode. If `request_id` already
    /// exists with a structurally different request, returns
    /// `StructuredWorkError::DuplicateRequest` without mutating the
    /// existing entry. For bit-exact duplicate re-submission use
    /// `submit_idempotent` instead.
    pub fn submit(&mut self, req: AgentWorkRequest) -> Result<(), StructuredWorkError> {
        if let Some(existing) = self.requests.get(&req.request_id) {
            if existing != &req {
                return Err(StructuredWorkError::DuplicateRequest {
                    id: req.request_id.clone(),
                    existing_task_ref: existing.task_ref.clone(),
                    new_task_ref: req.task_ref.clone(),
                });
            }
            // Bit-exact duplicate: no-op (preserves existing entry).
            return Ok(());
        }
        self.requests.insert(req.request_id.clone(), req);
        Ok(())
    }

    /// H02 of ROADMAP §2 C1: idempotent submit. Returns `Ok(true)` on
    /// acceptance (new or bit-exact duplicate) or `Err(DuplicateRequest)`
    /// if the request_id exists with a structurally different request.
    pub fn submit_idempotent(
        &mut self,
        req: AgentWorkRequest,
    ) -> Result<bool, StructuredWorkError> {
        self.submit(req).map(|_| true)
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
                                violations.push(format!(
                                    "field {name} expected {shape}, got {}",
                                    redacted_value_repr(v)
                                ));
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

/// H02 of ROADMAP §2 C1: redact the JSON value carried by a
/// schema-violation message. We never want host-controlled content
/// (which could carry secrets, PII, or canary tokens) to leak into
/// `StructuredRunOutcome::SchemaViolation(violations)` because that
/// vector is logged, archived, and possibly surfaced to users.
///
/// The shape of the value is preserved (so the bug remains
/// diagnosable); the content is not.
fn redacted_value_repr(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(_) => "bool".to_string(),
        serde_json::Value::Number(n) => format!("number({n})"),
        serde_json::Value::String(s) if s.is_empty() => "string(\"\")".to_string(),
        serde_json::Value::String(_) => "string(<redacted>)".to_string(),
        serde_json::Value::Array(a) => format!("array[len={}]", a.len()),
        serde_json::Value::Object(o) => format!("object[len={}]", o.len()),
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
        // H01 of ROADMAP §2 C1: unknown descriptors must NOT match.
        // Previously this arm was `_ => true` (pass-through), which silently
        // accepted any shape, allowing the executor to fabricate
        // `Contributed` outcomes for descriptors that were not part of
        // the supported allow-list ("string" | "u64" | "bool" |
        // "array<string>"). Any future descriptor must be added here
        // explicitly with its matcher.
        _ => false,
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
        let _ = ex.submit(request());
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
        let _ = ex.submit(request());
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
        let _ = ex.submit(request());
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
        let _ = ex.submit(r2);
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
        let _ = ex.submit(request());
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
        let _ = ex.submit(request());
        let _ = ex.run_structured("req-1", RawHostOutput::TimedOut).unwrap();
        let r = &ex.receipts()[0];
        assert_eq!(r.task_ref, "task:t-42");
        assert_eq!(r.context_basis, vec!["basis:rev-7".to_string()]);
        assert_eq!(r.host_compatibility, "structured-v1");
        assert_eq!(r.outcome_kind, OutcomeKind::TimedOut);
    }

    /// SAW-007 (H01 of ROADMAP §2 C1): unknown shape descriptor must
    /// NOT match. A descriptor not in the allow-list
    /// ("string" | "u64" | "bool" | "array<string>") must be rejected
    /// instead of being silently accepted.
    ///
    /// Before the fix: `_ => true` in `shape_matches` accepted any unknown
    /// descriptor (and any value type), which violates the contract.
    #[test]
    fn saw007_unknown_descriptor_rejected() {
        // The same-shape-known cases still work (regression check).
        assert!(shape_matches(&json!("hello"), "string"));
        assert!(shape_matches(&json!(42), "u64"));
        assert!(shape_matches(&json!(true), "bool"));
        assert!(shape_matches(&json!(["a", "b"]), "array<string>"));

        // The unknown-descriptor cases (the bug).
        assert!(!shape_matches(
            &json!("hello"),
            "this-descriptor-does-not-exist"
        ));
        assert!(!shape_matches(&json!("hello"), ""));
        assert!(!shape_matches(&json!("hello"), "integer"));
        assert!(!shape_matches(&json!("hello"), "u32"));
        assert!(!shape_matches(&json!("hello"), "i64"));
        assert!(!shape_matches(&json!("hello"), "object"));
        assert!(!shape_matches(&json!("hello"), "STRING")); // case-sensitive match
    }

    /// SAW-008 (H01 companion): unknown descriptor on a typed JSON value
    /// surfaces as `SchemaViolation`, not as a fabricated `Contributed`.
    /// This is the end-to-end manifestation of the bug at the executor
    /// boundary.
    #[test]
    fn saw008_unknown_descriptor_violation_not_contribution() {
        let mut ex = StructuredWorkExecutor::new();
        let mut req = request();
        // Inject a non-supported descriptor in the return schema.
        req.return_schema = ReturnSchema {
            fields: BTreeMap::from([
                ("summary".to_string(), "string".into()),
                (
                    "custom".to_string(),
                    "this-descriptor-does-not-exist".into(),
                ),
            ]),
        };
        let _ = ex.submit(req);
        let (outcome, receipt) = ex
            .run_structured(
                "req-1",
                RawHostOutput::Fields(BTreeMap::from([
                    ("summary".to_string(), json!("done")),
                    ("custom".to_string(), json!("anything")),
                ])),
            )
            .unwrap();
        match outcome {
            StructuredRunOutcome::SchemaViolation(v) => {
                assert!(
                    v.iter().any(|s| s.contains("custom")),
                    "violation must mention the offending field, got {v:?}"
                );
            }
            other => panic!("expected SchemaViolation, got {other:?}"),
        }
        assert_eq!(receipt.outcome_kind, OutcomeKind::SchemaViolation);
        assert_eq!(receipt.validated_fields, 0);
    }

    // ─────────────────────────────────────────────────────────────────────
    // SAW-009..SAW-017 — H02 of ROADMAP §2 C1:
    //   request_id duplicado no sustituye silenciosamente una petición
    //   no equivalente; errores de schema NUNCA interpolan valores.
    // ─────────────────────────────────────────────────────────────────────

    /// SAW-009: strict submit rejects duplicate request_id with a
    /// visible error (existing vs new task_ref), not silent replace.
    #[test]
    fn saw009_duplicate_request_strict_rejects() {
        let mut ex = StructuredWorkExecutor::new();
        let r1 = request();
        ex.submit(r1).expect("first submit accepts");
        let mut r2 = request();
        r2.task_ref = "task:WRONG".into();
        let err = ex.submit(r2).expect_err("duplicate must reject");
        match err {
            StructuredWorkError::DuplicateRequest {
                id,
                existing_task_ref,
                new_task_ref,
            } => {
                assert_eq!(id, "req-1");
                assert_eq!(existing_task_ref, "task:t-42");
                assert_eq!(new_task_ref, "task:WRONG");
            }
            other => panic!("expected DuplicateRequest, got {other:?}"),
        }
        // The original request is preserved (no silent replace).
        assert_eq!(
            ex.requests.get("req-1").map(|r| r.task_ref.clone()),
            Some("task:t-42".to_string())
        );
    }

    /// SAW-010: idempotent submit accepts structurally identical duplicate.
    #[test]
    fn saw010_duplicate_request_idempotent_accepts() {
        let mut ex = StructuredWorkExecutor::new();
        let r1 = request();
        assert!(ex.submit_idempotent(r1.clone()).unwrap());
        // Re-submitting the exact same request is a no-op (Ok(true)).
        assert!(ex.submit_idempotent(r1).unwrap());
        // Only one entry in the map.
        assert_eq!(ex.requests.len(), 1);
    }

    /// SAW-011: idempotent submit rejects when the duplicate differs.
    /// Same error variant as strict mode (per H02 contract).
    #[test]
    fn saw011_duplicate_request_idempotent_rejects_different() {
        let mut ex = StructuredWorkExecutor::new();
        let r1 = request();
        ex.submit_idempotent(r1).unwrap();
        let mut r2 = request();
        r2.task_ref = "task:DIFFERENT".into();
        let err = ex.submit_idempotent(r2).expect_err("diff must reject");
        assert!(matches!(err, StructuredWorkError::DuplicateRequest { .. }));
    }

    /// SAW-012: duplicate rejection AFTER a successful run does not
    /// corrupt the receipt trail (original receipt still present).
    #[test]
    fn saw012_duplicate_request_after_run() {
        let mut ex = StructuredWorkExecutor::new();
        ex.submit(request()).unwrap();
        let _ = ex
            .run_structured(
                "req-1",
                RawHostOutput::Fields(BTreeMap::from([
                    ("summary".to_string(), json!("done")),
                    ("confidence".to_string(), json!(1)),
                ])),
            )
            .unwrap();
        assert_eq!(ex.receipts().len(), 1);
        let mut r2 = request();
        r2.task_ref = "task:AFTER-RUN".into();
        let err = ex.submit(r2).expect_err("dup post-run must reject");
        assert!(matches!(err, StructuredWorkError::DuplicateRequest { .. }));
        // Receipts preserved.
        assert_eq!(ex.receipts().len(), 1);
        assert_eq!(ex.receipts()[0].outcome_kind, OutcomeKind::Contributed);
    }

    /// SAW-013: a host-provided string value (which could carry a secret)
    /// MUST NOT appear verbatim in the violation message. The shape
    /// mismatch message uses the redaction placeholder.
    #[test]
    fn saw013_schema_violation_does_not_leak_string() {
        let mut ex = StructuredWorkExecutor::new();
        let mut req = request();
        req.return_schema = ReturnSchema {
            fields: BTreeMap::from([("confidence".to_string(), "u64".into())]),
        };
        ex.submit(req).unwrap();
        let (outcome, _) = ex
            .run_structured(
                "req-1",
                RawHostOutput::Fields(BTreeMap::from([(
                    "confidence".to_string(),
                    json!("AKIA-real-key-12345"),
                )])),
            )
            .unwrap();
        let violations = match outcome {
            StructuredRunOutcome::SchemaViolation(v) => v,
            other => panic!("expected SchemaViolation, got {other:?}"),
        };
        assert!(
            !violations.iter().any(|s| s.contains("AKIA-real-key-12345")),
            "violation leaked the secret: {violations:?}"
        );
        assert!(
            violations.iter().any(|s| s.contains("string(<redacted>)")),
            "violation must carry the redaction placeholder: {violations:?}"
        );
    }

    /// SAW-014: object values must be reported by length only; field
    /// names and nested values must not appear.
    #[test]
    fn saw014_schema_violation_does_not_leak_object() {
        let mut ex = StructuredWorkExecutor::new();
        let mut req = request();
        req.return_schema = ReturnSchema {
            fields: BTreeMap::from([("confidence".to_string(), "u64".into())]),
        };
        ex.submit(req).unwrap();
        let (outcome, _) = ex
            .run_structured(
                "req-1",
                RawHostOutput::Fields(BTreeMap::from([(
                    "confidence".to_string(),
                    json!({"password": "hunter2", "token": "abc"}),
                )])),
            )
            .unwrap();
        let violations = match outcome {
            StructuredRunOutcome::SchemaViolation(v) => v,
            other => panic!("expected SchemaViolation, got {other:?}"),
        };
        for forbidden in ["password", "token", "hunter2", "abc"] {
            assert!(
                !violations.iter().any(|s| s.contains(forbidden)),
                "violation leaked {forbidden:?}: {violations:?}"
            );
        }
        assert!(
            violations.iter().any(|s| s.contains("object[len=2]")),
            "violation must report object length only: {violations:?}"
        );
    }

    /// SAW-015: array values must be reported by length only.
    #[test]
    fn saw015_schema_violation_does_not_leak_array() {
        let mut ex = StructuredWorkExecutor::new();
        let mut req = request();
        req.return_schema = ReturnSchema {
            fields: BTreeMap::from([("confidence".to_string(), "u64".into())]),
        };
        ex.submit(req).unwrap();
        let (outcome, _) = ex
            .run_structured(
                "req-1",
                RawHostOutput::Fields(BTreeMap::from([(
                    "confidence".to_string(),
                    json!([1, 2, 3, 4, 5]),
                )])),
            )
            .unwrap();
        let violations = match outcome {
            StructuredRunOutcome::SchemaViolation(v) => v,
            other => panic!("expected SchemaViolation, got {other:?}"),
        };
        assert!(
            violations.iter().any(|s| s.contains("array[len=5]")),
            "violation must report array length only: {violations:?}"
        );
        for n in ["1", "2", "3", "4", "5"] {
            // Numbers in array values must not be enumerated.
            // (The number 5 in `array[len=5]` is allowed as length, not as value.)
            let suspicious = violations
                .iter()
                .find(|s| s.contains(n) && !s.contains("array[len=5]"));
            assert!(
                suspicious.is_none(),
                "violation enumerated array value {n}: {violations:?}"
            );
        }
    }

    /// SAW-016: missing-field messages keep their existing wording.
    /// The redaction only applies to value-bearing messages.
    #[test]
    fn saw016_missing_field_message_unchanged() {
        let mut ex = StructuredWorkExecutor::new();
        ex.submit(request()).unwrap();
        let (outcome, _) = ex
            .run_structured(
                "req-1",
                RawHostOutput::Fields(BTreeMap::from([(
                    "summary".to_string(),
                    json!("only-summary"),
                )])),
            )
            .unwrap();
        let violations = match outcome {
            StructuredRunOutcome::SchemaViolation(v) => v,
            other => panic!("expected SchemaViolation, got {other:?}"),
        };
        assert!(
            violations.iter().any(|s| s == "missing field confidence"),
            "expected exact 'missing field confidence' line, got {violations:?}"
        );
    }

    /// SAW-017: shape-mismatch on a known descriptor reports the type
    /// only, never the value.
    #[test]
    fn saw017_known_descriptor_violation_unchanged() {
        let mut ex = StructuredWorkExecutor::new();
        ex.submit(request()).unwrap();
        let (outcome, _) = ex
            .run_structured(
                "req-1",
                RawHostOutput::Fields(BTreeMap::from([
                    ("summary".to_string(), json!("ok")),
                    ("confidence".to_string(), json!("not-a-number")),
                ])),
            )
            .unwrap();
        let violations = match outcome {
            StructuredRunOutcome::SchemaViolation(v) => v,
            other => panic!("expected SchemaViolation, got {other:?}"),
        };
        // No leak of the literal value.
        assert!(
            !violations.iter().any(|s| s.contains("not-a-number")),
            "violation leaked string value: {violations:?}"
        );
        // Carries the redaction form for strings.
        assert!(
            violations
                .iter()
                .any(|s| s.contains("expected u64, got string(<redacted>)")),
            "violation must carry 'expected u64, got string(<redacted>)', got {violations:?}"
        );
    }
}
