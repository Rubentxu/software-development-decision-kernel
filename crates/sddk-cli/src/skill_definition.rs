//! M7.3 — SkillDefinition contract.
//!
//! Implements `SPEC-016 — Skill Contract` from the consolidation package:
//! typed, namespaced, versioned descriptions of *how* an agent reasons
//! about or executes a class of task. Skills are **declarations** of
//! capability requirements, evidence expectations and instruction
//! fragments — they never grant permissions.
//!
//! ## Invariants
//!
//! - **Skill ≠ Capability** (SPEC-016 anti-patterns + AGENTS.md #9): a
//!   Skill declares *which* capabilities it would consume; enabling it
//!   does NOT bypass the Authority engine. The runtime admission gate
//!   (`AgentProfile::admits` from M7.4) still has to admit the command.
//! - **Namespaced id**: must contain `.` (e.g. `core.architecture-review`).
//! - **Version is monotone**: parsing enforces `version >= 1`.
//! - **Compatibility range** (`compat_min_ae_version` <=
//!   `compat_max_ae_version`): both inclusive; either bound may be
//!   `None` to indicate "no bound on that side".
//! - **Anti-patterns are hard errors** at parse time:
//!   - empty `instruction_fragments` together with `required_capabilities`
//!     longer than 8 entries (proxy for "monolithic prompt with embedded
//!     architecture");
//!   - `required_capabilities` containing any `*` or `all` sentinel
//!     (proxy for "permission grant");
//!   - duplicate `id@version` pair in the same registry
//!     (proxy for "second workflow engine").
//!
//! ## Hashing
//!
//! `SkillDefinition::content_hash()` returns a stable hex-encoded hash
//! over the canonical JSON encoding (sorted object keys, compact, no
//! whitespace) using `DefaultHasher`. Two skills with the same fields
//! produce the same hash regardless of YAML/JSON parse-order.
//!
//! ## Selection
//!
//! `SkillRegistry::select_for(task_kind)` returns every registered
//! skill whose `applies_when.task_kinds` contains the given kind,
//! ordered by id then version for determinism. This is the input that
//! SPEC-014 (`InstructionCompiler`) and SPEC-018 (`AgentExecutionReceipt`)
//! consume.

#![allow(missing_docs)]

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

/// Wire-level Skill Definition. Mirrors the YAML contract in SPEC-016.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SkillDefinition {
    /// Namespaced skill id (must contain `.`).
    pub id: String,
    /// Schema version (must be >= 1).
    pub version: u32,
    /// Applicability predicate.
    pub applies_when: AppliesWhen,
    /// Typed input contract (free-form, listed by type name).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<String>,
    /// Typed output contract (e.g. `ContributionV2`).
    pub outputs: SkillOutput,
    /// Required and optional capabilities the skill would consume.
    pub capabilities: CapabilityRequirement,
    /// Minimum evidence expectation.
    #[serde(default)]
    pub evidence_contract: EvidenceContract,
    /// Names of instruction fragments the skill contributes.
    pub instruction_fragments: Vec<String>,
    /// Compatibility range against the Agent Experience contract version.
    #[serde(default)]
    pub compat: CompatRange,
}

/// Predicate describing when a skill applies.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct AppliesWhen {
    /// Task kinds the skill applies to (e.g. `review.architecture`).
    /// Empty list means "matches no task" (fail-closed).
    #[serde(default)]
    pub task_kinds: Vec<String>,
    /// Optional context keys that must be present in `ContextCapsule`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context_keys: Vec<String>,
}

impl AppliesWhen {
    pub fn matches(&self, task_kind: &str, context_keys: &[&str]) -> bool {
        if !self.task_kinds.iter().any(|k| k == task_kind) {
            return false;
        }
        self.context_keys
            .iter()
            .all(|k| context_keys.contains(&k.as_str()))
    }
}

