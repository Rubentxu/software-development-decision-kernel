//! Typed `CommandSpec` metadata for CLI introspection (M6.1).
//!
//! Agent consumers can enumerate commands via `sddk introspect commands --format json`
//! or get a single command's spec via `sddk --command-spec <name>`.
//!
//! The spec is hand-curated from the `Command` enum in `lib.rs` to ensure
//! machine-readable consistency without runtime clap introspection.
//!
//! `pub` for integration tests under `tests/`; `missing_docs` allowed at module scope.
//!
#![allow(missing_docs)]

use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::{CliEnvironment, CommandOutput, OutputFormat};

// ============================================================================
// M7.1 — closed-set enums for the FULL SPEC-015 field set
// ============================================================================

/// Stability classification for a command (SPEC-015).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stability {
    /// Stable API: backward-compatible, fixture-tested.
    #[default]
    Stable,
    /// Experimental: opt-in, may break; rendered when SDDK_AGENT_EXPERIMENTAL=1.
    Experimental,
    /// Deprecated: still callable but warned; hidden from agent surface by default.
    Deprecated,
}

/// Side-effect classification for a command (SPEC-015, ADR-014).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SideEffectClass {
    /// Pure: no observable effect, deterministic.
    #[default]
    Pure,
    /// Read: reads state, no mutation.
    Read,
    /// Governed: mutates state under typed authority/approval gates.
    Governed,
    /// Destructive: irreversible state change (e.g., git push --force).
    Destructive,
}

/// Authority requirement for a command (SPEC-015 + ADR-009).
///
/// Reuses the same vocabulary as `sddk_engine::target_task::AuthorityRequirement`
/// without coupling the CLI to the engine module; the values are
/// deterministically equivalent and the integration layer (M7.2) will join them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityRequirement {
    /// No authority required (pure/read for system actor).
    #[default]
    None,
    /// Read access only.
    Read,
    /// Write access (system actor admitted, user/agent require approval).
    Write,
    /// Explicit approval required.
    Approval,
}

/// Output contract for a command (SPEC-015: outputs.{human, machine}).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct OutputContract {
    /// Human output format kind.
    pub human: OutputFormatKind,
    /// Machine schema name (e.g., "ExecutionReportV1", "TargetListV1").
    pub machine: String,
}

/// Sandbox mode for executable examples (SPEC-015 example integrity).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxMode {
    /// Dry-run: command receives --dry-run or equivalent; no mutation.
    DryRun,
    /// Fixture: command runs against a captured fixture in tests/.
    Fixture,
    /// Live: command runs against real state (only for Pure/Read).
    Live,
}

/// Sandbox envelope for executable examples.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SandboxSpec {
    /// Mode selector.
    pub mode: SandboxMode,
    /// Optional env-var overrides.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub env: Vec<(String, String)>,
}

impl Default for SandboxSpec {
    fn default() -> Self {
        Self {
            mode: SandboxMode::DryRun,
            env: Vec::new(),
        }
    }
}

/// Invocation contract for an example.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct InvocationSpec {
    /// argv (without the leading `sddk` program name).
    pub argv: Vec<String>,
    /// Output format to parse.
    pub format: OutputFormatKind,
}

/// Expected outcome for an example.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ExpectedSpec {
    /// Exit status (0 = success).
    pub status: i32,
    /// Substrings that must appear in the output (case-sensitive).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output_contains: Vec<String>,
    /// Optional machine-output kind discriminator (e.g., "command_specs").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_kind: Option<String>,
}

/// Typed example (SPEC-015 + ADR-014 "examples become executable contract tests").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ExampleSpec {
    /// Stable id (e.g., "target.list.json", "target.run.ship.dry_run").
    pub id: String,
    /// Human description of what the example demonstrates.
    pub description: String,
    /// Exact command string (for human rendering).
    pub command: String,
    /// Preconditions (project must be adopted, actor must be `system`, etc.).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub preconditions: Vec<String>,
    /// How to invoke the example (argv + format).
    pub invocation: InvocationSpec,
    /// Expected outcome (status + output assertions).
    pub expected: ExpectedSpec,
    /// Sandbox mode and env.
    #[serde(default)]
    pub sandbox: SandboxSpec,
    /// Which stabilities the example is published under.
    pub scope: Vec<Stability>,
}

/// Agent profile tag (placeholder until M7.4 delivers full AgentProfile).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AgentProfileTag {
    /// Default agent profile (most permissive).
    #[default]
    Default,
    /// Read-only agent (cannot mutate).
    ReadOnly,
    /// Approver agent (can approve gated commands).
    Approver,
}

