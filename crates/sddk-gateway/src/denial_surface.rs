//! AIW-S8 X02 — Denial surface for raw args/secretos at the host API boundary.
//!
//! Closes SCOPE-CONTRACT §S8-STOP-1: a malformed host action call (raw
//! bytes, secret-prefixed args, oversized payload) is denied at evaluation
//! time with **0 leak** of the raw content — denial reasons never carry the
//! offending argument value, only the structural fact that triggered them.

use serde::{Deserialize, Serialize};

/// Structural reason a host action was denied. Never carries raw argument
/// content: only prefixes, sizes, or the fact of malformed bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DenialReason {
    /// An argument starts with a configured secret prefix.
    SecretPrefixedArg {
        /// The matched secret prefix (never the argument value).
        prefix: String,
    },
    /// The payload exceeds the policy's maximum size.
    OversizedPayload {
        /// Actual payload size in bytes.
        size_bytes: usize,
        /// Policy maximum in bytes.
        max_bytes: usize,
    },
    /// An argument contains raw byte content (embedded NUL).
    RawBytesArg,
    /// An argument is not valid UTF-8.
    NonUtf8Arg,
    /// The action id is empty.
    EmptyActionId,
}

/// Verdict of evaluating a [`HostActionRequest`] against a [`DenialPolicy`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DenialVerdict {
    /// The request may proceed.
    Allowed,
    /// The request is denied for the given structural reason.
    Denied(DenialReason),
}

/// Denial policy: payload size cap and secret-prefix denylist.
#[derive(Debug, Clone, Copy)]
pub struct DenialPolicy {
    /// Maximum accepted payload size in bytes.
    pub max_payload_bytes: usize,
    /// Argument prefixes that indicate embedded secrets.
    pub secret_prefixes: &'static [&'static str],
}

impl Default for DenialPolicy {
    fn default() -> Self {
        Self {
            max_payload_bytes: 64 * 1024, // 64 KiB
            secret_prefixes: &["SECRET_", "TOKEN_", "PASSWORD_", "API_KEY_"],
        }
    }
}

/// A CLI-shaped host action request crossing the API boundary.
#[derive(Debug, Clone)]
pub struct HostActionRequest {
    /// Identifier of the action being requested.
    pub action_id: String,
    /// String arguments for the action.
    pub args: Vec<String>,
    /// Optional raw byte payload.
    pub payload: Option<Vec<u8>>,
}

impl HostActionRequest {
    /// Evaluates the request against the policy. Denials are decided at
    /// parse/evaluation time; no raw content escapes into the verdict.
    pub fn evaluate(&self, policy: &DenialPolicy) -> DenialVerdict {
        if self.action_id.is_empty() {
            return DenialVerdict::Denied(DenialReason::EmptyActionId);
        }
        for arg in &self.args {
            for prefix in policy.secret_prefixes {
                if arg.starts_with(prefix) {
                    return DenialVerdict::Denied(DenialReason::SecretPrefixedArg {
                        prefix: (*prefix).to_string(),
                    });
                }
            }
            if std::str::from_utf8(arg.as_bytes()).is_err() {
                return DenialVerdict::Denied(DenialReason::NonUtf8Arg);
            }
            if arg.as_bytes().contains(&0) {
                return DenialVerdict::Denied(DenialReason::RawBytesArg);
            }
        }
        if let Some(p) = &self.payload
            && p.len() > policy.max_payload_bytes
        {
            return DenialVerdict::Denied(DenialReason::OversizedPayload {
                size_bytes: p.len(),
                max_bytes: policy.max_payload_bytes,
            });
        }
        DenialVerdict::Allowed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_action_id_denied() {
        let req = HostActionRequest {
            action_id: String::new(),
            args: vec![],
            payload: None,
        };
        assert_eq!(
            req.evaluate(&DenialPolicy::default()),
            DenialVerdict::Denied(DenialReason::EmptyActionId)
        );
    }

    #[test]
    fn secret_prefix_denied() {
        let req = HostActionRequest {
            action_id: "deploy".into(),
            args: vec!["TOKEN_ABC123".into()],
            payload: None,
        };
        assert_eq!(
            req.evaluate(&DenialPolicy::default()),
            DenialVerdict::Denied(DenialReason::SecretPrefixedArg {
                prefix: "TOKEN_".into()
            })
        );
    }

    #[test]
    fn oversized_payload_denied() {
        let req = HostActionRequest {
            action_id: "ingest".into(),
            args: vec![],
            payload: Some(vec![0u8; 64 * 1024 + 1]),
        };
        assert_eq!(
            req.evaluate(&DenialPolicy::default()),
            DenialVerdict::Denied(DenialReason::OversizedPayload {
                size_bytes: 64 * 1024 + 1,
                max_bytes: 64 * 1024,
            })
        );
    }

    #[test]
    fn raw_bytes_arg_denied() {
        let req = HostActionRequest {
            action_id: "exec".into(),
            args: vec!["bad\u{0}arg".into()],
            payload: None,
        };
        assert_eq!(
            req.evaluate(&DenialPolicy::default()),
            DenialVerdict::Denied(DenialReason::RawBytesArg)
        );
    }

    #[test]
    fn clean_request_allowed() {
        let req = HostActionRequest {
            action_id: "exec".into(),
            args: vec!["--flag".into(), "value".into()],
            payload: Some(vec![1, 2, 3]),
        };
        assert_eq!(
            req.evaluate(&DenialPolicy::default()),
            DenialVerdict::Allowed
        );
    }

    #[test]
    fn denial_zero_leak_no_payload_in_reason() {
        let req = HostActionRequest {
            action_id: "ok".into(),
            args: vec!["SECRET_TOKEN=hello".into()],
            payload: None,
        };
        let verdict = req.evaluate(&DenialPolicy::default());
        match verdict {
            DenialVerdict::Denied(DenialReason::SecretPrefixedArg { prefix }) => {
                assert!(
                    ["SECRET_", "TOKEN_", "PASSWORD_", "API_KEY_"].contains(&prefix.as_str()),
                    "prefix leaked raw arg content: {prefix:?}"
                );
            }
            other => panic!("expected secret-prefix denial, got {other:?}"),
        }
    }
}
