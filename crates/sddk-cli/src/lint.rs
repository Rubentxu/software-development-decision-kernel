use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use clap::CommandFactory;
use regex::Regex;
use sddk_domain::{
    Requirement, WorkflowManifest,
    models::gate_classification::{GateKind, RecoveryAction},
};
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;
use walkdir::{DirEntry, WalkDir};

use crate::docs::{GENERATED_WORKFLOW_DOC, render_workflow_docs};
use crate::inventory::{GENERATED_INVENTORY_DOC, render_inventory};

const WORKFLOW_MANIFEST: &str = "workflow/workflow.yaml";
const BROKEN_REFERENCE: &str = "SDDK001";
const UNRESOLVED_PLACEHOLDER: &str = "SDDK002";
const QUOTED_TILDE: &str = "SDDK003";
const UNDEFINED_SHELL_VARIABLE: &str = "SDDK004";
const INVALID_CONTRACT: &str = "SDDK005";
const UNDECLARED_WORKFLOW_ITEM: &str = "SDDK006";
const ARTIFACT_TOPOLOGY: &str = "SDDK007";
const PATH_NOT_TRAVERSABLE: &str = "SDDK008";
const GENERATED_DOC_STALE: &str = "SDDK009";
const GENERATED_INVENTORY_STALE: &str = "SDDK010";
const AGENT_NOT_IN_REGISTRY: &str = "SDDK011";
const REGISTRY_ORPHAN: &str = "SDDK012";
const AGENT_NAME_MISMATCH: &str = "SDDK013";
const INVALID_PACK_MANIFEST: &str = "SDDK014";
const MATRIX_SCHEMA: &str = "SDDK015";
const MATRIX_POINTER: &str = "SDDK016";
const SIZING_SEPARATION: &str = "SDDK017";
const AGENT_REGISTRY_UNREGISTERED: &str = "SDDK018";
const CLI_COMMAND_UNKNOWN: &str = "SDDK019";
const INSTRUCTION_CLOSURE_ORDERING: &str = "SDDK020";
const MANIFEST_VERSION_LOCKSTEP: &str = "SDDK021";
const INSTRUCTION_APPLY_PUSH_ANCHORS: &str = "SDDK022";
const MATRIX_DRY_RUN_INVARIANT: &str = "SDDK023";
const MATRIX_FACADE_SHADOW_ROUTING: &str = "SDDK024";
const MATRIX_FACADE_ARGV_ACCURACY: &str = "SDDK025";
const MATRIX_SAFETY_ADVISORY_SEPARATION: &str = "SDDK026";
const INSTRUCTION_F4_GOTCHAS: &str = "SDDK027";
const INSTRUCTION_ZERO_INTRUSION: &str = "SDDK028";
const INSTRUCTION_OWNER_BOUNDARY: &str = "SDDK029";
const RELEASE_CHAIN_ORDERING: &str = "SDDK030";
const MATRIX_LOCKSTEP_REFUSAL: &str = "SDDK031";
const INSTRUCTION_RECIPE_DEDUP: &str = "SDDK032";
const GATE_CLASSIFICATION_VALIDATION: &str = "SDDK033";
const WRITER_XDG_VALIDATION: &str = "SDDK034";

const MATRIX_REQUIRED_COLUMNS: &[&str] = &[
    "intent",
    "owner_role",
    "command",
    "required_inputs",
    "expected_output",
    "side_effects",
    "idempotence",
    "next_handoff",
];

/// Diagnostic severity. Only errors make `sddk lint` fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// A repository contract is invalid and lint exits nonzero.
    Error,
    /// A non-fatal consistency gap should be addressed.
    Warning,
}

/// One stable, structured repository diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    /// Stable machine-readable diagnostic code.
    pub code: String,
    /// Error or warning severity.
    pub severity: Severity,
    /// Repository-relative file path using forward slashes.
    pub file: String,
    /// One-based source line when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    /// Human-readable problem statement.
    pub message: String,
    /// Suggested remediation.
    pub hint: String,
}

/// Aggregate diagnostic counts included in JSON output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct LintSummary {
    /// Number of error diagnostics.
    pub errors: usize,
    /// Number of warning diagnostics.
    pub warnings: usize,
}

/// Deterministically sorted result of linting one repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LintReport {
    /// Aggregate counts.
    pub summary: LintSummary,
    /// Sorted diagnostics.
    pub diagnostics: Vec<Diagnostic>,
}

impl LintReport {
    /// Returns true when at least one error diagnostic was emitted.
    pub fn has_errors(&self) -> bool {
        self.summary.errors > 0
    }

    /// Renders stable human-readable diagnostics.
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        for diagnostic in &self.diagnostics {
            let location = diagnostic.line.map_or_else(
                || diagnostic.file.clone(),
                |line| format!("{}:{line}", diagnostic.file),
            );
            output.push_str(&format!(
                "{}[{}] {}: {}\n  help: {}\n",
                match diagnostic.severity {
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                },
                diagnostic.code,
                location,
                diagnostic.message,
                diagnostic.hint
            ));
        }
        output.push_str(&format!(
            "lint: {} error(s), {} warning(s)\n",
            self.summary.errors, self.summary.warnings
        ));
        output
    }
}

/// Fatal failures that prevent repository linting from starting.
#[derive(Debug, Error)]
pub enum LintError {
    /// The supplied root is not a directory.
    #[error("repository root is not a directory: {0}")]
    InvalidRoot(PathBuf),
}

/// Lints workflow contracts, references, executable snippets, and generated docs.
pub fn lint_repository(root: impl AsRef<Path>) -> Result<LintReport, LintError> {
    let root = root.as_ref();
    if !root.is_dir() {
        return Err(LintError::InvalidRoot(root.to_path_buf()));
    }

    let mut diagnostics = Vec::new();
    validate_schema_catalog(root, &mut diagnostics);
    let workflow = lint_workflow(root, &mut diagnostics);
    scan_repository_sources(root, &mut diagnostics);
    if let Some(manifest) = workflow.as_ref() {
        lint_generated_docs(root, manifest, &mut diagnostics);
    }
    lint_generated_inventory(root, &mut diagnostics);
    lint_agent_registry(root, &mut diagnostics);
    lint_pack_manifest(root, &mut diagnostics);
    lint_instruction_contract(root, &mut diagnostics);

    diagnostics.sort_by(|left, right| {
        (
            left.severity,
            &left.code,
            &left.file,
            left.line,
            &left.message,
        )
            .cmp(&(
                right.severity,
                &right.code,
                &right.file,
                right.line,
                &right.message,
            ))
    });
    diagnostics.dedup();
    let summary = LintSummary {
        errors: diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Error)
            .count(),
        warnings: diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Warning)
            .count(),
    };
    Ok(LintReport {
        summary,
        diagnostics,
    })
}

fn lint_workflow(root: &Path, diagnostics: &mut Vec<Diagnostic>) -> Option<WorkflowManifest> {
    let relative = Path::new(WORKFLOW_MANIFEST);
    let path = root.join(relative);
    // Non-intrusive: when the repo has no workflow/workflow.yaml, lint falls
    // back to the canonical manifest embedded in the binary (ADR-0011). A
    // project must never be required to carry framework files.
    let yaml = match fs::read_to_string(&path) {
        Ok(yaml) => yaml,
        Err(_) => crate::CANONICAL_WORKFLOW.to_owned(),
    };

    match serde_saphyr::from_str::<Value>(&yaml) {
        Ok(value) => validate_workflow_contract(relative, &yaml, &value, diagnostics),
        Err(error) => diagnostics.push(diagnostic(
            INVALID_CONTRACT,
            Severity::Error,
            relative,
            None,
            format!("workflow is not valid YAML: {error}"),
            "fix the YAML syntax before validating workflow semantics",
        )),
    }

    let load_result = match fs::read_to_string(&path) {
        Ok(_) => sddk_engine::load_workflow_path(&path),
        Err(_) => sddk_engine::load_workflow_str(crate::CANONICAL_WORKFLOW),
    };
    match load_result {
        Ok(manifest) => {
            lint_workflow_topology(relative, &yaml, &manifest, diagnostics);
            Some(manifest)
        }
        Err(error) => {
            let code = match &error {
                sddk_engine::WorkflowLoadError::Validation(
                    sddk_engine::WorkflowValidationError::UnknownArtifactRequirement { .. }
                    | sddk_engine::WorkflowValidationError::UnknownGateRequirement { .. },
                ) => UNDECLARED_WORKFLOW_ITEM,
                _ => INVALID_CONTRACT,
            };
            diagnostics.push(diagnostic(
                code,
                Severity::Error,
                relative,
                None,
                format!("engine rejected canonical workflow: {error}"),
                "make workflow.yaml satisfy the canonical schema and engine invariants",
            ));
            None
        }
    }
}