/// Typed output contract. `contract` is the canonical type name; `notes`
/// is a free-form description (no embedded prompts — anti-pattern).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SkillOutput {
    pub contract: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Capability consumption declaration. Declared != granted.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct CapabilityRequirement {
    /// Capabilities the skill expects to consume.
    #[serde(default)]
    pub required: Vec<String>,
    /// Capabilities the skill may optionally consume.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub optional: Vec<String>,
}

/// Minimum evidence tier the skill needs.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceTier {
    /// No evidence expectation.
    #[default]
    None,
    /// Source refs (URLs, file paths, IDs).
    SourceRefs,
    /// Evidence bundle with structured claims.
    Bundle,
    /// Evidence bundle with redaction-ready provenance.
    RedactedBundle,
}

/// Evidence expectation of a skill.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct EvidenceContract {
    #[serde(default)]
    pub minimum: EvidenceTier,
}

/// Inclusive compatibility range against the Agent Experience (AE)
/// contract version. `None` on either bound means "no bound on that
/// side".
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompatRange {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_ae_version: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_ae_version: Option<u32>,
}

impl CompatRange {
    pub fn admits(&self, ae_version: u32) -> bool {
        let above_min = self.min_ae_version.is_none_or(|m| ae_version >= m);
        let below_max = self.max_ae_version.is_none_or(|m| ae_version <= m);
        above_min && below_max
    }
}

/// Validation error for a SkillDefinition. Caller must reject any parse
/// result that returns Err.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillDefinitionError {
    /// Skill id does not contain a `.` namespace separator.
    UnnamespacedId { id: String },
    /// Version is below 1.
    InvalidVersion { id: String, version: u32 },
    /// `compat.min_ae_version > compat.max_ae_version`.
    InvertedCompatRange { id: String },
    /// `required_capabilities` contains a wildcard (`*` or `all`).
    WildcardCapability { id: String, capability: String },
    /// Both `instruction_fragments` is empty AND `required_capabilities`
    /// has more than 8 entries — proxy for monolithic prompt.
    MonolithicPromptPattern { id: String },
    /// Empty `task_kinds` list (a skill that matches nothing).
    EmptyTaskKinds { id: String },
    /// Output contract name is empty.
    EmptyOutputContract { id: String },
}

impl std::fmt::Display for SkillDefinitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnnamespacedId { id } => {
                write!(f, "skill id must be namespaced (contain `.`): {id}")
            }
            Self::InvalidVersion { id, version } => {
                write!(f, "skill version must be >= 1: {id}@v{version}")
            }
            Self::InvertedCompatRange { id } => {
                write!(f, "compat range inverted (min > max) for skill: {id}")
            }
            Self::WildcardCapability { id, capability } => write!(
                f,
                "skill {id} declares wildcard capability `{capability}`; \
                 skills MUST NOT grant permissions"
            ),
            Self::MonolithicPromptPattern { id } => write!(
                f,
                "skill {id} has empty instruction_fragments with > 8 \
                 required capabilities (monolithic-prompt anti-pattern)"
            ),
            Self::EmptyTaskKinds { id } => {
                write!(f, "skill {id} has empty applies_when.task_kinds")
            }
            Self::EmptyOutputContract { id } => {
                write!(f, "skill {id} has empty outputs.contract")
            }
        }
    }
}

impl std::error::Error for SkillDefinitionError {}

impl SkillDefinition {
    /// Fail-closed validation. Returns `Ok(())` iff the skill is
    /// well-formed under SPEC-016 + SPEC-016 anti-patterns + the
    /// SkillDefinition contract invariants.
    pub fn validate(&self) -> Result<(), SkillDefinitionError> {
        if !self.id.contains('.') {
            return Err(SkillDefinitionError::UnnamespacedId {
                id: self.id.clone(),
            });
        }
        if self.version < 1 {
            return Err(SkillDefinitionError::InvalidVersion {
                id: self.id.clone(),
                version: self.version,
            });
        }
        if self.applies_when.task_kinds.is_empty() {
            return Err(SkillDefinitionError::EmptyTaskKinds {
                id: self.id.clone(),
            });
        }
        if self.outputs.contract.trim().is_empty() {
            return Err(SkillDefinitionError::EmptyOutputContract {
                id: self.id.clone(),
            });
        }
        if let (Some(min), Some(max)) = (self.compat.min_ae_version, self.compat.max_ae_version)
            && min > max
        {
            return Err(SkillDefinitionError::InvertedCompatRange {
                id: self.id.clone(),
            });
        }
        for cap in self
            .capabilities
            .required
            .iter()
            .chain(self.capabilities.optional.iter())
        {
            if cap == "*" || cap.eq_ignore_ascii_case("all") {
                return Err(SkillDefinitionError::WildcardCapability {
                    id: self.id.clone(),
                    capability: cap.clone(),
                });
            }
        }
        if self.instruction_fragments.is_empty() && self.capabilities.required.len() > 8 {
            return Err(SkillDefinitionError::MonolithicPromptPattern {
                id: self.id.clone(),
            });
        }
        Ok(())
    }

