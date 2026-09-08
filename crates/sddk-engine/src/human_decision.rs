//! Human Decision substrate — typed HITL contracts with immutable
//! receipts and a `HumanDecisionPort` seam for runtime integration.
//!
//! Spec: ~/.sddk-knowledge/sddk-framework/specs/engine/REQ-HumanDecision.md
//! ADR:  ~/.sddk-knowledge/sddk-framework/adrs/ADR-086-HUMAN-DECISION-SUBSTRATE.md

use std::collections::BTreeMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::continuation_candidate::Reversibility;

/// A single option a human can pick among.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HumanDecisionOption {
    pub option_id: String,
    pub label: String,
    pub summary: String,
    /// Optional override: declares this option's reversibility
    /// independently of the parent request.
    pub reversibility_override: Option<Reversibility>,
}

impl HumanDecisionOption {
    pub fn new(
        option_id: impl Into<String>,
        label: impl Into<String>,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            option_id: option_id.into(),
            label: label.into(),
            summary: summary.into(),
            reversibility_override: None,
        }
    }

    pub fn with_reversibility_override(mut self, r: Reversibility) -> Self {
        self.reversibility_override = Some(r);
        self
    }
}

/// A request asking for human input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HumanDecisionRequest {
    pub request_id: String,
    pub cycle_ref: String,
    pub question: String,
    pub context_refs: Vec<String>,
    pub options: Vec<HumanDecisionOption>,
    pub default_rationale: String,
    pub reversibility: Reversibility,
    pub requested_at_ms: i64,
    pub expires_at_ms: Option<i64>,
    pub requested_by: String,
}

impl HumanDecisionRequest {
    pub fn new(
        request_id: impl Into<String>,
        cycle_ref: impl Into<String>,
        question: impl Into<String>,
        requested_by: impl Into<String>,
        requested_at_ms: i64,
    ) -> Self {
        Self {
            request_id: request_id.into(),
            cycle_ref: cycle_ref.into(),
            question: question.into(),
            context_refs: Vec::new(),
            options: Vec::new(),
            default_rationale: String::new(),
            reversibility: Reversibility::Unknown,
            requested_at_ms,
            expires_at_ms: None,
            requested_by: requested_by.into(),
        }
    }

    pub fn with_options(mut self, options: Vec<HumanDecisionOption>) -> Self {
        self.options = options;
        self
    }

    pub fn with_reversibility(mut self, r: Reversibility) -> Self {
        self.reversibility = r;
        self
    }

    pub fn with_expires_at_ms(mut self, ms: i64) -> Self {
        self.expires_at_ms = Some(ms);
        self
    }
}

/// The human's reply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[non_exhaustive]
pub enum HumanDecision {
    Approved {
        approved_by: String,
        rationale: String,
        at_ms: i64,
    },
    Rejected {
        rejected_by: String,
        rationale: String,
        at_ms: i64,
    },
    Deferred {
        deferred_by: String,
        reason: String,
        at_ms: i64,
    },
    Escalated {
        escalated_by: String,
        to: String,
        reason: String,
        at_ms: i64,
    },
    Selected {
        selected_by: String,
        option_id: String,
        rationale: String,
        at_ms: i64,
    },
}

impl HumanDecision {
    /// Actor carrying out the decision.
    pub fn actor(&self) -> &str {
        match self {
            HumanDecision::Approved { approved_by, .. } => approved_by,
            HumanDecision::Rejected { rejected_by, .. } => rejected_by,
            HumanDecision::Deferred { deferred_by, .. } => deferred_by,
            HumanDecision::Escalated { escalated_by, .. } => escalated_by,
            HumanDecision::Selected { selected_by, .. } => selected_by,
        }
    }

    /// Timestamp of the decision.
    pub fn at_ms(&self) -> i64 {
        match self {
            HumanDecision::Approved { at_ms, .. }
            | HumanDecision::Rejected { at_ms, .. }
            | HumanDecision::Deferred { at_ms, .. }
            | HumanDecision::Escalated { at_ms, .. }
            | HumanDecision::Selected { at_ms, .. } => *at_ms,
        }
    }
}

/// Immutable record of a single resolved decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HumanDecisionReceipt {
    pub receipt_id: String,
    pub request_id: String,
    pub decision: HumanDecision,
    pub decided_at_ms: i64,
    pub digest: String,
    pub authority_path: Vec<String>,
    pub evidence_refs: Vec<String>,
}

