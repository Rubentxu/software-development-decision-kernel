//! `surface_integration` — programmatic surface for provider adapters (M7.2).
//!
//! Closes the "integration layer" gap that the M7.1 archive deferred:
//! today `cheat_sheet.rs` only renders Markdown/JSON for human/LLM
//! consumption via the `sddk help agent` CLI dispatcher. Provider
//! adapters (OpenAI, Anthropic, OpenCode, local model runtimes) need a
//! programmatic API that produces the exact JSON payload each provider
//! expects for `tools` (function-calling) so they can inject SDDK's
//! command surface without spawning a subprocess.
//!
//! The integration layer is **deterministic** (byte-for-byte stable over
//! the same registry + filter) and **filter-driven** (consumes an
//! existing `AgentCommandSurface`, so all M7.1 gates — target, profile,
//! stability, deprecated/experimental opt-ins — apply automatically).
//!
//! ## Adapters shipped
//!
//! - `render_openai_tools` — OpenAI function-calling shape:
//!   `{ "type": "function", "function": { name, description, parameters } }`
//! - `render_anthropic_tools` — Anthropic Tools shape:
//!   `{ name, description, input_schema }`
//! - `render_generic_tools` — provider-neutral shape used by adapters
//!   that translate per-provider (e.g., local model runtimes).
//!
//! ## Eligibility
//!
//! Only reachable entries with **at least one published example** are
//! surfaced as tools. A command without a demonstrable example cannot
//! be safely exposed to a model — the example contract is the proof of
//! executability.

use crate::command_spec::ExampleSpec;
use crate::command_surface::{AgentCommandSurface, CommandSurfaceEntry};
use serde::{Deserialize, Serialize};

/// One tool descriptor produced from a reachable command entry.
///
/// This is the provider-neutral shape. Provider-specific renderers
/// project this into OpenAI/Anthropic/generic payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ToolDescriptor {
    /// Stable tool name (provider-safe: lowercase, snake_case, no spaces).
    /// Derived from the command name by replacing whitespace with `_`.
    pub name: String,
    /// Human/LLM description (taken from `CommandSpec::about`).
    pub description: String,
    /// Original command name (e.g., "target list") for the integration
    /// payload — providers use this when echoing the tool call back
    /// to the user, while `name` is the tool identifier.
    pub command: String,
    /// Stability classification as a lowercase string
    /// ("stable" | "experimental" | "deprecated").
    pub stability: String,
    /// Side-effect classification as a lowercase string
    /// ("pure" | "read" | "governed" | "destructive").
    pub side_effect_class: String,
    /// Required authority as a lowercase string
    /// ("none" | "read" | "write" | "admin").
    pub required_authority: String,
    /// Machine output schema name (e.g., "ExecutionReportV1"). Used by
    /// adapters to wire up response validation.
    pub machine_schema: String,
    /// Executable examples proving the tool works. Adapters may surface
    /// these as few-shot demonstrations to the model.
    pub examples: Vec<ToolExample>,
}

/// One executable example attached to a tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ToolExample {
    /// Stable example id (e.g., "target.list.json").
    pub id: String,
    /// Human description.
    pub description: String,
    /// argv to invoke (without the `sddk` program name).
    pub argv: Vec<String>,
    /// Expected exit status.
    pub expected_status: i32,
    /// Substrings that the stdout/stderr must contain.
    pub output_contains: Vec<String>,
}

/// Convert an `AgentCommandSurface` into a vector of `ToolDescriptor`
/// for provider integration. Only reachable entries with at least one
/// published example are included; order matches the surface (registry
/// order). Output is deterministic — no timestamps, no random ids.
pub fn surface_for_provider(surface: &AgentCommandSurface) -> Vec<ToolDescriptor> {
    surface
        .commands()
        .iter()
        .filter(|e| e.reachable && !e.spec.examples.is_empty())
        .map(tool_descriptor_for)
        .collect()
}

fn tool_descriptor_for(entry: &CommandSurfaceEntry) -> ToolDescriptor {
    let spec = &entry.spec;
    ToolDescriptor {
        name: provider_safe_name(&spec.name),
        description: spec.about.clone(),
        command: spec.name.clone(),
        stability: format!("{:?}", spec.stability).to_lowercase(),
        side_effect_class: format!("{:?}", spec.side_effect_class).to_lowercase(),
        required_authority: format!("{:?}", spec.required_authority).to_lowercase(),
        machine_schema: spec.outputs.machine.clone(),
        examples: spec.examples.iter().map(tool_example_for).collect(),
    }
}

