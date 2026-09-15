// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architectural_contract/error.rs — `ContractError` enum.
//
// # State class
//
// `ContractError` is not durable; produced as a result of construction-time
// validation and consumed by callers for diagnostics.

use std::fmt;

/// Errors produced by the architectural contract substrate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractError {
    /// Invalid `ContractId` input.
    InvalidId {
        /// Human-readable reason.
        reason: String,
    },
    /// Invalid `Revision` input.
    InvalidRevision {
        /// Human-readable reason.
        reason: String,
    },
    /// Invalid free-form reference (`ComponentRef`, `EntityRef`, etc.).
    InvalidRef {
        /// Reference kind for diagnostics.
        kind: String,
        /// Human-readable reason.
        reason: String,
    },
    /// Invalid `ContractKindRef` input.
    InvalidKind {
        /// Raw input that was rejected.
        input: String,
        /// Human-readable reason.
        reason: String,
    },
    /// The `provider.<sdk>` namespace is reserved by ADR-0099 and cannot
    /// appear inside contract extension payloads.
    ProviderNamespaceForbidden {
        /// The kind string the caller attempted.
        kind: String,
    },
    /// `ContractKind` and `ContractPayload` tags do not match.
    KindPayloadMismatch {
        /// Kind tag.
        kind: String,
        /// Payload tag.
        payload_kind: String,
    },
}

impl fmt::Display for ContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContractError::InvalidId { reason } => write!(f, "invalid ContractId: {}", reason),
            ContractError::InvalidRevision { reason } => {
                write!(f, "invalid Revision: {}", reason)
            }
            ContractError::InvalidRef { kind, reason } => {
                write!(f, "invalid {}: {}", kind, reason)
            }
            ContractError::InvalidKind { input, reason } => {
                write!(f, "invalid ContractKindRef '{}': {}", input, reason)
            }
            ContractError::ProviderNamespaceForbidden { kind } => write!(
                f,
                "provider namespace forbidden for ContractKindRef '{}' (ADR-0099)",
                kind
            ),
            ContractError::KindPayloadMismatch { kind, payload_kind } => write!(
                f,
                "ContractKind '{}' does not match ContractPayload tag '{}'",
                kind, payload_kind
            ),
        }
    }
}

impl std::error::Error for ContractError {}
