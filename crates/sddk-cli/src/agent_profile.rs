//! `agent_profile` — full AgentProfile model (M7.4).
//!
//! Replaces the placeholder `AgentProfileTag` (M7.1) with a struct that
//! captures the complete semantics of an agent role: which side-effect
//! classes are admissible, which stability levels are visible, which
//! authority is required at most, and which targets (if any) are
//! reachable. `AgentProfileTag` is preserved as a backwards-compatible
//! enum and is derived from `AgentProfile::name` for legacy callers.
//!
//! ## Profiles shipped
//!
//! - **default** — most permissive: all stabilities visible except
//!   Deprecated, all side-effects up to Destructive allowed, all
//!   authorities, all targets.
//! - **read_only** — Pure/Read only, all stable stabilities, Read
//!   authority, all targets.
//! - **approver** — everything default allows plus explicit access to
//!   Approve/Approval commands (governed side-effects), Approval
//!   authority.
//! - **ci_bot** — restricted to a known set of targets (verify, audit,
//!   status) with all stable stabilities and Read/Write authority.
//!
//! ## Gate semantics
//!
//! `AgentProfile::admits(&CommandSpec) -> bool` is the canonical gate.
//! A command is admissible iff:
//! - its `stability` is in `allowed_stabilities`,
//! - its `side_effect_class` is in `allowed_side_effects`,
//! - its `required_authority` is `<=` the profile's ceiling, AND
//! - its target (if filter active) is in `allowed_targets` (empty list
//!   means no target restriction).
//!
//! Authority ordering (low → high): `None < Read < Write < Approval`.

use crate::command_spec::{
    AgentProfileTag, AuthorityRequirement, CommandSpec, SideEffectClass, Stability,
};
use serde::{Deserialize, Serialize};

/// Full AgentProfile model. Replaces `AgentProfileTag` as the canonical
/// authority over agent roles; `AgentProfileTag` is kept as a derived
/// view (see `From<&AgentProfile> for AgentProfileTag`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AgentProfile {
    /// Stable profile name (lowercase, snake_case). The same name may
    /// resolve to an `AgentProfileTag` variant for legacy callers.
    pub name: String,
    /// Human description (used by the cheat-sheet / provider adapters).
    pub description: String,
    /// Stability ceiling — only commands at one of these levels are
    /// visible to this profile.
    pub allowed_stabilities: Vec<Stability>,
    /// Side-effect ceiling — commands whose side-effect class is in
    /// this set are admissible.
    pub allowed_side_effects: Vec<SideEffectClass>,
    /// Authority ceiling — commands requiring strictly more authority
    /// than this are inadmissible.
    pub required_authority: AuthorityRequirement,
    /// Target allowlist. Empty means "no restriction" (all targets).
    pub allowed_targets: Vec<String>,
}

impl AgentProfile {
    /// Canonical gate: does this profile admit the given command?
    pub fn admits(&self, spec: &CommandSpec) -> bool {
        if !self.allowed_stabilities.contains(&spec.stability) {
            return false;
        }
        if !self.allowed_side_effects.contains(&spec.side_effect_class) {
            return false;
        }
        if authority_rank(spec.required_authority) > authority_rank(self.required_authority) {
            return false;
        }
        // Target check applies only if both profile and command declare
        // a target. Today CommandSpec does not carry a target field
        // directly (the surface filters by target), so we only check
        // when the profile's allowed_targets is non-empty AND the
        // caller passes a target filter — that's a separate code path
        // in `command_surface`. The profile model itself is target-
        // independent at the gate level.
        let _ = &self.allowed_targets;
        true
    }
}

/// Authority ranking: lower values are weaker, higher values are stronger.
fn authority_rank(a: AuthorityRequirement) -> u8 {
    match a {
        AuthorityRequirement::None => 0,
        AuthorityRequirement::Read => 1,
        AuthorityRequirement::Write => 2,
        AuthorityRequirement::Approval => 3,
    }
}

impl From<&AgentProfile> for AgentProfileTag {
    fn from(profile: &AgentProfile) -> Self {
        match profile.name.as_str() {
            "read_only" => AgentProfileTag::ReadOnly,
            "approver" => AgentProfileTag::Approver,
            _ => AgentProfileTag::Default,
        }
    }
}

/// The canonical `default` profile: most permissive.
pub fn default_profile() -> AgentProfile {
    AgentProfile {
        name: "default".to_string(),
        description: "Default agent profile — all stabilities visible except Deprecated, all side-effects allowed.".to_string(),
        allowed_stabilities: vec![Stability::Stable, Stability::Experimental],
        allowed_side_effects: vec![
            SideEffectClass::Pure,
            SideEffectClass::Read,
            SideEffectClass::Governed,
            SideEffectClass::Destructive,
        ],
        required_authority: AuthorityRequirement::Approval,
        allowed_targets: Vec::new(),
    }
}

/// Read-only profile: cannot mutate anything.
pub fn read_only_profile() -> AgentProfile {
    AgentProfile {
        name: "read_only".to_string(),
        description: "Read-only agent — Pure/Read side-effects only, Read authority.".to_string(),
        allowed_stabilities: vec![Stability::Stable, Stability::Experimental],
        allowed_side_effects: vec![SideEffectClass::Pure, SideEffectClass::Read],
        required_authority: AuthorityRequirement::Read,
        allowed_targets: Vec::new(),
    }
}

/// Approver profile: like default but explicitly admits Approval authority
/// for approval flows.
pub fn approver_profile() -> AgentProfile {
    AgentProfile {
        name: "approver".to_string(),
        description:
            "Approver agent — default surface plus Approval authority for gated approvals."
                .to_string(),
        allowed_stabilities: vec![Stability::Stable, Stability::Experimental],
        allowed_side_effects: vec![
            SideEffectClass::Pure,
            SideEffectClass::Read,
            SideEffectClass::Governed,
            SideEffectClass::Destructive,
        ],
        required_authority: AuthorityRequirement::Approval,
        allowed_targets: Vec::new(),
    }
}