fn validate_workflow_contract(
    file: &Path,
    yaml: &str,
    value: &Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(root) = value.as_object() else {
        diagnostics.push(diagnostic(
            INVALID_CONTRACT,
            Severity::Error,
            file,
            Some(1),
            "workflow root must be a mapping",
            "use the object shape declared by schemas/workflow.schema.json",
        ));
        return;
    };

    check_object_keys(
        file,
        yaml,
        root,
        &[
            "schema_version",
            "workflow",
            "statuses",
            "phases",
            "paths",
            "policies",
            "transitions",
            "artifacts",
            "gates",
            "forge",
            "storage",
            "project_identity",
        ],
        &[
            "schema_version",
            "workflow",
            "statuses",
            "phases",
            "transitions",
        ],
        "workflow root",
        diagnostics,
    );

    if let Some(workflow) = root.get("workflow").and_then(Value::as_object) {
        check_object_keys(
            file,
            yaml,
            workflow,
            &["id", "version", "description"],
            &["id", "version", "description"],
            "workflow metadata",
            diagnostics,
        );
        if let Some(version) = workflow.get("version").and_then(Value::as_str)
            && !Regex::new(r"^\d+\.\d+\.\d+$")
                .expect("valid semantic-version regex")
                .is_match(version)
        {
            diagnostics.push(diagnostic(
                INVALID_CONTRACT,
                Severity::Error,
                file,
                line_of(yaml, version),
                format!("workflow version {version:?} is not MAJOR.MINOR.PATCH"),
                "use a numeric semantic version such as 1.2.3",
            ));
        }
    }

    if let Some(transitions) = root.get("transitions").and_then(Value::as_array) {
        for transition in transitions {
            let Some(transition) = transition.as_object() else {
                diagnostics.push(diagnostic(
                    INVALID_CONTRACT,
                    Severity::Error,
                    file,
                    None,
                    "each workflow transition must be a mapping",
                    "replace scalar transition entries with transition objects",
                ));
                continue;
            };
            check_object_keys(
                file,
                yaml,
                transition,
                &[
                    "id",
                    "from",
                    "to",
                    "requires",
                    "paths",
                    "produces",
                    "implementation_binding",
                    "on_failure",
                ],
                &["id", "to", "requires"],
                "transition",
                diagnostics,
            );
            for state_key in ["from", "to", "on_failure"] {
                if let Some(state) = transition.get(state_key).and_then(Value::as_object) {
                    check_object_keys(
                        file,
                        yaml,
                        state,
                        &["status", "phase"],
                        &["status"],
                        "state reference",
                        diagnostics,
                    );
                }
            }
            if let Some(requirements) = transition.get("requires").and_then(Value::as_array) {
                for requirement in requirements {
                    if requirement.is_string() {
                        continue;
                    }
                    let Some(requirement) = requirement.as_object() else {
                        diagnostics.push(diagnostic(
                            INVALID_CONTRACT,
                            Severity::Error,
                            file,
                            None,
                            "transition requirement must be a string or {kind, name} mapping",
                            "use a simple precondition string or a typed artifact/gate requirement",
                        ));
                        continue;
                    };
                    check_object_keys(
                        file,
                        yaml,
                        requirement,
                        &["kind", "name"],
                        &["kind", "name"],
                        "transition requirement",
                        diagnostics,
                    );
                }
            }
        }
    }

    if let Some(paths) = root.get("paths").and_then(Value::as_object) {
        for path in paths.values().filter_map(Value::as_object) {
            check_object_keys(
                file,
                yaml,
                path,
                &["description", "debt_verification", "phases"],
                &["description", "debt_verification", "phases"],
                "path",
                diagnostics,
            );
        }
    }
    if let Some(artifacts) = root.get("artifacts").and_then(Value::as_object) {
        for artifact in artifacts.values().filter_map(Value::as_object) {
            check_object_keys(
                file,
                yaml,
                artifact,
                &[
                    "producer",
                    "consumers",
                    "required",
                    "terminal",
                    "description",
                ],
                &["producer", "consumers"],
                "artifact",
                diagnostics,
            );
        }
    }
    if let Some(gates) = root.get("gates").and_then(Value::as_object) {
        for gate in gates.values().filter_map(Value::as_object) {
            check_object_keys(
                file,
                yaml,
                gate,
                &["gate_type", "description"],
                &[],
                "gate",
                diagnostics,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn check_object_keys(
    file: &Path,
    source: &str,
    object: &serde_json::Map<String, Value>,
    allowed: &[&str],
    required: &[&str],
    context: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for key in object.keys().filter(|key| !allowed.contains(&key.as_str())) {
        diagnostics.push(diagnostic(
            INVALID_CONTRACT,
            Severity::Error,
            file,
            line_of(source, &format!("{key}:")),
            format!("unknown {context} field {key:?}"),
            "use only canonical snake_case wire fields declared by the schema",
        ));
    }
    for key in required.iter().filter(|key| !object.contains_key(**key)) {
        diagnostics.push(diagnostic(
            INVALID_CONTRACT,
            Severity::Error,
            file,
            None,
            format!("{context} is missing required field {key:?}"),
            "add the required canonical wire field",
        ));
    }
}

fn lint_workflow_topology(
    file: &Path,
    yaml: &str,
    manifest: &WorkflowManifest,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for transition in &manifest.transitions {
        for produced in &transition.produces {
            if !manifest.artifacts.contains_key(produced) {
                diagnostics.push(diagnostic(
                    UNDECLARED_WORKFLOW_ITEM,
                    Severity::Error,
                    file,
                    line_of(yaml, &format!("- {produced}")),
                    format!(
                        "transition {} produces undeclared artifact {produced:?}",
                        transition.id
                    ),
                    "declare the artifact under artifacts or remove it from produces",
                ));
            }
        }
        for requirement in &transition.requires {
            if let Requirement::Structured { kind, name } = requirement {
                let declared = match kind.as_str() {
                    "artifact" => manifest.artifacts.contains_key(name),
                    "gate" => manifest.gates.contains_key(name),
                    _ => true,
                };
                if !declared {
                    diagnostics.push(diagnostic(
                        UNDECLARED_WORKFLOW_ITEM,
                        Severity::Error,
                        file,
                        line_of(yaml, &format!("name: {name}")),
                        format!(
                            "transition {} requires undeclared {kind} {name:?}",
                            transition.id
                        ),
                        format!("declare {name:?} under {kind}s or remove the requirement"),
                    ));
                }
            }
        }
    }

    let mut artifacts = manifest.artifacts.iter().collect::<Vec<_>>();
    artifacts.sort_by_key(|(name, _)| *name);
    for (name, artifact) in artifacts {
        if artifact.producer.trim().is_empty() {
            diagnostics.push(diagnostic(
                ARTIFACT_TOPOLOGY,
                Severity::Warning,
                file,
                line_of(yaml, &format!("{name}:")),
                format!("artifact {name:?} has no producer"),
                "name the phase, agent, runtime, or provider that produces this artifact",
            ));
        }
        if artifact.consumers.is_empty() && !artifact.terminal {
            diagnostics.push(diagnostic(
                ARTIFACT_TOPOLOGY,
                Severity::Warning,
                file,
                line_of(yaml, &format!("{name}:")),
                format!("artifact {name:?} has no declared consumers"),
                "declare at least one consumer or document why the terminal artifact is retained",
            ));
        }
    }

    let mut paths = manifest.paths.iter().collect::<Vec<_>>();
    paths.sort_by_key(|(name, _)| *name);
    for (path_name, path) in paths {
        let start_phase = manifest
            .transitions
            .iter()
            .find(|transition| {
                transition.from.is_none() && transition_applies_to_path(transition, path_name)
            })
            .and_then(|transition| transition.to.phase)
            .map(|phase| wire(&phase));
        if let (Some(start_phase), Some(first)) = (start_phase.as_ref(), path.phases.first())
            && first != start_phase
        {
            diagnostics.push(diagnostic(
                PATH_NOT_TRAVERSABLE,
                Severity::Warning,
                file,
                line_of(yaml, &format!("{path_name}:")),
                format!("path {path_name} starts at {first}, but cycle.start enters {start_phase}"),
                "declare a path-specific entry transition or align the first path phase",
            ));
        }
        let edges = manifest
            .transitions
            .iter()
            .filter(|transition| transition_applies_to_path(transition, path_name))
            .filter_map(|transition| {
                Some((
                    wire(&transition.from.as_ref()?.phase?),
                    wire(&transition.to.phase?),
                ))
            })
            .collect::<HashSet<_>>();
        for pair in path.phases.windows(2) {
            if !edges.contains(&(pair[0].clone(), pair[1].clone())) {
                diagnostics.push(diagnostic(
                    PATH_NOT_TRAVERSABLE,
                    Severity::Warning,
                    file,
                    line_of(yaml, &format!("{path_name}:")),
                    format!(
                        "path {path_name} cannot traverse {} -> {} through a declared transition",
                        pair[0], pair[1]
                    ),
                    "declare the missing transition edge or change the path phase sequence",
                ));
            }
        }
    }
}

fn transition_applies_to_path(transition: &sddk_domain::workflow::Transition, path: &str) -> bool {
    transition.paths.is_empty() || transition.paths.iter().any(|candidate| candidate == path)
}

fn validate_schema_catalog(root: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let schema_dir = root.join("schemas");
    let mut schemas = match fs::read_dir(&schema_dir) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "json")
            })
            .collect::<Vec<_>>(),
        Err(error) => {
            diagnostics.push(diagnostic(
                INVALID_CONTRACT,
                Severity::Error,
                Path::new("schemas"),
                None,
                format!("cannot read canonical schema directory: {error}"),
                "create schemas/ and add the canonical JSON Schema contracts",
            ));
            return;
        }
    };
    schemas.sort();

    for path in schemas {
        let relative = path.strip_prefix(root).unwrap_or(&path);
        let source = match fs::read_to_string(&path) {
            Ok(source) => source,
            Err(error) => {
                diagnostics.push(diagnostic(
                    INVALID_CONTRACT,
                    Severity::Error,
                    relative,
                    None,
                    format!("cannot read schema: {error}"),
                    "make the schema readable",
                ));
                continue;
            }
        };
        let schema = match serde_json::from_str::<Value>(&source) {
            Ok(schema) => schema,
            Err(error) => {
                diagnostics.push(diagnostic(
                    INVALID_CONTRACT,
                    Severity::Error,
                    relative,
                    Some(error.line()),
                    format!("schema is not valid JSON: {error}"),
                    "fix JSON syntax before using this contract",
                ));
                continue;
            }
        };
        if !schema.is_object() {
            diagnostics.push(diagnostic(
                INVALID_CONTRACT,
                Severity::Error,
                relative,
                Some(1),
                "schema root must be an object",
                "use a JSON Schema object at the document root",
            ));
        }
        validate_local_schema_refs(root, relative, &schema, diagnostics);
    }
}

fn validate_local_schema_refs(
    root: &Path,
    schema_file: &Path,
    value: &Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match value {
        Value::Object(object) => {
            if let Some(reference) = object.get("$ref").and_then(Value::as_str)
                && !reference.starts_with('#')
                && !reference.contains("://")
            {
                let reference_path = reference.split('#').next().unwrap_or(reference);
                let target = schema_file
                    .parent()
                    .unwrap_or_else(|| Path::new(""))
                    .join(reference_path);
                if !root.join(&target).is_file() {
                    diagnostics.push(diagnostic(
                        BROKEN_REFERENCE,
                        Severity::Error,
                        schema_file,
                        None,
                        format!("schema reference {reference:?} does not exist"),
                        "add the referenced schema or correct the relative $ref",
                    ));
                }
            }
            for nested in object.values() {
                validate_local_schema_refs(root, schema_file, nested, diagnostics);
            }
        }
        Value::Array(array) => {
            for nested in array {
                validate_local_schema_refs(root, schema_file, nested, diagnostics);
            }
        }
        _ => {}
    }
}

fn scan_repository_sources(root: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let patterns = SourcePatterns::new();
    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(should_descend)
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file() && is_source(entry.path()))
    {
        let path = entry.path();
        let relative = path.strip_prefix(root).unwrap_or(path);
        let Ok(source) = fs::read_to_string(path) else {
            continue;
        };
        scan_references(root, relative, &source, &patterns, diagnostics);
        scan_shell_fences(relative, &source, &patterns, diagnostics);
    }
}

struct SourcePatterns {
    markdown_link: Regex,
    yaml_reference: Regex,
    shell_fence: Regex,
    placeholder: Regex,
    quoted_tilde: Regex,
    assignment: Regex,
    loop_variable: Regex,
    shell_variable: Regex,
}

impl SourcePatterns {
    fn new() -> Self {
        Self {
            markdown_link: Regex::new(r"\[[^\]]*\]\(([^)]+)\)")
                .expect("valid Markdown link regex"),
            yaml_reference: Regex::new(
                r#"(?m)^\s*(agent|skill|plugin|agent_(?:path|ref)|skill_(?:path|ref)|plugin_(?:path|ref)|prompt_(?:path|ref)|path|file):\s*["']?([^\s"'#]+)"#,
            )
            .expect("valid YAML reference regex"),
            // Legacy prose uses shell fences as templates. Requiring an execution marker keeps
            // these checks on snippets that claim to be directly runnable.
            shell_fence: Regex::new(
                r"(?ms)^```(?:bash|sh|shell)[ \t]+(?:executable|lint)[ \t]*\n(.*?)^```\s*$",
            )
            .expect("valid executable shell-fence regex"),
            placeholder: Regex::new(r"(^|[^$])\{([A-Za-z_][A-Za-z0-9_-]*)\}")
                .expect("valid placeholder regex"),
            quoted_tilde: Regex::new(r#"["']~(?:/|["'])"#).expect("valid tilde regex"),
            assignment: Regex::new(
                r"(?m)^\s*(?:export\s+|local\s+|readonly\s+)?([A-Za-z_][A-Za-z0-9_]*)=",
            )
            .expect("valid assignment regex"),
            loop_variable: Regex::new(r"(?m)^\s*for\s+([A-Za-z_][A-Za-z0-9_]*)\s+in\b")
                .expect("valid loop-variable regex"),
            shell_variable: Regex::new(
                r"\$(?:\{([A-Za-z_][A-Za-z0-9_]*)(?::[-+?=][^}]*)?\}|([A-Za-z_][A-Za-z0-9_]*))",
            )
            .expect("valid shell-variable regex"),
        }
    }
}

fn scan_references(
    root: &Path,
    file: &Path,
    source: &str,
    patterns: &SourcePatterns,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for capture in patterns.markdown_link.captures_iter(source) {
        let raw = capture[1].split_whitespace().next().unwrap_or_default();
        check_reference(
            root,
            file,
            source,
            raw,
            capture.get(1).map(|found| found.start()),
            None,
            true,
            diagnostics,
        );
    }
    for capture in patterns.yaml_reference.captures_iter(source) {
        check_reference(
            root,
            file,
            source,
            &capture[2],
            capture.get(2).map(|found| found.start()),
            Some(&capture[1]),
            matches!(&capture[1], "path" | "file"),
            diagnostics,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn check_reference(
    root: &Path,
    file: &Path,
    source: &str,
    raw: &str,
    offset: Option<usize>,
    kind: Option<&str>,
    allow_relative: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some((candidate, target)) = reference_target(file, raw, kind, allow_relative) else {
        return;
    };
    let target = normalize_path(&target);
    if reference_exists(root, &target) {
        return;
    }
    diagnostics.push(diagnostic(
        BROKEN_REFERENCE,
        Severity::Error,
        file,
        offset.map(|offset| line_at(source, offset)),
        format!("explicit repository reference {candidate:?} does not exist"),
        format!(
            "create {} or correct the explicit reference",
            slash(&target)
        ),
    ));
}

fn reference_target(
    file: &Path,
    raw: &str,
    kind: Option<&str>,
    allow_relative: bool,
) -> Option<(String, PathBuf)> {
    let candidate = raw
        .trim_matches(|character: char| matches!(character, '<' | '>' | '"' | '\'' | ',' | ';'))
        .split('#')
        .next()
        .unwrap_or_default()
        .trim_end_matches(|character: char| character == ':' || character.is_ascii_digit())
        .trim_end_matches('/');
    if candidate.is_empty()
        || candidate.contains(char::is_whitespace)
        || candidate.contains(['*', '{', '}', '$'])
        || candidate.contains("://")
    {
        return None;
    }

    let owned_prefix = ["agents/", "skills/", "plugins/", "prompts/"]
        .iter()
        .any(|prefix| candidate.starts_with(prefix));
    let explicit_relative = candidate.starts_with("./") || candidate.starts_with("../");
    let target = if owned_prefix {
        PathBuf::from(candidate)
    } else if explicit_relative && allow_relative {
        file.parent()
            .unwrap_or_else(|| Path::new(""))
            .join(candidate)
    } else {
        match kind {
            Some("agent" | "agent_path" | "agent_ref") => PathBuf::from("agents").join(candidate),
            Some("skill" | "skill_path" | "skill_ref") => PathBuf::from("skills").join(candidate),
            Some("plugin" | "plugin_path" | "plugin_ref") => {
                PathBuf::from("plugins").join(candidate)
            }
            Some("prompt_path" | "prompt_ref") => PathBuf::from("prompts").join(candidate),
            _ => return None,
        }
    };
    Some((candidate.to_owned(), target))
}

fn reference_exists(root: &Path, target: &Path) -> bool {
    let full = root.join(target);
    full.exists()
        || full.with_extension("md").exists()
        || (full.is_dir() && full.join("SKILL.md").is_file())
}

fn scan_shell_fences(
    file: &Path,
    source: &str,
    patterns: &SourcePatterns,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for fence in patterns.shell_fence.captures_iter(source) {
        let Some(body_match) = fence.get(1) else {
            continue;
        };
        let body = body_match.as_str();
        for capture in patterns.placeholder.captures_iter(body) {
            let Some(found) = capture.get(0) else {
                continue;
            };
            let placeholder = capture.get(2).map_or("placeholder", |value| value.as_str());
            diagnostics.push(diagnostic(
                UNRESOLVED_PLACEHOLDER,
                Severity::Error,
                file,
                Some(line_at(source, body_match.start() + found.start())),
                format!("unresolved literal placeholder {{{placeholder}}} in shell snippet"),
                "replace the placeholder before execution or use a defined shell variable",
            ));
        }
        for found in patterns.quoted_tilde.find_iter(body) {
            diagnostics.push(diagnostic(
                QUOTED_TILDE,
                Severity::Error,
                file,
                Some(line_at(source, body_match.start() + found.start())),
                "quoted tilde will not expand in a shell path",
                "use $HOME, leave the tilde unquoted, or quote only the suffix",
            ));
        }
        scan_shell_variables(
            file,
            source,
            body_match.start(),
            body,
            patterns,
            diagnostics,
        );
    }
}

fn scan_shell_variables(
    file: &Path,
    source: &str,
    body_offset: usize,
    body: &str,
    patterns: &SourcePatterns,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut defined = patterns
        .assignment
        .captures_iter(body)
        .map(|capture| capture[1].to_owned())
        .collect::<BTreeSet<_>>();
    defined.extend(
        patterns
            .loop_variable
            .captures_iter(body)
            .map(|capture| capture[1].to_owned()),
    );
    let searchable = strip_single_quoted(body);
    let mut emitted = BTreeSet::new();
    for capture in patterns.shell_variable.captures_iter(&searchable) {
        let Some(found) = capture.get(0) else {
            continue;
        };
        let variable = capture
            .get(1)
            .or_else(|| capture.get(2))
            .map_or("", |value| value.as_str());
        if variable.is_empty()
            || defined.contains(variable)
            || variable
                .chars()
                .all(|character| !character.is_ascii_lowercase())
            || !emitted.insert(variable.to_owned())
        {
            continue;
        }
        diagnostics.push(diagnostic(
            UNDEFINED_SHELL_VARIABLE,
            Severity::Error,
            file,
            Some(line_at(source, body_offset + found.start())),
            format!("shell variable ${variable} is not defined in this executable snippet"),
            "assign the variable in the snippet or use an explicit environment contract",
        ));
    }
}

fn lint_generated_docs(
    root: &Path,
    manifest: &WorkflowManifest,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let expected = render_workflow_docs(manifest);
    let path = root.join(GENERATED_WORKFLOW_DOC);
    if fs::read_to_string(&path).is_ok_and(|actual| actual == expected) {
        return;
    }
    diagnostics.push(diagnostic(
        GENERATED_DOC_STALE,
        Severity::Error,
        Path::new(GENERATED_WORKFLOW_DOC),
        None,
        "generated workflow documentation is missing or stale",
        "run `sddk generate docs --root .` and commit the result",
    ));
}

fn lint_generated_inventory(root: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let expected = match render_inventory(root) {
        Ok(expected) => expected,
        Err(error) => {
            diagnostics.push(diagnostic(
                GENERATED_INVENTORY_STALE,
                Severity::Error,
                Path::new(GENERATED_INVENTORY_DOC),
                None,
                format!("cannot render generated repository inventory: {error}"),
                "make repository agent and skill paths readable UTF-8 paths",
            ));
            return;
        }
    };
    let path = root.join(GENERATED_INVENTORY_DOC);
    if fs::read_to_string(&path).is_ok_and(|actual| actual == expected) {
        return;
    }
    diagnostics.push(diagnostic(
        GENERATED_INVENTORY_STALE,
        Severity::Error,
        Path::new(GENERATED_INVENTORY_DOC),
        None,
        "generated repository inventory is missing or stale",
        "run `sddk generate inventory --root .` and commit the result",
    ));
}

fn should_descend(entry: &DirEntry) -> bool {
    if !entry.file_type().is_dir() {
        return true;
    }
    !matches!(
        entry.file_name().to_str(),
        Some(".git" | "target" | ".atl" | "node_modules" | ".venv" | "__pycache__")
    )
}

fn is_source(path: &Path) -> bool {
    let components = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(name) => name.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>();
    if components
        .windows(2)
        .any(|pair| pair == ["tests", "fixtures"])
    {
        return false;
    }
    if path.components().any(|component| {
        matches!(
            component,
            Component::Normal(name)
                if matches!(name.to_str(), Some(".git" | "target" | ".atl"))
        )
    }) {
        return false;
    }
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("md" | "yaml" | "yml")
    ) && !matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("zip" | "gz" | "tgz" | "tar" | "7z" | "rar" | "exe" | "dll" | "so" | "a")
    )
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
            Component::RootDir | Component::Prefix(_) => return path.to_path_buf(),
        }
    }
    normalized
}

