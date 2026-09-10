//! SPEC-018 — Agent execution provenance and reproducibility envelope.
//!
//! An `AgentExecutionReceiptV1` answers:
//! "what did this agent receive, what contract governed it, which
//! model/adapter executed it, and what did it produce?"
//!
//! Producers (live in v1.166.x):
//! - `effective_instructions_hash` — produced by
//!   `EffectiveInstructions::content_hash` (M7.5 / SPEC-014).
//! - `selected_skill_refs/hash` — produced by `SkillDefinition::ref_token`
//!   and `SkillDefinition::content_hash` (M7.3 / SPEC-013).
//!
//! Producer gaps (deferred to future cycles):
//! - `context_capsule_hash`         — needs ContextCapsule contract (M7+).
//! - `command_surface_hash`         — needs AgentCommandSurface hashing.
//! - `tool_contract_hash`           — needs tool contract layer.
//! - `policy_snapshot_hash`         — needs PolicySnapshot layer.
//! - `resolved_config_hash`         — needs config hashing primitive.
//! - `execution_outcome_ref`        — needs ExecutionOutcome (M7+).
//! - `contribution_ref?`            — needs Contribution (M7+).
//!
//! These deferred references are `Option<String>`, so receipts remain
//! constructible today; missing values become fail-closed warnings
//! (see `ValidationWarning`) instead of hard errors, because the receipt
//! itself is still useful as a provenance anchor.

#![allow(missing_docs)]

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

use crate::instruction_compiler::EffectiveInstructions;
use crate::skill_definition::SkillDefinition;

// ============================================================
// Replay classes (SPEC-018 §Replay classes)
// ============================================================

/// Whether the recorded execution can be replayed bit-for-bit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplayClass {
    /// No nondeterministic provider/tool I/O.
    Deterministic,
    /// Replay possible using captured external responses.
    RecordedIo,
    /// Provenance comparable, output not guaranteed identical.
    Nondeterministic,
}

impl std::fmt::Display for ReplayClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Deterministic => "deterministic",
            Self::RecordedIo => "recorded_io",
            Self::Nondeterministic => "nondeterministic",
        };
        f.write_str(s)
    }
}

impl std::str::FromStr for ReplayClass {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "deterministic" => Ok(Self::Deterministic),
            "recorded_io" => Ok(Self::RecordedIo),
            "nondeterministic" => Ok(Self::Nondeterministic),
            other => Err(format!("unknown replay_class: {other}")),
        }
    }
}

// ============================================================
// Provider usage (SPEC-018 §Provider usage)
// ============================================================

/// Counts and rough sizes for a single provider invocation. Privacy
/// rule (SPEC-018 §Privacy): do not persist hidden chain-of-thought.
/// These fields are pulled from the public provider API response
/// metadata only.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderUsage {
    /// Total prompt tokens consumed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u64>,
    /// Total completion tokens generated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u64>,
    /// Wall-clock duration of the provider call.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Provider-reported model identifier (e.g. `gpt-4o-2024-08-06`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

// ============================================================
// Selected skill entry (SPEC-018 §selected_skill_refs/hash)
// ============================================================

/// A reference to a skill that was selected for the execution, with its
/// content hash for receipts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectedSkillRef {
    /// `id@version` (e.g. `core.contract-review@v1`).
    pub ref_token: String,
    /// Content hash from `SkillDefinition::content_hash`.
    pub content_hash: String,
}

// ============================================================
// Validation warning (SPEC-018 §Producer gaps)
// ============================================================

/// A non-fatal warning emitted at receipt construction when an expected
/// field is missing (because the corresponding producer is not yet
/// wired). These warnings make deferred-state visible without blocking
/// the receipt from being produced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "field")]
pub enum ValidationWarning {
    /// `context_capsule_hash` was not provided.
    MissingContextCapsule,
    /// `command_surface_hash` was not provided.
    MissingCommandSurface,
    /// `tool_contract_hash` was not provided.
    MissingToolContract,
    /// `policy_snapshot_hash` was not provided.
    MissingPolicySnapshot,
    /// `resolved_config_hash` was not provided.
    MissingResolvedConfig,
    /// `execution_outcome_ref` was not provided.
    MissingExecutionOutcome,
    /// `contribution_ref` was not provided (advisory; field is optional).
    MissingContribution,
    /// `provider_usage` was not populated.
    MissingProviderUsage,
}

// ============================================================
// AgentExecutionReceiptV1 (SPEC-018 §Receipt)
// ============================================================

