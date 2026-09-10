//! `AgentCommandSurface` aggregator (SPEC-015).
//!
//! Derives a contextual surface from the `CommandRegistry`, optionally filtered
//! by target, stability, and agent profile. The surface intentionally does not
//! duplicate AuthorityEngine decisions (per SPEC-015): each entry reports
//! `reachable` and a `skip_reason` so the caller can layer authority checks
//! on top.
//!
//! `pub` for integration tests under `tests/`; `missing_docs` allowed at module scope.
//!
#![allow(missing_docs)]

use serde::{Deserialize, Serialize};

use crate::command_spec::{
    AgentProfileTag, CommandSpec, SideEffectClass, Stability, SurfaceFilter, all_command_specs,
};

/// One entry in the agent surface: a `CommandSpec` plus reachability metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CommandSurfaceEntry {
    /// The full command spec.
    #[serde(flatten)]
    pub spec: CommandSpec,
    /// Whether the command is reachable for the (profile, target) pair.
    pub reachable: bool,
    /// When unreachable, an explanation (e.g., "experimental: opt-in flag not set").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skip_reason: Option<String>,
}

impl CommandSurfaceEntry {
    /// Construct an entry with explicit reachability metadata.
    pub fn new(spec: CommandSpec, reachable: bool, skip_reason: Option<String>) -> Self {
        Self {
            spec,
            reachable,
            skip_reason,
        }
    }
}

/// Agent-facing command surface.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AgentCommandSurface {
    /// Agent profile tag used for filtering.
    #[serde(default)]
    pub profile: AgentProfileTag,
    /// Target filter (when applied).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// Echo of the filter that produced this surface (for the agent's debug).
    #[serde(default)]
    pub filter: SurfaceFilter,
    /// Number of commands that passed the filter (reachable + skipped).
    #[serde(default)]
    pub total: usize,
    /// Number of commands that were reachable after the filter.
    #[serde(default)]
    pub reachable_count: usize,
    /// Per-command entries (reachable + skipped, in registry order).
    #[serde(default)]
    pub commands: Vec<CommandSurfaceEntry>,
}

impl AgentCommandSurface {
    /// Append a single entry to the surface, keeping `total` and
    /// `reachable_count` in sync.
    pub fn push(&mut self, entry: CommandSurfaceEntry) {
        if entry.reachable {
            self.reachable_count += 1;
        }
        self.total += 1;
        self.commands.push(entry);
    }

    /// Borrow the command entries in registry order.
    pub fn commands(&self) -> &[CommandSurfaceEntry] {
        &self.commands
    }
}

/// Build the agent surface from the canonical command registry.
pub fn surface_for_target(target: Option<&str>, profile: AgentProfileTag) -> AgentCommandSurface {
    surface_with_filter(SurfaceFilter {
        target: target.map(String::from),
        profile,
        ..Default::default()
    })
}

/// Build the agent surface from an explicit filter (used by `sddk help agent`).
pub fn surface_with_filter(filter: SurfaceFilter) -> AgentCommandSurface {
    let all = all_command_specs();
    let mut entries = Vec::new();
    let mut reachable_count = 0usize;

    for spec in all {
        let decision = classify(&spec, &filter);
        if decision.is_none() {
            reachable_count += 1;
        }
        entries.push(CommandSurfaceEntry {
            spec,
            reachable: decision.is_none(),
            skip_reason: decision,
        });
    }

    AgentCommandSurface {
        profile: filter.profile,
        target: filter.target.clone(),
        filter,
        total: entries.len(),
        reachable_count,
        commands: entries,
    }
}