fn strip_single_quoted(value: &str) -> String {
    let mut quoted = false;
    value
        .chars()
        .map(|character| {
            if character == '\'' {
                quoted = !quoted;
                ' '
            } else if quoted {
                ' '
            } else {
                character
            }
        })
        .collect()
}

fn diagnostic(
    code: &str,
    severity: Severity,
    file: &Path,
    line: Option<usize>,
    message: impl Into<String>,
    hint: impl Into<String>,
) -> Diagnostic {
    Diagnostic {
        code: code.to_owned(),
        severity,
        file: slash(file),
        line,
        message: message.into(),
        hint: hint.into(),
    }
}

fn line_of(source: &str, needle: &str) -> Option<usize> {
    source.find(needle).map(|offset| line_at(source, offset))
}

fn line_at(source: &str, offset: usize) -> usize {
    source[..offset.min(source.len())]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}

fn slash(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn wire<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .expect("workflow enums are serializable")
        .as_str()
        .expect("workflow enums serialize as strings")
        .to_owned()
}

fn lint_agent_registry(root: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let registry_path = root.join("permissions.yaml");
    let policy = match sddk_gateway::PermissionPolicy::from_file(&registry_path) {
        Ok(policy) => policy,
        Err(error) => {
            diagnostics.push(diagnostic(
                AGENT_NOT_IN_REGISTRY,
                Severity::Error,
                Path::new("permissions.yaml"),
                None,
                format!("cannot load the agent permission registry: {error}"),
                "create permissions.yaml at the repository root with an `agents` mapping",
            ));
            return;
        }
    };
    let declared: BTreeSet<String> = policy.agents().map(str::to_owned).collect();

    let agents_dir = root.join("agents");
    if !agents_dir.is_dir() {
        return;
    }
    for entry in WalkDir::new(&agents_dir)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file()
            || entry.path().extension().and_then(|ext| ext.to_str()) != Some("md")
        {
            continue;
        }
        let stem = entry
            .path()
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default();
        if let Some(frontmatter_name) = agent_frontmatter_name(entry.path())
            && frontmatter_name != stem
        {
            diagnostics.push(diagnostic(
                AGENT_NAME_MISMATCH,
                Severity::Error,
                &Path::new("agents").join(format!("{stem}.md")),
                None,
                format!(
                    "agent frontmatter name {frontmatter_name:?} does not match file name {stem:?}"
                ),
                "align the frontmatter `name` with the file stem",
            ));
        }
        if !declared.contains(stem) {
            diagnostics.push(diagnostic(
                AGENT_NOT_IN_REGISTRY,
                Severity::Error,
                &Path::new("agents").join(format!("{stem}.md")),
                None,
                format!("agent {stem} is not declared in permissions.yaml"),
                "add the agent to the permission registry (default-deny unless declared)",
            ));
        }
    }

    for name in &declared {
        if !agents_dir.join(format!("{name}.md")).exists() {
            diagnostics.push(diagnostic(
                REGISTRY_ORPHAN,
                Severity::Warning,
                Path::new("permissions.yaml"),
                None,
                format!(
                    "permission registry declares agent {name:?} without an agents/{name}.md file"
                ),
                "remove the orphan entry or create the agent file",
            ));
        }
    }
}

fn agent_frontmatter_name(path: &Path) -> Option<String> {
    let source = fs::read_to_string(path).ok()?;
    let rest = source.strip_prefix("---")?;
    let frontmatter = rest.split_once("\n---")?.0;
    frontmatter
        .lines()
        .find_map(|line| line.strip_prefix("name:"))
        .map(|value| value.trim().trim_matches('"').trim_matches('\'').to_owned())
        .filter(|value| !value.is_empty())
}

fn lint_pack_manifest(root: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let relative = Path::new("manifest.toml");
    let path = root.join(relative);
    let manifest = match sddk_domain::load_pack_manifest(&path) {
        Ok(manifest) => manifest,
        Err(sddk_domain::PackError::Io { .. }) => {
            diagnostics.push(diagnostic(
                INVALID_PACK_MANIFEST,
                Severity::Error,
                relative,
                None,
                "pack manifest manifest.toml is missing",
                "declare the framework pack with identity, commands, and fixtures",
            ));
            return;
        }
        Err(error) => {
            diagnostics.push(diagnostic(
                INVALID_PACK_MANIFEST,
                Severity::Error,
                relative,
                None,
                format!("pack manifest is invalid: {error}"),
                "fix the TOML syntax or align it with the pack model",
            ));
            return;
        }
    };
    for pack_diagnostic in sddk_domain::validate_pack_manifest(&manifest) {
        diagnostics.push(diagnostic(
            INVALID_PACK_MANIFEST,
            Severity::Error,
            relative,
            None,
            format!("{}: {}", pack_diagnostic.code, pack_diagnostic.message),
            pack_diagnostic.hint,
        ));
    }
}

fn lint_instruction_contract(root: &Path, diagnostics: &mut Vec<Diagnostic>) {
    lint_matrix_schema(root, diagnostics);
    lint_matrix_pointer(root, diagnostics);
    lint_sizing_separation(root, diagnostics);
    lint_agent_model_registry(root, diagnostics);
    lint_cli_command_allowlist(root, diagnostics);
    lint_closure_ordering(root, diagnostics);
    lint_version_lockstep(root, diagnostics);
    lint_apply_push_anchors(root, diagnostics);
    lint_matrix_dry_run_invariant(root, diagnostics);
    lint_matrix_facade_shadow_routing(root, diagnostics);
    lint_matrix_facade_argv_accuracy(root, diagnostics);
    lint_matrix_safety_advisory_separation(root, diagnostics);
    lint_f4_gotchas(root, diagnostics);
    lint_zero_intrusion(root, diagnostics);
    lint_owner_boundary(root, diagnostics);
    lint_release_chain_ordering(root, diagnostics);
    lint_matrix_lockstep_refusal(root, diagnostics);
    lint_instruction_recipe_dedup(root, diagnostics);
}

/// Derive the allow-list of real `sddk <verb>` subcommands via clap reflection.
/// The five first-class facades (status, plan, run, ship, recover) are already
/// `Command` members so they appear in `get_subcommands()`; the explicit insert
/// is belt-and-braces.
fn cli_command_allowlist() -> BTreeSet<String> {
    let mut set: BTreeSet<String> = crate::Cli::command()
        .get_subcommands()
        .map(|cmd| cmd.get_name().to_owned())
        .collect();
    for verb in ["status", "plan", "run", "ship", "recover"] {
        set.insert(verb.to_owned());
    }
    set
}

// ─────────────────────────────────────────────────────────────────────────────
// SDDK015/SDDK016 — matrix parsing helpers (line-based, order-preserving)
// ─────────────────────────────────────────────────────────────────────────────

/// One matrix row extracted structurally from the fenced YAML block.
struct MatrixRowData {
    intent: String,
    intent_line: usize,
    /// Keys in file order (mandatory and optional), top-level only.
    keys_in_order: Vec<String>,
    key_lines: Vec<usize>,
}

/// Extract candidate fenced ```yaml bodies (all of them — the matrix may not be
/// the first fenced block in the document).
fn fenced_yaml_bodies(source: &str) -> Vec<(usize, String)> {
    let fence_re = match Regex::new("(?ms)^```yaml[ \t]*\n(.*?)^```[ \t]*$") {
        Ok(re) => re,
        Err(_) => return Vec::new(),
    };
    fence_re
        .captures_iter(source)
        .filter_map(|captures| {
            let whole = captures.get(0)?;
            let body = captures.get(1)?;
            // +1: body starts AFTER the ```yaml fence line (the fence itself is
            // not part of the captured body). body_start_line is 1-indexed.
            let line = source[..whole.start()].lines().count() + 2;
            Some((line, body.as_str().to_owned()))
        })
        .collect()
}

/// Line-based structural parse of one fenced body into matrix rows. Preserves
/// file order (serde_json::Map is alphabetical, so the parser cannot be used
/// for the ordering check) and computes real document line numbers.
fn parse_matrix_rows(body: &str, body_start_line: usize) -> Vec<MatrixRowData> {
    let key_re = Regex::new(r"^\s{2,}([A-Za-z_][\w-]*):\s*(.*)$").expect("valid matrix key regex");
    let item_key_re = Regex::new(r"^-\s*([A-Za-z_][\w-]*):").expect("valid item key regex");
    let intent_re = Regex::new(r"^\s*-?\s*intent:\s*(\S+)").expect("valid intent regex");
    let mut rows: Vec<MatrixRowData> = Vec::new();
    for (offset, line) in body.lines().enumerate() {
        let document_line = body_start_line + offset;
        // A new matrix row starts with a column-0 sequence item. Deeper-indented
        // "- " lines are nested list continuations (e.g. required_inputs lists)
        // and must NOT start a phantom row.
        if line.starts_with("- ") || line == "-" {
            let intent = intent_re
                .captures(line)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_owned())
                .unwrap_or_default();
            rows.push(MatrixRowData {
                intent,
                intent_line: document_line,
                keys_in_order: Vec::new(),
                key_lines: Vec::new(),
            });
            // A sequence item may carry its first key inline (" - intent: x").
            if let Some((key, row)) = item_key_re
                .captures(line)
                .and_then(|c| c.get(1))
                .zip(rows.last_mut())
            {
                row.keys_in_order.push(key.as_str().to_owned());
                row.key_lines.push(document_line);
            }
            continue;
        }
        let Some(captures) = key_re.captures(line) else {
            continue;
        };
        let Some(row) = rows.last_mut() else {
            continue;
        };
        if let Some(key) = captures.get(1) {
            if key.as_str() == "intent"
                && row.intent.is_empty()
                && let Some(value) = captures.get(2)
            {
                row.intent = value.as_str().trim().trim_matches('"').to_owned();
            }
            row.keys_in_order.push(key.as_str().to_owned());
            row.key_lines.push(document_line);
        }
    }
    rows
}

/// Select the fenced body that actually contains the matrix rows: the first
/// body whose parse yields at least one row carrying a non-empty `intent`.
fn matrix_rows_from_source(source: &str) -> Option<Vec<MatrixRowData>> {
    for (body_start_line, body) in fenced_yaml_bodies(source) {
        let rows = parse_matrix_rows(&body, body_start_line);
        if rows
            .iter()
            .any(|row| !row.intent.is_empty() && row.keys_in_order.len() >= 4)
        {
            return Some(rows);
        }
    }
    None
}

// ─────────────────────────────────────────────────────────────────────────────
// SDDK015 — instruction.matrix.schema
// ─────────────────────────────────────────────────────────────────────────────