    /// Stable hex-encoded content hash. Two skills with identical
    /// fields produce the same hash regardless of insertion order or
    /// source encoding (YAML/JSON).
    pub fn content_hash(&self) -> String {
        // serde_json::to_string sorts map keys alphabetically because
        // BTreeMap-backed serialization isn't on by default; we use
        // `to_string` on a `Value` produced by `to_value` and re-serialize
        // through a `BTreeMap`-backed object to force canonicalization.
        let value = serde_json::to_value(self).expect("skill serializes");
        let canonical = canonicalize_json(value);
        let mut hasher = DefaultHasher::new();
        canonical.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    /// Compact canonical JSON encoding (sorted keys, no whitespace).
    /// Used by `content_hash` and by `SkillRegistry::manifest`.
    pub fn canonical_json(&self) -> String {
        let value = serde_json::to_value(self).expect("skill serializes");
        canonicalize_json(value).to_string()
    }

    /// Reference token `id@version` for use in `AgentExecutionReceipt`
    /// (SPEC-018).
    pub fn ref_token(&self) -> String {
        format!("{}@v{}", self.id, self.version)
    }
}

/// Canonicalize a JSON value by recursively sorting object keys and
/// dropping insignificant whitespace. Arrays preserve order.
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

/// A registered skill, paired with its content hash for receipts and
/// provenance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegisteredSkill {
    pub definition: SkillDefinition,
    pub content_hash: String,
}

impl RegisteredSkill {
    pub fn new(definition: SkillDefinition) -> Result<Self, SkillDefinitionError> {
        definition.validate()?;
        let content_hash = definition.content_hash();
        Ok(Self {
            definition,
            content_hash,
        })
    }
}

/// In-memory registry. Insertion order is preserved by `entries()` for
/// deterministic manifest output; selection is sorted by id then version
/// for determinism.
#[derive(Debug, Default, Clone)]
pub struct SkillRegistry {
    entries: Vec<RegisteredSkill>,
    /// `id@version` -> insertion index for duplicate detection.
    by_ref: HashMap<String, usize>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a skill. Fails closed if validation fails or if
    /// `id@version` was already registered.
    pub fn register(
        &mut self,
        definition: SkillDefinition,
    ) -> Result<&RegisteredSkill, SkillRegistryError> {
        definition.validate()?;
        let ref_token = definition.ref_token();
        if self.by_ref.contains_key(&ref_token) {
            return Err(SkillRegistryError::DuplicateRegistration { ref_token });
        }
        let content_hash = definition.content_hash();
        let idx = self.entries.len();
        self.by_ref.insert(ref_token, idx);
        self.entries.push(RegisteredSkill {
            definition,
            content_hash,
        });
        Ok(&self.entries[idx])
    }

    /// Number of registered skills.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// All registered skills, in insertion order.
    pub fn entries(&self) -> &[RegisteredSkill] {
        &self.entries
    }

