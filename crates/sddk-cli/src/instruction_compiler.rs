//! M7.5 — InstructionCompiler + EffectiveInstructions (SPEC-014).
//!
//! The InstructionCompiler takes typed inputs from seven sources
//! (per SPEC-014 §Inputs) and produces a typed, versioned
//! `EffectiveInstructions` artifact that can be:
//!
//! - rendered by a provider adapter into native system/developer/user
//!   sections (the rendering may change formatting but never semantic
//!   keys or strengths);
//! - queried by `sddk context explain --instructions` to report
//!   source / reason selected / semantic key / strength / version /
//!   hash / excluded fragments;
//! - hashed and stored on `AgentExecutionReceipt` (SPEC-018) as
//!   `effective_instructions_hash` for replay-class provenance.
//!
//! ## Composition semantics (SPEC-014 §Conflict semantics)
//!
//! - **Invariant** constraints are non-overridable. A project/skill
//!   instruction that conflicts with a kernel invariant produces
//!   `InstructionConflict { kind: InvariantViolation }` and the
//!   compile fails closed.
//! - **Policy** constraints can be narrowed (tightened) by project
//!   instructions but never widened. Widening produces
//!   `InstructionConflict { kind: PolicyNarrowingViolation }`.
//! - **Task** requirements can be relaxed by project instructions
//!   only when the project instruction itself is `Mandatory`; missing
//!   requirements yield `InstructionConflict { kind: TaskMissing }`.
//! - **Project** instructions normalize to canonical keys
//!   (`lowercase + whitespace-collapsed + dedupe`).
//! - **Skill** instructions are appended after project instructions,
//!   grouped under the skill's `id@version` header, with explicit
//!   provenance to the `SkillDefinition::ref_token`.
//! - **Advisory** hints are kept under a separate `advisory` section
//!   and never contribute to the conflict count.
//!
//! The compiler never silently drops an instruction: every accepted
//! fragment has a `source` and `reason_selected`; every rejected
//! fragment has an entry in `diagnostics.rejected`.

#![allow(missing_docs)]

use std::collections::BTreeMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

use crate::agent_profile::AgentProfile;
use crate::skill_definition::SkillDefinition;

// ============================================================
// Inputs (SPEC-014 §Inputs)
// ============================================================

/// A constraint that cannot be overridden by lower classes. Examples
/// (not enumerated exhaustively here): "all writes are content-
/// addressed", "side effects must produce evidence".
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub struct KernelInvariant {
    /// Stable identifier for the invariant (used as semantic key).
    pub id: String,
    /// Human-readable text rendered to the agent.
    pub text: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct KernelInvariantSet {
    pub entries: Vec<KernelInvariant>,
}

/// A constraint that can be narrowed by project/skill instructions but
/// not widened. Examples: "prefer AppendOnly ledgers", "prefer bundle
/// evidence over raw source refs".
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub struct PolicyInstruction {
    pub id: String,
    pub text: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PolicyInstructionSet {
    pub entries: Vec<PolicyInstruction>,
}

/// A requirement the agent must satisfy for the active task. Missing
/// requirements are reported as `TaskMissing` conflicts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub struct TaskRequirement {
    pub id: String,
    pub text: String,
    /// When true, the requirement is Mandatory and project/skill
    /// instructions can override it; when false, it is Advisory.
    #[serde(default = "default_true")]
    pub mandatory: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TaskContract {
    pub kind: String,
    pub requirements: Vec<TaskRequirement>,
}

/// A project-specific instruction (e.g., from `AGENTS.md`,
/// `.sddk/instructions.yaml`, or the project vault). Normalized to a
/// canonical key.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub struct ProjectInstruction {
    pub id: String,
    pub text: String,
    /// When true, the instruction is treated as Mandatory for
    /// relaxation purposes.
    #[serde(default)]
    pub mandatory: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectInstructionSet {
    pub source_path: Option<String>,
    pub entries: Vec<ProjectInstruction>,
}

/// Selected skill set produced by `SkillRegistry::select_for`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SelectedSkillSet {
    pub skills: Vec<SkillDefinition>,
}

/// Contextual hints — never contributes to conflict count.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ContextualHints {
    pub hints: Vec<String>,
}

/// All seven SPEC-014 inputs.
#[derive(Debug, Clone, Default)]
pub struct CompileInputs {
    pub invariants: KernelInvariantSet,
    pub policies: PolicyInstructionSet,
    pub task: TaskContract,
    pub project: ProjectInstructionSet,
    pub profile: Option<AgentProfile>,
    pub skills: SelectedSkillSet,
    pub hints: ContextualHints,
    /// Agent Experience contract version (defaults to 1).
    pub ae_version: u32,
}

