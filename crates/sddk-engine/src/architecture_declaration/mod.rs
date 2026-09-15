// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architecture_declaration/mod.rs — A3-S10 declarative architecture contracts.
//
// Parses and **validates** a declarative architecture description into AC1
// `ArchitecturalContract`s and AC2 `SoftwareUnit`s, fail-closed.
//
// Cycle: `p-63676b11dc0ef88f/a3-10-ac-cli-surface` (A3-S10)
// Spec: `docs/architecture/specs/arch-spec-A3-S10-architecture-cli-surface.md`
// ADR: `ADR-0120-DECLARATIVE-ARCHITECTURE-CONTRACTS`
//
// # Input, not authority
//
// The declaration file is a *manifest of intent*. `validate` converts it into
// the AC1/AC2 semantic objects; only those objects are authority. This module
// is format-agnostic: it takes the already-parsed `DeclarationFile` and performs
// no IO, so the CLI owns YAML and the engine owns semantics.
//
// # Fail-closed (AC-UAT-001)
//
// A blank id, a duplicate id, a missing kind-specific field, an unrecognised
// kind, or an AC1 rejection is an error — never a silent skip or a default.
//
// # Submodules
//
// - `types` — the declaration shapes + the error taxonomy.
// - `validate` — the fail-closed conversion.
// - `tests` — acceptance + anti-encroachment pins.

pub mod types;
pub mod validate;

#[cfg(test)]
mod tests;

pub use types::{ContractDecl, DeclarationError, DeclarationFile, DeclaredArchitecture, UnitDecl};
pub use validate::{contract_from_decl, unit_from_decl, validate};
