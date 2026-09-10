//! `agent_surface_golden` — locked JSON shape for the `AgentCommandSurface`.
//!
//! M7.1B (v1.161.0): closes A6 (registry golden snapshot). The fixture
//! locks the public surface shape: command name + stability +
//! side_effect_class + required_authority + outputs + examples count.
//! Example bodies are intentionally NOT included — they move too much
//! between cycles and the walker handles body-level conformance.
//!
//! Update path:
//! ```sh
//! UPDATE_SNAPSHOTS=1 cargo test -p sddk-cli --lib agent_surface_golden
//! ```

use crate::command_spec::SurfaceFilter;
use crate::command_surface::surface_with_filter;
use serde::{Deserialize, Serialize};

/// One entry of the golden fixture — locked subset of `CommandSpec`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct GoldenEntry {
    /// Command name (e.g., "target list").
    pub name: String,
    /// Stability classification.
    pub stability: String,
    /// Side-effect classification.
    pub side_effect_class: String,
    /// Authority requirement.
    pub required_authority: String,
    /// Machine output schema name.
    pub machine: String,
    /// Number of examples published (intentionally not the bodies).
    pub examples_n: usize,
}

/// Top-level golden fixture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct GoldenSurface {
    /// Schema version of this fixture (bumped on shape changes).
    pub schema_version: u32,
    /// Number of entries.
    pub total: usize,
    /// Per-command entries, in registry order.
    pub entries: Vec<GoldenEntry>,
}

const SCHEMA_VERSION: u32 = 1;

/// Build the current golden surface from the live registry.
pub fn current() -> GoldenSurface {
    let surface = surface_with_filter(SurfaceFilter::default());
    let entries: Vec<GoldenEntry> = surface
        .commands()
        .iter()
        .map(|e| GoldenEntry {
            name: e.spec.name.clone(),
            stability: format!("{:?}", e.spec.stability).to_lowercase(),
            side_effect_class: format!("{:?}", e.spec.side_effect_class).to_lowercase(),
            required_authority: format!("{:?}", e.spec.required_authority).to_lowercase(),
            machine: e.spec.outputs.machine.clone(),
            examples_n: e.spec.examples.len(),
        })
        .collect();
    GoldenSurface {
        schema_version: SCHEMA_VERSION,
        total: entries.len(),
        entries,
    }
}

/// Render `current()` as deterministic JSON suitable for snapshot
/// comparison.
pub fn current_json() -> String {
    let mut s = serde_json::to_string_pretty(&current()).expect("golden serializes");
    s.push('\n');
    s
}

/// Path to the checked-in golden fixture.
pub const GOLDEN_PATH: &str = "tests/fixtures/agent-surface.golden.json";

/// Compare `current_json()` with the fixture on disk. Returns `Ok(())`
/// when they match byte-for-byte, or `Err(diff_summary)` with a brief
/// description of the first divergence.
pub fn matches_fixture() -> Result<(), String> {
    let observed = current_json();
    let expected = std::fs::read_to_string(GOLDEN_PATH).map_err(|e| {
        format!(
            "missing golden fixture at {}: {} (regenerate with UPDATE_SNAPSHOTS=1)",
            GOLDEN_PATH, e
        )
    })?;
    if observed == expected {
        Ok(())
    } else {
        Err(format!(
            "golden mismatch ({} bytes observed vs {} bytes expected)",
            observed.len(),
            expected.len()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_produces_deterministic_json() {
        let a = current_json();
        let b = current_json();
        assert_eq!(a, b);
        assert!(a.contains("\"schema_version\": 1"));
        assert!(a.contains("\"total\""));
    }

    #[test]
    fn current_covers_at_least_one_command_with_examples() {
        let g = current();
        assert!(
            g.total > 0,
            "expected registry to expose at least one command"
        );
        // M7.1 published 6 examples on target list/resolve/run; the
        // golden must reflect that.
        let with_examples = g.entries.iter().filter(|e| e.examples_n > 0).count();
        assert!(
            with_examples >= 1,
            "expected at least one command to publish examples (M7.1 contract)"
        );
    }

    #[test]
    fn matches_checked_in_fixture() {
        if std::env::var("UPDATE_SNAPSHOTS").is_ok() {
            std::fs::write(GOLDEN_PATH, current_json()).expect("write golden");
            return;
        }
        matches_fixture().unwrap_or_else(|e| panic!("golden mismatch: {}", e));
    }
}
