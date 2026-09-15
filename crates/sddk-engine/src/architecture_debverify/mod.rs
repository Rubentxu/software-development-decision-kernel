// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architecture_debverify/mod.rs — A3-S7 / AC5 public surface.
//
// DebVerify: the **global** architecture challenge pass. It looks for
// duplicate/shadow authorities, missing owners, bypasses, stale compatibility
// and contradictions even when no recent delta points at them.
//
// Cycle: `p-63676b11dc0ef88f/a3-7-ac5-debverify-audit` (A3-S7)
// Spec: `docs/architecture/specs/arch-spec-A3-S7-ac5-debverify-audit.md`
// Upstream: `arch-spec-034` AC-034-002 (remains `proposed`)
//
// # Exit criterion: not `verify --full` (AC-034-002)
//
// The distinction from AC4 is structural, not cosmetic:
//
// | | AC4 `architecture_conformance` | AC5 `architecture_debverify` |
// |---|---|---|
// | input | a change basis (`changed_units`) | the whole contract set |
// | scope | contracts reachable from changed units | every registered contract |
// | runs with no change? | nothing affected | **yes** |
// | output | `ArchitectureConformanceDelta` | `DebVerifyAudit` |
//
// `run_debverify_audit(overlay, contracts, now)` therefore takes **no** change
// basis, this module never imports AC4, and the two output types have no
// conversion between them.
//
// # State classes (per ADR-0095)
//
// - `DebVerifyAudit` — **PROJECTION**: derived, reconstructible from
//   (graph, contracts, now). Not a persistence authority; no IO.
// - `DebVerifyFinding`, `FindingSeverity`, `DebVerifyFindingKind` —
//   **EPHEMERAL** computed values / closed enums.
// - `ArchitecturalContract` — **OBJECT** (AC1); read-only here.
//
// # Anti-encroachment (REQ-AC5-020..022)
//
// This module does NOT `use` `architecture_conformance`, `architecture_mutation`,
// `paradigm_profile`, `provider`, `host_sdk`, `agent_host`, `capability` or
// `effective_instructions`. It constructs no `ArchitectureClaim`, mutates no
// graph, performs no IO, and emits no numeric score.
//
// # Submodules
//
// - `types` — closed vocabularies + the audit receipt.
// - `detectors` — the five pure global detectors.
// - `audit` — `run_debverify_audit` + digest.
// - `tests` — acceptance + exit-criterion + anti-encroachment pins.

pub mod audit;
pub mod detectors;
pub mod types;

#[cfg(test)]
mod tests;

pub use audit::{AUDIT_DIGEST_DOMAIN, audit_digest, run_debverify_audit};
pub use detectors::{
    authority_bypass, contradiction, kind_is_auditable, missing_owner, shadow_authority,
    stale_compatibility,
};
pub use types::{
    DebVerifyAudit, DebVerifyError, DebVerifyFinding, DebVerifyFindingKind, FindingSeverity,
};