/// Provenance envelope for an agent execution.
///
/// All fields are explicit (no auto-fill); missing producers surface as
/// `ValidationWarning`. Construct via `AgentExecutionReceipt::builder`
/// to get a typed, validated, fail-closed build.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentExecutionReceipt {
    /// Stable identifier for this execution (ULID/UUID).
    pub execution_id: String,
    /// Run / task identifier (e.g. `cycle-49-m7-5-apply`).
    pub run_id: String,
    /// Reference to the `AgentProfile` used (e.g. `core.orchestrator@v1`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_profile_ref: Option<String>,
    /// Provider model descriptor (free-form: `provider/model/version`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_descriptor: Option<String>,
    /// Reference to the `ProviderAdapter` used (e.g. `openai.v1`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_adapter_ref: Option<String>,
    /// `EffectiveInstructions::content_hash` (M7.5 / SPEC-014).
    pub effective_instructions_hash: String,
    /// Selected skill refs (M7.3 / SPEC-013).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selected_skill_refs: Vec<SelectedSkillRef>,
    /// Hash of the command surface available to this execution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_surface_hash: Option<String>,
    /// Hash of the tool contract available to this execution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_contract_hash: Option<String>,
    /// Hash of the policy snapshot at execution time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_snapshot_hash: Option<String>,
    /// Hash of the resolved effective config.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_config_hash: Option<String>,
    /// Hash of the context capsule used (M7+ context contract).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_capsule_hash: Option<String>,
    /// Started metadata (RFC3339).
    pub started_at: String,
    /// Finished metadata (RFC3339).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    /// Reference to the execution outcome (M7+ ExecutionOutcome).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_outcome_ref: Option<String>,
    /// Optional reference to a typed `Contribution`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contribution_ref: Option<String>,
    /// Provider usage metrics.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_usage: Option<ProviderUsage>,
    /// Replay class.
    pub replay_class: ReplayClass,
}

// ============================================================
// Errors
// ============================================================

/// Fail-closed validation errors. The receipt MUST NOT be produced if
/// any of these errors fire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "details")]
pub enum ReceiptError {
    /// `execution_id` is empty or whitespace-only.
    EmptyExecutionId,
    /// `run_id` is empty or whitespace-only.
    EmptyRunId,
    /// `started_at` is empty or whitespace-only.
    EmptyStartedAt,
    /// `effective_instructions_hash` is empty or not 16-char hex.
    InvalidEffectiveInstructionsHash { hash: String },
    /// A selected skill's `ref_token` does not match `id@vN`.
    InvalidSkillRefToken { ref_token: String },
    /// A selected skill's `content_hash` is empty or not 16-char hex.
    InvalidSkillContentHash { ref_token: String, hash: String },
    /// `finished_at` is set but not RFC3339-like.
    FinishedAtNotRfc3339 { value: String },
}

// ============================================================
// Validation + content hash
// ============================================================

impl AgentExecutionReceipt {
    /// Validates the receipt. Returns `Err(ReceiptError)` for any
    /// fail-closed violation. (Deferred-producer warnings are NOT
    /// surfaced here; see `missing_producer_warnings()`.)
    pub fn validate(&self) -> Result<(), ReceiptError> {
        if self.execution_id.trim().is_empty() {
            return Err(ReceiptError::EmptyExecutionId);
        }
        if self.run_id.trim().is_empty() {
            return Err(ReceiptError::EmptyRunId);
        }
        if self.started_at.trim().is_empty() {
            return Err(ReceiptError::EmptyStartedAt);
        }
        if !is_hex16(&self.effective_instructions_hash) {
            return Err(ReceiptError::InvalidEffectiveInstructionsHash {
                hash: self.effective_instructions_hash.clone(),
            });
        }
        for s in &self.selected_skill_refs {
            if !is_valid_ref_token(&s.ref_token) {
                return Err(ReceiptError::InvalidSkillRefToken {
                    ref_token: s.ref_token.clone(),
                });
            }
            if !is_hex16(&s.content_hash) {
                return Err(ReceiptError::InvalidSkillContentHash {
                    ref_token: s.ref_token.clone(),
                    hash: s.content_hash.clone(),
                });
            }
        }
        if let Some(f) = &self.finished_at {
            if !looks_like_rfc3339(f) {
                return Err(ReceiptError::FinishedAtNotRfc3339 { value: f.clone() });
            }
        }
        Ok(())
    }