impl HumanDecisionReceipt {
    pub fn new(
        receipt_id: impl Into<String>,
        request_id: impl Into<String>,
        decision: HumanDecision,
        decided_at_ms: i64,
        digest: impl Into<String>,
        authority_path: Vec<String>,
        evidence_refs: Vec<String>,
    ) -> Self {
        Self {
            receipt_id: receipt_id.into(),
            request_id: request_id.into(),
            decision,
            decided_at_ms,
            digest: digest.into(),
            authority_path,
            evidence_refs,
        }
    }
}

/// Error taxonomy (HX-DECISION-001). Closed-set, `#[non_exhaustive]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[non_exhaustive]
pub enum HumanDecisionError {
    AlreadyDecided {
        request_id: String,
    },
    ExpiredRequest {
        request_id: String,
        expired_at_ms: i64,
    },
    UnknownRequest {
        request_id: String,
    },
    InvalidAuthority {
        request_id: String,
        authority_path: Vec<String>,
    },
    MissingAuthority {
        request_id: String,
    },
    Forbidden {
        request_id: String,
        actor: String,
    },
    InvalidOption {
        request_id: String,
        option_id: String,
    },
    NonMatchingReversibility {
        declared: Reversibility,
        implied: Reversibility,
    },
}

impl std::fmt::Display for HumanDecisionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HumanDecisionError::AlreadyDecided { request_id } => {
                write!(f, "request {request_id} already decided")
            }
            HumanDecisionError::ExpiredRequest {
                request_id,
                expired_at_ms,
            } => write!(f, "request {request_id} expired at {expired_at_ms}"),
            HumanDecisionError::UnknownRequest { request_id } => {
                write!(f, "unknown request {request_id}")
            }
            HumanDecisionError::InvalidAuthority {
                request_id,
                authority_path,
            } => write!(
                f,
                "request {request_id} invalid authority {authority_path:?}"
            ),
            HumanDecisionError::MissingAuthority { request_id } => {
                write!(f, "request {request_id} missing authority path")
            }
            HumanDecisionError::Forbidden { request_id, actor } => {
                write!(f, "request {request_id} forbids actor {actor}")
            }
            HumanDecisionError::InvalidOption {
                request_id,
                option_id,
            } => write!(f, "request {request_id} invalid option {option_id}"),
            HumanDecisionError::NonMatchingReversibility { declared, implied } => write!(
                f,
                "reversibility mismatch declared={declared:?} implied={implied:?}"
            ),
        }
    }
}

impl std::error::Error for HumanDecisionError {}

fn severity(r: &Reversibility) -> i32 {
    match r {
        Reversibility::Unknown => 0,
        Reversibility::FullyReversible => 1,
        Reversibility::PartiallyReversible => 2,
        Reversibility::Irreversible => 3,
    }
}

/// Validate a `HumanDecisionRequest` before opening it on a port.
/// Returns `Ok(())` or the first invariant violation.
pub fn validate_request(req: &HumanDecisionRequest) -> Result<(), HumanDecisionError> {
    // (1) non-empty question + non-empty options
    if req.question.trim().is_empty() {
        return Err(HumanDecisionError::InvalidAuthority {
            request_id: req.request_id.clone(),
            authority_path: vec!["empty_question".to_string()],
        });
    }
    if req.options.is_empty() {
        return Err(HumanDecisionError::InvalidAuthority {
            request_id: req.request_id.clone(),
            authority_path: vec!["empty_options".to_string()],
        });
    }
    // (2) unique option_id
    let mut seen: BTreeMap<&str, ()> = BTreeMap::new();
    for opt in &req.options {
        if seen.insert(opt.option_id.as_str(), ()).is_some() {
            return Err(HumanDecisionError::InvalidOption {
                request_id: req.request_id.clone(),
                option_id: opt.option_id.clone(),
            });
        }
    }
    // (3) reversibility severity: any option declaring Irreversible
    // overrides the parent request reversibility ⇒ NonMatchingReversibility.
    for opt in &req.options {
        if let Some(r) = opt.reversibility_override
            && severity(&r) > severity(&req.reversibility)
        {
            return Err(HumanDecisionError::NonMatchingReversibility {
                declared: req.reversibility,
                implied: r,
            });
        }
    }
    // (4) expires_at_ms ≥ requested_at_ms
    if let Some(exp) = req.expires_at_ms
        && exp < req.requested_at_ms
    {
        return Err(HumanDecisionError::InvalidAuthority {
            request_id: req.request_id.clone(),
            authority_path: vec!["expiry_before_request".to_string()],
        });
    }
    Ok(())
}