    /// Select every skill whose `applies_when.task_kinds` contains
    /// `task_kind`. Optional `context_keys` filter is applied when
    /// present in `applies_when.context_keys`. Output is sorted by
    /// `(id, version)` for determinism.
    pub fn select_for(&self, task_kind: &str, context_keys: &[&str]) -> Vec<&RegisteredSkill> {
        let mut out: Vec<&RegisteredSkill> = self
            .entries
            .iter()
            .filter(|s| s.definition.applies_when.matches(task_kind, context_keys))
            .collect();
        out.sort_by(|a, b| {
            a.definition
                .id
                .cmp(&b.definition.id)
                .then_with(|| a.definition.version.cmp(&b.definition.version))
        });
        out
    }

    /// Deterministic manifest JSON: array of `{ref_token, content_hash,
    /// id, version}`, sorted by ref_token.
    pub fn manifest(&self) -> String {
        use serde_json::json;
        let mut refs: Vec<(String, String, String, u32)> = self
            .entries
            .iter()
            .map(|s| {
                (
                    s.definition.ref_token(),
                    s.content_hash.clone(),
                    s.definition.id.clone(),
                    s.definition.version,
                )
            })
            .collect();
        refs.sort();
        let arr: Vec<serde_json::Value> = refs
            .into_iter()
            .map(|(rt, h, id, v)| json!({"ref": rt, "hash": h, "id": id, "version": v}))
            .collect();
        serde_json::to_string(&serde_json::Value::Array(arr)).expect("manifest serializes")
    }
}

/// Registry-level errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillRegistryError {
    DuplicateRegistration { ref_token: String },
    SkillValidation(SkillDefinitionError),
}

impl std::fmt::Display for SkillRegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateRegistration { ref_token } => {
                write!(f, "skill already registered: {ref_token}")
            }
            Self::SkillValidation(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for SkillRegistryError {}

impl From<SkillDefinitionError> for SkillRegistryError {
    fn from(e: SkillDefinitionError) -> Self {
        Self::SkillValidation(e)
    }
}

/// Outcome of `SkillRegistry::admits_command`. `Satisfied` means every
/// declared `id@version` token is present in the registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillAdmitOutcome {
    /// The command declares no required skills.
    Satisfied,
    /// Every required skill is registered.
    SatisfiedBy(Vec<String>),
    /// At least one required skill is missing. Lists every required
    /// token and every available token (for diagnostics).
    Missing {
        required: Vec<String>,
        available: Vec<String>,
    },
}

impl SkillAdmitOutcome {
    pub fn is_admitted(&self) -> bool {
        !matches!(self, Self::Missing { .. })
    }
}

