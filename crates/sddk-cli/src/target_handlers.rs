// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
//! Host handlers for Target task bodies (M6.3).
//!
//! The engine declares the `TaskHandler` trait; this module supplies the
//! CLI's concrete handlers so `sddk target run` executes real work instead
//! of stub no-ops. Handlers receive the walk `HandlerContext` (upstream
//! task outputs) and return `HandlerOutcome`.
//!
//! Honesty contract (SP-07): handlers fail loudly on missing context or
//! hostile state — a Target run never fabricates success.

use sddk_engine::target_task::handlers::{
    HandlerContext, HandlerOutcome, HandlerRegistry, TaskHandler,
};

use crate::{
    CliEnvironment, OutputFormat, change,
    cycle::{RuntimeArgs, resolve_cycle_context},
};

/// `context.resolve` — resolve the active cycle id for the workspace.
///
/// Delegates to the same inference path every cycle command uses
/// (`resolve_cycle_context`). On success, publishes `cycle_id` into the
/// context for downstream handlers.
pub struct ContextResolveHandler {
    runtime: RuntimeArgs,
    environment: CliEnvironment,
}

impl ContextResolveHandler {
    pub fn new(runtime: RuntimeArgs, environment: CliEnvironment) -> Self {
        Self {
            runtime,
            environment,
        }
    }
}

impl TaskHandler for ContextResolveHandler {
    fn execute(&mut self, ctx: &mut HandlerContext) -> HandlerOutcome {
        match resolve_cycle_context(&self.runtime, &self.environment, None) {
            Ok(resolved) => match resolved.cycle_id {
                Some(cycle_id) => {
                    ctx.values.insert("cycle_id".into(), cycle_id.clone());
                    HandlerOutcome::Ok(format!("active cycle: {cycle_id}"))
                }
                None => HandlerOutcome::Err(
                    "context.resolve: no active cycle found; start one with `sddk cycle start`"
                        .into(),
                ),
            },
            Err(e) => HandlerOutcome::Err(format!("context.resolve: {e}")),
        }
    }
}

/// `plan.workitem.create` — create a work item in the resolved cycle.
///
/// Requires `cycle_id` in the context (produced by `context.resolve`).
/// Delegates to the existing `change` facade (`plan work-item create`).
pub struct WorkItemCreateHandler {
    pub title: String,
    pub description: String,
    pub actor: Option<String>,
    environment: CliEnvironment,
}

impl WorkItemCreateHandler {
    pub fn new(
        title: String,
        description: String,
        actor: Option<String>,
        environment: CliEnvironment,
    ) -> Self {
        Self {
            title,
            description,
            actor,
            environment,
        }
    }
}

impl TaskHandler for WorkItemCreateHandler {
    fn execute(&mut self, ctx: &mut HandlerContext) -> HandlerOutcome {
        let cycle_id = match ctx.values.get("cycle_id") {
            Some(id) => id.clone(),
            None => return HandlerOutcome::Err(
                "plan.workitem.create: no cycle_id in context (upstream context.resolve missing?)"
                    .into(),
            ),
        };
        let out = change::run_change(
            cycle_id,
            self.title.clone(),
            self.description.clone(),
            self.actor.clone(),
            OutputFormat::Json,
            &self.environment,
        );
        if out.status == 0 {
            HandlerOutcome::Ok("work item created".into())
        } else {
            let detail = if out.stderr.is_empty() {
                out.stdout.trim().to_string()
            } else {
                out.stderr.trim().to_string()
            };
            HandlerOutcome::Err(format!("plan.workitem.create: {detail}"))
        }
    }
}

/// Builds the handler registry for the `change` target.
pub fn change_handlers(
    environment: &CliEnvironment,
    title: String,
    description: String,
    actor: Option<String>,
) -> HandlerRegistry {
    let mut registry = HandlerRegistry::new();
    registry.register(
        "context.resolve",
        Box::new(ContextResolveHandler::new(
            RuntimeArgs {
                root: None,
                scope: None,
                remote: None,
                fallback_seed: None,
                no_infer: false,
            },
            environment.clone(),
        )),
    );
    registry.register(
        "plan.workitem.create",
        Box::new(WorkItemCreateHandler::new(
            title,
            description,
            actor,
            environment.clone(),
        )),
    );
    registry
}