fn tool_example_for(example: &ExampleSpec) -> ToolExample {
    ToolExample {
        id: example.id.clone(),
        description: example.description.clone(),
        argv: example.invocation.argv.clone(),
        expected_status: example.expected.status,
        output_contains: example.expected.output_contains.clone(),
    }
}

fn provider_safe_name(command_name: &str) -> String {
    // Replace whitespace with underscores; lowercase; collapse runs of
    // underscores. Provider tool names are conventionally snake_case
    // ASCII — we keep ASCII-only as a safe default.
    let mut out = String::with_capacity(command_name.len());
    let mut prev_underscore = false;
    for c in command_name.chars() {
        if c.is_whitespace() || c == '-' {
            if !prev_underscore {
                out.push('_');
                prev_underscore = true;
            }
        } else {
            out.push(c.to_ascii_lowercase());
            prev_underscore = false;
        }
    }
    // Strip leading/trailing underscores.
    out.trim_matches('_').to_string()
}

/// Render the surface as the OpenAI function-calling payload.
///
/// Output shape:
/// ```json
/// {
///   "tools": [
///     {
///       "type": "function",
///       "function": {
///         "name": "target_list",
///         "description": "...",
///         "parameters": { "type": "object", "properties": {}, "required": [] }
///       }
///     }
///   ]
/// }
/// ```
///
/// `parameters` is always an empty object schema: SDDK commands are
/// invoked by exact argv (per `ExampleSpec::invocation::argv`) and do
/// not currently expose a JSON-Schema for free-form arguments. Adapters
/// are expected to populate `parameters` from per-command ArgSpec if
/// they need a richer contract (deferred to M7.4).
pub fn render_openai_tools(surface: &AgentCommandSurface) -> serde_json::Value {
    let tools: Vec<serde_json::Value> = surface_for_provider(surface)
        .iter()
        .map(|t| {
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": {
                        "type": "object",
                        "properties": {},
                        "required": []
                    }
                },
                "x-sddk": {
                    "command": t.command,
                    "stability": t.stability,
                    "side_effect_class": t.side_effect_class,
                    "required_authority": t.required_authority,
                    "machine_schema": t.machine_schema,
                }
            })
        })
        .collect();
    serde_json::json!({ "tools": tools })
}

/// Render the surface as the Anthropic Tools payload.
///
/// Output shape:
/// ```json
/// {
///   "tools": [
///     {
///       "name": "target_list",
///       "description": "...",
///       "input_schema": { "type": "object", "properties": {}, "required": [] }
///     }
///   ]
/// }
/// ```
pub fn render_anthropic_tools(surface: &AgentCommandSurface) -> serde_json::Value {
    let tools: Vec<serde_json::Value> = surface_for_provider(surface)
        .iter()
        .map(|t| {
            serde_json::json!({
                "name": t.name,
                "description": t.description,
                "input_schema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                },
                "x-sddk": {
                    "command": t.command,
                    "stability": t.stability,
                    "side_effect_class": t.side_effect_class,
                    "required_authority": t.required_authority,
                    "machine_schema": t.machine_schema,
                }
            })
        })
        .collect();
    serde_json::json!({ "tools": tools })
}