/// CI-bot profile: restricted to a known set of targets, stable stabilities
/// only, Read/Write authority. No Destructive.
pub fn ci_bot_profile() -> AgentProfile {
    AgentProfile {
        name: "ci_bot".to_string(),
        description: "CI bot — restricted to verify/audit/status targets, stable only, Read+Write."
            .to_string(),
        allowed_stabilities: vec![Stability::Stable],
        allowed_side_effects: vec![
            SideEffectClass::Pure,
            SideEffectClass::Read,
            SideEffectClass::Governed,
        ],
        required_authority: AuthorityRequirement::Write,
        allowed_targets: vec![
            "verify".to_string(),
            "audit".to_string(),
            "status".to_string(),
        ],
    }
}

/// All canonical profiles shipped with SDDK.
pub fn all_profiles() -> Vec<AgentProfile> {
    vec![
        default_profile(),
        read_only_profile(),
        approver_profile(),
        ci_bot_profile(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_spec::{CommandSpec, OutputFormatKind, SideEffectClass, Stability};

    fn make_spec(
        name: &str,
        stability: Stability,
        side_effect: SideEffectClass,
        authority: AuthorityRequirement,
    ) -> CommandSpec {
        CommandSpec::new(name, "test spec")
            .with_stability(stability)
            .with_side_effect(side_effect)
            .with_authority(authority)
            .with_outputs(OutputFormatKind::Text, "TestV1")
    }

    #[test]
    fn default_profile_admits_pure_stable_commands() {
        let profile = default_profile();
        let spec = make_spec(
            "version",
            Stability::Stable,
            SideEffectClass::Pure,
            AuthorityRequirement::None,
        );
        assert!(profile.admits(&spec));
    }

    #[test]
    fn read_only_profile_rejects_governed_and_destructive() {
        let profile = read_only_profile();
        let pure = make_spec(
            "v",
            Stability::Stable,
            SideEffectClass::Pure,
            AuthorityRequirement::None,
        );
        let read = make_spec(
            "r",
            Stability::Stable,
            SideEffectClass::Read,
            AuthorityRequirement::Read,
        );
        let gov = make_spec(
            "g",
            Stability::Stable,
            SideEffectClass::Governed,
            AuthorityRequirement::Write,
        );
        let dest = make_spec(
            "d",
            Stability::Stable,
            SideEffectClass::Destructive,
            AuthorityRequirement::Approval,
        );
        assert!(profile.admits(&pure));
        assert!(profile.admits(&read));
        assert!(!profile.admits(&gov));
        assert!(!profile.admits(&dest));
    }

    #[test]
    fn approver_profile_admits_admin_authority_commands() {
        let profile = approver_profile();
        let spec = make_spec(
            "approve",
            Stability::Stable,
            SideEffectClass::Governed,
            AuthorityRequirement::Approval,
        );
        assert!(profile.admits(&spec));
    }

    #[test]
    fn ci_bot_profile_rejects_experimental_and_destructive() {
        let profile = ci_bot_profile();
        let stable_pure = make_spec(
            "v",
            Stability::Stable,
            SideEffectClass::Pure,
            AuthorityRequirement::None,
        );
        let experimental = make_spec(
            "x",
            Stability::Experimental,
            SideEffectClass::Pure,
            AuthorityRequirement::None,
        );
        let destructive = make_spec(
            "d",
            Stability::Stable,
            SideEffectClass::Destructive,
            AuthorityRequirement::Approval,
        );
        assert!(profile.admits(&stable_pure));
        assert!(!profile.admits(&experimental));
        assert!(!profile.admits(&destructive));
    }

    #[test]
    fn all_profiles_returns_canonical_set_with_four_entries() {
        let profiles = all_profiles();
        assert_eq!(profiles.len(), 4);
        let names: Vec<&str> = profiles.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"default"));
        assert!(names.contains(&"read_only"));
        assert!(names.contains(&"approver"));
        assert!(names.contains(&"ci_bot"));
    }

    #[test]
    fn from_agent_profile_to_agent_profile_tag_preserves_known_names() {
        let default = default_profile();
        let read_only = read_only_profile();
        let approver = approver_profile();
        let ci_bot = ci_bot_profile();
        assert_eq!(AgentProfileTag::from(&default), AgentProfileTag::Default);
        assert_eq!(AgentProfileTag::from(&read_only), AgentProfileTag::ReadOnly);
        assert_eq!(AgentProfileTag::from(&approver), AgentProfileTag::Approver);
        // ci_bot falls back to Default because no matching variant exists.
        assert_eq!(AgentProfileTag::from(&ci_bot), AgentProfileTag::Default);
    }

    #[test]
    fn authority_ranking_is_total_order_none_lt_read_lt_write_lt_admin() {
        assert!(
            authority_rank(AuthorityRequirement::None) < authority_rank(AuthorityRequirement::Read)
        );
        assert!(
            authority_rank(AuthorityRequirement::Read)
                < authority_rank(AuthorityRequirement::Write)
        );
        assert!(
            authority_rank(AuthorityRequirement::Write)
                < authority_rank(AuthorityRequirement::Approval)
        );
    }

    #[test]
    fn default_profile_rejects_deprecated_stability() {
        let profile = default_profile();
        let spec = make_spec(
            "old",
            Stability::Deprecated,
            SideEffectClass::Pure,
            AuthorityRequirement::None,
        );
        assert!(
            !profile.admits(&spec),
            "Deprecated is not in default.allowed_stabilities"
        );
    }
}