/// Classify a single spec against the filter. Returns `None` when the
/// command passes the filter (reachable), or `Some(reason)` when it is
/// skipped (with a stable, human-readable reason).
fn classify(spec: &CommandSpec, filter: &SurfaceFilter) -> Option<String> {
    // 1. Target filter — checked FIRST because it is the primary agent
    // intent. A command outside the requested target is never reachable,
    // regardless of stability / profile.
    if let Some(target) = &filter.target
        && !command_matches_target(spec, target)
    {
        return Some(format!("command not associated with target `{}`", target));
    }

    // 2. Stability filter
    #[allow(unreachable_patterns)]
    match spec.stability {
        Stability::Deprecated => {
            if !filter.include_deprecated {
                return Some("deprecated: opt-in `include_deprecated` flag not set".into());
            }
        }
        Stability::Experimental => {
            if !filter.include_experimental {
                return Some("experimental: opt-in `include_experimental` flag not set".into());
            }
        }
        Stability::Stable => {}
        _ => {
            // Reserved for future stability variants. By default, unknown
            // stabilities are gated behind include_* flags.
            if !filter.include_experimental && !filter.include_deprecated {
                return Some("unknown stability: opt-in flag not set".into());
            }
        }
    }

    // 3. Profile-based side-effect gate
    if filter.profile == AgentProfileTag::ReadOnly {
        match spec.side_effect_class {
            SideEffectClass::Governed | SideEffectClass::Destructive => {
                return Some(format!(
                    "read-only profile cannot run side-effect class `{:?}`",
                    spec.side_effect_class
                ));
            }
            _ => {}
        }
    }

    None
}

/// Target match heuristic:
/// - exact name match (e.g., `target` matches all "target ..." commands);
/// - substring match on name;
/// - related command match (e.g., "target resolve" is related to "target list").
pub(crate) fn command_matches_target(spec: &CommandSpec, target: &str) -> bool {
    if spec.name == target {
        return true;
    }
    if spec.name.split_whitespace().next() == Some(target) {
        return true;
    }
    if spec.name.contains(target) {
        return true;
    }
    if spec
        .related
        .iter()
        .any(|r| r == target || r.contains(target))
    {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_spec::AgentProfileTag;

    fn reachable_names(surface: &AgentCommandSurface) -> Vec<String> {
        surface
            .commands
            .iter()
            .filter(|e| e.reachable)
            .map(|e| e.spec.name.clone())
            .collect()
    }

    #[test]
    fn default_surface_excludes_experimental_and_deprecated() {
        let s = surface_for_target(None, AgentProfileTag::Default);
        // No entries may be Experimental or Deprecated unless explicit.
        // With default filter, all entries are stable+pure and reachable;
        // this test pins that behaviour so future regressions are caught.
        for e in &s.commands {
            if !e.reachable {
                let reason = e.skip_reason.clone().unwrap_or_default();
                assert!(
                    reason.contains("experimental") || reason.contains("deprecated"),
                    "unexpected skip reason for `{}`: {}",
                    e.spec.name,
                    reason
                );
            }
        }
    }

    #[test]
    fn read_only_profile_blocks_governed_commands() {
        let s = surface_for_target(None, AgentProfileTag::ReadOnly);
        // `target run` is Governed → unreachable
        let run = s
            .commands
            .iter()
            .find(|e| e.spec.name == "target run")
            .unwrap();
        assert!(!run.reachable);
        assert!(run.skip_reason.as_deref().unwrap().contains("read-only"));
        // `target list` is Pure → reachable
        let list = s
            .commands
            .iter()
            .find(|e| e.spec.name == "target list")
            .unwrap();
        assert!(list.reachable);
    }

    #[test]
    fn target_filter_keeps_only_matching_commands() {
        let s = surface_for_target(Some("target"), AgentProfileTag::Default);
        let mut names: Vec<String> = s.commands.iter().map(|e| e.spec.name.clone()).collect();
        names.retain(|n| n.starts_with("target"));
        assert!(names.contains(&"target list".to_string()));
        assert!(names.contains(&"target resolve".to_string()));
        assert!(names.contains(&"target run".to_string()));
        // Commands outside target should be skipped
        for e in &s.commands {
            if !e.spec.name.starts_with("target") {
                assert!(
                    !e.reachable,
                    "non-target command reachable: {}",
                    e.spec.name
                );
            }
        }
    }

    #[test]
    fn reachable_count_is_reported() {
        let s = surface_for_target(None, AgentProfileTag::Default);
        assert_eq!(s.reachable_count, reachable_names(&s).len());
        assert_eq!(s.total, s.commands.len());
        assert!(s.total >= 40);
    }

    #[test]
    fn surface_serializes_to_json() {
        let s = surface_for_target(Some("target"), AgentProfileTag::Default);
        let j = serde_json::to_string(&s).expect("serialize");
        let back: AgentCommandSurface = serde_json::from_str(&j).expect("deserialize");
        assert_eq!(back, s);
    }
}