    /// Returns one `ValidationWarning` per deferred-producer field that
    /// was not provided. Empty vec means all producers wired.
    pub fn missing_producer_warnings(&self) -> Vec<ValidationWarning> {
        let mut w = Vec::new();
        if self.context_capsule_hash.is_none() {
            w.push(ValidationWarning::MissingContextCapsule);
        }
        if self.command_surface_hash.is_none() {
            w.push(ValidationWarning::MissingCommandSurface);
        }
        if self.tool_contract_hash.is_none() {
            w.push(ValidationWarning::MissingToolContract);
        }
        if self.policy_snapshot_hash.is_none() {
            w.push(ValidationWarning::MissingPolicySnapshot);
        }
        if self.resolved_config_hash.is_none() {
            w.push(ValidationWarning::MissingResolvedConfig);
        }
        if self.execution_outcome_ref.is_none() {
            w.push(ValidationWarning::MissingExecutionOutcome);
        }
        if self.contribution_ref.is_none() {
            w.push(ValidationWarning::MissingContribution);
        }
        if self.provider_usage.is_none() {
            w.push(ValidationWarning::MissingProviderUsage);
        }
        w
    }

    /// Content hash over canonical JSON (sorted keys, no whitespace).
    /// 16-char lowercase hex.
    pub fn content_hash(&self) -> String {
        let value = serde_json::to_value(self).expect("receipt serializes");
        let canonical = canonicalize_json(value);
        let mut hasher = DefaultHasher::new();
        canonical.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    /// Compact canonical JSON encoding (sorted keys, no whitespace).
    pub fn canonical_json(&self) -> String {
        let value = serde_json::to_value(self).expect("receipt serializes");
        canonicalize_json(value).to_string()
    }
}

// ============================================================
// Builder (typed, fail-closed)
// ============================================================

/// Type-safe builder for `AgentExecutionReceipt` that wires the
/// effective-instructions and selected-skill producers from their
/// canonical sources (M7.5 + M7.3) at construction time.
#[derive(Debug, Clone)]
pub struct AgentExecutionReceiptBuilder {
    execution_id: String,
    run_id: String,
    started_at: String,
    finished_at: Option<String>,
    agent_profile_ref: Option<String>,
    model_descriptor: Option<String>,
    provider_adapter_ref: Option<String>,
    command_surface_hash: Option<String>,
    tool_contract_hash: Option<String>,
    policy_snapshot_hash: Option<String>,
    resolved_config_hash: Option<String>,
    context_capsule_hash: Option<String>,
    execution_outcome_ref: Option<String>,
    contribution_ref: Option<String>,
    provider_usage: Option<ProviderUsage>,
    replay_class: ReplayClass,
}

impl AgentExecutionReceiptBuilder {
    /// Start a new builder. `execution_id`, `run_id`, `started_at` are
    /// required and must be non-empty; `replay_class` defaults to
    /// `Nondeterministic` (safest assumption).
    pub fn new(
        execution_id: impl Into<String>,
        run_id: impl Into<String>,
        started_at: impl Into<String>,
    ) -> Self {
        Self {
            execution_id: execution_id.into(),
            run_id: run_id.into(),
            started_at: started_at.into(),
            finished_at: None,
            agent_profile_ref: None,
            model_descriptor: None,
            provider_adapter_ref: None,
            command_surface_hash: None,
            tool_contract_hash: None,
            policy_snapshot_hash: None,
            resolved_config_hash: None,
            context_capsule_hash: None,
            execution_outcome_ref: None,
            contribution_ref: None,
            provider_usage: None,
            replay_class: ReplayClass::Nondeterministic,
        }
    }