/// Surface filter for AgentCommandSurface aggregation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SurfaceFilter {
    /// Restrict by target (e.g., "verify").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// Restrict by minimum stability (e.g., Experimental excluded by default).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_stability: Option<Stability>,
    /// When true, include Deprecated commands.
    #[serde(default)]
    pub include_deprecated: bool,
    /// When true, include Experimental commands (default false).
    #[serde(default)]
    pub include_experimental: bool,
    /// Agent profile tag.
    #[serde(default)]
    pub profile: AgentProfileTag,
}

#[derive(Debug, Subcommand)]
pub enum IntrospectCommand {
    /// List all available commands and their typed specs.
    Commands {
        /// Restrict to a single command name. If absent, lists all.
        #[arg(long)]
        command: Option<String>,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[allow(dead_code)]
#[serde(rename_all = "snake_case")]
pub enum ArgKind {
    #[default]
    Flag,
    Option,
    Positional,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[allow(dead_code)]
#[serde(rename_all = "snake_case")]
pub enum ValueType {
    #[default]
    String,
    Int,
    Bool,
    Path,
    Enum(Vec<String>),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArgSpec {
    pub name: String,
    pub kind: ArgKind,
    pub value_type: ValueType,
    pub required: bool,
    pub default: Option<String>,
    pub help: String,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormatKind {
    #[default]
    Text,
    Json,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct CommandSpec {
    pub name: String,
    pub about: String,
    pub output_format: OutputFormatKind,
    pub args: Vec<ArgSpec>,
    pub has_subcommands: bool,
    pub cycle_phase: Option<String>,
    pub spec_ref: Option<String>,
    /// M7.1 — exact syntax (e.g., `sddk target run [options]`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub syntax: Option<String>,
    /// M7.1 — stability classification.
    #[serde(default)]
    pub stability: Stability,
    /// M7.1 — side-effect class.
    #[serde(default)]
    pub side_effect_class: SideEffectClass,
    /// M7.1 — required authority class.
    #[serde(default)]
    pub required_authority: AuthorityRequirement,
    /// M7.1 — output contract (human + machine schema name).
    #[serde(default)]
    pub outputs: OutputContract,
    /// M7.1 — preconditions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub preconditions: Vec<String>,
    /// M7.1 — typed examples (ADR-014 executable contract tests).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<ExampleSpec>,
    /// M7.1 — related commands (by name).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related: Vec<String>,
}

/// Return the canonical CommandSpec list for the entire CLI surface.
pub fn all_command_specs() -> Vec<CommandSpec> {
    vec![
        spec(
            "version",
            "Show the resolved framework version",
            OutputFormatKind::Text,
            vec![],
            false,
            None,
            None,
        ),
        spec(
            "project",
            "Resolve deterministic project and workspace identity",
            OutputFormatKind::Both,
            vec![],
            false,
            None,
            None,
        ),
        spec(
            "adopt",
            "Plan, apply, inspect, or repair project adoption",
            OutputFormatKind::Both,
            vec![],
            true,
            None,
            None,
        ),
        spec(
            "lint",
            "Validate repository contracts and generated workflow documentation",
            OutputFormatKind::Text,
            vec![],
            false,
            None,
            None,
        ),
        spec(
            "generate",
            "Generate deterministic repository documentation",
            OutputFormatKind::Text,
            vec![],
            false,
            None,
            None,
        ),
        spec(
            "cycle",
            "Plan and apply workflow cycles under the local authority",
            OutputFormatKind::Both,
            vec![],
            true,
            None,
            Some("arch-spec-005"),
        ),
        spec(
            "ledger",
            "Verify the causal ledger and list its events",
            OutputFormatKind::Both,
            vec![],
            true,
            None,
            Some("arch-spec-008"),
        ),
        spec(
            "capability",
            "Plan and execute typed capabilities under the default-deny policy",
            OutputFormatKind::Both,
            vec![],
            true,
            None,
            Some("arch-spec-008"),
        ),
        spec(
            "git",
            "Run typed local Git operations with verified postconditions",
            OutputFormatKind::Text,
            vec![],
            false,
            None,
            None,
        ),
        spec(
            "artifact",
            "Store and verify content-addressed artifacts",
            OutputFormatKind::Text,
            vec![],
            false,
            None,
            None,
        ),
        spec(
            "permission",
            "Check agent phase and capability permissions",
            OutputFormatKind::Text,
            vec![],
            false,
            None,
            None,
        ),
        spec(
            "validate",
            "Validate structured results against canonical schemas",
            OutputFormatKind::Text,
            vec![],
            false,
            None,
            None,
        ),
        spec(
            "release",
            "Plan, apply, and ship local Git releases",
            OutputFormatKind::Text,
            vec![],
            true,
            None,
            None,
        ),
        spec(
            "vault",
            "Index, validate, search, and export knowledge vaults",
            OutputFormatKind::Text,
            vec![],
            true,
            None,
            None,
        ),
        spec(
            "knowledge",
            "Resolve the canonical knowledge vault path and profile",
            OutputFormatKind::Text,
            vec![],
            false,
            None,
            None,
        ),
        spec(
            "dev",
            "Developer tooling: doctor, gates, and atomic install/verify",
            OutputFormatKind::Text,
            vec![],
            true,
            None,
            None,
        ),
        spec(
            "pack",
            "Validate declarative pack manifests",
            OutputFormatKind::Text,
            vec![],
            true,
            None,
            None,
        ),
        spec(
            "graph",
            "Query, inspect, and rebuild the reactive knowledge graph",
            OutputFormatKind::Text,
            vec![],
            false,
            None,
            None,
        ),
        spec(
            "metrics",
            "Record, aggregate, and tune cycle telemetry metrics",
            OutputFormatKind::Text,
            vec![],
            false,
            None,
            None,
        ),
        spec(
            "analytics",
            "Report, trend, and bottleneck analytics from cycle metrics",
            OutputFormatKind::Text,
            vec![],
            false,
            None,
            None,
        ),
        spec(
            "telemetry",
            "Central telemetry control plane",
            OutputFormatKind::Text,
            vec![],
            false,
            None,
            None,
        ),
        spec(
            "uat",
            "User Acceptance Testing: data-driven YAML plans rendered to dashboards",
            OutputFormatKind::Text,
            vec![],
            true,
            None,
            None,
        ),
        spec(
            "approval",
            "Manage human approval decisions for governed capabilities",
            OutputFormatKind::Text,
            vec![],
            false,
            None,
            Some("arch-spec-008"),
        ),
        spec(
            "rules",
            "Architecture-rule registry: evaluate rules against baseline JSON (SDDK2-003)",
            OutputFormatKind::Text,
            vec![],
            false,
            None,
            None,
        ),
        spec(
            "stale",
            "Staleness and impact queries over the reactive graph (SPEC-012)",
            OutputFormatKind::Text,
            vec![],
            false,
            None,
            None,
        ),
        spec(
            "fork",
            "Fork, replay, diff and promote controlled experiments (SPEC-009)",
            OutputFormatKind::Text,
            vec![],
            true,
            None,
            None,
        ),
        spec(
            "memory",
            "Decision memory: status, log, tree, show, diff, merge-base, reflog, audit (SPEC-004)",
            OutputFormatKind::Both,
            vec![],
            true,
            None,
            Some("arch-spec-004"),
        ),
        spec(
            "explore",
            "Render task-specific views over the reactive graph (SPEC-013)",
            OutputFormatKind::Text,
            vec![],
            false,
            None,
            Some("arch-spec-013"),
        ),
        spec(
            "completion",
            "Generate or install shell completion scripts",
            OutputFormatKind::Text,
            vec![],
            false,
            None,
            None,
        ),
        spec(
            "debt",
            "Debt management: report generation, INC listing, INC backfill, gate evaluation",
            OutputFormatKind::Text,
            vec![],
            false,
            None,
            None,
        ),
        spec(
            "status",
            "Show the current cycle snapshot and lease",
            OutputFormatKind::Both,
            vec![arg(
                "cycle",
                ArgKind::Option,
                ValueType::String,
                false,
                None,
                "Cycle identifier",
            )],
            false,
            Some("status"),
            None,
        ),
        spec(
            "plan",
            "Plan and apply workflow cycles under the local authority",
            OutputFormatKind::Both,
            vec![arg(
                "workitem",
                ArgKind::Option,
                ValueType::String,
                false,
                None,
                "DEPRECATED: use sddk plan workitem",
            )],
            true,
            Some("plan"),
            None,
        ),
        spec(
            "run",
            "Execute a capability",
            OutputFormatKind::Both,
            vec![],
            false,
            Some("run"),
            Some("arch-spec-008"),
        ),
        spec(
            "ship",
            "Plan a release (dry-run)",
            OutputFormatKind::Both,
            vec![],
            false,
            Some("ship"),
            None,
        ),
        spec(
            "recover",
            "Rebuild a cycle from ledger events (dry-run)",
            OutputFormatKind::Both,
            vec![arg(
                "dry_run",
                ArgKind::Flag,
                ValueType::Bool,
                false,
                Some("false".into()),
                "Dry-run",
            )],
            false,
            Some("recover"),
            None,
        ),
        spec(
            "change",
            "Create a planning work item in a cycle (M6.1)",
            OutputFormatKind::Both,
            vec![
                arg(
                    "cycle_id",
                    ArgKind::Option,
                    ValueType::String,
                    false,
                    None,
                    "Cycle identifier",
                ),
                arg(
                    "title",
                    ArgKind::Option,
                    ValueType::String,
                    true,
                    None,
                    "Work item title",
                ),
                arg(
                    "description",
                    ArgKind::Option,
                    ValueType::String,
                    true,
                    None,
                    "Work item description",
                ),
                arg(
                    "actor",
                    ArgKind::Option,
                    ValueType::String,
                    false,
                    None,
                    "Actor id",
                ),
            ],
            false,
            Some("plan"),
            Some("arch-spec-006"),
        ),
        spec(
            "verify",
            "Verify ledger continuity and capability policy snapshot (M6.1)",
            OutputFormatKind::Both,
            vec![],
            false,
            Some("verify"),
            Some("arch-spec-008"),
        ),
        spec(
            "audit",
            "Audit memory reflog plus ledger events (M6.1)",
            OutputFormatKind::Both,
            vec![
                arg(
                    "since",
                    ArgKind::Option,
                    ValueType::String,
                    false,
                    None,
                    "Optional RFC3339 filter (currently a no-op)",
                ),
                arg(
                    "format",
                    ArgKind::Option,
                    ValueType::Enum(vec!["text".into(), "json".into()]),
                    false,
                    Some("text".into()),
                    "Output format",
                ),
            ],
            false,
            Some("audit"),
            Some("arch-spec-004"),
        ),
        spec(
            "config",
            "Inspect configuration precedence chain (M6.1)",
            OutputFormatKind::Both,
            vec![arg(
                "format",
                ArgKind::Option,
                ValueType::Enum(vec!["text".into(), "json".into()]),
                false,
                Some("text".into()),
                "Output format",
            )],
            true,
            None,
            None,
        ),
        spec(
            "docs",
            "Generate rendered documentation from a chapter contract",
            OutputFormatKind::Text,
            vec![],
            true,
            None,
            None,
        ),
        spec(
            "inventory",
            "Query, inspect, and rebuild the reactive knowledge graph",
            OutputFormatKind::Text,
            vec![],
            true,
            None,
            None,
        ),
        spec(
            "introspect",
            "Enumerate commands and their typed specs (M6.1)",
            OutputFormatKind::Json,
            vec![
                arg(
                    "command",
                    ArgKind::Option,
                    ValueType::String,
                    false,
                    None,
                    "Single command name; if absent, lists all",
                ),
                arg(
                    "format",
                    ArgKind::Option,
                    ValueType::Enum(vec!["text".into(), "json".into()]),
                    false,
                    Some("json".into()),
                    "Output format",
                ),
            ],
            true,
            None,
            None,
        ),
        // M7.1 — explicit entries that publish examples and override defaults.
        // These are appended after the legacy entries to keep the diff minimal;
        // a future cycle will consolidate the registry to a single source.
        spec_target_list_with_examples(),
        spec_target_resolve_with_examples(),
        spec_target_run_with_examples(),
    ]
}

// ============================================================================
// M7.1 — command overrides with examples + non-trivial defaults
// ============================================================================

fn spec_target_list_with_examples() -> CommandSpec {
    spec(
        "target list",
        "List all built-in and registered targets (M6.2)",
        OutputFormatKind::Both,
        vec![arg(
            "format",
            ArgKind::Option,
            ValueType::Enum(vec!["text".into(), "json".into()]),
            false,
            Some("text".into()),
            "Output format",
        )],
        false,
        None,
        None,
    )
    .with_syntax("sddk target list [--format text|json]")
    .with_side_effect(SideEffectClass::Pure)
    .with_outputs(OutputFormatKind::Both, "TargetListV1")
    .with_precondition("project adopted")
    .with_related("target resolve")
    .with_related("target run")
    .with_related("introspect commands")
    .with_example(ExampleSpec {
        id: "target.list.text".into(),
        description: "List all targets as a human-readable table.".into(),
        command: "sddk target list --format text".into(),
        preconditions: vec!["project adopted".into()],
        invocation: InvocationSpec {
            argv: vec![
                "target".to_string(),
                "list".to_string(),
                "--format".to_string(),
                "text".to_string(),
            ],
            format: OutputFormatKind::Text,
        },
        expected: ExpectedSpec {
            status: 0,
            output_contains: vec![
                "status".into(),
                "run".into(),
                "ship".into(),
                "recover".into(),
                "change".into(),
                "verify".into(),
                "audit".into(),
            ],
            output_kind: None,
        },
        sandbox: SandboxSpec::default(),
        scope: vec![Stability::Stable],
    })
    .with_example(ExampleSpec {
        id: "target.list.json".into(),
        description: "List all targets as a JSON envelope.".into(),
        command: "sddk target list --format json".into(),
        preconditions: vec!["project adopted".into()],
        invocation: InvocationSpec {
            argv: vec![
                "target".to_string(),
                "list".to_string(),
                "--format".to_string(),
                "json".to_string(),
            ],
            format: OutputFormatKind::Json,
        },
        expected: ExpectedSpec {
            status: 0,
            output_contains: vec!["\"targets\"".into(), "run".into(), "ship".into()],
            output_kind: Some("target_list".into()),
        },
        sandbox: SandboxSpec::default(),
        scope: vec![Stability::Stable],
    })
}

fn spec_target_resolve_with_examples() -> CommandSpec {
    spec(
        "target resolve",
        "Resolve a target name to its TaskDag and validation report (M6.2)",
        OutputFormatKind::Both,
        vec![
            arg(
                "name",
                ArgKind::Option,
                ValueType::String,
                false,
                None,
                "Target name (status | run | ship | recover | change | verify | audit)",
            ),
            arg(
                "format",
                ArgKind::Option,
                ValueType::Enum(vec!["text".into(), "json".into()]),
                false,
                Some("text".into()),
                "Output format",
            ),
        ],
        false,
        None,
        None,
    )
    .with_syntax("sddk target resolve --name <name> [--format text|json]")
    .with_side_effect(SideEffectClass::Pure)
    .with_outputs(OutputFormatKind::Both, "TargetResolutionV1")
    .with_precondition("project adopted")
    .with_related("target list")
    .with_related("target run")
    .with_example(ExampleSpec {
        id: "target.resolve.run.text".into(),
        description: "Show the TaskDag for the `run` target in text form.".into(),
        command: "sddk target resolve --name run --format text".into(),
        preconditions: vec!["project adopted".into()],
        invocation: InvocationSpec {
            argv: vec![
                "target".to_string(),
                "resolve".to_string(),
                "--name".to_string(),
                "run".to_string(),
                "--format".to_string(),
                "text".to_string(),
            ],
            format: OutputFormatKind::Text,
        },
        expected: ExpectedSpec {
            status: 0,
            output_contains: vec![
                "context.resolve".into(),
                "ledger.snapshot".into(),
                "render".into(),
            ],
            output_kind: None,
        },
        sandbox: SandboxSpec::default(),
        scope: vec![Stability::Stable],
    })
    .with_example(ExampleSpec {
        id: "target.resolve.unknown.error".into(),
        description: "Resolve an unknown target — fails closed.".into(),
        command: "sddk target resolve --name ghost --format json".into(),
        preconditions: vec!["project adopted".into()],
        invocation: InvocationSpec {
            argv: vec![
                "target".to_string(),
                "resolve".to_string(),
                "--name".to_string(),
                "ghost".to_string(),
                "--format".to_string(),
                "json".to_string(),
            ],
            format: OutputFormatKind::Json,
        },
        expected: ExpectedSpec {
            status: 1,
            output_contains: vec!["unknown target".into(), "ghost".into()],
            output_kind: None,
        },
        sandbox: SandboxSpec {
            mode: SandboxMode::Fixture,
            env: Vec::new(),
        },
        scope: vec![Stability::Stable],
    })
}

fn spec_target_run_with_examples() -> CommandSpec {
    spec(
        "target run",
        "Execute a target DAG with per-task authority gating (M6.3)",
        OutputFormatKind::Both,
        vec![
            arg(
                "name",
                ArgKind::Option,
                ValueType::String,
                false,
                None,
                "Target name",
            ),
            arg(
                "actor",
                ArgKind::Option,
                ValueType::String,
                false,
                Some("system".into()),
                "Actor (system | user:* | agent:*)",
            ),
            arg(
                "dry_run",
                ArgKind::Flag,
                ValueType::Bool,
                false,
                Some("false".into()),
                "When true, skip task bodies (emit Skipped outcomes)",
            ),
            arg(
                "allow_high_band",
                ArgKind::Flag,
                ValueType::Bool,
                false,
                Some("false".into()),
                "Bypass RequireApproval gates",
            ),
            arg(
                "format",
                ArgKind::Option,
                ValueType::Enum(vec!["text".into(), "json".into()]),
                false,
                Some("json".into()),
                "Output format",
            ),
        ],
        false,
        None,
        None,
    )
    .with_syntax("sddk target run --name <name> [--actor X] [--dry-run] [--allow-high-band] [--format text|json]")
    .with_side_effect(SideEffectClass::Governed)
    .with_authority(AuthorityRequirement::Write)
    .with_outputs(OutputFormatKind::Both, "ExecutionReportV1")
    .with_precondition("project adopted")
    .with_related("target list")
    .with_related("target resolve")
    .with_example(ExampleSpec {
        id: "target.run.status.dry_run".into(),
        description: "Dry-run the status target as `system`. Tasks emit Skipped.".into(),
        command: "sddk target run --name status --actor system --dry-run --format json".into(),
        preconditions: vec!["project adopted".into(), "actor is `system`".into()],
        invocation: InvocationSpec {
            argv: vec![
                "target".to_string(),
                "run".to_string(),
                "--name".to_string(),
                "status".to_string(),
                "--actor".to_string(),
                "system".to_string(),
                "--dry-run".to_string(),
                "--format".to_string(),
                "json".to_string(),
            ],
            format: OutputFormatKind::Json,
        },
        expected: ExpectedSpec {
            status: 0,
            output_contains: vec!["\"status\":\"dry_run\"".into(), "context.resolve".into()],
            output_kind: Some("execution_report".into()),
        },
        sandbox: SandboxSpec::default(),
        scope: vec![Stability::Stable],
    })
    .with_example(ExampleSpec {
        id: "target.run.ship.user_denied".into(),
        description: "Ship target halts on Write gate for a user actor.".into(),
        command: "sddk target run --name ship --actor user:alice --format json".into(),
        preconditions: vec!["project adopted".into(), "actor is `user:alice`".into()],
        invocation: InvocationSpec {
            argv: vec![
                "target".to_string(),
                "run".to_string(),
                "--name".to_string(),
                "ship".to_string(),
                "--actor".to_string(),
                "user:alice".to_string(),
                "--format".to_string(),
                "json".to_string(),
            ],
            format: OutputFormatKind::Json,
        },
        expected: ExpectedSpec {
            status: 0, // CLI exits 0; status field is "denied"
            output_contains: vec!["\"status\":\"denied\"".into(), "awaiting approval".into()],
            output_kind: Some("execution_report".into()),
        },
        sandbox: SandboxSpec {
            mode: SandboxMode::Fixture,
            env: Vec::new(),
        },
        scope: vec![Stability::Stable],
    })
}

fn spec(
    name: &str,
    about: &str,
    output_format: OutputFormatKind,
    args: Vec<ArgSpec>,
    has_subcommands: bool,
    cycle_phase: Option<&str>,
    spec_ref: Option<&str>,
) -> CommandSpec {
    let machine = machine_schema_name(name);
    CommandSpec {
        name: name.to_string(),
        about: about.to_string(),
        output_format,
        args,
        has_subcommands,
        cycle_phase: cycle_phase.map(String::from),
        spec_ref: spec_ref.map(String::from),
        syntax: None,
        stability: Stability::Stable,
        side_effect_class: SideEffectClass::Pure,
        required_authority: AuthorityRequirement::None,
        outputs: OutputContract {
            human: output_format,
            machine,
        },
        preconditions: Vec::new(),
        examples: Vec::new(),
        related: Vec::new(),
    }
}

/// Mutable builder extension for `CommandSpec` to override defaults after
/// construction. Used by the small set of commands that publish examples or
/// override the default `side_effect_class` / `required_authority`.
impl CommandSpec {
    pub fn with_stability(mut self, stability: Stability) -> Self {
        self.stability = stability;
        self
    }

    pub fn with_side_effect(mut self, side_effect_class: SideEffectClass) -> Self {
        self.side_effect_class = side_effect_class;
        self
    }

    pub fn with_authority(mut self, required_authority: AuthorityRequirement) -> Self {
        self.required_authority = required_authority;
        self
    }

    pub fn with_syntax(mut self, syntax: &str) -> Self {
        self.syntax = Some(syntax.to_string());
        self
    }

    pub fn with_outputs(mut self, human: OutputFormatKind, machine: &str) -> Self {
        self.outputs = OutputContract {
            human,
            machine: machine.to_string(),
        };
        self
    }

    pub fn with_precondition(mut self, pre: &str) -> Self {
        self.preconditions.push(pre.to_string());
        self
    }

    pub fn with_related(mut self, related: &str) -> Self {
        self.related.push(related.to_string());
        self
    }

    pub fn with_example(mut self, example: ExampleSpec) -> Self {
        self.examples.push(example);
        self
    }
}

/// Infer the machine schema name for a given command (used as default
/// when a command does not declare one). Convention: `<NameInPascal>ReportV1`
/// or `<NameInPascal>ListV1` depending on whether the command surfaces a
/// single resource or a list.
fn machine_schema_name(name: &str) -> String {
    let parts: Vec<&str> = name.split_whitespace().collect();
    let pascal: String = parts
        .iter()
        .map(|p| {
            let mut chars = p.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect();
    let suffix = if pascal.ends_with("s") || name.contains("list") {
        "ListV1"
    } else {
        "ReportV1"
    };
    format!("{pascal}{suffix}")
}

fn arg(
    name: &str,
    kind: ArgKind,
    value_type: ValueType,
    required: bool,
    default: Option<String>,
    help: &str,
) -> ArgSpec {
    ArgSpec {
        name: name.to_string(),
        kind,
        value_type,
        required,
        default,
        help: help.to_string(),
    }
}

/// Render the full command spec list (or a single spec) as a `CommandOutput`.
pub fn render_command_spec(
    name: Option<&str>,
    format: OutputFormat,
    _environment: &CliEnvironment,
) -> CommandOutput {
    let specs = all_command_specs();
    let payload = match name {
        Some(n) => {
            let found: Vec<&CommandSpec> = specs.iter().filter(|s| s.name == n).collect();
            if found.is_empty() {
                serde_json::json!({
                    "kind": "command_spec",
                    "error": format!("unknown command: {n}"),
                    "available": specs.iter().map(|s| s.name.clone()).collect::<Vec<_>>(),
                })
            } else {
                serde_json::json!({
                    "kind": "command_spec",
                    "spec": found[0],
                })
            }
        }
        None => serde_json::json!({
            "kind": "command_specs",
            "specs": specs,
        }),
    };
    match format {
        OutputFormat::Json => {
            let stdout = serde_json::to_string_pretty(&payload).unwrap_or_default();
            CommandOutput {
                status: 0,
                stdout,
                stderr: String::new(),
            }
        }
        OutputFormat::Text => {
            let stdout = match name {
                Some(n) => {
                    let mut out = format!("command spec for {n}:\n");
                    if let Some(s) = specs.iter().find(|s| s.name == n) {
                        out.push_str(&format!("  about: {}\n", s.about));
                        out.push_str(&format!("  output_format: {:?}\n", s.output_format));
                        out.push_str(&format!("  has_subcommands: {}\n", s.has_subcommands));
                        if let Some(p) = &s.cycle_phase {
                            out.push_str(&format!("  cycle_phase: {p}\n"));
                        }
                    } else {
                        out.push_str(&format!("  (unknown command: {n})\n"));
                    }
                    out
                }
                None => {
                    let mut out = String::from("available commands:\n");
                    for s in &specs {
                        out.push_str(&format!("  {:<14} {}\n", s.name, s.about));
                    }
                    out
                }
            };
            CommandOutput {
                status: 0,
                stdout,
                stderr: String::new(),
            }
        }
    }
}

/// Run the `introspect` subcommand dispatcher.
pub(crate) fn run_introspect(
    command: IntrospectCommand,
    environment: &CliEnvironment,
) -> CommandOutput {
    match command {
        IntrospectCommand::Commands { command, format } => {
            render_command_spec(command.as_deref(), format, environment)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_command_specs_includes_change_verify_audit() {
        let names: Vec<String> = all_command_specs().into_iter().map(|s| s.name).collect();
        assert!(names.iter().any(|n| n == "change"));
        assert!(names.iter().any(|n| n == "verify"));
        assert!(names.iter().any(|n| n == "audit"));
        assert!(names.iter().any(|n| n == "config"));
        assert!(names.iter().any(|n| n == "introspect"));
    }

    #[test]
    fn render_all_in_json_is_valid() {
        let env = CliEnvironment::default();
        let out = render_command_spec(None, OutputFormat::Json, &env);
        let parsed: serde_json::Value = serde_json::from_str(&out.stdout).expect("valid JSON");
        assert_eq!(parsed["kind"], "command_specs");
        let arr = parsed["specs"].as_array().unwrap();
        assert!(arr.len() >= 40);
    }

    #[test]
    fn render_single_known_command() {
        let env = CliEnvironment::default();
        let out = render_command_spec(Some("memory"), OutputFormat::Json, &env);
        let parsed: serde_json::Value = serde_json::from_str(&out.stdout).expect("valid JSON");
        assert_eq!(parsed["kind"], "command_spec");
        assert_eq!(parsed["spec"]["name"], "memory");
        assert_eq!(parsed["spec"]["spec_ref"], "arch-spec-004");
    }

    #[test]
    fn render_single_unknown_command_reports_available() {
        let env = CliEnvironment::default();
        let out = render_command_spec(Some("does_not_exist"), OutputFormat::Json, &env);
        let parsed: serde_json::Value = serde_json::from_str(&out.stdout).expect("valid JSON");
        assert!(parsed["error"].as_str().unwrap().contains("does_not_exist"));
        let available = parsed["available"].as_array().unwrap();
        assert!(available.len() >= 40);
    }

    #[test]
    fn change_spec_marks_required_args() {
        let s = all_command_specs()
            .into_iter()
            .find(|s| s.name == "change")
            .expect("change spec");
        let required: Vec<&str> = s
            .args
            .iter()
            .filter(|a| a.required)
            .map(|a| a.name.as_str())
            .collect();
        assert!(required.contains(&"title"));
        assert!(required.contains(&"description"));
    }

    // M7.1 — closed-set enums for Stability, SideEffectClass, AuthorityRequirement,
    // OutputContractKind, ExampleSandbox, AgentProfileTag. Each must serialize to
    // snake_case and roundtrip through JSON without losing discriminants.

    #[test]
    fn stability_serializes_snake_case_and_roundtrips() {
        for s in [
            Stability::Stable,
            Stability::Experimental,
            Stability::Deprecated,
        ] {
            let j = serde_json::to_string(&s).unwrap();
            let back: Stability = serde_json::from_str(&j).unwrap();
            assert_eq!(back, s);
        }
        assert_eq!(
            serde_json::to_string(&Stability::Stable).unwrap(),
            "\"stable\""
        );
        assert_eq!(
            serde_json::to_string(&Stability::Experimental).unwrap(),
            "\"experimental\""
        );
        assert_eq!(
            serde_json::to_string(&Stability::Deprecated).unwrap(),
            "\"deprecated\""
        );
    }

    #[test]
    fn side_effect_class_serializes_snake_case_and_roundtrips() {
        for c in [
            SideEffectClass::Pure,
            SideEffectClass::Read,
            SideEffectClass::Governed,
            SideEffectClass::Destructive,
        ] {
            let j = serde_json::to_string(&c).unwrap();
            let back: SideEffectClass = serde_json::from_str(&j).unwrap();
            assert_eq!(back, c);
        }
    }

    #[test]
    fn output_contract_serializes_with_human_and_machine() {
        let oc = OutputContract {
            human: OutputFormatKind::Both,
            machine: "ExecutionReportV1".into(),
        };
        let j = serde_json::to_string(&oc).unwrap();
        assert!(j.contains("\"human\":\"both\""));
        assert!(j.contains("\"machine\":\"ExecutionReportV1\""));
        let back: OutputContract = serde_json::from_str(&j).unwrap();
        assert_eq!(back, oc);
    }

    #[test]
    fn example_spec_default_is_dry_run_sandbox() {
        let ex = ExampleSpec {
            id: "demo".into(),
            description: "demo command".into(),
            command: "sddk version".into(),
            preconditions: vec![],
            invocation: InvocationSpec {
                argv: vec!["sddk".to_string(), "version".to_string()],
                format: OutputFormatKind::Text,
            },
            expected: ExpectedSpec {
                status: 0,
                output_contains: vec!["sddk".into()],
                output_kind: None,
            },
            sandbox: SandboxSpec::default(),
            scope: vec![Stability::Stable],
        };
        assert_eq!(ex.sandbox.mode, SandboxMode::DryRun);
    }

    #[test]
    fn command_spec_carries_new_fields() {
        let s = all_command_specs()
            .into_iter()
            .find(|s| s.name == "version")
            .expect("version spec");
        // Stability, side-effect class, authority must be populated.
        assert_eq!(s.stability, Stability::Stable);
        assert_eq!(s.side_effect_class, SideEffectClass::Pure);
        assert_eq!(s.required_authority, AuthorityRequirement::None);
        // Outputs must include both human + machine fields.
        assert_eq!(s.outputs.human, OutputFormatKind::Text);
        assert!(!s.outputs.machine.is_empty());
        // Defaults for related/preconditions are empty.
        assert!(s.related.is_empty());
    }

    #[test]
    fn target_run_publishes_examples() {
        let s = all_command_specs()
            .into_iter()
            .find(|s| s.name == "target run")
            .expect("target run spec");
        assert!(
            s.examples.len() >= 2,
            "target run needs ≥2 examples, has {}",
            s.examples.len()
        );
        // First example should be a happy-path dry-run
        assert!(
            s.examples[0].sandbox.mode == SandboxMode::DryRun
                || s.examples[0].sandbox.mode == SandboxMode::Fixture
        );
    }
}