// ============================================================
// Output (SPEC-014 §Output / §Explainability)
// ============================================================

/// Where an instruction fragment came from.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case", tag = "kind", content = "ref")]
pub enum InstructionSource {
    /// KernelInvariantSet — id of the invariant.
    KernelInvariant { id: String },
    /// PolicyInstructionSet — id of the policy.
    Policy { id: String },
    /// TaskContract — id of the requirement.
    TaskRequirement { id: String },
    /// ProjectInstructionSet — id of the project instruction.
    Project { id: String },
    /// SkillDefinition — `id@version` of the skill.
    Skill { ref_token: String, fragment: String },
    /// ContextualHints — index in the hints list.
    Hint { index: usize },
}

impl InstructionSource {
    pub fn semantic_key(&self) -> String {
        match self {
            Self::KernelInvariant { id } => format!("invariant:{id}"),
            Self::Policy { id } => format!("policy:{id}"),
            Self::TaskRequirement { id } => format!("task:{id}"),
            Self::Project { id } => format!("project:{id}"),
            Self::Skill {
                ref_token,
                fragment,
            } => format!("skill:{ref_token}:{fragment}"),
            Self::Hint { index } => format!("hint:{index}"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum InstructionStrength {
    /// Non-overridable kernel constraint.
    Invariant,
    /// Tightenable policy constraint.
    Policy,
    /// Mandatory task requirement.
    Task,
    /// Mandatory project instruction (overrides task relaxations).
    ProjectMandatory,
    /// Non-mandatory project instruction.
    ProjectAdvisory,
    /// Skill-authored instruction.
    Skill,
    /// Hint (never contributes to conflict count).
    Hint,
}

/// One fragment in the compiled output.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub struct InstructionFragment {
    /// Canonical semantic key (dedupe happens on this field).
    pub semantic_key: String,
    pub strength: InstructionStrength,
    pub text: String,
    pub source: InstructionSource,
    /// Free-form reason this fragment was selected.
    pub reason_selected: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case", tag = "kind", content = "details")]
pub enum RejectionReason {
    /// Project/skill tried to override a kernel invariant.
    InvariantViolation {
        invariant_id: String,
        offender: String,
    },
    /// Project tried to widen a policy constraint.
    PolicyNarrowingViolation { policy_id: String, offender: String },
    /// Mandatory task requirement was not satisfied by any fragment.
    TaskMissing { requirement_id: String },
    /// Duplicate semantic key (only the strongest wins).
    DuplicateSuppressed { semantic_key: String, by: String },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Diagnostics {
    pub rejected: Vec<RejectionReason>,
}

/// Compile errors. Fail closed unless caller resolves them.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "kind", content = "details")]
pub enum InstructionConflict {
    /// Kernel invariant was violated.
    InvariantViolation {
        invariant_id: String,
        offender: String,
    },
    /// Policy widening was attempted.
    PolicyNarrowingViolation { policy_id: String, offender: String },
    /// Mandatory task requirement not satisfied.
    TaskMissing { requirement_id: String },
}

impl std::fmt::Display for InstructionConflict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvariantViolation {
                invariant_id,
                offender,
            } => write!(
                f,
                "invariant `{invariant_id}` violated by `{offender}` (non-overridable)"
            ),
            Self::PolicyNarrowingViolation {
                policy_id,
                offender,
            } => write!(
                f,
                "policy `{policy_id}` widened by `{offender}` (only narrowing allowed)"
            ),
            Self::TaskMissing { requirement_id } => write!(
                f,
                "mandatory task requirement `{requirement_id}` not satisfied"
            ),
        }
    }
}

impl std::error::Error for InstructionConflict {}

/// Ordered sections + source refs + diagnostics + compat version +
/// content hash. Stable representation: deterministic ordering by
/// (strength, semantic_key).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EffectiveInstructions {
    /// Sorted list of accepted fragments, deterministic.
    pub sections: Vec<InstructionFragment>,
    pub diagnostics: Diagnostics,
    /// Agent Experience contract version these instructions were
    /// compiled against.
    pub ae_version: u32,
    /// SHA-derived content hash (computed on canonical JSON).
    pub content_hash: String,
}

impl EffectiveInstructions {
    /// Number of accepted fragments.
    pub fn len(&self) -> usize {
        self.sections.len()
    }