    pub fn finished_at(mut self, v: impl Into<String>) -> Self {
        self.finished_at = Some(v.into());
        self
    }
    pub fn agent_profile_ref(mut self, v: impl Into<String>) -> Self {
        self.agent_profile_ref = Some(v.into());
        self
    }
    pub fn model_descriptor(mut self, v: impl Into<String>) -> Self {
        self.model_descriptor = Some(v.into());
        self
    }
    pub fn provider_adapter_ref(mut self, v: impl Into<String>) -> Self {
        self.provider_adapter_ref = Some(v.into());
        self
    }
    pub fn command_surface_hash(mut self, v: impl Into<String>) -> Self {
        self.command_surface_hash = Some(v.into());
        self
    }
    pub fn tool_contract_hash(mut self, v: impl Into<String>) -> Self {
        self.tool_contract_hash = Some(v.into());
        self
    }
    pub fn policy_snapshot_hash(mut self, v: impl Into<String>) -> Self {
        self.policy_snapshot_hash = Some(v.into());
        self
    }
    pub fn resolved_config_hash(mut self, v: impl Into<String>) -> Self {
        self.resolved_config_hash = Some(v.into());
        self
    }
    pub fn context_capsule_hash(mut self, v: impl Into<String>) -> Self {
        self.context_capsule_hash = Some(v.into());
        self
    }
    pub fn execution_outcome_ref(mut self, v: impl Into<String>) -> Self {
        self.execution_outcome_ref = Some(v.into());
        self
    }
    pub fn contribution_ref(mut self, v: impl Into<String>) -> Self {
        self.contribution_ref = Some(v.into());
        self
    }
    pub fn provider_usage(mut self, v: ProviderUsage) -> Self {
        self.provider_usage = Some(v);
        self
    }
    pub fn replay_class(mut self, v: ReplayClass) -> Self {
        self.replay_class = v;
        self
    }

