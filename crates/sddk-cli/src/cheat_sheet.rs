//! Cheat-sheet renderer for `sddk help agent` (SPEC-015 + ADR-014).
//!
//! Two renderers:
//!
//! - `render_text`: deterministic markdown (1 heading per reachable command;
//!   subsections `Syntax`, `Purpose`, `Examples`, `Related`).
//! - `render_json`: the `AgentCommandSurface` serialized (re-exported from
//!   `command_surface`).
//!
//! `pub` for integration tests under `tests/`; `missing_docs` allowed at module scope.
//!
#![allow(missing_docs)]

use crate::command_surface::{AgentCommandSurface, CommandSurfaceEntry};

/// Markdown cheat-sheet (default for `--format text`).
pub fn render_text(surface: &AgentCommandSurface) -> String {
    let mut out = String::new();
    out.push_str("# Agent cheat sheet\n\n");
    if let Some(target) = &surface.target {
        out.push_str(&format!("**Target filter:** `{target}`\n"));
    }
    out.push_str(&format!("**Profile:** `{:?}`\n\n", surface.profile));
    out.push_str(&format!(
        "Showing **{}/{}** commands.\n\n",
        surface.reachable_count, surface.total
    ));

    let reachable: Vec<&CommandSurfaceEntry> =
        surface.commands.iter().filter(|e| e.reachable).collect();

    if reachable.is_empty() {
        out.push_str("_No commands match the filter._\n");
        return out;
    }

    for entry in reachable {
        let spec = &entry.spec;
        out.push_str(&format!("## {}\n\n", spec.name));
        if let Some(syntax) = &spec.syntax {
            out.push_str(&format!("**Syntax:** `{}`\n\n", syntax));
        }
        out.push_str(&format!("**Purpose:** {}\n\n", spec.about));
        out.push_str(&format!(
            "*Stability:* `{:?}` *Side-effect:* `{:?}` *Authority:* `{:?}`\n\n",
            spec.stability, spec.side_effect_class, spec.required_authority
        ));
        if !spec.preconditions.is_empty() {
            out.push_str("**Preconditions:**\n");
            for p in &spec.preconditions {
                out.push_str(&format!("- {p}\n"));
            }
            out.push('\n');
        }
        if !spec.examples.is_empty() {
            out.push_str("**Examples:**\n");
            for ex in &spec.examples {
                out.push_str(&format!("- `{}` — {}\n", ex.command, ex.description));
            }
            out.push('\n');
        }
        if !spec.related.is_empty() {
            out.push_str(&format!("**Related:** {}\n\n", spec.related.join(", ")));
        }
    }
    out
}

/// JSON cheat-sheet (re-export convenience).
pub fn render_json(surface: &AgentCommandSurface) -> String {
    serde_json::to_string_pretty(surface).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_spec::AgentProfileTag;
    use crate::command_surface::surface_for_target;

    #[test]
    fn text_renderer_starts_with_heading_and_summary() {
        let s = surface_for_target(None, AgentProfileTag::Default);
        let md = render_text(&s);
        assert!(md.starts_with("# Agent cheat sheet"));
        assert!(md.contains("Profile"));
        assert!(md.contains("commands"));
    }

    #[test]
    fn text_renderer_includes_examples_and_related() {
        let s = surface_for_target(Some("target"), AgentProfileTag::Default);
        let md = render_text(&s);
        // target list, target resolve, target run should each appear with examples
        assert!(md.contains("target list"));
        assert!(md.contains("target resolve"));
        assert!(md.contains("target run"));
        assert!(md.contains("Examples"));
    }

    #[test]
    fn text_renderer_reports_empty_when_no_match() {
        let s = surface_for_target(Some("nonexistent_target"), AgentProfileTag::Default);
        let md = render_text(&s);
        assert!(md.contains("No commands match"));
    }

    #[test]
    fn json_renderer_is_valid_json() {
        let s = surface_for_target(None, AgentProfileTag::Default);
        let j = render_json(&s);
        let parsed: serde_json::Value = serde_json::from_str(&j).expect("valid JSON");
        assert_eq!(parsed["total"], s.total);
    }
}