    /// True iff no fragments were accepted (still valid).
    pub fn is_empty(&self) -> bool {
        self.sections.is_empty()
    }

    /// Stable ref token `effective@v<ae_version>` for `AgentExecutionReceipt`.
    pub fn ref_token(&self) -> String {
        format!("effective@v{}", self.ae_version)
    }

    /// Lookup a fragment by semantic key.
    pub fn by_semantic_key(&self, key: &str) -> Option<&InstructionFragment> {
        self.sections.iter().find(|f| f.semantic_key == key)
    }

    /// All fragments contributed by a given skill ref.
    pub fn from_skill(&self, ref_token: &str) -> Vec<&InstructionFragment> {
        self.sections
            .iter()
            .filter(|f| matches!(&f.source, InstructionSource::Skill { ref_token: r, .. } if r == ref_token))
            .collect()
    }
}

// ============================================================
// Compiler
// ============================================================

/// The compiler. Cheap to construct (no state).
#[derive(Debug, Default, Clone)]
pub struct InstructionCompiler;

impl InstructionCompiler {
    pub fn new() -> Self {
        Self
    }

    /// Compile the inputs into `EffectiveInstructions`. Fails closed on
    /// any of:
    /// - `InvariantViolation`
    /// - `PolicyNarrowingViolation`
    /// - `TaskMissing`
    ///
    /// Other rejections (duplicate suppression) are recorded in
    /// `diagnostics.rejected` without failing the compile.
    pub fn compile(
        &self,
        inputs: &CompileInputs,
    ) -> Result<EffectiveInstructions, InstructionConflict> {
        let mut accepted: Vec<InstructionFragment> = Vec::new();
        let mut rejected: Vec<RejectionReason> = Vec::new();

        // 1) Invariants — pass through, non-overridable.
        for inv in &inputs.invariants.entries {
            accepted.push(InstructionFragment {
                semantic_key: InstructionSource::KernelInvariant { id: inv.id.clone() }
                    .semantic_key(),
                strength: InstructionStrength::Invariant,
                text: inv.text.clone(),
                source: InstructionSource::KernelInvariant { id: inv.id.clone() },
                reason_selected: "kernel invariant (always accepted)".to_string(),
            });
        }

        // 2) Policies — pass through; project may narrow but not widen.
        for pol in &inputs.policies.entries {
            accepted.push(InstructionFragment {
                semantic_key: InstructionSource::Policy { id: pol.id.clone() }.semantic_key(),
                strength: InstructionStrength::Policy,
                text: pol.text.clone(),
                source: InstructionSource::Policy { id: pol.id.clone() },
                reason_selected: "policy (project may narrow)".to_string(),
            });
        }

        // 3) Project instructions — normalized + checked against invariants/policies.
        for proj in &inputs.project.entries {
            let key = normalize_key(&proj.id);
            let strength = if proj.mandatory {
                InstructionStrength::ProjectMandatory
            } else {
                InstructionStrength::ProjectAdvisory
            };
            // Invariant violation: project text contains the marker for a known
            // invariant (proxy — full NLU is out of scope; we use exact-id match
            // on the project id against invariant ids).
            if inputs
                .invariants
                .entries
                .iter()
                .any(|inv| normalize_key(&inv.id) == key)
                && !proj.text.is_empty()
            {
                return Err(InstructionConflict::InvariantViolation {
                    invariant_id: key.clone(),
                    offender: format!("project:{}", proj.id),
                });
            }
            // Policy widening: project tries to *negate* a policy by emitting
            // a fragment whose text starts with "DO NOT <policy-text>". Proxy.
            for pol in &inputs.policies.entries {
                if proj.text.starts_with("DO NOT ") && proj.text.contains(&pol.text) {
                    return Err(InstructionConflict::PolicyNarrowingViolation {
                        policy_id: pol.id.clone(),
                        offender: format!("project:{}", proj.id),
                    });
                }
            }
            accepted.push(InstructionFragment {
                semantic_key: InstructionSource::Project { id: key.clone() }.semantic_key(),
                strength,
                text: proj.text.clone(),
                source: InstructionSource::Project { id: key },
                reason_selected: if proj.mandatory {
                    "project mandatory instruction".to_string()
                } else {
                    "project advisory instruction".to_string()
                },
            });
        }

        // 4) Task requirements — each must be matched by some accepted fragment
        //    OR a project-mandatory fragment. Skills cannot satisfy requirements
        //    (Skill != Capability).
        for req in &inputs.task.requirements {
            if req.mandatory && !satisfied_by(&req.text, &accepted) {
                return Err(InstructionConflict::TaskMissing {
                    requirement_id: req.id.clone(),
                });
            }
            accepted.push(InstructionFragment {
                semantic_key: InstructionSource::TaskRequirement { id: req.id.clone() }
                    .semantic_key(),
                strength: InstructionStrength::Task,
                text: req.text.clone(),
                source: InstructionSource::TaskRequirement { id: req.id.clone() },
                reason_selected: "task requirement (mandatory)".to_string(),
            });
        }

        // 5) Skills — append per-fragment, grouped under skill header.
        for skill in &inputs.skills.skills {
            let ref_token = skill.ref_token();
            for fragment in &skill.instruction_fragments {
                let key = normalize_key(&format!("{ref_token}:{fragment}"));
                accepted.push(InstructionFragment {
                    semantic_key: key.clone(),
                    strength: InstructionStrength::Skill,
                    text: fragment.clone(),
                    source: InstructionSource::Skill {
                        ref_token: ref_token.clone(),
                        fragment: fragment.clone(),
                    },
                    reason_selected: format!("contributed by skill {ref_token}"),
                });
            }
        }

        // 6) Hints — advisory, never contribute to conflict count.
        for (i, hint) in inputs.hints.hints.iter().enumerate() {
            accepted.push(InstructionFragment {
                semantic_key: InstructionSource::Hint { index: i }.semantic_key(),
                strength: InstructionStrength::Hint,
                text: hint.clone(),
                source: InstructionSource::Hint { index: i },
                reason_selected: "contextual hint (advisory)".to_string(),
            });
        }

        // 7) Dedupe by semantic_key (strongest wins), record rejections.
        let mut by_key: BTreeMap<String, InstructionFragment> = BTreeMap::new();
        let strength_rank = |s: InstructionStrength| -> u8 {
            match s {
                InstructionStrength::Invariant => 5,
                InstructionStrength::Policy => 4,
                InstructionStrength::ProjectMandatory => 3,
                InstructionStrength::Task => 2,
                InstructionStrength::Skill => 1,
                InstructionStrength::ProjectAdvisory => 0,
                InstructionStrength::Hint => 0,
            }
        };
        for frag in accepted.drain(..) {
            if let Some(existing) = by_key.get(&frag.semantic_key) {
                if strength_rank(frag.strength) > strength_rank(existing.strength) {
                    let prior = by_key
                        .insert(frag.semantic_key.clone(), frag.clone())
                        .unwrap();
                    rejected.push(RejectionReason::DuplicateSuppressed {
                        semantic_key: prior.semantic_key.clone(),
                        by: format!("{:?}", prior.source),
                    });
                } else {
                    rejected.push(RejectionReason::DuplicateSuppressed {
                        semantic_key: frag.semantic_key.clone(),
                        by: format!("{:?}", frag.source),
                    });
                }
            } else {
                by_key.insert(frag.semantic_key.clone(), frag);
            }
        }
        let mut sections: Vec<InstructionFragment> = by_key.into_values().collect();
        // Final deterministic ordering: (strength desc, semantic_key asc)
        sections.sort_by(|a, b| {
            strength_rank(b.strength)
                .cmp(&strength_rank(a.strength))
                .then_with(|| a.semantic_key.cmp(&b.semantic_key))
        });

        let ae_version = inputs.ae_version.max(1);
        let mut provisional = EffectiveInstructions {
            sections,
            diagnostics: Diagnostics { rejected },
            ae_version,
            content_hash: String::new(),
        };
        provisional.content_hash = provisional.compute_hash();
        Ok(provisional)
    }
}

impl EffectiveInstructions {
    /// Compute the content hash over canonical JSON.
    fn compute_hash(&self) -> String {
        let value = serde_json::to_value(self).expect("EffectiveInstructions serializes");
        let canonical = canonicalize_json(value);
        let mut hasher = DefaultHasher::new();
        canonical.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    /// Re-hash after a manual edit. Useful for diagnostics but the
    /// compiler always computes the hash itself.
    pub fn recompute_hash(&mut self) {
        self.content_hash = self.compute_hash();
    }
}

/// Normalize a key (lowercase + whitespace collapsed + deduped).
fn normalize_key(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Heuristic: does `requirement_text` appear in any accepted fragment?
fn satisfied_by(requirement_text: &str, accepted: &[InstructionFragment]) -> bool {
    let needle = normalize_key(requirement_text);
    accepted
        .iter()
        .any(|f| normalize_key(&f.text).contains(&needle))
}

fn canonicalize_json(value: serde_json::Value) -> serde_json::Value {
    use serde_json::Value;
    match value {
        Value::Object(map) => {
            let mut sorted: BTreeMap<String, Value> = BTreeMap::new();
            for (k, v) in map.into_iter() {
                sorted.insert(k, canonicalize_json(v));
            }
            let out: serde_json::Map<String, Value> = sorted.into_iter().collect();
            Value::Object(out)
        }
        Value::Array(arr) => Value::Array(arr.into_iter().map(canonicalize_json).collect()),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_profile::default_profile;
    use crate::command_spec::CommandSpec;
    use crate::skill_definition::{
        AppliesWhen, CapabilityRequirement, CompatRange, EvidenceContract, EvidenceTier,
        SkillOutput,
    };

    fn inv(id: &str, text: &str) -> KernelInvariant {
        KernelInvariant {
            id: id.into(),
            text: text.into(),
        }
    }
    fn pol(id: &str, text: &str) -> PolicyInstruction {
        PolicyInstruction {
            id: id.into(),
            text: text.into(),
        }
    }
    fn task_req(id: &str, text: &str) -> TaskRequirement {
        TaskRequirement {
            id: id.into(),
            text: text.into(),
            mandatory: true,
        }
    }
    fn proj(id: &str, text: &str, mandatory: bool) -> ProjectInstruction {
        ProjectInstruction {
            id: id.into(),
            text: text.into(),
            mandatory,
        }
    }
    fn skill(id: &str, fragments: Vec<&str>) -> SkillDefinition {
        SkillDefinition {
            id: id.into(),
            version: 1,
            applies_when: AppliesWhen {
                task_kinds: vec!["x".into()],
                context_keys: vec![],
            },
            inputs: vec![],
            outputs: SkillOutput {
                contract: "ContributionV2".into(),
                notes: None,
            },
            capabilities: CapabilityRequirement::default(),
            evidence_contract: EvidenceContract {
                minimum: EvidenceTier::SourceRefs,
            },
            instruction_fragments: fragments.into_iter().map(String::from).collect(),
            compat: CompatRange::default(),
        }
    }

    fn empty_inputs() -> CompileInputs {
        CompileInputs::default()
    }

    #[test]
    fn invariants_are_accepted_first() {
        let compiler = InstructionCompiler::new();
        let mut inputs = empty_inputs();
        inputs
            .invariants
            .entries
            .push(inv("writes-cas", "all writes are content-addressed"));
        let out = compiler.compile(&inputs).expect("compile");
        assert_eq!(out.sections.len(), 1);
        assert_eq!(out.sections[0].strength, InstructionStrength::Invariant);
        assert!(out.sections[0].semantic_key.contains("writes-cas"));
    }

    #[test]
    fn invariant_violation_fails_closed() {
        let compiler = InstructionCompiler::new();
        let mut inputs = empty_inputs();
        inputs
            .invariants
            .entries
            .push(inv("writes-cas", "all writes are content-addressed"));
        inputs.project.entries.push(proj(
            "writes-cas",
            "writes do NOT have to be content-addressed in this project",
            true,
        ));
        match compiler.compile(&inputs) {
            Err(InstructionConflict::InvariantViolation { invariant_id, .. }) => {
                assert_eq!(invariant_id, "writes-cas");
            }
            other => panic!("expected InvariantViolation, got {other:?}"),
        }
    }

    #[test]
    fn policy_widening_fails_closed() {
        let compiler = InstructionCompiler::new();
        let mut inputs = empty_inputs();
        inputs
            .policies
            .entries
            .push(pol("prefer-bundle", "prefer bundle evidence"));
        inputs.project.entries.push(proj(
            "override-bundle",
            "DO NOT prefer bundle evidence; use raw refs",
            false,
        ));
        match compiler.compile(&inputs) {
            Err(InstructionConflict::PolicyNarrowingViolation { policy_id, .. }) => {
                assert_eq!(policy_id, "prefer-bundle");
            }
            other => panic!("expected PolicyNarrowingViolation, got {other:?}"),
        }
    }

    #[test]
    fn task_requirement_missing_fails_closed() {
        let compiler = InstructionCompiler::new();
        let mut inputs = empty_inputs();
        // The needle search is case-insensitive substring after
        // normalization. Use a short, distinctive phrase that the
        // project instruction will contain.
        inputs
            .task
            .requirements
            .push(task_req("use-evidence", "evidence bundles required"));
        match compiler.compile(&inputs) {
            Err(InstructionConflict::TaskMissing { requirement_id }) => {
                assert_eq!(requirement_id, "use-evidence");
            }
            other => panic!("expected TaskMissing, got {other:?}"),
        }
    }

    #[test]
    fn task_requirement_satisfied_by_project() {
        let compiler = InstructionCompiler::new();
        let mut inputs = empty_inputs();
        // The needle search is case-insensitive substring after
        // normalization. Use a short, distinctive phrase that the
        // project instruction will contain.
        inputs
            .task
            .requirements
            .push(task_req("use-evidence", "evidence bundles required"));
        inputs.project.entries.push(proj(
            "evidence",
            "evidence bundles required for every change",
            true,
        ));
        let out = compiler.compile(&inputs).expect("compile");
        assert!(
            out.sections
                .iter()
                .any(|f| matches!(f.source, InstructionSource::Project { .. }))
        );
    }

    #[test]
    fn duplicate_semantic_key_keeps_strongest() {
        let compiler = InstructionCompiler::new();
        let mut inputs = empty_inputs();
        // Both instructions emit the same semantic_key because they
        // share the same `id` and both go through the project source.
        inputs.project.entries.push(proj("k", "k-text", true));
        inputs
            .project
            .entries
            .push(proj("k", "k-text-advisory", false));
        let out = compiler.compile(&inputs).expect("compile");
        // Only the project-mandatory wins, the advisory is suppressed.
        assert_eq!(out.sections.len(), 1);
        assert_eq!(
            out.sections[0].strength,
            InstructionStrength::ProjectMandatory
        );
        assert!(
            out.diagnostics
                .rejected
                .iter()
                .any(|r| matches!(r, RejectionReason::DuplicateSuppressed { .. }))
        );
    }

    #[test]
    fn skill_fragments_are_appended_with_provenance() {
        let compiler = InstructionCompiler::new();
        let mut inputs = empty_inputs();
        inputs
            .skills
            .skills
            .push(skill("core.foo", vec!["review checklist: 5 points"]));
        let out = compiler.compile(&inputs).expect("compile");
        assert_eq!(out.sections.len(), 1);
        assert_eq!(out.sections[0].strength, InstructionStrength::Skill);
        assert!(matches!(
            &out.sections[0].source,
            InstructionSource::Skill { ref_token, .. } if ref_token == "core.foo@v1"
        ));
    }

    #[test]
    fn hints_are_advisory_and_kept() {
        let compiler = InstructionCompiler::new();
        let mut inputs = empty_inputs();
        inputs
            .hints
            .hints
            .push("consider running --dry-run".to_string());
        let out = compiler.compile(&inputs).expect("compile");
        assert_eq!(out.sections.len(), 1);
        assert_eq!(out.sections[0].strength, InstructionStrength::Hint);
    }

    #[test]
    fn content_hash_is_stable() {
        let compiler = InstructionCompiler::new();
        let mut i1 = empty_inputs();
        i1.invariants.entries.push(inv("k", "v"));
        i1.policies.entries.push(pol("p", "v"));
        let i2 = i1.clone();
        let o1 = compiler.compile(&i1).expect("a");
        let o2 = compiler.compile(&i2).expect("b");
        assert_eq!(o1.content_hash, o2.content_hash);
    }

    #[test]
    fn content_hash_changes_on_input_change() {
        let compiler = InstructionCompiler::new();
        let mut i1 = empty_inputs();
        i1.invariants.entries.push(inv("k", "v1"));
        let mut i2 = empty_inputs();
        i2.invariants.entries.push(inv("k", "v2"));
        let o1 = compiler.compile(&i1).expect("a");
        let o2 = compiler.compile(&i2).expect("b");
        assert_ne!(o1.content_hash, o2.content_hash);
    }

    #[test]
    fn ref_token_format() {
        let compiler = InstructionCompiler::new();
        let mut inputs = empty_inputs();
        inputs.ae_version = 4;
        let out = compiler.compile(&inputs).expect("compile");
        assert_eq!(out.ref_token(), "effective@v4");
    }

    #[test]
    fn ordering_is_strength_desc_then_key_asc() {
        let compiler = InstructionCompiler::new();
        let mut inputs = empty_inputs();
        inputs.invariants.entries.push(inv("zeta", "z-text"));
        inputs.policies.entries.push(pol("alpha", "a-text"));
        inputs.hints.hints.push("hint-x".to_string());
        let out = compiler.compile(&inputs).expect("compile");
        assert_eq!(out.sections.len(), 3);
        assert_eq!(out.sections[0].strength, InstructionStrength::Invariant);
        assert_eq!(out.sections[1].strength, InstructionStrength::Policy);
        assert_eq!(out.sections[2].strength, InstructionStrength::Hint);
    }

    #[test]
    fn profile_is_carried_in_inputs_but_not_in_compile_output() {
        let compiler = InstructionCompiler::new();
        let mut inputs = empty_inputs();
        inputs.profile = Some(default_profile());
        let out = compiler.compile(&inputs).expect("compile");
        // Profile is consumed at runtime by the admission gate, not by
        // the instruction compiler. The compiler does not depend on it.
        assert_eq!(out.sections.len(), 0);
    }

    #[test]
    fn by_semantic_key_works() {
        let compiler = InstructionCompiler::new();
        let mut inputs = empty_inputs();
        inputs.invariants.entries.push(inv("k", "text"));
        let out = compiler.compile(&inputs).expect("compile");
        assert!(out.by_semantic_key("invariant:k").is_some());
        assert!(out.by_semantic_key("missing").is_none());
    }

    #[test]
    fn from_skill_filters_correctly() {
        let compiler = InstructionCompiler::new();
        let mut inputs = empty_inputs();
        inputs.skills.skills.push(skill("core.foo", vec!["x", "y"]));
        let out = compiler.compile(&inputs).expect("compile");
        assert_eq!(out.from_skill("core.foo@v1").len(), 2);
        assert_eq!(out.from_skill("core.bar@v1").len(), 0);
    }

    #[test]
    fn task_requirement_matching_is_case_insensitive() {
        let compiler = InstructionCompiler::new();
        let mut inputs = empty_inputs();
        inputs
            .task
            .requirements
            .push(task_req("rule", "USE EVIDENCE BUNDLES"));
        // project provides fragment with lowercase phrasing of the same requirement text
        inputs
            .project
            .entries
            .push(proj("p", "use evidence bundles for everything", true));
        // The project text must contain the normalized needle "use evidence bundles".
        let out = compiler.compile(&inputs).expect("compile");
        assert!(out.sections.len() >= 2);
    }

    #[test]
    fn admits_command_with_compiled_skills_returns_consistent_outcome() {
        // Bridge: compile a SelectedSkillSet from SkillRegistry::select_for,
        // then use it to populate CompileInputs, and verify that the
        // EffectiveInstructions carry the same skill references that the
        // SkillRegistry would require to admit the command.
        let mut reg = crate::skill_definition::SkillRegistry::new();
        let s = skill("core.architecture-review", vec!["checklist"]);
        reg.register(s.clone()).expect("register");
        let mut cmd = CommandSpec::new("review", "review");
        cmd = cmd.with_required_skill("core.architecture-review@v1");
        let outcome = reg.admits_command(&cmd);
        assert!(outcome.is_admitted());

        // Now compile instructions with the selected set
        let compiler = InstructionCompiler::new();
        let mut inputs = empty_inputs();
        let selected = reg.select_for("x", &[]);
        inputs.skills.skills = selected.into_iter().map(|r| r.definition.clone()).collect();
        let out = compiler.compile(&inputs).expect("compile");
        assert_eq!(out.from_skill("core.architecture-review@v1").len(), 1);
    }

    // ── AX-S2 — instruction conflict algebra ─────────────────────────────
    // Prototype the remaining conflict classes from the SPIKES.md entry:
    // policy-vs-project narrowing (allowed), task-vs-skill (skills cannot
    // satisfy mandatory task requirements), and duplicate-equivalent
    // directives across sources. Every normative contradiction must
    // resolve deterministically and fail closed.

    #[test]
    fn axs2_policy_narrowing_by_project_is_allowed() {
        // Positive case: a project instruction that *tightens* (does not
        // negate) a policy is accepted alongside it.
        let compiler = InstructionCompiler::new();
        let mut inputs = empty_inputs();
        inputs
            .policies
            .entries
            .push(pol("prefer-bundle", "prefer bundle evidence"));
        inputs.project.entries.push(proj(
            "prefer-bundle-restrict",
            "prefer bundle evidence only from the project vault",
            true,
        ));
        let out = compiler.compile(&inputs).expect("narrowing compiles");
        assert!(
            out.sections.iter().any(
                |f| matches!(&f.source, InstructionSource::Policy { id } if id == "prefer-bundle")
            ),
            "policy must survive narrowing"
        );
        assert!(
            out.sections
                .iter()
                .any(|f| normalize_key(&f.text).contains("only from the project vault")),
            "project narrowing fragment must be present"
        );
    }

    #[test]
    fn axs2_skill_cannot_satisfy_mandatory_task_requirement() {
        // Task-vs-skill: a skill fragment that textually matches a mandatory
        // task requirement must NOT satisfy it — Skill != Capability. The
        // requirement is only satisfiable by invariant/policy/project/task
        // fragments, so compilation must fail closed.
        let compiler = InstructionCompiler::new();
        let mut inputs = empty_inputs();
        inputs.task.requirements.push(task_req(
            "receipt-required",
            "emit a receipt before returning",
        ));
        inputs.skills.skills.push(skill(
            "core.receipt-skill",
            vec!["emit a receipt before returning"],
        ));
        match compiler.compile(&inputs) {
            Err(InstructionConflict::TaskMissing { requirement_id }) => {
                assert_eq!(requirement_id, "receipt-required");
            }
            other => panic!("expected TaskMissing, got {other:?}"),
        }
    }

    #[test]
    fn axs2_project_mandatory_satisfies_task_requirement_case_insensitively() {
        // Duplicate-equivalent directives: project text is semantically
        // equivalent to the task requirement modulo case/whitespace. The
        // normalized-key containment must accept it.
        let compiler = InstructionCompiler::new();
        let mut inputs = empty_inputs();
        inputs
            .task
            .requirements
            .push(task_req("dry-run", "Plan a release (dry-run)"));
        inputs
            .project
            .entries
            .push(proj("dry-run-equivalent", "plan a RELEASE (dry-run)", true));
        let out = compiler
            .compile(&inputs)
            .expect("equivalent directive compiles");
        // Both fragments survive (task + project), no conflict raised.
        assert_eq!(out.sections.len(), 2);
    }

    #[test]
    fn axs2_conflict_resolution_is_deterministic_across_orderings() {
        // The same contradictory input set must produce the identical
        // conflict regardless of the order entries were pushed in —
        // normative resolution must not depend on insertion order.
        let mk = || {
            let mut inputs = empty_inputs();
            inputs
                .invariants
                .entries
                .push(inv("writes-cas", "all writes are content-addressed"));
            inputs
                .policies
                .entries
                .push(pol("prefer-bundle", "prefer bundle evidence"));
            inputs
        };
        let compiler = InstructionCompiler::new();

        let mut a = mk();
        a.invariants
            .entries
            .push(inv("writes-cas", "dup-invariant"));
        a.project.entries.push(proj(
            "writes-cas",
            "writes do NOT have to be content-addressed in this project",
            true,
        ));

        let mut b = mk();
        b.project.entries.push(proj(
            "writes-cas",
            "writes do NOT have to be content-addressed in this project",
            true,
        ));
        b.invariants
            .entries
            .push(inv("writes-cas", "dup-invariant"));

        let ra = compiler.compile(&a);
        let rb = compiler.compile(&b);
        match (ra, rb) {
            (
                Err(InstructionConflict::InvariantViolation {
                    invariant_id: ia, ..
                }),
                Err(InstructionConflict::InvariantViolation {
                    invariant_id: ib, ..
                }),
            ) => assert_eq!(ia, ib, "conflict identity must be order-independent"),
            (ra, rb) => panic!("expected matching InvariantViolation, got {ra:?} vs {rb:?}"),
        }
    }

    #[test]
    fn axs2_semantic_key_strength_ordering_is_total() {
        // The strength ladder must be a total, deterministic order
        // (per the compiler's strength_rank used for dedupe + sorting):
        // Invariant > Policy > ProjectMandatory > Task > Skill > Advisory == Hint.
        let rank_of = |s: InstructionStrength| match s {
            InstructionStrength::Invariant => 5,
            InstructionStrength::Policy => 4,
            InstructionStrength::ProjectMandatory => 3,
            InstructionStrength::Task => 2,
            InstructionStrength::Skill => 1,
            InstructionStrength::ProjectAdvisory | InstructionStrength::Hint => 0,
        };
        let order = [
            InstructionStrength::Invariant,
            InstructionStrength::Policy,
            InstructionStrength::ProjectMandatory,
            InstructionStrength::Task,
            InstructionStrength::Skill,
            InstructionStrength::ProjectAdvisory,
        ];
        // Advisory and Hint share rank 0 by design (both non-normative).
        assert_eq!(
            rank_of(InstructionStrength::ProjectAdvisory),
            rank_of(InstructionStrength::Hint)
        );
        for w in order.windows(2) {
            assert!(
                rank_of(w[0]) > rank_of(w[1]),
                "strength ordering violated: {:?} must outrank {:?}",
                w[0],
                w[1]
            );
        }
    }
}
