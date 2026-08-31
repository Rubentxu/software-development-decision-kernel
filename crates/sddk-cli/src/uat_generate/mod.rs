//! E14.5 — Generate pipeline: requirements → UAT plan.
//!
//! Orchestrates: optional discover → plan → enrich → quality → approval → validate.
//! All inputs validated BEFORE any file write (atomic write rule).
//!
//! # Module structure
//!
//! - `parsing.rs` — text parsing utilities (criteria extraction, changelog parsing)
//! - `validator.rs` — input validation (requirements, changelog, discover)
//! - `planner/` — pure planner with merge and build submodules
//! - `runner/` — pipeline orchestration with injectable ApprovalIo
//! - `tests.rs` — integration and unit tests for the generate pipeline

mod parsing;
pub mod planner;
pub(crate) mod runner;
#[cfg(test)]
mod tests;
mod validator;
