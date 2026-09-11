// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
//! SPIKE AX-S3 — contextual command-surface token budget.
//!
//! Question (SPIKES.md AX-S3): measure the global CLI contract against
//! task-specific surfaces for the common `change`, `verify`, `ship` and
//! `recover` agents. Select minimal-surface heuristics that preserve task
//! success while reducing irrelevant commands.
//!
//! Method: for each flow, expand the command surface from a seed command
//! at increasing relatedness depths (0 = only the seed, 1 = + related
//! and same-verb commands, 2 = + related of related) and estimate the
//! rendered token cost (chars / 4, a standard rough approximation).
//! The current production heuristic (`command_matches_target`) is depth-0
//! plus a substring pass, so this harness quantifies what richer
//! expansion would cost and what the global baseline is.
//!
//! Findings are recorded in
//! `docs/architecture/spikes/AX-S3-command-surface-token-budget.md`.

use crate::command_spec::AgentProfileTag;
use crate::command_spec::all_command_specs;
use crate::command_surface::surface_for_target;

/// Rough token estimate: ~4 characters per token for English + code text.
pub fn estimate_tokens(text: &str) -> usize {
    text.len().div_ceil(4)
}

/// Rendered size of the agent cheat sheet for a target filter, in tokens.
/// `None` target = the full 49-command global contract.
pub fn surface_tokens(target: Option<&str>) -> usize {
    let surface = surface_for_target(target, AgentProfileTag::Default);
    estimate_tokens(&render_reachable(&surface))
}

fn render_reachable(surface: &crate::command_surface::AgentCommandSurface) -> String {
    let mut out = String::new();
    for entry in &surface.commands {
        if !entry.reachable {
            continue;
        }
        out.push_str(&entry.spec.name);
        out.push_str(&format!("\n{}\n", entry.spec.about));
        for a in &entry.spec.args {
            out.push_str(&format!("--{} {}\n", a.name, a.help));
        }
    }
    out
}

/// One flow measurement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowMeasurement {
    /// The AX-S3 flow name (`change`, `verify`, `ship`, `recover`).
    pub flow: &'static str,
    /// Commands reachable at the production (depth-0 + substring) heuristic.
    pub depth0_commands: Vec<String>,
    /// Commands added by one hop of `related` expansion.
    pub depth1_added: Vec<String>,
    /// Token cost of the global contract.
    pub global_tokens: usize,
    /// Token cost of the depth-0 task surface.
    pub depth0_tokens: usize,
    /// Token cost of depth-0 + one related hop.
    pub depth1_tokens: usize,
}

/// Measure the four AX-S3 flows against the live spec table.
pub fn measure_flows() -> Vec<FlowMeasurement> {
    let flows = ["change", "verify", "ship", "recover"];
    let specs = all_command_specs();
    let global_tokens = surface_tokens(None);

    flows
        .into_iter()
        .map(|flow| {
            let surface = surface_for_target(Some(flow), AgentProfileTag::Default);
            let depth0: Vec<String> = surface
                .commands
                .iter()
                .filter(|e| e.reachable)
                .map(|e| e.spec.name.clone())
                .collect();
            // One related hop: any spec whose name appears in the `related`
            // lists of the depth-0 set.
            let mut depth1_added: Vec<String> = Vec::new();
            for entry in surface.commands.iter().filter(|e| e.reachable) {
                for rel in &entry.spec.related {
                    let rel_word = rel.split_whitespace().next().unwrap_or(rel);
                    if !depth0.iter().any(|n| n == rel || n.starts_with(rel_word))
                        && !depth1_added.contains(rel)
                        && specs.iter().any(|s| s.name == *rel)
                    {
                        depth1_added.push(rel.clone());
                    }
                }
            }
            let depth0_tokens = surface_tokens(Some(flow));
            // depth-1 estimate: depth0 tokens + rendered cost of added specs.
            let depth1_render: usize = depth1_added
                .iter()
                .map(|name| {
                    let s = specs.iter().find(|s| &s.name == name).expect("checked");
                    estimate_tokens(&s.name) + estimate_tokens(&s.about)
                })
                .sum();
            FlowMeasurement {
                flow,
                depth0_commands: depth0,
                depth1_added,
                global_tokens,
                depth0_tokens,
                depth1_tokens: depth0_tokens + depth1_render,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_contract_is_the_budget_ceiling() {
        // The global surface renders ~8.4 KB (~2.1k tokens) for the current
        // 49-command table. Pin a sanity band rather than a brittle exact
        // number: it must be at least 5x any single task surface, or the
        // "contextual surfaces save budget" premise is void.
        let global = surface_tokens(None);
        for m in measure_flows() {
            assert!(
                global >= m.depth0_tokens * 5,
                "global ({global}) must dwarf {} depth-0 surface ({})",
                m.flow,
                m.depth0_tokens
            );
        }
    }

    #[test]
    fn task_surfaces_are_strictly_smaller_than_global() {
        let global = surface_tokens(None);
        for m in measure_flows() {
            assert!(
                m.depth0_tokens < global,
                "{} surface must be smaller",
                m.flow
            );
        }
    }

    #[test]
    fn depth1_expansion_stays_within_half_the_global_budget() {
        // A heuristic that adds one related hop must still cost well under
        // the global contract — that is the design headroom for richer
        // task surfaces without falling back to "send everything".
        for m in measure_flows() {
            assert!(
                m.depth1_tokens * 2 <= surface_tokens(None),
                "{} depth-1 surface ({}) exceeds half the global budget",
                m.flow,
                m.depth1_tokens
            );
        }
    }

    #[test]
    fn measurements_are_deterministic() {
        let a = measure_flows();
        let b = measure_flows();
        assert_eq!(a, b);
    }
}