/// Port seam for runtime integration.
pub trait HumanDecisionPort: Send + Sync + std::fmt::Debug {
    fn open_request(&self, request: HumanDecisionRequest) -> Result<String, HumanDecisionError>;
    fn fetch(&self, request_id: &str) -> Option<HumanDecisionRequest>;
    fn submit(
        &self,
        request_id: &str,
        decision: HumanDecision,
    ) -> Result<HumanDecisionReceipt, HumanDecisionError>;
    fn receipt(&self, request_id: &str) -> Option<HumanDecisionReceipt>;
}

/// Reference in-memory impl.
#[derive(Debug, Default)]
pub struct InMemoryHumanDecisionPort {
    requests: Mutex<BTreeMap<String, HumanDecisionRequest>>,
    receipts: Mutex<BTreeMap<String, HumanDecisionReceipt>>,
}

impl InMemoryHumanDecisionPort {
    pub fn new() -> Self {
        Self::default()
    }

    /// Helper: compute a deterministic digest over (request_id, decision).
    /// The cycle-1 hash function is FNV-1a 64-bit formatted as hex;
    /// cycle-2 (HX-DECISION-002) replaces it with a canonical SHA-256 chain.
    fn compute_digest(request_id: &str, decision: &HumanDecision) -> String {
        let mut hash: u64 = 0xcbf29ce484222325;
        for byte in request_id.as_bytes() {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        let actor = decision.actor();
        for byte in actor.as_bytes() {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        format!("{:016x}", hash)
    }
}

impl HumanDecisionPort for InMemoryHumanDecisionPort {
    fn open_request(&self, request: HumanDecisionRequest) -> Result<String, HumanDecisionError> {
        validate_request(&request)?;
        let mut g = self
            .requests
            .lock()
            .expect("InMemoryHumanDecisionPort requests mutex poisoned");
        let id = request.request_id.clone();
        g.insert(id.clone(), request);
        Ok(id)
    }

    fn fetch(&self, request_id: &str) -> Option<HumanDecisionRequest> {
        let g = self
            .requests
            .lock()
            .expect("InMemoryHumanDecisionPort requests mutex poisoned");
        g.get(request_id).cloned()
    }

    fn submit(
        &self,
        request_id: &str,
        decision: HumanDecision,
    ) -> Result<HumanDecisionReceipt, HumanDecisionError> {
        let g = self
            .requests
            .lock()
            .expect("InMemoryHumanDecisionPort requests mutex poisoned");
        let req = g
            .get(request_id)
            .ok_or_else(|| HumanDecisionError::UnknownRequest {
                request_id: request_id.to_string(),
            })?;

        // (8) submit-after-expiry
        if let Some(exp) = req.expires_at_ms
            && decision.at_ms() > exp
        {
            return Err(HumanDecisionError::ExpiredRequest {
                request_id: request_id.to_string(),
                expired_at_ms: exp,
            });
        }

        // (8 — Selected option must exist)
        if let HumanDecision::Selected { option_id, .. } = &decision
            && !req.options.iter().any(|o| &o.option_id == option_id)
        {
            return Err(HumanDecisionError::InvalidOption {
                request_id: request_id.to_string(),
                option_id: option_id.clone(),
            });
        }

        let mut r = self
            .receipts
            .lock()
            .expect("InMemoryHumanDecisionPort receipts mutex poisoned");
        if r.contains_key(request_id) {
            return Err(HumanDecisionError::AlreadyDecided {
                request_id: request_id.to_string(),
            });
        }

        let actor = decision.actor().to_string();
        let authority_path = vec![format!("role://{}", actor)];
        if authority_path.is_empty() {
            return Err(HumanDecisionError::MissingAuthority {
                request_id: request_id.to_string(),
            });
        }

        let digest = Self::compute_digest(request_id, &decision);
        if digest.is_empty() {
            return Err(HumanDecisionError::InvalidAuthority {
                request_id: request_id.to_string(),
                authority_path,
            });
        }

        let receipt = HumanDecisionReceipt::new(
            format!("{}-R", request_id),
            request_id,
            decision,
            chrono_like_now_ms(),
            digest,
            authority_path,
            Vec::new(),
        );

        r.insert(request_id.to_string(), receipt.clone());
        Ok(receipt)
    }

    fn receipt(&self, request_id: &str) -> Option<HumanDecisionReceipt> {
        let g = self
            .receipts
            .lock()
            .expect("InMemoryHumanDecisionPort receipts mutex poisoned");
        g.get(request_id).cloned()
    }
}

fn chrono_like_now_ms() -> i64 {
    0 // Cycle-1 placeholder; HX-DECISION-002 replaces with clock injection.
}

#[cfg(test)]
mod tests {
    use super::*;

    fn option_a() -> HumanDecisionOption {
        HumanDecisionOption::new("A", "approve deploy", "ship it")
    }
    fn option_b() -> HumanDecisionOption {
        HumanDecisionOption::new("B", "block deploy", "wait")
    }

    fn make_request() -> HumanDecisionRequest {
        HumanDecisionRequest::new("R1", "cycle-269", "ship the candidate?", "orch", 1_000)
            .with_options(vec![option_a(), option_b()])
            .with_reversibility(Reversibility::PartiallyReversible)
    }

    #[test]
    fn empty_question_rejected() {
        let mut req = make_request();
        req.question = "".into();
        let err = validate_request(&req).unwrap_err();
        assert!(matches!(err, HumanDecisionError::InvalidAuthority { .. }));
    }

    #[test]
    fn duplicate_option_rejected() {
        let mut req = make_request();
        let dup = HumanDecisionOption::new("A", "dupe", "x");
        req.options.push(dup);
        let err = validate_request(&req).unwrap_err();
        assert!(matches!(err, HumanDecisionError::InvalidOption { .. }));
    }

    #[test]
    fn irreversibility_override_mismatch() {
        let mut req = make_request();
        req.reversibility = Reversibility::FullyReversible;
        req.options = vec![
            HumanDecisionOption::new("A", "label", "sum")
                .with_reversibility_override(Reversibility::Irreversible),
        ];
        let err = validate_request(&req).unwrap_err();
        assert!(matches!(
            err,
            HumanDecisionError::NonMatchingReversibility { .. }
        ));
    }

    #[test]
    fn expiry_before_request() {
        let mut req = make_request();
        req.expires_at_ms = Some(500);
        let err = validate_request(&req).unwrap_err();
        assert!(matches!(err, HumanDecisionError::InvalidAuthority { .. }));
    }

    #[test]
    fn single_submit_semantics() {
        let port = InMemoryHumanDecisionPort::new();
        port.open_request(make_request()).unwrap();
        let d1 = HumanDecision::Approved {
            approved_by: "alice".into(),
            rationale: "ok".into(),
            at_ms: 1_500,
        };
        port.submit("R1", d1).unwrap();
        let d2 = HumanDecision::Approved {
            approved_by: "bob".into(),
            rationale: "double".into(),
            at_ms: 1_600,
        };
        let err = port.submit("R1", d2).unwrap_err();
        assert!(matches!(err, HumanDecisionError::AlreadyDecided { .. }));
    }

    #[test]
    fn submit_after_expiry() {
        let port = InMemoryHumanDecisionPort::new();
        let mut req = make_request();
        req.expires_at_ms = Some(1_000);
        port.open_request(req).unwrap();
        let d = HumanDecision::Approved {
            approved_by: "alice".into(),
            rationale: "ok".into(),
            at_ms: 2_000,
        };
        let err = port.submit("R1", d).unwrap_err();
        assert!(matches!(err, HumanDecisionError::ExpiredRequest { .. }));
    }

    #[test]
    fn selected_option_must_exist() {
        let port = InMemoryHumanDecisionPort::new();
        port.open_request(make_request()).unwrap();
        let d = HumanDecision::Selected {
            selected_by: "alice".into(),
            option_id: "Z".into(),
            rationale: "nope".into(),
            at_ms: 1_500,
        };
        let err = port.submit("R1", d).unwrap_err();
        assert!(matches!(err, HumanDecisionError::InvalidOption { .. }));
    }

    #[test]
    fn roundtrip_open_submit_receipt() {
        let port = InMemoryHumanDecisionPort::new();
        port.open_request(make_request()).unwrap();
        let d = HumanDecision::Approved {
            approved_by: "alice".into(),
            rationale: "ok".into(),
            at_ms: 1_500,
        };
        let r = port.submit("R1", d).unwrap();
        assert_eq!(r.request_id, "R1");
        assert!(!r.digest.is_empty());
        assert!(!r.authority_path.is_empty());
        let fetched = port.receipt("R1").unwrap();
        assert_eq!(fetched.receipt_id, r.receipt_id);
    }

    #[test]
    fn fetch_unknown_returns_none() {
        let port = InMemoryHumanDecisionPort::new();
        assert!(port.fetch("missing").is_none());
        assert!(port.receipt("missing").is_none());
    }

    #[test]
    fn unknown_request_submit() {
        let port = InMemoryHumanDecisionPort::new();
        let d = HumanDecision::Approved {
            approved_by: "alice".into(),
            rationale: "ok".into(),
            at_ms: 1_500,
        };
        let err = port.submit("missing", d).unwrap_err();
        assert!(matches!(err, HumanDecisionError::UnknownRequest { .. }));
    }
}