    /// Build the receipt by **pulling** `effective_instructions_hash`
    /// from a live `EffectiveInstructions` (M7.5 producer) and
    /// `selected_skill_refs` from a slice of `SkillDefinition`s (M7.3
    /// producer). Validates and returns either the receipt or the
    /// first validation error.
    pub fn build(
        self,
        effective: &EffectiveInstructions,
        selected_skills: &[SkillDefinition],
    ) -> Result<AgentExecutionReceipt, ReceiptError> {
        let selected_skill_refs: Vec<SelectedSkillRef> = selected_skills
            .iter()
            .map(|s| SelectedSkillRef {
                ref_token: s.ref_token(),
                content_hash: s.content_hash(),
            })
            .collect();

        let receipt = AgentExecutionReceipt {
            execution_id: self.execution_id,
            run_id: self.run_id,
            agent_profile_ref: self.agent_profile_ref,
            model_descriptor: self.model_descriptor,
            provider_adapter_ref: self.provider_adapter_ref,
            effective_instructions_hash: effective.content_hash.clone(),
            selected_skill_refs,
            command_surface_hash: self.command_surface_hash,
            tool_contract_hash: self.tool_contract_hash,
            policy_snapshot_hash: self.policy_snapshot_hash,
            resolved_config_hash: self.resolved_config_hash,
            context_capsule_hash: self.context_capsule_hash,
            started_at: self.started_at,
            finished_at: self.finished_at,
            execution_outcome_ref: self.execution_outcome_ref,
            contribution_ref: self.contribution_ref,
            provider_usage: self.provider_usage,
            replay_class: self.replay_class,
        };
        receipt.validate()?;
        Ok(receipt)
    }
}

// ============================================================
// Helpers
// ============================================================

fn is_hex16(s: &str) -> bool {
    s.len() == 16 && s.chars().all(|c| c.is_ascii_hexdigit())
}

fn is_valid_ref_token(s: &str) -> bool {
    // Must contain a '.' AND end with `@v<digit>+`.
    if !s.contains('.') {
        return false;
    }
    match s.rsplit_once("@v") {
        Some((_, ver)) => !ver.is_empty() && ver.chars().all(|c| c.is_ascii_digit()),
        None => false,
    }
}

fn looks_like_rfc3339(s: &str) -> bool {
    // Cheap structural check: YYYY-MM-DDTHH:MM:SS[.frac][Z|+HH:MM]
    if s.len() < 20 {
        return false;
    }
    let bytes = s.as_bytes();
    // Year-MM-DD
    bytes.get(4) == Some(&b'-')
        && bytes.get(7) == Some(&b'-')
        && bytes.get(10) == Some(&b'T')
        && bytes.get(13) == Some(&b':')
        && bytes.get(16) == Some(&b':')
}

fn canonicalize_json(value: serde_json::Value) -> serde_json::Value {
    use serde_json::Value;
    match value {
        Value::Object(map) => {
            let mut sorted: std::collections::BTreeMap<String, Value> =
                std::collections::BTreeMap::new();
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

// ============================================================
// Tests (inline; SPEC-018 unit-test coverage)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_effective() -> EffectiveInstructions {
        use crate::instruction_compiler::*;
        let inputs = CompileInputs {
            ae_version: 1,
            ..Default::default()
        };
        InstructionCompiler::new().compile(&inputs).expect("compile")
    }

    fn fixture_skill() -> SkillDefinition {
        use crate::skill_definition::*;
        SkillDefinition {
            id: "core.contract-review".into(),
            version: 1,
            applies_when: AppliesWhen {
                task_kinds: vec!["review".into()],
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
            instruction_fragments: vec!["checklist".into()],
            compat: CompatRange::default(),
        }
    }

    #[test]
    fn builder_pulls_effective_hash_from_m75_producer() {
        let eff = fixture_effective();
        let expected_hash = eff.content_hash.clone();
        let r = AgentExecutionReceiptBuilder::new(
            "01J0EXEC0",
            "cycle-49-m7-6",
            "2026-09-10T12:00:00Z",
        )
        .build(&eff, &[])
        .expect("build");
        assert_eq!(r.effective_instructions_hash, expected_hash);
        assert!(r.selected_skill_refs.is_empty());
    }

    #[test]
    fn builder_pulls_skill_refs_from_m73_producer() {
        let eff = fixture_effective();
        let skill = fixture_skill();
        let expected_token = skill.ref_token();
        let expected_hash = skill.content_hash();
        let r = AgentExecutionReceiptBuilder::new(
            "01J0EXEC1",
            "cycle-49-m7-6",
            "2026-09-10T12:00:00Z",
        )
        .build(&eff, std::slice::from_ref(&skill))
        .expect("build");
        assert_eq!(r.selected_skill_refs.len(), 1);
        assert_eq!(r.selected_skill_refs[0].ref_token, expected_token);
        assert_eq!(r.selected_skill_refs[0].content_hash, expected_hash);
    }

    #[test]
    fn validate_rejects_empty_execution_id() {
        let eff = fixture_effective();
        let r = AgentExecutionReceiptBuilder::new("", "run", "2026-09-10T12:00:00Z")
            .build(&eff, &[]);
        assert!(matches!(r, Err(ReceiptError::EmptyExecutionId)));
    }

    #[test]
    fn validate_rejects_empty_run_id() {
        let eff = fixture_effective();
        let r = AgentExecutionReceiptBuilder::new("exec", "  ", "2026-09-10T12:00:00Z")
            .build(&eff, &[]);
        assert!(matches!(r, Err(ReceiptError::EmptyRunId)));
    }

    #[test]
    fn validate_rejects_invalid_effective_hash() {
        let eff = fixture_effective();
        let mut bad = AgentExecutionReceiptBuilder::new("exec", "run", "2026-09-10T12:00:00Z")
            .build(&eff, &[])
            .expect("build");
        bad.effective_instructions_hash = "not-hex".into();
        let v = bad.validate();
        assert!(matches!(v, Err(ReceiptError::InvalidEffectiveInstructionsHash { .. })));
    }

    #[test]
    fn validate_rejects_invalid_skill_ref_token() {
        let eff = fixture_effective();
        let r0 = AgentExecutionReceiptBuilder::new("exec", "run", "2026-09-10T12:00:00Z")
            .build(&eff, &[])
            .expect("base");
        let mut bad = r0.clone();
        bad.selected_skill_refs.push(SelectedSkillRef {
            ref_token: "no-dot-no-at".into(),
            content_hash: "0000000000000000".into(),
        });
        let v = bad.validate();
        assert!(matches!(v, Err(ReceiptError::InvalidSkillRefToken { .. })));
    }

    #[test]
    fn validate_rejects_non_rfc3339_finished_at() {
        let eff = fixture_effective();
        let r = AgentExecutionReceiptBuilder::new("exec", "run", "2026-09-10T12:00:00Z")
            .finished_at("not-a-date")
            .build(&eff, &[]);
        assert!(matches!(r, Err(ReceiptError::FinishedAtNotRfc3339 { .. })));
    }

    #[test]
    fn content_hash_is_stable_across_calls() {
        let eff = fixture_effective();
        let r = AgentExecutionReceiptBuilder::new("exec", "run", "2026-09-10T12:00:00Z")
            .build(&eff, &[])
            .expect("build");
        let h1 = r.content_hash();
        let h2 = r.content_hash();
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 16);
        assert!(h1.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn content_hash_differs_when_input_differs() {
        let eff = fixture_effective();
        let r1 = AgentExecutionReceiptBuilder::new("exec-1", "run", "2026-09-10T12:00:00Z")
            .build(&eff, &[])
            .expect("build");
        let r2 = AgentExecutionReceiptBuilder::new("exec-2", "run", "2026-09-10T12:00:00Z")
            .build(&eff, &[])
            .expect("build");
        assert_ne!(r1.content_hash(), r2.content_hash());
    }

    #[test]
    fn missing_producer_warnings_enumerate_deferred_fields() {
        let eff = fixture_effective();
        let r = AgentExecutionReceiptBuilder::new("exec", "run", "2026-09-10T12:00:00Z")
            .build(&eff, &[])
            .expect("build");
        let w = r.missing_producer_warnings();
        // All 8 deferred-producer fields are missing → 8 warnings.
        assert_eq!(w.len(), 8);
        assert!(w.contains(&ValidationWarning::MissingContextCapsule));
        assert!(w.contains(&ValidationWarning::MissingCommandSurface));
        assert!(w.contains(&ValidationWarning::MissingToolContract));
        assert!(w.contains(&ValidationWarning::MissingPolicySnapshot));
        assert!(w.contains(&ValidationWarning::MissingResolvedConfig));
        assert!(w.contains(&ValidationWarning::MissingExecutionOutcome));
        assert!(w.contains(&ValidationWarning::MissingContribution));
        assert!(w.contains(&ValidationWarning::MissingProviderUsage));
    }

    #[test]
    fn warning_set_shrinks_when_producers_provided() {
        let eff = fixture_effective();
        let usage = ProviderUsage {
            input_tokens: Some(100),
            output_tokens: Some(50),
            duration_ms: Some(1234),
            model: Some("test-model".into()),
        };
        let r = AgentExecutionReceiptBuilder::new("exec", "run", "2026-09-10T12:00:00Z")
            .context_capsule_hash("1111111111111111")
            .execution_outcome_ref("outcome-1")
            .contribution_ref("contrib-1")
            .provider_usage(usage)
            .build(&eff, &[])
            .expect("build");
        let w = r.missing_producer_warnings();
        // Still missing: command_surface, tool_contract, policy_snapshot, resolved_config
        assert_eq!(w.len(), 4);
        assert!(!w.contains(&ValidationWarning::MissingContextCapsule));
        assert!(!w.contains(&ValidationWarning::MissingExecutionOutcome));
        assert!(!w.contains(&ValidationWarning::MissingContribution));
        assert!(!w.contains(&ValidationWarning::MissingProviderUsage));
    }

    #[test]
    fn json_round_trip_preserves_fields() {
        let eff = fixture_effective();
        let r = AgentExecutionReceiptBuilder::new("exec", "run", "2026-09-10T12:00:00Z")
            .agent_profile_ref("core.orchestrator@v1")
            .model_descriptor("openai/gpt-4o")
            .replay_class(ReplayClass::Deterministic)
            .build(&eff, &[])
            .expect("build");
        let json = serde_json::to_string(&r).expect("serialize");
        let parsed: AgentExecutionReceipt = serde_json::from_str(&json).expect("parse");
        assert_eq!(parsed, r);
    }

    #[test]
    fn replay_class_serialization_round_trip() {
        for c in [ReplayClass::Deterministic, ReplayClass::RecordedIo, ReplayClass::Nondeterministic] {
            let j = serde_json::to_string(&c).unwrap();
            let back: ReplayClass = serde_json::from_str(&j).unwrap();
            assert_eq!(back, c);
            assert!(j.starts_with('"') && j.ends_with('"'));
            // snake_case
            assert!(!j.chars().any(|c| c.is_uppercase()));
        }
    }

    #[test]
    fn ref_token_validation_strict() {
        assert!(is_valid_ref_token("core.foo@v1"));
        assert!(is_valid_ref_token("a.b.c@v123"));
        assert!(!is_valid_ref_token("no-dot"));
        assert!(!is_valid_ref_token("foo.bar"));       // no @v
        assert!(!is_valid_ref_token("foo.bar@v"));     // empty version
        assert!(!is_valid_ref_token("foo.bar@vX"));    // non-digit
    }

    #[test]
    fn rfc3339_validation() {
        assert!(looks_like_rfc3339("2026-09-10T12:00:00Z"));
        assert!(looks_like_rfc3339("2026-09-10T12:00:00.123Z"));
        assert!(looks_like_rfc3339("2026-09-10T12:00:00+02:00"));
        assert!(!looks_like_rfc3339("not-a-date"));
        assert!(!looks_like_rfc3339("2026/09/10 12:00:00"));
        assert!(!looks_like_rfc3339(""));
    }
}