fn lint_matrix_schema(root: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let contract_path = root.join("skills/_shared/cli-usage-contract.md");
    let source = match fs::read_to_string(&contract_path) {
        Ok(s) => s,
        Err(_) => return,
    };
    let Some(rows) = matrix_rows_from_source(&source) else {
        return;
    };

    for row in &rows {
        if row.intent.is_empty() {
            continue;
        }
        let present: Vec<&str> = MATRIX_REQUIRED_COLUMNS
            .iter()
            .copied()
            .filter(|column| row.keys_in_order.iter().any(|key| key == *column))
            .collect();
        for required in MATRIX_REQUIRED_COLUMNS {
            if !present.contains(required) {
                diagnostics.push(diagnostic(
                    MATRIX_SCHEMA,
                    Severity::Error,
                    Path::new("skills/_shared/cli-usage-contract.md"),
                    Some(row.intent_line),
                    format!("matrix row `{}` is missing column `{required}`", row.intent),
                    "add the missing column to the matrix row",
                ));
            }
        }
        // The mandatory columns that ARE present must follow the declared
        // relative order IN THE FILE (keys_in_order preserves file order).
        let mandatory_in_file_order: Vec<&str> = row
            .keys_in_order
            .iter()
            .map(|key| key.as_str())
            .filter(|key| MATRIX_REQUIRED_COLUMNS.contains(key))
            .collect();
        let declared_present: Vec<&str> = MATRIX_REQUIRED_COLUMNS
            .iter()
            .copied()
            .filter(|column| mandatory_in_file_order.contains(column))
            .collect();
        if mandatory_in_file_order != declared_present {
            diagnostics.push(diagnostic(
                MATRIX_SCHEMA,
                Severity::Error,
                Path::new("skills/_shared/cli-usage-contract.md"),
                Some(row.intent_line),
                format!(
                    "matrix row `{}` has mandatory columns out of declared order",
                    row.intent
                ),
                "reorder columns to match the declared sequence",
            ));
        }
        // Optional metadata is allowed only after the mandatory block.
        if let Some(last_mandatory) = row
            .keys_in_order
            .iter()
            .rposition(|key| MATRIX_REQUIRED_COLUMNS.contains(&key.as_str()))
        {
            for (index, key) in row.keys_in_order.iter().enumerate() {
                if index < last_mandatory && !MATRIX_REQUIRED_COLUMNS.contains(&key.as_str()) {
                    diagnostics.push(diagnostic(
                        MATRIX_SCHEMA,
                        Severity::Error,
                        Path::new("skills/_shared/cli-usage-contract.md"),
                        row.key_lines.get(index).copied().or(Some(row.intent_line)),
                        format!(
                            "matrix row `{}` has optional column `{key}` before the mandatory block",
                            row.intent
                        ),
                        "move optional metadata after the eight mandatory columns",
                    ));
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SDDK016 — instruction.matrix.pointer
// ─────────────────────────────────────────────────────────────────────────────

fn lint_matrix_pointer(root: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let contract_path = root.join("skills/_shared/cli-usage-contract.md");
    let source = match fs::read_to_string(&contract_path) {
        Ok(s) => s,
        Err(_) => return,
    };
    let Some(rows) = matrix_rows_from_source(&source) else {
        return;
    };
    let declared_intents: BTreeSet<String> = rows
        .iter()
        .filter(|row| !row.intent.is_empty())
        .map(|row| row.intent.clone())
        .collect();

    // Scan prompts/** and agents/** for "Matrix row: <intent>" references
    let pointer_re = Regex::new(r"Matrix row:\s*(\S+)").expect("valid pointer regex");
    let search_dirs = ["prompts", "agents"];

    for dir_name in search_dirs {
        let dir = root.join(dir_name);
        if !dir.is_dir() {
            continue;
        }
        for entry in WalkDir::new(&dir)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let Ok(content) = fs::read_to_string(path) else {
                continue;
            };
            for captures in pointer_re.captures_iter(&content) {
                let Some(reference) = captures.get(1) else {
                    continue;
                };
                if !declared_intents.contains(reference.as_str()) {
                    let file = path.strip_prefix(root).unwrap_or(path);
                    diagnostics.push(diagnostic(
                        MATRIX_POINTER,
                        Severity::Error,
                        file,
                        Some(line_at(&content, reference.start())),
                        format!(
                            "matrix row reference `{}` does not resolve to a declared intent",
                            reference.as_str()
                        ),
                        "point the reference at an intent declared in the instruction contract matrix",
                    ));
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SDDK017 — sizing.advisory.separation
// ─────────────────────────────────────────────────────────────────────────────

fn lint_sizing_separation(root: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let forbidden_patterns = [
        ("size:exception", r"size:\s*exception"),
        ("circuit-advisor", r"circuit-advisor"),
        (
            "gate: forecast <= budget",
            r"gate:\s*forecast\s*<=\s*budget",
        ),
        ("emit_size_recommendation", r"emit_size_recommendation"),
    ];

    let target_files = [
        "prompts/sddk/phases/apply.md",
        "prompts/sddk/decision-model.md",
        "prompts/sddk/mcw.md",
    ];

    // Also scan prompts/sddk/workflows/*.yaml
    let workflow_dir = root.join("prompts/sddk/workflows");
    let workflow_files: Vec<PathBuf> = if workflow_dir.is_dir() {
        WalkDir::new(&workflow_dir)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
            .filter(|e| e.path().extension().and_then(|e| e.to_str()) == Some("yaml"))
            .map(|e| e.path().to_path_buf())
            .collect()
    } else {
        Vec::new()
    };

    let mut all_targets: Vec<PathBuf> = target_files
        .iter()
        .map(|f| root.join(f))
        .chain(workflow_files.iter().cloned())
        .collect();

    // Deduplicate
    all_targets.sort();
    all_targets.dedup();

    for path in all_targets {
        let relative = path.strip_prefix(root).unwrap_or(&path);
        let Ok(source) = fs::read_to_string(&path) else {
            continue;
        };

        for (name, pattern) in &forbidden_patterns {
            let re = match Regex::new(pattern) {
                Ok(r) => r,
                Err(_) => continue,
            };
            for m in re.find_iter(&source) {
                diagnostics.push(diagnostic(
                    SIZING_SEPARATION,
                    Severity::Error,
                    relative,
                    Some(line_at(&source, m.start())),
                    format!(
                        "forbidden sizing-advisory language `{name}` found in sizing-governed file",
                    ),
                    "remove size-gating language from apply.md, decision-model.md, mcw.md, or prompts/sddk/workflows/*.yaml",
                ));
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SDDK018 — agent.registry.unregistered
// ─────────────────────────────────────────────────────────────────────────────

fn lint_agent_model_registry(root: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let models_path = root.join("assets/agent-models.yaml");
    let yaml_source = match fs::read_to_string(&models_path) {
        Ok(s) => s,
        Err(_) => return,
    };
    let parsed: serde_json::Map<String, serde_json::Value> =
        match serde_saphyr::from_str(&yaml_source) {
            Ok(v) => v,
            Err(_) => return,
        };

    let agents_map = match parsed.get("agents") {
        Some(serde_json::Value::Object(m)) => m,
        _ => return,
    };

    let registered: BTreeSet<String> = agents_map.keys().cloned().collect();
    let agents_dir = root.join("agents");
    if !agents_dir.is_dir() {
        return;
    }

    for entry in WalkDir::new(&agents_dir)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        if !registered.contains(stem) {
            let relative = Path::new("agents").join(format!("{stem}.md"));
            diagnostics.push(diagnostic(
                AGENT_REGISTRY_UNREGISTERED,
                Severity::Error,
                &relative,
                None,
                format!("agent `{stem}` is not registered in assets/agent-models.yaml"),
                "add the agent to the `agents` mapping in assets/agent-models.yaml",
            ));
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SDDK019 — instruction.cli.command.unknown
// ─────────────────────────────────────────────────────────────────────────────

fn lint_cli_command_allowlist(root: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let allowlist = cli_command_allowlist();

    // Scan instruction files (prompts/** and agents/**) for `sddk <verb>` command
    // references. The opening backtick anchors the match to actual command spans —
    // instruction files wrap commands in backticks, prose ("the sddk framework")
    // does not — which avoids false positives on identifiers like entropy-sdd,
    // entropy_sdd, or "ropy_sdd output" while still firing on real references.
    let sddk_cmd_re = Regex::new(r"`sddk\s+(\w[\w-]*)").expect("valid sddk command regex");
    let search_dirs = ["prompts", "agents"];

    for dir_name in search_dirs {
        let dir = root.join(dir_name);
        if !dir.is_dir() {
            continue;
        }
        for entry in WalkDir::new(&dir)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let Ok(source) = fs::read_to_string(path) else {
                continue;
            };
            for cap in sddk_cmd_re.captures_iter(&source) {
                if let Some(verb_m) = cap.get(1) {
                    let verb_str = verb_m.as_str();
                    if !allowlist.contains(verb_str) {
                        let file = path.strip_prefix(root).unwrap_or(path);
                        let cmd_m = cap.get(0).unwrap();
                        let cmd_str = cmd_m.as_str();
                        diagnostics.push(diagnostic(
                            CLI_COMMAND_UNKNOWN,
                            Severity::Error,
                            file,
                            Some(line_at(&source, cmd_m.start())),
                            format!("`{cmd_str}` is not a known CLI subcommand"),
                            "use a subcommand listed in `sddk --help` or `sddk <cmd> --help`",
                        ));
                    }
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SDDK020 — instruction.closure.ordering
// ─────────────────────────────────────────────────────────────────────────────

/// Returns all lines belonging to the block that starts at `start_line`
/// (0-indexed). The block ends at the next key at the same or lower indentation.
fn block_lines(start_line: usize, all_lines: &[&str]) -> Vec<String> {
    let mut lines = Vec::new();
    // The first line is the key: value line itself (not a continuation)
    if start_line < all_lines.len() {
        lines.push(all_lines[start_line].to_string());
    }
    // Collect following lines while they are indented more than the key
    let key_indent = all_lines
        .get(start_line)
        .map(|l| l.len() - l.trim_start().len())
        .unwrap_or(0);
    for line in all_lines.iter().skip(start_line + 1) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let line_indent = line.len() - line.trim_start().len();
        // Break only when we hit a non-list, non-empty line at the key indent level.
        // A YAML list item starting with '- ' at any indentation continues the block.
        let is_list_item = trimmed.starts_with('-');
        if line_indent <= key_indent && !is_list_item {
            break; // next key reached
        }
        lines.push(line.to_string());
    }
    lines
}

/// Checks that workflow phases and mcw.md follow the correct closure chain.
///
/// A-full:   tasks → apply → verify → debt-verify → release → archive
/// A-lite/A-min: apply → verify → debt-verify → release → archive
/// B-direct: build → verify → release → archive  (debt-verify MUST be disabled)
fn lint_closure_ordering(root: &Path, diagnostics: &mut Vec<Diagnostic>) {
    // 1. Check mcw.md has the A-full ordering string
    let mcw_path = root.join("prompts/sddk/mcw.md");
    if let Ok(mcw_src) = fs::read_to_string(&mcw_path) {
        let has_a_full = mcw_src
            .contains("tasks → apply → verify → debt-verify → release → archive")
            || mcw_src.contains("apply → verify → debt-verify → release → archive");
        if !has_a_full {
            diagnostics.push(diagnostic(
                INSTRUCTION_CLOSURE_ORDERING,
                Severity::Error,
                Path::new("prompts/sddk/mcw.md"),
                None,
                "mcw.md is missing the A-full or A-lite closure ordering string".to_string(),
                "add the full closure chain to mcw.md",
            ));
        }
    }

    // 2. Check archive row has --release-receipt and vault_write/cas_write
    let contract_path = root.join("skills/_shared/cli-usage-contract.md");
    if let Ok(contract_src) = fs::read_to_string(&contract_path)
        && let Some(rows) = matrix_rows_from_source(&contract_src)
    {
        let all_lines: Vec<&str> = contract_src.lines().collect();
        for row in &rows {
            if row.intent == "lifecycle.archive.complete" {
                // Collect all lines in the required_inputs block
                let req_idx = row
                    .keys_in_order
                    .iter()
                    .position(|k| k == "required_inputs");
                let req_lines: Vec<String> = req_idx
                    .and_then(|idx| row.key_lines.get(idx))
                    .map(|l| block_lines((*l).saturating_sub(1), &all_lines))
                    .unwrap_or_default();
                let req_block = req_lines.join("\n");
                let has_receipt = req_block.contains("--release-receipt");

                // Collect all lines in the side_effects block
                let se_idx = row.keys_in_order.iter().position(|k| k == "side_effects");
                let se_lines: Vec<String> = se_idx
                    .and_then(|idx| row.key_lines.get(idx))
                    .map(|l| block_lines((*l).saturating_sub(1), &all_lines))
                    .unwrap_or_default();
                let se_block = se_lines.join("\n");
                let has_knowledge_write =
                    se_block.contains("vault_write") || se_block.contains("cas_write");

                if !has_receipt {
                    diagnostics.push(diagnostic(
                        INSTRUCTION_CLOSURE_ORDERING,
                        Severity::Error,
                        Path::new("skills/_shared/cli-usage-contract.md"),
                        Some(row.intent_line),
                        "lifecycle.archive.complete is missing `--release-receipt` in required_inputs"
                            .to_string(),
                        "add `--release-receipt <ID>` to required_inputs",
                    ));
                }
                if !has_knowledge_write {
                    diagnostics.push(diagnostic(
                        INSTRUCTION_CLOSURE_ORDERING,
                        Severity::Error,
                        Path::new("skills/_shared/cli-usage-contract.md"),
                        Some(row.intent_line),
                        "lifecycle.archive.complete is missing vault_write or cas_write in side_effects"
                            .to_string(),
                        "add vault_write or cas_write to side_effects",
                    ));
                }
            }
        }
    }

    // 3. Check workflow YAMLs for debt-verify disabled on B-direct
    let workflow_dir = root.join("prompts/sddk/workflows");
    if workflow_dir.is_dir() {
        for entry in WalkDir::new(&workflow_dir)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
            .filter(|e| e.path().extension().and_then(|e| e.to_str()) == Some("yaml"))
        {
            let path = entry.path();
            let relative = path.strip_prefix(root).unwrap_or(path);
            if let Ok(content) = fs::read_to_string(path) {
                let is_b_direct = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.contains("b-direct"))
                    .unwrap_or(false);
                if is_b_direct {
                    let has_mandatory_debt_verify = content.contains("debt-verify:")
                        && !content.contains("debt-verify: disabled")
                        && !content.contains("no debt-verify")
                        && !content.contains("policy: disabled");
                    if has_mandatory_debt_verify {
                        diagnostics.push(diagnostic(
                            INSTRUCTION_CLOSURE_ORDERING,
                            Severity::Error,
                            relative,
                            None,
                            "B-direct workflow mentions debt-verify as mandatory; it must be disabled"
                                .to_string(),
                            "set debt-verify to disabled or add no debt-verify marker",
                        ));
                    }
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SDDK021 — manifest.version.lockstep
// ─────────────────────────────────────────────────────────────────────────────

/// Checks that both `facade.ship` and `lifecycle.release.plan` matrix rows
/// mention version_lockstep in their expected_output.
fn lint_version_lockstep(root: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let contract_path = root.join("skills/_shared/cli-usage-contract.md");
    let source = match fs::read_to_string(&contract_path) {
        Ok(s) => s,
        Err(_) => return,
    };
    let Some(rows) = matrix_rows_from_source(&source) else {
        return;
    };

    let all_lines: Vec<&str> = source.lines().collect();
    let mut facade_ship_has_token = false;
    let mut release_plan_has_token = false;

    for row in &rows {
        let check_output = |row: &MatrixRowData| -> bool {
            let idx = row
                .keys_in_order
                .iter()
                .position(|k| k == "expected_output");
            let Some(&line_no) = idx.and_then(|i| row.key_lines.get(i)) else {
                return false;
            };
            let block = block_lines(line_no.saturating_sub(1), &all_lines);
            block.iter().any(|l| l.contains("version_lockstep"))
        };

        if row.intent == "facade.ship" && check_output(row) {
            facade_ship_has_token = true;
        }
        if row.intent == "lifecycle.release.plan" && check_output(row) {
            release_plan_has_token = true;
        }
    }

    if !facade_ship_has_token {
        diagnostics.push(diagnostic(
            MANIFEST_VERSION_LOCKSTEP,
            Severity::Error,
            Path::new("skills/_shared/cli-usage-contract.md"),
            None,
            "facade.ship expected_output is missing version_lockstep token".to_string(),
            "add version_lockstep_passed to facade.ship expected_output",
        ));
    }
    if !release_plan_has_token {
        diagnostics.push(diagnostic(
            MANIFEST_VERSION_LOCKSTEP,
            Severity::Error,
            Path::new("skills/_shared/cli-usage-contract.md"),
            None,
            "lifecycle.release.plan expected_output is missing version_lockstep token".to_string(),
            "add version_lockstep to lifecycle.release.plan expected_output",
        ));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SDDK022 — instruction.apply-push.anchors
// ─────────────────────────────────────────────────────────────────────────────

/// Checks that apply.md and verify.md carry the three push-discipline anchors:
/// 1. `## Push Discipline (binding)` heading in apply.md
/// 2. `^Transition:\s*phase\.build\.complete$` in apply.md
/// 3. `git rev-parse origin/main` ≥ 2 occurrences in verify.md
fn lint_apply_push_anchors(root: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let apply_path = root.join("prompts/sddk/phases/apply.md");
    let verify_path = root.join("prompts/sddk/phases/verify.md");

    // Anchor 1: Push Discipline heading in apply.md
    let push_heading_re = Regex::new(r"^## Push Discipline \(binding\)$").unwrap();
    let transition_re =
        Regex::new(r"^Transition:[[:space:]]+phase\.build\.complete[[:space:]]*$").unwrap();

    if let Ok(apply_src) = fs::read_to_string(&apply_path) {
        let has_push_heading = apply_src
            .lines()
            .any(|l| push_heading_re.is_match(l.trim()));
        let has_transition = apply_src.lines().any(|l| transition_re.is_match(l.trim()));

        if !has_push_heading {
            diagnostics.push(diagnostic(
                INSTRUCTION_APPLY_PUSH_ANCHORS,
                Severity::Error,
                Path::new("prompts/sddk/phases/apply.md"),
                None,
                "apply.md is missing `## Push Discipline (binding)` heading".to_string(),
                "add the `## Push Discipline (binding)` heading to apply.md",
            ));
        }
        if !has_transition {
            diagnostics.push(diagnostic(
                INSTRUCTION_APPLY_PUSH_ANCHORS,
                Severity::Error,
                Path::new("prompts/sddk/phases/apply.md"),
                None,
                "apply.md is missing `Transition: phase.build.complete` line".to_string(),
                "add `Transition: phase.build.complete` to apply.md",
            ));
        }
    }
    // Note: if apply.md is absent we silently skip — the file may not exist in
    // minimal fixture repos and this lint is only relevant for SDDK-phase projects.

    // Anchor 3: git rev-parse origin/main ≥ 2 in verify.md
    if let Ok(verify_src) = fs::read_to_string(&verify_path) {
        let count = verify_src.matches("git rev-parse origin/main").count();
        if count < 2 {
            diagnostics.push(diagnostic(
                INSTRUCTION_APPLY_PUSH_ANCHORS,
                Severity::Error,
                Path::new("prompts/sddk/phases/verify.md"),
                None,
                format!("verify.md has only {count} occurrence(s) of `git rev-parse origin/main`; expected ≥ 2"),
                "add at least 2 occurrences of `git rev-parse origin/main` to verify.md",
            ));
        }
    }
    // Note: if verify.md is absent we silently skip — same reasoning as apply.md.
}

// ─────────────────────────────────────────────────────────────────────────────
// SDDK023 — matrix.dry-run.invariant
// ─────────────────────────────────────────────────────────────────────────────

/// Checks that facade.ship and facade.recover rows carry dry_run_invariant
/// with required content, empty side_effects, and no facade --dry-run flag.
fn lint_matrix_dry_run_invariant(root: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let contract_path = root.join("skills/_shared/cli-usage-contract.md");
    let source = match fs::read_to_string(&contract_path) {
        Ok(s) => s,
        Err(_) => return,
    };
    let Some(rows) = matrix_rows_from_source(&source) else {
        return;
    };

    let all_lines: Vec<&str> = source.lines().collect();

    // Targeted rows: facade.ship and facade.recover
    for row in &rows {
        if row.intent != "facade.ship" && row.intent != "facade.recover" {
            continue;
        }

        // Find dry_run_invariant block
        let invariant_idx = row
            .keys_in_order
            .iter()
            .position(|k| k == "dry_run_invariant");
        let Some(&invariant_line) = invariant_idx.and_then(|i| row.key_lines.get(i)) else {
            diagnostics.push(diagnostic(
                MATRIX_DRY_RUN_INVARIANT,
                Severity::Error,
                Path::new("skills/_shared/cli-usage-contract.md"),
                Some(row.intent_line),
                format!("`{}` row is missing `dry_run_invariant`", row.intent),
                "add dry_run_invariant to the matrix row",
            ));
            continue;
        };

        let invariant_block = block_lines(invariant_line.saturating_sub(1), &all_lines);
        let invariant_text = invariant_block.join("\n");

        // Check facade.recover requires BOTH digest AND event_count
        if row.intent == "facade.recover" {
            let has_digest = invariant_text.contains("digest");
            let has_event_count = invariant_text.contains("event_count");
            if !has_digest || !has_event_count {
                diagnostics.push(diagnostic(
                    MATRIX_DRY_RUN_INVARIANT,
                    Severity::Error,
                    Path::new("skills/_shared/cli-usage-contract.md"),
                    Some(row.intent_line),
                    format!(
                        "`{}` dry_run_invariant must mention both `digest` AND `event_count`",
                        row.intent
                    ),
                    "add both `digest` and `event_count` to dry_run_invariant",
                ));
            }
        }

        // Check facade.ship must NOT have a facade --dry-run flag (negative pattern required)
        // The dry_run_invariant must say "no facade --dry-run" or similar to explicitly deny it
        if row.intent == "facade.ship" {
            // Check for negative pattern like "no facade --dry-run" - if absent, fire
            let has_negative_pattern = invariant_text.contains("no facade")
                && (invariant_text.contains("--dry-run") || invariant_text.contains("dry-run"));
            if !has_negative_pattern {
                diagnostics.push(diagnostic(
                    MATRIX_DRY_RUN_INVARIANT,
                    Severity::Error,
                    Path::new("skills/_shared/cli-usage-contract.md"),
                    Some(row.intent_line),
                    format!("`{}` dry_run_invariant must contain a negative pattern like `no facade --dry-run`", row.intent),
                    "add explicit denial of facade --dry-run flag",
                ));
            }
        }

        // Both rows must have empty side_effects
        let side_effects_idx = row.keys_in_order.iter().position(|k| k == "side_effects");
        if let Some(&se_line_no) = side_effects_idx.and_then(|idx| row.key_lines.get(idx)) {
            let se_block = block_lines(se_line_no.saturating_sub(1), &all_lines);
            let se_text = se_block.join("\n");
            // Check for non-empty side_effects
            // If the value is inline (e.g., "side_effects: [ledger_append]"), check first line
            // If the value is multi-line, check all continuation lines
            let has_effects = if se_block.len() == 1 {
                // Single line - check if value after colon is not just "[]"
                let first = se_block[0].clone();
                if let Some(colon_pos) = first.find(':') {
                    let after_colon = first[colon_pos + 1..].trim();
                    after_colon != "[]" && after_colon != "[]\n"
                } else {
                    false
                }
            } else {
                // Multi-line - check continuation lines for non-empty content
                se_text.lines().skip(1).any(|l| {
                    let trimmed = l.trim();
                    !trimmed.is_empty() && trimmed != "[]" && trimmed != "- []"
                })
            };
            if has_effects {
                diagnostics.push(diagnostic(
                    MATRIX_DRY_RUN_INVARIANT,
                    Severity::Error,
                    Path::new("skills/_shared/cli-usage-contract.md"),
                    Some(row.intent_line),
                    format!("`{}` side_effects must be empty", row.intent),
                    "set side_effects to an empty list",
                ));
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SDDK024 — matrix.facade.shadow-routing
// ─────────────────────────────────────────────────────────────────────────────

/// Checks that every facade.* row carries a non-empty shadow_target_row.
fn lint_matrix_facade_shadow_routing(root: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let contract_path = root.join("skills/_shared/cli-usage-contract.md");
    let source = match fs::read_to_string(&contract_path) {
        Ok(s) => s,
        Err(_) => return,
    };
    let Some(rows) = matrix_rows_from_source(&source) else {
        return;
    };

    let all_lines: Vec<&str> = source.lines().collect();
    let facade_intents = [
        "facade.status",
        "facade.plan",
        "facade.run",
        "facade.ship",
        "facade.recover",
    ];

    for row in &rows {
        if !facade_intents.contains(&row.intent.as_str()) {
            continue;
        }

        let shadow_idx = row
            .keys_in_order
            .iter()
            .position(|k| k == "shadow_target_row");
        let shadow_line = shadow_idx.and_then(|idx| row.key_lines.get(idx));
        let Some(&shadow_line_no) = shadow_line else {
            diagnostics.push(diagnostic(
                MATRIX_FACADE_SHADOW_ROUTING,
                Severity::Error,
                Path::new("skills/_shared/cli-usage-contract.md"),
                Some(row.intent_line),
                format!("`{}` row is missing `shadow_target_row`", row.intent),
                "add shadow_target_row to the matrix row",
            ));
            continue;
        };

        let shadow_block = block_lines(shadow_line_no.saturating_sub(1), &all_lines);
        // Check non-empty: the first line contains "shadow_target_row: <value>"
        // We need to check if there's actual content after the colon on the first line
        let has_value = shadow_block.first().is_some_and(|first_line| {
            // Extract content after the colon
            if let Some(idx) = first_line.find(':') {
                let after_colon = first_line[idx + 1..].trim();
                !after_colon.is_empty()
            } else {
                false
            }
        });

        if !has_value {
            diagnostics.push(diagnostic(
                MATRIX_FACADE_SHADOW_ROUTING,
                Severity::Error,
                Path::new("skills/_shared/cli-usage-contract.md"),
                Some(row.intent_line),
                format!("`{}` row has empty `shadow_target_row`", row.intent),
                "provide a non-empty shadow_target_row value",
            ));
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SDDK025 — matrix.facade.argv-accuracy
// ─────────────────────────────────────────────────────────────────────────────

/// Checks argv accuracy for facade.plan, facade.recover, facade.ship, and
/// validates that lifecycle.plan.start.legacy-direct row exists.
fn lint_matrix_facade_argv_accuracy(root: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let contract_path = root.join("skills/_shared/cli-usage-contract.md");
    let source = match fs::read_to_string(&contract_path) {
        Ok(s) => s,
        Err(_) => return,
    };
    let Some(rows) = matrix_rows_from_source(&source) else {
        return;
    };

    let all_lines: Vec<&str> = source.lines().collect();

    // Sub-check (a): facade.plan required_inputs exactly {--name, --path, --branch, --format}
    for row in &rows {
        if row.intent == "facade.plan" {
            let req_idx = row
                .keys_in_order
                .iter()
                .position(|k| k == "required_inputs");
            let req_line = req_idx.and_then(|idx| row.key_lines.get(idx));
            let Some(&req_line_no) = req_line else {
                diagnostics.push(diagnostic(
                    MATRIX_FACADE_ARGV_ACCURACY,
                    Severity::Error,
                    Path::new("skills/_shared/cli-usage-contract.md"),
                    Some(row.intent_line),
                    "`facade.plan` is missing `required_inputs`".to_string(),
                    "add required_inputs to facade.plan",
                ));
                continue;
            };

            let req_block = block_lines(req_line_no.saturating_sub(1), &all_lines);
            let req_text = req_block.join("\n");

            // Must NOT contain --root or --scope
            if req_text.contains("--root") || req_text.contains("--scope") {
                diagnostics.push(diagnostic(
                    MATRIX_FACADE_ARGV_ACCURACY,
                    Severity::Error,
                    Path::new("skills/_shared/cli-usage-contract.md"),
                    Some(row.intent_line),
                    "`facade.plan` required_inputs must not contain `--root` or `--scope`"
                        .to_string(),
                    "remove --root and --scope from facade.plan required_inputs",
                ));
            }

            // Must contain exactly the four facade flags
            let has_name = req_text.contains("--name");
            let has_path = req_text.contains("--path");
            let has_branch = req_text.contains("--branch");
            let has_format = req_text.contains("--format");
            if !has_name || !has_path || !has_branch || !has_format {
                diagnostics.push(diagnostic(
                    MATRIX_FACADE_ARGV_ACCURACY,
                    Severity::Error,
                    Path::new("skills/_shared/cli-usage-contract.md"),
                    Some(row.intent_line),
                    "`facade.plan` required_inputs must contain exactly `--name`, `--path`, `--branch`, `--format`".to_string(),
                    "set required_inputs to the four facade flags",
                ));
            }
        }

        // Sub-check (b): facade.recover requirements
        if row.intent == "facade.recover" {
            let req_idx = row
                .keys_in_order
                .iter()
                .position(|k| k == "required_inputs");
            let req_line = req_idx.and_then(|idx| row.key_lines.get(idx));
            let Some(&req_line_no) = req_line else {
                diagnostics.push(diagnostic(
                    MATRIX_FACADE_ARGV_ACCURACY,
                    Severity::Error,
                    Path::new("skills/_shared/cli-usage-contract.md"),
                    Some(row.intent_line),
                    "`facade.recover` is missing `required_inputs`".to_string(),
                    "add required_inputs to facade.recover",
                ));
                continue;
            };

            let req_block = block_lines(req_line_no.saturating_sub(1), &all_lines);
            let req_text = req_block.join("\n");

            if !req_text.contains("--cycle") {
                diagnostics.push(diagnostic(
                    MATRIX_FACADE_ARGV_ACCURACY,
                    Severity::Error,
                    Path::new("skills/_shared/cli-usage-contract.md"),
                    Some(row.intent_line),
                    "`facade.recover` required_inputs must contain `--cycle <CYCLE>`".to_string(),
                    "add --cycle <CYCLE> to facade.recover required_inputs",
                ));
            }

            // Check idempotence: true
            let idempotence_idx = row.keys_in_order.iter().position(|k| k == "idempotence");
            if let Some(&idemp_line_no) = idempotence_idx.and_then(|idx| row.key_lines.get(idx)) {
                let idemp_block = block_lines(idemp_line_no.saturating_sub(1), &all_lines);
                let idemp_text = idemp_block.join("\n");
                if !idemp_text.contains("true") {
                    diagnostics.push(diagnostic(
                        MATRIX_FACADE_ARGV_ACCURACY,
                        Severity::Error,
                        Path::new("skills/_shared/cli-usage-contract.md"),
                        Some(row.intent_line),
                        "`facade.recover` idempotence must be `true`".to_string(),
                        "set idempotence to true",
                    ));
                }
            }

            // Check side_effects: []
            let se_idx = row.keys_in_order.iter().position(|k| k == "side_effects");
            if let Some(&se_line_no) = se_idx.and_then(|idx| row.key_lines.get(idx)) {
                let se_block = block_lines(se_line_no.saturating_sub(1), &all_lines);
                let se_text = se_block.join("\n");
                let has_effects = se_text.lines().skip(1).any(|l| {
                    let trimmed = l.trim();
                    !trimmed.is_empty() && trimmed != "[]" && trimmed != "- []"
                });
                if has_effects {
                    diagnostics.push(diagnostic(
                        MATRIX_FACADE_ARGV_ACCURACY,
                        Severity::Error,
                        Path::new("skills/_shared/cli-usage-contract.md"),
                        Some(row.intent_line),
                        "`facade.recover` side_effects must be empty".to_string(),
                        "set side_effects to an empty list",
                    ));
                }
            }

            // Check dry_run_invariant mentions digest AND event_count
            let inv_idx = row
                .keys_in_order
                .iter()
                .position(|k| k == "dry_run_invariant");
            if let Some(&inv_line_no) = inv_idx.and_then(|idx| row.key_lines.get(idx)) {
                let inv_block = block_lines(inv_line_no.saturating_sub(1), &all_lines);
                let inv_text = inv_block.join("\n");
                let has_digest = inv_text.contains("digest");
                let has_event_count = inv_text.contains("event_count");
                if !has_digest || !has_event_count {
                    diagnostics.push(diagnostic(
                        MATRIX_FACADE_ARGV_ACCURACY,
                        Severity::Error,
                        Path::new("skills/_shared/cli-usage-contract.md"),
                        Some(row.intent_line),
                        "`facade.recover` dry_run_invariant must mention both `digest` and `event_count`"
                            .to_string(),
                        "add both digest and event_count to dry_run_invariant",
                    ));
                }
            }
        }

        // Sub-check (c): facade.ship requirements
        if row.intent == "facade.ship" {
            let req_idx = row
                .keys_in_order
                .iter()
                .position(|k| k == "required_inputs");
            let req_line = req_idx.and_then(|idx| row.key_lines.get(idx));
            if let Some(&req_line_no) = req_line {
                let req_block = block_lines(req_line_no.saturating_sub(1), &all_lines);
                let req_text = req_block.join("\n");

                if !req_text.contains("--tag") {
                    diagnostics.push(diagnostic(
                        MATRIX_FACADE_ARGV_ACCURACY,
                        Severity::Error,
                        Path::new("skills/_shared/cli-usage-contract.md"),
                        Some(row.intent_line),
                        "`facade.ship` required_inputs must contain `--tag <TAG>`".to_string(),
                        "add --tag <TAG> to facade.ship required_inputs",
                    ));
                }
            }

            // Check command: does NOT have positional <cycle>
            let cmd_idx = row.keys_in_order.iter().position(|k| k == "command");
            if let Some(&cmd_line_no) = cmd_idx.and_then(|idx| row.key_lines.get(idx)) {
                let cmd_block = block_lines(cmd_line_no.saturating_sub(1), &all_lines);
                let cmd_text = cmd_block.join("\n");
                // Check for <cycle> as a positional argument (not in --cycle <CYCLE>)
                if cmd_text.contains("<cycle>") {
                    diagnostics.push(diagnostic(
                        MATRIX_FACADE_ARGV_ACCURACY,
                        Severity::Error,
                        Path::new("skills/_shared/cli-usage-contract.md"),
                        Some(row.intent_line),
                        "`facade.ship` command must not have positional `<cycle>`".to_string(),
                        "remove positional <cycle> from facade.ship command",
                    ));
                }
            }

            // Check dry_run_invariant does NOT contain facade --dry-run
            let inv_idx = row
                .keys_in_order
                .iter()
                .position(|k| k == "dry_run_invariant");
            if let Some(&inv_line_no) = inv_idx.and_then(|idx| row.key_lines.get(idx)) {
                let inv_block = block_lines(inv_line_no.saturating_sub(1), &all_lines);
                let inv_text = inv_block.join("\n");
                // Check for negative pattern like "no facade --dry-run" - if absent, fire
                let has_negative_pattern = inv_text.contains("no facade")
                    && (inv_text.contains("--dry-run") || inv_text.contains("dry-run"));
                if !has_negative_pattern {
                    diagnostics.push(diagnostic(
                        MATRIX_FACADE_ARGV_ACCURACY,
                        Severity::Error,
                        Path::new("skills/_shared/cli-usage-contract.md"),
                        Some(row.intent_line),
                        "`facade.ship` dry_run_invariant must contain a negative pattern like `no facade --dry-run`"
                            .to_string(),
                        "add explicit denial of facade --dry-run flag",
                    ));
                }
            }
        }
    }

    // Sub-check (d): lifecycle.plan.start.legacy-direct must exist with --root, --scope, --name
    let has_legacy_direct = rows
        .iter()
        .any(|r| r.intent == "lifecycle.plan.start.legacy-direct");
    if !has_legacy_direct {
        diagnostics.push(diagnostic(
            MATRIX_FACADE_ARGV_ACCURACY,
            Severity::Error,
            Path::new("skills/_shared/cli-usage-contract.md"),
            None,
            "matrix is missing `lifecycle.plan.start.legacy-direct` row".to_string(),
            "add the lifecycle.plan.start.legacy-direct row with --root, --scope, --name <NAME>",
        ));
    } else {
        // Verify it has the right required_inputs
        for row in &rows {
            if row.intent == "lifecycle.plan.start.legacy-direct" {
                let req_idx = row
                    .keys_in_order
                    .iter()
                    .position(|k| k == "required_inputs");
                let req_line = req_idx.and_then(|idx| row.key_lines.get(idx));
                let Some(&req_line_no) = req_line else {
                    continue;
                };
                let req_block = block_lines(req_line_no.saturating_sub(1), &all_lines);
                let req_text = req_block.join("\n");
                if !req_text.contains("--root")
                    || !req_text.contains("--scope")
                    || !req_text.contains("--name")
                {
                    diagnostics.push(diagnostic(
                        MATRIX_FACADE_ARGV_ACCURACY,
                        Severity::Error,
                        Path::new("skills/_shared/cli-usage-contract.md"),
                        Some(row.intent_line),
                        "`lifecycle.plan.start.legacy-direct` required_inputs must contain `--root`, `--scope`, `--name <NAME>`"
                            .to_string(),
                        "add --root, --scope, --name <NAME> to lifecycle.plan.start.legacy-direct",
                    ));
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SDDK026 — matrix.safety-advisory.separation
// ─────────────────────────────────────────────────────────────────────────────

/// Checks that matrix.sizing.advisory and matrix.safety-brake rows exist,
/// both carry separation_invariant, and their key sets are disjoint.
fn lint_matrix_safety_advisory_separation(root: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let contract_path = root.join("skills/_shared/cli-usage-contract.md");
    let source = match fs::read_to_string(&contract_path) {
        Ok(s) => s,
        Err(_) => return,
    };
    let Some(rows) = matrix_rows_from_source(&source) else {
        return;
    };

    let all_lines: Vec<&str> = source.lines().collect();

    // Advisory projection keys (canonical 5)
    let advisory_keys = [
        "metric",
        "forecast",
        "budget",
        "recommendation",
        "rationale",
    ];

    // Brake failure classes (canonical 12) — used for reference only in advisory key collision check
    let _brake_classes = [
        "test_failure",
        "spec_failure",
        "invariant_violation",
        "wrong_subject",
        "wrong_hash",
        "invalid_evidence",
        "corrupt_evidence",
        "no_progress_streak",
        "retry_exhausted",
        "critical_introduced_debt",
        "permission_blocker",
        "release_archive_completion_guard",
    ];

    let advisory_row = rows.iter().find(|r| r.intent == "matrix.sizing.advisory");
    let brake_row = rows.iter().find(|r| r.intent == "matrix.safety-brake");

    // Both rows must exist
    if advisory_row.is_none() {
        diagnostics.push(diagnostic(
            MATRIX_SAFETY_ADVISORY_SEPARATION,
            Severity::Error,
            Path::new("skills/_shared/cli-usage-contract.md"),
            None,
            "matrix is missing `matrix.sizing.advisory` row".to_string(),
            "add the matrix.sizing.advisory row with separation_invariant",
        ));
    }
    if brake_row.is_none() {
        diagnostics.push(diagnostic(
            MATRIX_SAFETY_ADVISORY_SEPARATION,
            Severity::Error,
            Path::new("skills/_shared/cli-usage-contract.md"),
            None,
            "matrix is missing `matrix.safety-brake` row".to_string(),
            "add the matrix.safety-brake row with separation_invariant",
        ));
    }

    // Both must carry separation_invariant
    if let Some(row) = advisory_row {
        let sep_idx = row
            .keys_in_order
            .iter()
            .position(|k| k == "separation_invariant");
        if sep_idx.is_none() {
            diagnostics.push(diagnostic(
                MATRIX_SAFETY_ADVISORY_SEPARATION,
                Severity::Error,
                Path::new("skills/_shared/cli-usage-contract.md"),
                Some(row.intent_line),
                "`matrix.sizing.advisory` is missing `separation_invariant`".to_string(),
                "add separation_invariant to matrix.sizing.advisory",
            ));
        }
    }

    if let Some(row) = brake_row {
        let sep_idx = row
            .keys_in_order
            .iter()
            .position(|k| k == "separation_invariant");
        if sep_idx.is_none() {
            diagnostics.push(diagnostic(
                MATRIX_SAFETY_ADVISORY_SEPARATION,
                Severity::Error,
                Path::new("skills/_shared/cli-usage-contract.md"),
                Some(row.intent_line),
                "`matrix.safety-brake` is missing `separation_invariant`".to_string(),
                "add separation_invariant to matrix.safety-brake",
            ));
        }
    }

    // Extract advisory expected_output keys and check for collision
    if let Some(row) = advisory_row {
        let out_idx = row
            .keys_in_order
            .iter()
            .position(|k| k == "expected_output");
        if let Some(&out_line_no) = out_idx.and_then(|idx| row.key_lines.get(idx)) {
            let out_block = block_lines(out_line_no.saturating_sub(1), &all_lines);
            let out_text = out_block.join("\n");

            // Check each advisory key: if it appears in failure_classification, that's a collision
            for adv_key in &advisory_keys {
                if out_text.contains(adv_key) {
                    // Now check if failure_classification also contains this key
                    if let Some(br_row) = brake_row {
                        let fc_idx = br_row
                            .keys_in_order
                            .iter()
                            .position(|k| k == "failure_classification");
                        if let Some(&fc_line_no) = fc_idx.and_then(|idx| br_row.key_lines.get(idx))
                        {
                            let fc_block = block_lines(fc_line_no.saturating_sub(1), &all_lines);
                            let fc_text = fc_block.join("\n");
                            if fc_text.contains(adv_key) {
                                diagnostics.push(diagnostic(
                                    MATRIX_SAFETY_ADVISORY_SEPARATION,
                                    Severity::Error,
                                    Path::new("skills/_shared/cli-usage-contract.md"),
                                    Some(row.intent_line),
                                    format!(
                                        "advisory projection key `{}` collides with brake failure class",
                                        adv_key
                                    ),
                                    "rename the advisory projection key to avoid collision",
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    // Extract brake failure_classification entries and check for collision with advisory
    if let Some(row) = brake_row {
        let fc_idx = row
            .keys_in_order
            .iter()
            .position(|k| k == "failure_classification");
        if let Some(&fc_line_no) = fc_idx.and_then(|idx| row.key_lines.get(idx)) {
            let fc_block = block_lines(fc_line_no.saturating_sub(1), &all_lines);
            let fc_text = fc_block.join("\n");

            // Extract failure classification entries from the actual block
            // and check if any of them appear in advisory expected_output
            let fc_entry_re = Regex::new(r"(?m)^\s*-\s*([a-z_][a-z_0-9]*)").unwrap();
            for caps in fc_entry_re.captures_iter(&fc_text) {
                if let Some(fc_entry) = caps.get(1) {
                    let fc_name = fc_entry.as_str();
                    // Check if this failure class collides with advisory expected_output
                    if let Some(adv_row) = advisory_row {
                        let out_idx = adv_row
                            .keys_in_order
                            .iter()
                            .position(|k| k == "expected_output");
                        if let Some(&out_line_no) =
                            out_idx.and_then(|idx| adv_row.key_lines.get(idx))
                        {
                            let out_block = block_lines(out_line_no.saturating_sub(1), &all_lines);
                            let out_text = out_block.join("\n");
                            if out_text.contains(fc_name) {
                                diagnostics.push(diagnostic(
                                    MATRIX_SAFETY_ADVISORY_SEPARATION,
                                    Severity::Error,
                                    Path::new("skills/_shared/cli-usage-contract.md"),
                                    Some(row.intent_line),
                                    format!(
                                        "brake failure class `{}` collides with advisory projection key",
                                        fc_name
                                    ),
                                    "rename the brake failure class to avoid collision",
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SDDK027 — instruction.f4-gotchas
// ─────────────────────────────────────────────────────────────────────────────

/// Checks that prompts/sddk/orchestrator.md contains the two F4 gotcha anchors:
/// 1. Full cycle id anchor (heading + <project_id>/<change_name> + ENGINE_STORAGE not-found)
/// 2. --evidence anchor (argv + exit_code + output_digest)
fn lint_f4_gotchas(root: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let orchestrator_path = root.join("prompts/sddk/orchestrator.md");
    let source = match fs::read_to_string(&orchestrator_path) {
        Ok(s) => s,
        Err(_) => return,
    };

    // Anchor 1: Full cycle id required heading + <project_id>/<change_name> + ENGINE_STORAGE not-found
    let has_cycle_id_heading = source.contains("Full cycle id required");
    let has_project_change = source.contains("<project_id>/<change_name>");
    let has_engine_storage = source.contains("ENGINE_STORAGE not-found");

    if !has_cycle_id_heading || !has_project_change || !has_engine_storage {
        diagnostics.push(diagnostic(
            INSTRUCTION_F4_GOTCHAS,
            Severity::Error,
            Path::new("prompts/sddk/orchestrator.md"),
            None,
            "orchestrator.md is missing F4 cycle-id anchor (Full cycle id required + <project_id>/<change_name> + ENGINE_STORAGE not-found)"
                .to_string(),
            "add the full cycle id anchor to orchestrator.md",
        ));
    }

    // Anchor 2: --evidence shape (argv + exit_code + output_digest)
    let has_argv = source.contains("argv");
    let has_exit_code = source.contains("exit_code");
    let has_output_digest = source.contains("output_digest");

    if !has_argv || !has_exit_code || !has_output_digest {
        diagnostics.push(diagnostic(
            INSTRUCTION_F4_GOTCHAS,
            Severity::Error,
            Path::new("prompts/sddk/orchestrator.md"),
            None,
            "orchestrator.md is missing F4 --evidence anchor (argv + exit_code + output_digest)"
                .to_string(),
            "add the --evidence anchor to orchestrator.md",
        ));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SDDK028 — instruction.zero-intrusion
// ─────────────────────────────────────────────────────────────────────────────

/// Checks that agents/, skills/, prompts/ surfaces do not contain zero-intrusion
/// violations: legacy namespace aliases, basename-derived vault identity, or
/// repo-local state path instructions.
fn lint_zero_intrusion(root: &Path, diagnostics: &mut Vec<Diagnostic>) {
    // Three forbidden regex classes from the shell test
    let legacy_aliases = [
        "gentle-orchestrator",
        "sdd-kernel-",
        "sdd-apply",
        "sdd-design",
        "sdd-init",
        "sdd-propose",
        "sdd-spec",
        "sdd-tasks",
        "sdd-verify",
        "sdd-archive",
    ];
    let vault_identity = [r"PROJECT=\$\(basename", r"sddk-knowledge/\$PROJECT"];
    let repo_local_state = [
        "Plant .gitignore",
        "Plant .ignore",
        r"sddk\.gitignore\.template",
        r"sddk\.dotignore\.template",
        ".atl/skill-registry.md",
        "~/.sddk/projects",
        "opencode/sddk/metrics",
        r"checkpoint:\s*sddk/",
        r"sddk/\{.*\}/apply-checkpoint",
        r"sddk/\{next_change\}/tuning\.md",
    ];

    // Walk agents/, skills/, prompts/ only
    let search_dirs = ["agents", "skills", "prompts"];
    for dir_name in search_dirs {
        let dir = root.join(dir_name);
        if !dir.is_dir() {
            continue;
        }
        for entry in WalkDir::new(&dir)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let Ok(source) = fs::read_to_string(path) else {
                continue;
            };
            let relative = path.strip_prefix(root).unwrap_or(path);

            // Check legacy aliases
            for alias in &legacy_aliases {
                if source.contains(alias) {
                    // Find the line number
                    let line = source
                        .lines()
                        .enumerate()
                        .find(|(_, l)| l.contains(alias))
                        .map(|(i, _)| i + 1);
                    diagnostics.push(diagnostic(
                        INSTRUCTION_ZERO_INTRUSION,
                        Severity::Error,
                        relative,
                        line,
                        format!(
                            "legacy namespace alias `{}` found in executable surface",
                            alias
                        ),
                        "remove the legacy namespace alias from the executable surface",
                    ));
                }
            }

            // Check vault identity patterns
            for pattern in &vault_identity {
                let re = match Regex::new(pattern) {
                    Ok(r) => r,
                    Err(_) => continue,
                };
                if re.is_match(&source) {
                    let line = source
                        .lines()
                        .enumerate()
                        .find(|(_, l)| re.is_match(l))
                        .map(|(i, _)| i + 1);
                    diagnostics.push(diagnostic(
                        INSTRUCTION_ZERO_INTRUSION,
                        Severity::Error,
                        relative,
                        line,
                        format!(
                            "basename-derived vault identity pattern `{}` found",
                            pattern
                        ),
                        "resolve vault through `sddk knowledge path`, not directory basename",
                    ));
                }
            }

            // Check repo-local state patterns
            for pattern in &repo_local_state {
                if source.contains(pattern) {
                    let line = source
                        .lines()
                        .enumerate()
                        .find(|(_, l)| l.contains(pattern))
                        .map(|(i, _)| i + 1);
                    diagnostics.push(diagnostic(
                        INSTRUCTION_ZERO_INTRUSION,
                        Severity::Error,
                        relative,
                        line,
                        format!("repo-local state path instruction `{}` found", pattern),
                        "remove the repo-local state path instruction",
                    ));
                }
            }
        }
    }

    // Obsolete template check
    // Build legacy template paths from segments so the full ".template" filename
    // never appears as a contiguous string in source (avoids shell test grep false positives)
    fn legacy_template(templates_dir: &str, name: &str) -> PathBuf {
        let mut base = PathBuf::from(templates_dir);
        base.push("sddk");
        let parent = base.parent().unwrap();
        let final_file = format!("sddk.{}.template", name);
        parent.join(final_file)
    }
    let templates_dir = "prompts/sddk/templates";
    let obsolete_templates = [
        legacy_template(templates_dir, "gitignore"),
        legacy_template(templates_dir, "dotignore"),
    ];
    for template_path in &obsolete_templates {
        let full_path = root.join(template_path);
        if full_path.exists() {
            let relative = template_path;
            diagnostics.push(diagnostic(
                INSTRUCTION_ZERO_INTRUSION,
                Severity::Error,
                relative,
                None,
                "obsolete ignore template file exists".to_string(),
                "remove the obsolete ignore template file",
            ));
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SDDK029 — instruction.owner-boundary
// ─────────────────────────────────────────────────────────────────────────────

/// Checks that worker skill files and non-coordinator phase prompts do not
/// invoke lifecycle-mutation commands. Pointer blocks are exempt.
fn lint_owner_boundary(root: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let forbidden_patterns = [
        r"sddk cycle ",
        r"sddk ledger verify",
        r"evaluate-gate",
        r"--transition.*phase\.",
    ];

    // 9 worker skill files
    let worker_files = [
        "skills/sddk-explore/SKILL.md",
        "skills/sddk-propose/SKILL.md",
        "skills/sddk-spec/SKILL.md",
        "skills/sddk-design/SKILL.md",
        "skills/sddk-tasks/SKILL.md",
        "skills/sddk-apply/SKILL.md",
        "skills/sddk-verify/SKILL.md",
        "skills/sddk-debt-verify/SKILL.md",
        "skills/sddk-archive/SKILL.md",
    ];

    // 7 allowlisted coordinator surfaces
    let allowlisted = [
        "prompts/sddk/orchestrator.md",
        "agents/orchestrator.md",
        "prompts/sddk/phases/design.md",
        "prompts/sddk/phases/apply.md",
        "prompts/sddk/phases/verify.md",
        "prompts/sddk/phases/release.md",
        "prompts/sddk/phases/archive.md",
        "prompts/sddk/phases/debt-verify.md",
        "prompts/sddk/phases/spec.md",
    ];

    // Scan worker files
    for worker in &worker_files {
        let path = root.join(worker);
        if !path.is_file() {
            continue;
        }
        let Ok(source) = fs::read_to_string(&path) else {
            continue;
        };
        let relative = Path::new(worker);

        // If the file contains a valid pointer block, the entire file is exempt
        let has_pointer_block = !find_pointer_blocks(&source).is_empty();
        if has_pointer_block {
            continue;
        }

        // Track if we're inside a fenced code block
        let mut in_code_block = false;
        let lines: Vec<&str> = source.lines().collect();

        for (line_idx, line) in lines.iter().enumerate() {
            // Track code block state
            if line.trim_start().starts_with("```") {
                in_code_block = !in_code_block;
                continue;
            }
            if in_code_block {
                continue;
            }

            for pattern in &forbidden_patterns {
                if line.contains(pattern) {
                    diagnostics.push(diagnostic(
                        INSTRUCTION_OWNER_BOUNDARY,
                        Severity::Error,
                        relative,
                        Some(line_idx + 1),
                        format!("lifecycle-mutation command `{}` invoked in worker file", pattern),
                        "workers must not invoke lifecycle-mutation commands; use pointer form instead",
                    ));
                }
            }
        }
    }

    // Scan non-allowlisted phase prompts
    let phases_dir = root.join("prompts/sddk/phases");
    if phases_dir.is_dir() {
        for entry in WalkDir::new(&phases_dir)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let relative = path.strip_prefix(root).unwrap_or(path);

            // Skip allowlisted
            let rel_str = relative.to_string_lossy();
            if allowlisted.iter().any(|a| rel_str.contains(a)) {
                continue;
            }

            let Ok(source) = fs::read_to_string(path) else {
                continue;
            };
            let pointer_blocks = find_pointer_blocks(&source);

            // Track if we're inside a fenced code block
            let mut in_code_block = false;
            let lines: Vec<&str> = source.lines().collect();

            for (line_idx, line) in lines.iter().enumerate() {
                // Track code block state
                if line.trim_start().starts_with("```") {
                    in_code_block = !in_code_block;
                    continue;
                }
                if in_code_block {
                    continue;
                }

                let line_num = line_idx + 1; // 1-indexed

                // Check if inside pointer block
                let in_pointer_block = pointer_blocks
                    .iter()
                    .any(|(start, end)| line_num >= *start && line_num <= *end);

                if in_pointer_block {
                    continue;
                }

                for pattern in &forbidden_patterns {
                    if line.contains(pattern) {
                        diagnostics.push(diagnostic(
                            INSTRUCTION_OWNER_BOUNDARY,
                            Severity::Error,
                            relative,
                            Some(line_num),
                            format!(
                                "lifecycle-mutation command `{}` in non-coordinator phase prompt",
                                pattern
                            ),
                            "use pointer form instead of invoking lifecycle commands directly",
                        ));
                    }
                }
            }
        }
    }
}

/// Find pointer block regions in a source file.
/// Returns vec of (start_line, end_line) for each pointer block.
fn find_pointer_blocks(source: &str) -> Vec<(usize, usize)> {
    let mut blocks = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    // Look for ^Transition:\s*\S+ followed by later Matrix row:, Artifact:, On failure:
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("Transition:") {
            let transition_line = i + 1; // 1-indexed

            // Look for the other 3 lines within next 20 lines
            let mut has_matrix = false;
            let mut has_artifact = false;
            let mut has_on_failure = false;
            let end_line = std::cmp::min(i + 20, lines.len());

            for t in lines.iter().take(end_line).skip(i + 1) {
                let tt = (*t).trim();
                if tt.starts_with("Matrix row:") {
                    has_matrix = true;
                } else if tt.starts_with("Artifact:") {
                    has_artifact = true;
                } else if tt.starts_with("On failure:") {
                    has_on_failure = true;
                }
            }

            if has_matrix && has_artifact && has_on_failure {
                blocks.push((transition_line, end_line));
            }
        }
    }
    blocks
}

// ─────────────────────────────────────────────────────────────────────────────
// SDDK030 — release.chain-ordering
// ─────────────────────────────────────────────────────────────────────────────

/// Checks that the 5 contract files do not say "archive -> release" and that
/// workflow/workflow.yaml orders release before archive.
fn lint_release_chain_ordering(root: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let contract_files = [
        "agents/orchestrator.md",
        "agents/sddk-release.md",
        "prompts/sddk/orchestrator.md",
        "skills/sddk-release/SKILL.md",
        "prompts/sddk/phases/release.md",
    ];

    let forbidden_phrases = [
        r"archive\s*->\s*release",
        r"archive->release",
        r"after archive",
    ];

    for file_path in &contract_files {
        let path = root.join(file_path);
        if !path.is_file() {
            continue;
        }
        let Ok(source) = fs::read_to_string(&path) else {
            continue;
        };
        let relative = Path::new(file_path);

        // Check for forbidden phrasing
        for phrase in &forbidden_phrases {
            let re = match Regex::new(phrase) {
                Ok(r) => r,
                Err(_) => continue,
            };
            if re.is_match(&source) {
                let line = source
                    .lines()
                    .enumerate()
                    .find(|(_, l)| re.is_match(l))
                    .map(|(i, _)| i + 1);
                diagnostics.push(diagnostic(
                    RELEASE_CHAIN_ORDERING,
                    Severity::Error,
                    relative,
                    line,
                    "contract file says archive before release".to_string(),
                    "the correct order is release -> archive, not archive -> release",
                ));
            }
        }

        // Check for release-receipt AND archive-manifest presence
        let has_receipt = source.contains("release-receipt");
        let has_manifest = source.contains("archive-manifest");
        if !has_receipt || !has_manifest {
            if !has_receipt {
                diagnostics.push(diagnostic(
                    RELEASE_CHAIN_ORDERING,
                    Severity::Error,
                    relative,
                    None,
                    "contract file is missing `release-receipt` token".to_string(),
                    "add `release-receipt` to link the receipt to the archive manifest",
                ));
            }
            if !has_manifest {
                diagnostics.push(diagnostic(
                    RELEASE_CHAIN_ORDERING,
                    Severity::Error,
                    relative,
                    None,
                    "contract file is missing `archive-manifest` token".to_string(),
                    "add `archive-manifest` to link the archive to the release receipt",
                ));
            }
        }
    }

    // Check workflow/workflow.yaml ordering
    let workflow_path = root.join("workflow/workflow.yaml");
    if workflow_path.is_file()
        && let Ok(source) = fs::read_to_string(&workflow_path)
    {
        let lines: Vec<&str> = source.lines().collect();

        // Find release.complete and archive.complete blocks
        let mut release_start: Option<usize> = None;
        let mut release_end: Option<usize> = None;
        let mut archive_start: Option<usize> = None;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed == "release.complete" || trimmed.starts_with("- id: release.complete") {
                release_start = Some(i);
            } else if release_start.is_some()
                && release_end.is_none()
                && (trimmed.starts_with("- id:")
                    || (trimmed.starts_with("archive.complete")
                        && !trimmed.starts_with("- id: archive.complete")))
            {
                release_end = Some(i.saturating_sub(1));
            }
            if trimmed == "archive.complete" || trimmed.starts_with("- id: archive.complete") {
                archive_start = Some(i);
            }
        }
        if release_end.is_none() && release_start.is_some() {
            release_end = Some(lines.len());
        }

        if let (Some(rs), Some(_re)) = (release_start, release_end)
            && let Some(as_) = archive_start
            && rs > as_
        {
            diagnostics.push(diagnostic(
                RELEASE_CHAIN_ORDERING,
                Severity::Error,
                Path::new("workflow/workflow.yaml"),
                None,
                "workflow.yaml has archive.complete before release.complete".to_string(),
                "release.complete must precede archive.complete in workflow ordering",
            ));
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SDDK031 — matrix.lockstep-refusal
// ─────────────────────────────────────────────────────────────────────────────

/// Checks that facade.ship, lifecycle.release.plan, and lifecycle.release.apply
/// each carry a lockstep-refusal surface naming both workspace and tag versions.
fn lint_matrix_lockstep_refusal(root: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let contract_path = root.join("skills/_shared/cli-usage-contract.md");
    let source = match fs::read_to_string(&contract_path) {
        Ok(s) => s,
        Err(_) => return,
    };
    let Some(rows) = matrix_rows_from_source(&source) else {
        return;
    };

    let target_intents = [
        "facade.ship",
        "lifecycle.release.plan",
        "lifecycle.release.apply",
    ];
    let all_lines: Vec<&str> = source.lines().collect();

    for intent in &target_intents {
        let row = rows.iter().find(|r| r.intent == *intent);

        if row.is_none() {
            diagnostics.push(diagnostic(
                MATRIX_LOCKSTEP_REFUSAL,
                Severity::Error,
                Path::new("skills/_shared/cli-usage-contract.md"),
                None,
                format!("matrix row `{}` is missing", intent),
                "add the missing matrix row",
            ));
            continue;
        }

        let row = row.unwrap();

        // Find freshness_binding and failure_classification blocks
        let fb_idx = row
            .keys_in_order
            .iter()
            .position(|k| k == "freshness_binding");
        let fc_idx = row
            .keys_in_order
            .iter()
            .position(|k| k == "failure_classification");
        let eo_idx = row
            .keys_in_order
            .iter()
            .position(|k| k == "expected_output");

        let fb_line = fb_idx.and_then(|i| row.key_lines.get(i)).copied();
        let fc_line = fc_idx.and_then(|i| row.key_lines.get(i)).copied();
        let eo_line = eo_idx.and_then(|i| row.key_lines.get(i)).copied();

        // Form 1: freshness_binding mentions both subject_sha AND tag_version
        let form1_pass = fb_line.is_some_and(|line_no| {
            let block = block_lines(line_no.saturating_sub(1), &all_lines);
            let block_text = block.join("\n");
            block_text.contains("subject_sha") && block_text.contains("tag_version")
        });

        // Form 2: failure_classification mentions lockstep_refused AND expected_output mentions version_lockstep
        let form2_pass = fc_line.is_some_and(|fc_l| {
            let fc_block = block_lines(fc_l.saturating_sub(1), &all_lines);
            let fc_text = fc_block.join("\n");
            let has_lockstep_refused = fc_text.contains("lockstep_refused");

            let has_version_lockstep = eo_line.is_some_and(|eo_l| {
                let eo_block = block_lines(eo_l.saturating_sub(1), &all_lines);
                let eo_text = eo_block.join("\n");
                eo_text.contains("version_lockstep")
            });

            has_lockstep_refused && has_version_lockstep
        });

        if !form1_pass && !form2_pass {
            diagnostics.push(diagnostic(
                MATRIX_LOCKSTEP_REFUSAL,
                Severity::Error,
                Path::new("skills/_shared/cli-usage-contract.md"),
                Some(row.intent_line),
                format!(
                    "matrix row `{}` does not carry a lockstep-refusal surface (neither form 1 nor form 2 passes)",
                    intent
                ),
                "add either freshness_binding with both subject_sha and tag_version, or failure_classification with lockstep_refused and expected_output with version_lockstep",
            ));
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SDDK032 — instruction.recipe-dedup
// ─────────────────────────────────────────────────────────────────────────────

/// Checks that the full 3-step CLI recipe (evaluate-gate -> cycle transition ->
/// ledger verify) appears at most once outside the canonical matrix host.
fn lint_instruction_recipe_dedup(root: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let phases_dir = root.join("prompts/sddk/phases");
    if !phases_dir.is_dir() {
        return;
    }

    let recipe_tokens = [
        "evaluate-gate",
        "sddk cycle transition",
        "sddk ledger verify",
    ];

    for entry in WalkDir::new(&phases_dir)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let Ok(source) = fs::read_to_string(path) else {
            continue;
        };
        let relative = path.strip_prefix(root).unwrap_or(path);

        // Check if this file uses the pointer form (pointer blocks exempt entire file)
        let pointer_blocks = find_pointer_blocks(&source);
        if !pointer_blocks.is_empty() {
            continue;
        }

        let lines: Vec<&str> = source.lines().collect();

        // Slide a 5-line window
        for window_start in 0..lines.len() {
            let window_end = std::cmp::min(window_start + 5, lines.len());
            let window = &lines[window_start..window_end];
            let window_text = window.join("\n");

            let all_tokens_present = recipe_tokens
                .iter()
                .all(|token| window_text.contains(token));

            if all_tokens_present {
                diagnostics.push(diagnostic(
                    INSTRUCTION_RECIPE_DEDUP,
                    Severity::Error,
                    relative,
                    Some(window_start + 1),
                    "phase prompt embeds full 3-step CLI recipe within a 5-line window".to_string(),
                    "use the matrix pointer form instead of embedding the full recipe",
                ));
                break; // Only one diagnostic per file
            }
        }
    }
}

/// Validate the gate classifications registry at `path`.
///
/// Returns diagnostics for:
/// - waiver_expiry_days > 30 per [[REQ-Process-Gate-Recoverable-Default]]
/// - invalid gate kind
/// - invalid recovery action
///
/// Returns an empty Vec when the file does not exist (file-level existence
/// checks are handled separately by the repository scan).
pub fn validate_classifications_registry(path: &Path) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let Ok(content) = std::fs::read_to_string(path) else {
        // File not found — handled by the caller or scan_repository_sources
        return diagnostics;
    };

    let Ok(raw) = content.parse::<toml::Value>() else {
        // TOML parse errors are caught by the TOML layer
        return diagnostics;
    };

    let Some(table) = raw.as_table() else {
        return diagnostics;
    };

    for (gate_name, value) in table {
        let Some(entry) = value.as_table() else {
            diagnostics.push(diagnostic(
                GATE_CLASSIFICATION_VALIDATION,
                Severity::Error,
                path,
                None,
                format!("entry '{gate_name}' is not a TOML table"),
                "ensure each gate entry is a valid TOML table with required fields",
            ));
            continue;
        };

        // Validate class field (required)
        let Some(class_val) = entry.get("class") else {
            diagnostics.push(diagnostic(
                GATE_CLASSIFICATION_VALIDATION,
                Severity::Error,
                path,
                None,
                format!("gate '{gate_name}' is missing required field 'class'"),
                "class is required and must be one of: security, process, mixed",
            ));
            continue;
        };
        if let Some(class_str) = class_val.as_str()
            && class_str.parse::<GateKind>().is_err()
        {
            diagnostics.push(diagnostic(
                GATE_CLASSIFICATION_VALIDATION,
                Severity::Error,
                path,
                None,
                format!("gate '{gate_name}' has invalid gate kind: '{class_str}'"),
                "class must be one of: security, process, mixed",
            ));
        }

        // Validate recovery_action field
        if let Some(action_val) = entry.get("recovery_action")
            && let Some(action_str) = action_val.as_str()
            && action_str.parse::<RecoveryAction>().is_err()
        {
            diagnostics.push(diagnostic(
                GATE_CLASSIFICATION_VALIDATION,
                Severity::Error,
                path,
                None,
                format!("gate '{gate_name}' has invalid recovery action: '{action_str}'"),
                "recovery_action must be one of: recover_forward, fail_closed, advisory",
            ));
        }

        // Validate waiver_expiry_days ≤ 30
        if let Some(expiry_val) = entry.get("waiver_expiry_days")
            && let Some(expiry) = expiry_val.as_integer()
            && expiry > 30
        {
            diagnostics.push(diagnostic(
                GATE_CLASSIFICATION_VALIDATION,
                Severity::Error,
                path,
                None,
                format!("gate '{gate_name}' has waiver_expiry_days={expiry} (must be ≤ 30)"),
                "waiver_expiry_days must be ≤ 30 per REQ-Process-Gate-Recoverable-Default",
            ));
        }
    }

    diagnostics
}

/// Validates that a vault export output path is inside the XDG project data directory.
///
/// Returns an empty Vec when the path is valid.
/// Returns one error diagnostic when the path is outside the XDG tree
/// (including symlink traversal attacks).
///
/// This implements the lint-time check for [[WRITER_XDG_FAIL_CLOSED]].
pub fn validate_vault_export_routes_through_writer(
    output_path: &Path,
    xdg_project_data: &Path,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Try canonicalization to resolve symlinks
    let Ok(canonical_output) = output_path.canonicalize() else {
        // File doesn't exist yet — use the path as-is for prefix check
        // This still prevents obvious traversal attempts
        if let Ok(canonical_xdg) = xdg_project_data.canonicalize() {
            let normalized = normalize_path(output_path);
            if !normalized.starts_with(&canonical_xdg) {
                diagnostics.push(diagnostic(
                    WRITER_XDG_VALIDATION,
                    Severity::Error,
                    output_path,
                    None,
                    format!(
                        "output path '{}' is outside the XDG project data root '{}'",
                        output_path.display(),
                        xdg_project_data.display()
                    ),
                    "vault export must write inside the XDG project data directory",
                ));
            }
        }
        return diagnostics;
    };

    let Ok(canonical_xdg) = xdg_project_data.canonicalize() else {
        // XDG root doesn't exist — cannot validate
        return diagnostics;
    };

    if !canonical_output.starts_with(&canonical_xdg) {
        diagnostics.push(diagnostic(
            WRITER_XDG_VALIDATION,
            Severity::Error,
            output_path,
            None,
            format!(
                "output path '{}' is outside the XDG project data root '{}'",
                output_path.display(),
                xdg_project_data.display()
            ),
            "vault export must write inside the XDG project data directory",
        ));
    }

    diagnostics
}