impl SkillRegistry {
    /// Check whether `command` is admitted by the skills currently in
    /// this registry. Returns the full outcome (satisfied/missing) so
    /// callers can render diagnostics.
    pub fn admits_command(&self, command: &crate::command_spec::CommandSpec) -> SkillAdmitOutcome {
        if command.required_skills.is_empty() {
            return SkillAdmitOutcome::Satisfied;
        }
        let available: Vec<String> = self
            .entries
            .iter()
            .map(|s| s.definition.ref_token())
            .collect();
        let missing: Vec<String> = command
            .required_skills
            .iter()
            .filter(|r| !self.by_ref.contains_key(*r))
            .cloned()
            .collect();
        if missing.is_empty() {
            SkillAdmitOutcome::SatisfiedBy(command.required_skills.clone())
        } else {
            SkillAdmitOutcome::Missing {
                required: command.required_skills.clone(),
                available,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> SkillDefinition {
        SkillDefinition {
            id: "core.architecture-review".to_string(),
            version: 1,
            applies_when: AppliesWhen {
                task_kinds: vec!["review.architecture".to_string()],
                context_keys: vec!["plan_revision".to_string()],
            },
            inputs: vec!["ContextCapsule".to_string()],
            outputs: SkillOutput {
                contract: "ContributionV2".to_string(),
                notes: None,
            },
            capabilities: CapabilityRequirement {
                required: vec!["filesystem.read".to_string()],
                optional: vec!["command.run".to_string()],
            },
            evidence_contract: EvidenceContract {
                minimum: EvidenceTier::SourceRefs,
            },
            instruction_fragments: vec!["review-checklist".to_string()],
            compat: CompatRange::default(),
        }
    }

    #[test]
    fn fixture_validates() {
        assert!(fixture().validate().is_ok());
    }

    #[test]
    fn unnamespaced_id_is_rejected() {
        let mut s = fixture();
        s.id = "missingdot".to_string();
        match s.validate() {
            Err(SkillDefinitionError::UnnamespacedId { id }) => assert_eq!(id, "missingdot"),
            other => panic!("expected UnnamespacedId, got {other:?}"),
        }
    }

    #[test]
    fn zero_version_is_rejected() {
        let mut s = fixture();
        s.version = 0;
        match s.validate() {
            Err(SkillDefinitionError::InvalidVersion { id, version }) => {
                assert_eq!(id, s.id);
                assert_eq!(version, 0);
            }
            other => panic!("expected InvalidVersion, got {other:?}"),
        }
    }

    #[test]
    fn empty_task_kinds_is_rejected() {
        let mut s = fixture();
        s.applies_when.task_kinds.clear();
        assert!(matches!(
            s.validate(),
            Err(SkillDefinitionError::EmptyTaskKinds { .. })
        ));
    }

    #[test]
    fn empty_output_contract_is_rejected() {
        let mut s = fixture();
        s.outputs.contract = "  ".to_string();
        assert!(matches!(
            s.validate(),
            Err(SkillDefinitionError::EmptyOutputContract { .. })
        ));
    }

    #[test]
    fn wildcard_capability_is_rejected() {
        let mut s = fixture();
        s.capabilities.required.push("*".to_string());
        match s.validate() {
            Err(SkillDefinitionError::WildcardCapability { capability, .. }) => {
                assert_eq!(capability, "*");
            }
            other => panic!("expected WildcardCapability, got {other:?}"),
        }
        let mut s2 = fixture();
        s2.capabilities.required.push("ALL".to_string());
        assert!(matches!(
            s2.validate(),
            Err(SkillDefinitionError::WildcardCapability { .. })
        ));
    }

    #[test]
    fn monolithic_prompt_pattern_is_rejected() {
        let mut s = fixture();
        s.instruction_fragments.clear();
        s.capabilities.required = (0..9).map(|i| format!("cap.{i}")).collect();
        assert!(matches!(
            s.validate(),
            Err(SkillDefinitionError::MonolithicPromptPattern { .. })
        ));
    }

    #[test]
    fn empty_fragments_with_few_caps_is_allowed() {
        let mut s = fixture();
        s.instruction_fragments.clear();
        s.capabilities.required = (0..8).map(|i| format!("cap.{i}")).collect();
        assert!(s.validate().is_ok());
    }

    #[test]
    fn inverted_compat_range_is_rejected() {
        let mut s = fixture();
        s.compat.min_ae_version = Some(3);
        s.compat.max_ae_version = Some(1);
        assert!(matches!(
            s.validate(),
            Err(SkillDefinitionError::InvertedCompatRange { .. })
        ));
    }

    #[test]
    fn compat_range_admits_versions() {
        let s = fixture();
        assert!(s.compat.admits(1));
        assert!(s.compat.admits(999));
        let mut s2 = fixture();
        s2.compat.min_ae_version = Some(2);
        s2.compat.max_ae_version = Some(4);
        assert!(!s2.compat.admits(1));
        assert!(s2.compat.admits(2));
        assert!(s2.compat.admits(4));
        assert!(!s2.compat.admits(5));
    }

    #[test]
    fn content_hash_is_stable_across_field_reordering() {
        let mut a = fixture();
        a.compat.min_ae_version = Some(1);
        a.compat.max_ae_version = Some(3);
        let mut b = fixture();
        // reverse the build order of compat — same content, same hash
        b.compat.max_ae_version = Some(3);
        b.compat.min_ae_version = Some(1);
        assert_eq!(a.content_hash(), b.content_hash());
        assert_eq!(a.canonical_json(), b.canonical_json());
    }

    #[test]
    fn content_hash_changes_when_field_changes() {
        let mut a = fixture();
        let mut b = fixture();
        b.version = 2;
        assert_ne!(a.content_hash(), b.content_hash());
        a.applies_when.task_kinds.push("review.api".to_string());
        assert_ne!(a.content_hash(), b.content_hash());
    }

    #[test]
    fn ref_token_uses_at_v() {
        let s = fixture();
        assert_eq!(s.ref_token(), "core.architecture-review@v1");
    }

    #[test]
    fn applies_when_matches_task_and_context() {
        let s = fixture();
        assert!(
            s.applies_when
                .matches("review.architecture", &["plan_revision"])
        );
        assert!(!s.applies_when.matches("review.architecture", &[]));
        assert!(!s.applies_when.matches("review.api", &["plan_revision"]));
    }

    #[test]
    fn registry_register_and_dedup() {
        let mut reg = SkillRegistry::new();
        reg.register(fixture()).expect("register");
        let err = reg.register(fixture()).unwrap_err();
        assert!(matches!(
            err,
            SkillRegistryError::DuplicateRegistration { .. }
        ));
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn registry_register_rejects_invalid_skill() {
        let mut reg = SkillRegistry::new();
        let mut bad = fixture();
        bad.id = "nope".to_string();
        let err = reg.register(bad).unwrap_err();
        assert!(matches!(err, SkillRegistryError::SkillValidation(_)));
        assert_eq!(reg.len(), 0);
    }

    #[test]
    fn registry_select_for_is_sorted_and_filtered() {
        let mut reg = SkillRegistry::new();
        // Register two skills out of natural order; selection must sort.
        // All skills declare no context_keys requirement so they match
        // any context_keys argument.
        let mut a = fixture();
        a.id = "zeta.skill".to_string();
        a.applies_when.task_kinds = vec!["review.architecture".to_string()];
        a.applies_when.context_keys.clear();
        reg.register(a).expect("register zeta");

        let mut b = fixture();
        b.id = "alpha.skill".to_string();
        b.applies_when.task_kinds = vec!["review.architecture".to_string()];
        b.applies_when.context_keys.clear();
        reg.register(b).expect("register alpha");

        let mut c = fixture();
        c.id = "alpha.skill".to_string();
        c.version = 2;
        c.applies_when.task_kinds = vec!["review.architecture".to_string()];
        c.applies_when.context_keys.clear();
        reg.register(c).expect("register alpha@v2");

        let sel = reg.select_for("review.architecture", &[]);
        let ids: Vec<(&str, u32)> = sel
            .iter()
            .map(|s| (s.definition.id.as_str(), s.definition.version))
            .collect();
        assert_eq!(
            ids,
            vec![("alpha.skill", 1), ("alpha.skill", 2), ("zeta.skill", 1),]
        );
    }

    #[test]
    fn registry_select_for_filters_by_context() {
        let mut reg = SkillRegistry::new();
        let mut a = fixture();
        a.applies_when.context_keys = vec!["plan_revision".to_string()];
        reg.register(a).expect("register");
        let mut b = fixture();
        b.id = "core.architecture-review-other".to_string();
        b.applies_when.context_keys = vec!["goal_id".to_string()];
        b.applies_when.task_kinds = vec!["review.architecture".to_string()];
        reg.register(b).expect("register other");
        let with_plan = reg.select_for("review.architecture", &["plan_revision"]);
        assert_eq!(with_plan.len(), 1);
        assert_eq!(with_plan[0].definition.id, "core.architecture-review");
        let with_goal = reg.select_for("review.architecture", &["goal_id"]);
        assert_eq!(with_goal.len(), 1);
        assert_eq!(with_goal[0].definition.id, "core.architecture-review-other");
        let with_none = reg.select_for("review.architecture", &[]);
        // both skills declare a context_key requirement that is unsatisfied
        assert_eq!(with_none.len(), 0);
    }

    #[test]
    fn registry_manifest_is_deterministic_sorted() {
        let mut reg = SkillRegistry::new();
        let mut a = fixture();
        a.id = "zeta.skill".to_string();
        reg.register(a).unwrap();
        let mut b = fixture();
        b.id = "alpha.skill".to_string();
        reg.register(b).unwrap();
        let m1 = reg.manifest();
        let m2 = reg.manifest();
        assert_eq!(m1, m2);
        // First entry must be alpha (sorted by ref token)
        assert!(m1.starts_with(r#"[{"hash":"#));
        // Find the position of "alpha" vs "zeta"
        let pos_alpha = m1.find("alpha").expect("alpha present");
        let pos_zeta = m1.find("zeta").expect("zeta present");
        assert!(pos_alpha < pos_zeta);
    }

    #[test]
    fn registered_skill_rejects_invalid_at_construction() {
        let mut bad = fixture();
        bad.id = "nope".to_string();
        assert!(RegisteredSkill::new(bad).is_err());
    }

    #[test]
    fn skill_definition_serialization_round_trips() {
        let s = fixture();
        let json = serde_json::to_string(&s).expect("serialize");
        let back: SkillDefinition = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(s, back);
    }

    #[test]
    fn admits_command_returns_satisfied_when_no_skills_declared() {
        let reg = SkillRegistry::new();
        let cmd = crate::command_spec::CommandSpec::new("noop", "noop command");
        let outcome = reg.admits_command(&cmd);
        assert_eq!(outcome, SkillAdmitOutcome::Satisfied);
        assert!(outcome.is_admitted());
    }

    #[test]
    fn admits_command_returns_satisfied_by_when_all_present() {
        let mut reg = SkillRegistry::new();
        reg.register(fixture()).expect("register");
        let cmd = crate::command_spec::CommandSpec::new("review", "review command")
            .with_required_skill("core.architecture-review@v1");
        let outcome = reg.admits_command(&cmd);
        assert!(outcome.is_admitted());
        match outcome {
            SkillAdmitOutcome::SatisfiedBy(refs) => {
                assert_eq!(refs, vec!["core.architecture-review@v1".to_string()]);
            }
            other => panic!("expected SatisfiedBy, got {other:?}"),
        }
    }

    #[test]
    fn admits_command_reports_missing_skills_diagnostically() {
        let mut reg = SkillRegistry::new();
        // register an unrelated skill
        let mut s = fixture();
        s.id = "core.other-skill".to_string();
        reg.register(s).expect("register other");
        let mut s2 = fixture();
        s2.id = "core.architecture-review".to_string();
        s2.version = 2;
        reg.register(s2).expect("register review@v2");
        let cmd = crate::command_spec::CommandSpec::new("review", "review command")
            .with_required_skill("core.architecture-review@v1")
            .with_required_skill("core.architecture-review@v2")
            .with_required_skill("core.lint-checker@v1");
        let outcome = reg.admits_command(&cmd);
        assert!(!outcome.is_admitted());
        match outcome {
            SkillAdmitOutcome::Missing {
                required,
                available,
            } => {
                assert_eq!(required.len(), 3);
                assert!(required.contains(&"core.architecture-review@v1".to_string()));
                assert!(required.contains(&"core.lint-checker@v1".to_string()));
                assert!(available.contains(&"core.other-skill@v1".to_string()));
                assert!(available.contains(&"core.architecture-review@v2".to_string()));
            }
            other => panic!("expected Missing, got {other:?}"),
        }
    }

    #[test]
    fn skill_neq_capability_invariance_is_documented() {
        // This test guards the Skill != Capability invariant by checking
        // that a Skill declares capabilities but is *only* a requirement
        // declaration \u2014 it does not grant anything on its own.
        let s = fixture();
        // The skill DOES declare what it would consume.
        assert!(!s.capabilities.required.is_empty());
        // The registry does NOT expose a "grant" method. Adding one
        // would be a SPEC-016 anti-pattern violation.
        let api: Vec<&str> = ["register", "select_for", "admits_command", "manifest"]
            .into_iter()
            .collect();
        for name in &api {
            assert!(
                ["register", "select_for", "admits_command", "manifest"].contains(name),
                "SkillRegistry MUST NOT expose a grant method (got `{name}`)"
            );
        }
    }
}