/// Render the surface as a provider-neutral payload.
///
/// Output shape:
/// ```json
/// {
///   "tools": [
///     {
///       "name": "target_list",
///       "description": "...",
///       "command": "target list",
///       "stability": "stable",
///       "side_effect_class": "pure",
///       "required_authority": "none",
///       "machine_schema": "TargetListV1",
///       "examples": [
///         { "id": "...", "description": "...", "argv": [...], "expected_status": 0, "output_contains": [...] }
///       ]
///     }
///   ]
/// }
/// ```
pub fn render_generic_tools(surface: &AgentCommandSurface) -> serde_json::Value {
    let tools: Vec<serde_json::Value> = surface_for_provider(surface)
        .iter()
        .map(|t| {
            serde_json::json!({
                "name": t.name,
                "description": t.description,
                "command": t.command,
                "stability": t.stability,
                "side_effect_class": t.side_effect_class,
                "required_authority": t.required_authority,
                "machine_schema": t.machine_schema,
                "examples": t.examples,
            })
        })
        .collect();
    serde_json::json!({ "tools": tools })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_spec::{
        AuthorityRequirement, CommandSpec, ExampleSpec, InvocationSpec, OutputFormatKind,
        SandboxMode, SandboxSpec, SideEffectClass, Stability, SurfaceFilter,
    };
    use crate::command_surface::{AgentCommandSurface, CommandSurfaceEntry, surface_with_filter};

    fn make_entry(name: &str, about: &str, examples: Vec<ExampleSpec>) -> CommandSurfaceEntry {
        let mut spec = CommandSpec::new(name, about)
            .with_stability(Stability::Stable)
            .with_side_effect(SideEffectClass::Pure)
            .with_authority(AuthorityRequirement::None)
            .with_outputs(OutputFormatKind::Text, "TestSchemaV1");
        for ex in examples {
            spec = spec.with_example(ex);
        }
        CommandSurfaceEntry::new(spec, true, None)
    }

    fn make_example(id: &str, argv: Vec<&str>, status: i32, needles: Vec<&str>) -> ExampleSpec {
        ExampleSpec {
            id: id.to_string(),
            description: format!("example {}", id),
            command: format!("sddk {}", argv.join(" ")),
            preconditions: Vec::new(),
            invocation: InvocationSpec {
                argv: argv.into_iter().map(String::from).collect(),
                format: OutputFormatKind::Text,
            },
            expected: crate::command_spec::ExpectedSpec {
                status,
                output_contains: needles.into_iter().map(String::from).collect(),
                output_kind: None,
            },
            sandbox: SandboxSpec {
                mode: SandboxMode::DryRun,
                env: Vec::new(),
            },
            scope: vec![Stability::Stable],
        }
    }

    #[test]
    fn provider_safe_name_lowercases_and_replaces_whitespace() {
        assert_eq!(provider_safe_name("target list"), "target_list");
        assert_eq!(provider_safe_name("target run"), "target_run");
        assert_eq!(provider_safe_name("Version"), "version");
        assert_eq!(provider_safe_name("multi  word"), "multi_word");
        assert_eq!(provider_safe_name("already_snake"), "already_snake");
    }

    #[test]
    fn surface_for_provider_includes_only_reachable_with_examples() {
        let with_examples = make_entry(
            "demo ok",
            "demo with examples",
            vec![make_example(
                "demo.ok.basic",
                vec!["demo", "ok"],
                0,
                vec!["hello"],
            )],
        );
        let no_examples = make_entry("demo bare", "demo without examples", vec![]);
        let unreachable_with_examples = {
            let mut e = make_entry(
                "demo hidden",
                "demo hidden",
                vec![make_example(
                    "demo.hidden",
                    vec!["demo", "hidden"],
                    0,
                    vec![],
                )],
            );
            e.reachable = false;
            e
        };
        let mut surface = AgentCommandSurface::default();
        surface.push(with_examples);
        surface.push(no_examples);
        surface.push(unreachable_with_examples);

        let tools = surface_for_provider(&surface);
        assert_eq!(tools.len(), 1, "only one entry has reachable+examples");
        assert_eq!(tools[0].command, "demo ok");
        assert_eq!(tools[0].name, "demo_ok");
        assert_eq!(tools[0].examples.len(), 1);
        assert_eq!(tools[0].examples[0].id, "demo.ok.basic");
    }

    #[test]
    fn tool_descriptor_carries_full_command_metadata() {
        let entry = make_entry(
            "demo meta",
            "Demo with full metadata",
            vec![make_example(
                "demo.meta.basic",
                vec!["demo", "meta"],
                0,
                vec!["x"],
            )],
        );
        let mut surface = AgentCommandSurface::default();
        surface.push(entry);
        let tools = surface_for_provider(&surface);
        let t = &tools[0];
        assert_eq!(t.name, "demo_meta");
        assert_eq!(t.description, "Demo with full metadata");
        assert_eq!(t.command, "demo meta");
        assert_eq!(t.stability, "stable");
        assert_eq!(t.side_effect_class, "pure");
        assert_eq!(t.required_authority, "none");
        assert_eq!(t.machine_schema, "TestSchemaV1");
    }

    #[test]
    fn render_openai_tools_matches_openai_schema_shape() {
        let entry = make_entry(
            "demo openai",
            "demo for openai",
            vec![make_example(
                "demo.openai.basic",
                vec!["demo", "openai"],
                0,
                vec!["ok"],
            )],
        );
        let mut surface = AgentCommandSurface::default();
        surface.push(entry);
        let payload = render_openai_tools(&surface);

        let tools = payload["tools"].as_array().expect("tools array");
        assert_eq!(tools.len(), 1);
        let tool = &tools[0];
        assert_eq!(tool["type"], "function");
        assert!(tool["function"].is_object());
        assert_eq!(tool["function"]["name"], "demo_openai");
        assert!(tool["function"]["parameters"]["properties"].is_object());
        assert!(tool["x-sddk"].is_object());
        assert_eq!(tool["x-sddk"]["command"], "demo openai");
    }

    #[test]
    fn render_anthropic_tools_uses_input_schema_field() {
        let entry = make_entry(
            "demo anthropic",
            "demo for anthropic",
            vec![make_example(
                "demo.anthropic.basic",
                vec!["demo", "anthropic"],
                0,
                vec!["ok"],
            )],
        );
        let mut surface = AgentCommandSurface::default();
        surface.push(entry);
        let payload = render_anthropic_tools(&surface);

        let tools = payload["tools"].as_array().expect("tools array");
        assert_eq!(tools.len(), 1);
        let tool = &tools[0];
        assert_eq!(tool["name"], "demo_anthropic");
        assert!(tool["input_schema"].is_object());
        assert_eq!(tool["input_schema"]["type"], "object");
        // No `function` wrapper (Anthropic doesn't use that envelope).
        assert!(tool.get("function").is_none() || tool["function"].is_null());
        // x-sddk carries the metadata.
        assert_eq!(tool["x-sddk"]["command"], "demo anthropic");
    }

    #[test]
    fn render_generic_tools_is_deterministic_byte_for_byte() {
        let entry = make_entry(
            "demo generic",
            "demo for generic",
            vec![make_example(
                "demo.generic.basic",
                vec!["demo", "generic"],
                0,
                vec!["ok"],
            )],
        );
        let mut surface = AgentCommandSurface::default();
        surface.push(entry);
        let a = serde_json::to_string(&render_generic_tools(&surface)).unwrap();
        let b = serde_json::to_string(&render_generic_tools(&surface)).unwrap();
        assert_eq!(a, b, "two renders of the same surface must be byte-equal");

        // Cross-verify the JSON parses and contains the tool entry.
        let parsed: serde_json::Value = serde_json::from_str(&a).unwrap();
        assert_eq!(parsed["tools"][0]["name"], "demo_generic");
        assert_eq!(parsed["tools"][0]["command"], "demo generic");
    }

    #[test]
    fn render_generic_tools_skips_entries_without_examples() {
        let with = make_entry(
            "demo with",
            "has examples",
            vec![make_example("demo.with.x", vec!["demo", "with"], 0, vec![])],
        );
        let without = make_entry("demo without", "no examples", vec![]);
        let mut surface = AgentCommandSurface::default();
        surface.push(with);
        surface.push(without);
        let payload = render_generic_tools(&surface);
        let tools = payload["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "demo_with");
    }

    #[test]
    fn integration_target_filter_drops_irrelevant_tools() {
        // Uses the live registry: surface_with_filter(target="nonexistent")
        // should produce no tools regardless of provider.
        let surface = surface_with_filter(SurfaceFilter {
            target: Some("__no_such_target__".to_string()),
            ..Default::default()
        });
        let openai = render_openai_tools(&surface);
        let anthropic = render_anthropic_tools(&surface);
        let generic = render_generic_tools(&surface);
        assert_eq!(openai["tools"].as_array().unwrap().len(), 0);
        assert_eq!(anthropic["tools"].as_array().unwrap().len(), 0);
        assert_eq!(generic["tools"].as_array().unwrap().len(), 0);
    }
}
