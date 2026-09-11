// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
//! `handlers` — task body dispatch for the Target/Task DAG (M6.3).
//!
//! A `TaskHandler` executes the real body of one task id. The engine
//! defines the trait; the CLI (or any host) supplies the concrete
//! handlers, so the engine stays free of host concerns (fs, env).
//!
//! Dispatch rules in `DagExecutor::walk`:
//! - `has_body: true` + handler registered → handler runs; success emits
//!   `executed`, error emits `failed` and halts downstream tasks.
//! - `has_body: true` + NO handler → `failed` with an explicit note (a
//!   declared body that the host cannot execute is a host defect, not a
//!   successful no-op).
//! - `has_body: false` → `not_implemented` (SP-07 honesty invariant,
//!   unchanged).

use std::collections::BTreeMap;

/// Outcome of a single handler invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandlerOutcome {
    /// Body ran to completion; note is appended to the receipt.
    Ok(String),
    /// Body failed; note explains why. Downstream tasks are skipped.
    Err(String),
}

/// Executes the real body of one task id. Object safe so hosts can pass
/// arbitrary context via self.
pub trait TaskHandler {
    fn execute(&mut self, ctx: &mut HandlerContext) -> HandlerOutcome;
}

/// Context threaded through a DAG walk: outputs of upstream tasks are
/// visible to downstream handlers (e.g. cycle_id from context.resolve).
#[derive(Debug, Default)]
pub struct HandlerContext {
    /// Values produced by upstream tasks, keyed by task id.
    pub values: BTreeMap<String, String>,
}

/// Map of task id → handler.
#[derive(Default)]
pub struct HandlerRegistry {
    handlers: BTreeMap<String, Box<dyn TaskHandler>>,
}

impl HandlerRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, task_id: &str, handler: Box<dyn TaskHandler>) {
        self.handlers.insert(task_id.to_string(), handler);
    }

    pub fn get_mut(&mut self, task_id: &str) -> Option<&mut Box<dyn TaskHandler>> {
        self.handlers.get_mut(task_id)
    }

    pub fn contains_key(&self, task_id: &str) -> bool {
        self.handlers.contains_key(task_id)
    }
}

impl std::fmt::Debug for HandlerRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HandlerRegistry")
            .field("handlers", &self.handlers.keys().collect::<Vec<_>>())
            .finish()
    }
}
