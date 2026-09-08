//! Signed Gates, Policy Ratchets & Controlled Overrides
//!
//! Plain-data gate authority that evaluates a signed policy + an
//! optional ratchet + an optional override against the runtime
//! timestamp supplied by the caller.
//!
//! Pattern: P-a (struct + trait + plain-data evaluator).

// ── Audit guard constants ────────────────────────────────────────────────

/// Closed-set size of [`Strictness`]. Bump when a variant is added.
pub const STRICTNESS_VARIANT_COUNT: usize = 4;

/// Closed-set size of [`GateDecision`]. Bump when a variant is added.
pub const GATE_DECISION_VARIANT_COUNT: usize = 3;

// ── Enums ────────────────────────────────────────────────────────────────

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GateDecision {
    Allow,
    Deny,
    Warn,
}

impl GateDecision {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Deny => "deny",
            Self::Warn => "warn",
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Strictness {
    Lax,
    Standard,
    Strict,
    Hardened,
}

impl Strictness {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Lax => "lax",
            Self::Standard => "standard",
            Self::Strict => "strict",
            Self::Hardened => "hardened",
        }
    }
}

// ── Records ─────────────────────────────────────────────────────────────

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatePolicy {
    pub policy_id: String,
    pub strictness: Strictness,
    pub signed_by: String,
    /// 64 hex chars.
    pub signature: String,
    pub recorded_at: String,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyRatchet {
    pub before: Strictness,
    pub after: Strictness,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateOverride {
    Allow { reason: String, expires_at: String },
    Deny { reason: String, expires_at: String },
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateEvaluation {
    pub gate_id: String,
    pub policy_id: String,
    pub decision: GateDecision,
    pub ratchet: Option<PolicyRatchet>,
    pub override_used: Option<GateOverride>,
    pub recorded_at: String,
}

// ── Error + authority ──────────────────────────────────────────────────

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateError {
    UnknownSigner,
    InvalidSignature,
    NonMonotonicStrictness { from: Strictness, to: Strictness },
    UnknownPolicy,
    ExpiredOverride,
    EmptySignature,
}

pub trait SignedGateAuthority {
    #[allow(clippy::too_many_arguments)]
    fn evaluate(
        &self,
        policy: &GatePolicy,
        attempted_ratchet: Option<PolicyRatchet>,
        override_used: Option<GateOverride>,
        known_signers: &[String],
        current_strictness: Option<Strictness>,
        gate_id: &str,
        now: &str,
    ) -> Result<GateEvaluation, GateError>;
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DefaultSignedGateAuthority;

impl SignedGateAuthority for DefaultSignedGateAuthority {
    #[allow(clippy::too_many_arguments)]
    fn evaluate(
        &self,
        policy: &GatePolicy,
        attempted_ratchet: Option<PolicyRatchet>,
        override_used: Option<GateOverride>,
        known_signers: &[String],
        current_strictness: Option<Strictness>,
        gate_id: &str,
        now: &str,
    ) -> Result<GateEvaluation, GateError> {
        // Signer.
        if !known_signers.iter().any(|s| s == &policy.signed_by) {
            return Err(GateError::UnknownSigner);
        }
        // Signature format.
        if policy.signature.is_empty() {
            return Err(GateError::EmptySignature);
        }
        if policy.signature.len() != 64 || !policy.signature.chars().all(|c| c.is_ascii_hexdigit())
        {
            return Err(GateError::InvalidSignature);
        }
        // Ratchet.
        if let Some(r) = attempted_ratchet {
            match current_strictness {
                Some(cur) if cur == r.before && r.after > r.before => {}
                Some(_) | None => {
                    return Err(GateError::NonMonotonicStrictness {
                        from: r.before,
                        to: r.after,
                    });
                }
            }
            let eval = GateEvaluation {
                gate_id: gate_id.to_string(),
                policy_id: policy.policy_id.clone(),
                decision: GateDecision::Allow,
                ratchet: Some(r),
                override_used,
                recorded_at: now.to_string(),
            };
            return Ok(eval);
        }
        // Override.
        if let Some(o) = &override_used {
            let exp = match o {
                GateOverride::Allow { expires_at, .. } => expires_at,
                GateOverride::Deny { expires_at, .. } => expires_at,
            };
            if exp.as_str() < now {
                return Err(GateError::ExpiredOverride);
            }
        }
        Ok(GateEvaluation {
            gate_id: gate_id.to_string(),
            policy_id: policy.policy_id.clone(),
            decision: GateDecision::Allow,
            ratchet: None,
            override_used,
            recorded_at: now.to_string(),
        })
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn authority() -> DefaultSignedGateAuthority {
        DefaultSignedGateAuthority
    }

    fn good_sig() -> String {
        "a".repeat(64)
    }

    fn policy(strictness: Strictness) -> GatePolicy {
        GatePolicy {
            policy_id: "p1".to_string(),
            strictness,
            signed_by: "alice".to_string(),
            signature: good_sig(),
            recorded_at: "t0".to_string(),
        }
    }

    #[test]
    fn s1_valid_signed_policy_allow() {
        let p = policy(Strictness::Standard);
        let r = authority().evaluate(&p, None, None, &["alice".to_string()], None, "g1", "t1");
        assert!(matches!(r, Ok(ref e) if e.decision == GateDecision::Allow));
    }

    #[test]
    fn s2_unknown_signer_rejected() {
        let p = policy(Strictness::Standard);
        let r = authority().evaluate(&p, None, None, &[], None, "g1", "t1");
        assert_eq!(r, Err(GateError::UnknownSigner));
    }

    #[test]
    fn s3_invalid_signature_rejected() {
        let mut p = policy(Strictness::Standard);
        p.signature = "x".repeat(63);
        let r = authority().evaluate(&p, None, None, &["alice".to_string()], None, "g1", "t1");
        assert_eq!(r, Err(GateError::InvalidSignature));
    }

    #[test]
    fn s4_non_monotonic_ratchet_rejected() {
        let p = policy(Strictness::Hardened);
        let r = Some(PolicyRatchet {
            before: Strictness::Hardened,
            after: Strictness::Lax,
        });
        let result = authority().evaluate(
            &p,
            r,
            None,
            &["alice".to_string()],
            Some(Strictness::Hardened),
            "g1",
            "t1",
        );
        assert_eq!(
            result,
            Err(GateError::NonMonotonicStrictness {
                from: Strictness::Hardened,
                to: Strictness::Lax,
            })
        );
    }

    #[test]
    fn s5_valid_monotonic_ratchet_accepted() {
        let p = policy(Strictness::Standard);
        let r = Some(PolicyRatchet {
            before: Strictness::Standard,
            after: Strictness::Strict,
        });
        let result = authority().evaluate(
            &p,
            r,
            None,
            &["alice".to_string()],
            Some(Strictness::Standard),
            "g1",
            "t1",
        );
        match result {
            Ok(e) => assert!(e.ratchet.is_some()),
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    #[test]
    fn s6_expired_override_rejected() {
        let p = policy(Strictness::Standard);
        let o = Some(GateOverride::Allow {
            reason: "ops".to_string(),
            expires_at: "2020-01-01".to_string(),
        });
        let r = authority().evaluate(
            &p,
            None,
            o,
            &["alice".to_string()],
            None,
            "g1",
            "2099-01-01",
        );
        assert_eq!(r, Err(GateError::ExpiredOverride));
    }

    #[test]
    fn s7_closed_set_audit() {
        assert_eq!(STRICTNESS_VARIANT_COUNT, 4);
        let strictness = [
            Strictness::Lax,
            Strictness::Standard,
            Strictness::Strict,
            Strictness::Hardened,
        ];
        assert_eq!(strictness.len(), STRICTNESS_VARIANT_COUNT);

        assert_eq!(GATE_DECISION_VARIANT_COUNT, 3);
        let decisions = [GateDecision::Allow, GateDecision::Deny, GateDecision::Warn];
        assert_eq!(decisions.len(), GATE_DECISION_VARIANT_COUNT);
    }

    #[test]
    fn s8_empty_signature_rejected() {
        let mut p = policy(Strictness::Standard);
        p.signature = String::new();
        let r = authority().evaluate(&p, None, None, &["alice".to_string()], None, "g1", "t1");
        assert_eq!(r, Err(GateError::EmptySignature));
    }
}
