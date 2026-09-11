// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
//! SPIKE AX-S4 — provider adapter portability.
//!
//! Question (SPIKES.md AX-S4): run one reviewer profile/task/fixture
//! through two provider adapters (fake-compatible) using identical
//! semantic contracts. Identify provider-specific data that truly
//! belongs outside `AgentProfile`.
//!
//! Method: define the `InstructionsRenderer` port (the seam implied by
//! SPEC-014's "rendered by a provider adapter into native sections"),
//! then two deliberately different adapters — `SectionedRenderer`
//! (Anthropic/OpenAI-style system+developer+user sections) and
//! `FlatRenderer` (single-block providers) — and verify that both
//! produce byte-different but semantically identical renders of the
//! same `EffectiveInstructions` for the same reviewer profile and
//! fixture task. Semantic identity = same fragment set (semantic keys,
//! strengths, texts) in the same order, regardless of layout.
//!
//! Findings recorded in
//! `docs/architecture/spikes/AX-S4-provider-adapter-portability.md`.

use crate::instruction_compiler::{
    CompileInputs, EffectiveInstructions, InstructionCompiler, InstructionStrength,
};

/// What a provider adapter must produce from compiled instructions.
///
/// The port owns *layout only*: it may regroup and reformat fragments
/// but must not drop, reword, or reorder them semantically. Providers
/// that need extra transport metadata (model name, temperature, token
/// budget) take it through their own constructor — never through
/// `AgentProfile`, which stays purely semantic (stability / side-effect
/// / authority / target ceilings).
#[allow(dead_code)] // spike port: consumers arrive with real providers
pub trait InstructionsRenderer {
    /// The provider-native rendering of the compiled instructions.
    fn render(&self, instructions: &EffectiveInstructions) -> RenderedPrompt;

    /// Provider identity for telemetry/debug (e.g. "sectioned-fake").
    fn name(&self) -> &'static str;
}

/// One provider-native prompt section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptSection {
    /// Section name in the provider's native taxonomy
    /// (e.g. "system", "developer", "user", "body").
    pub kind: String,
    /// Full rendered text of the section.
    pub text: String,
}

/// Provider-native prompt: layout varies by adapter, semantics do not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedPrompt {
    /// Which adapter produced this.
    pub adapter: &'static str,
    /// Provider-native sections, in render order.
    pub sections: Vec<PromptSection>,
}

impl RenderedPrompt {
    /// Semantic fingerprint: the ordered (key, strength, text) triples
    /// recovered from the render. Two renders of the same instructions
    /// must produce equal fingerprints regardless of adapter layout.
    pub fn semantic_fingerprint(&self) -> Vec<(String, String, String)> {
        // Fragments render as "KEY\tSTRENGTH\tTEXT" lines; recover them.
        let mut out = Vec::new();
        for section in &self.sections {
            for line in section.text.lines() {
                if let Some((key, rest)) = line.split_once('\t')
                    && let Some((strength, text)) = rest.split_once('\t')
                {
                    out.push((key.to_string(), strength.to_string(), text.to_string()));
                }
            }
        }
        out
    }
}

fn strength_tag(s: InstructionStrength) -> &'static str {
    match s {
        InstructionStrength::Invariant => "invariant",
        InstructionStrength::Policy => "policy",
        InstructionStrength::Task => "task",
        InstructionStrength::ProjectMandatory => "project-mandatory",
        InstructionStrength::ProjectAdvisory => "project-advisory",
        InstructionStrength::Skill => "skill",
        InstructionStrength::Hint => "hint",
    }
}

fn fragment_line(f: &crate::instruction_compiler::InstructionFragment) -> String {
    format!(
        "{}\t{}\t{}",
        f.semantic_key,
        strength_tag(f.strength),
        f.text
    )
}

/// Adapter A: three-section layout (system + developer + user), in the
/// style of providers that separate role sections natively.
#[derive(Debug, Default)]
pub struct SectionedRenderer;

impl InstructionsRenderer for SectionedRenderer {
    fn name(&self) -> &'static str {
        "sectioned-fake"
    }

    fn render(&self, i: &EffectiveInstructions) -> RenderedPrompt {
        let mut system = String::from("# system\n");
        let mut developer = String::from("# developer\n");
        let mut user = String::from("# user\n");
        for f in &i.sections {
            let line = fragment_line(f);
            match f.strength {
                InstructionStrength::Invariant | InstructionStrength::Policy => {
                    system.push_str(&line);
                    system.push('\n');
                }
                InstructionStrength::Task | InstructionStrength::ProjectMandatory => {
                    developer.push_str(&line);
                    developer.push('\n');
                }
                _ => {
                    user.push_str(&line);
                    user.push('\n');
                }
            }
        }
        RenderedPrompt {
            adapter: self.name(),
            sections: vec![
                PromptSection {
                    kind: "system".into(),
                    text: system,
                },
                PromptSection {
                    kind: "developer".into(),
                    text: developer,
                },
                PromptSection {
                    kind: "user".into(),
                    text: user,
                },
            ],
        }
    }
}

/// Adapter B: single-block layout, in the style of providers with one
/// flat prompt body.
#[derive(Debug, Default)]
pub struct FlatRenderer;

impl InstructionsRenderer for FlatRenderer {
    fn name(&self) -> &'static str {
        "flat-fake"
    }

    fn render(&self, i: &EffectiveInstructions) -> RenderedPrompt {
        let mut body = String::from("# body\n");
        for f in &i.sections {
            body.push_str(&fragment_line(f));
            body.push('\n');
        }
        RenderedPrompt {
            adapter: self.name(),
            sections: vec![PromptSection {
                kind: "body".into(),
                text: body,
            }],
        }
    }
}

/// Fixture: compile the reviewer task's instructions once, render twice.
pub fn compile_reviewer_fixture() -> EffectiveInstructions {
    let mut inputs = CompileInputs::default();
    inputs
        .invariants
        .entries
        .push(crate::instruction_compiler::KernelInvariant {
            id: "writes-cas".into(),
            text: "all writes are content-addressed".into(),
        });
    inputs
        .policies
        .entries
        .push(crate::instruction_compiler::PolicyInstruction {
            id: "prefer-bundle".into(),
            text: "prefer bundle evidence".into(),
        });
    inputs
        .task
        .requirements
        .push(crate::instruction_compiler::TaskRequirement {
            id: "review-scope".into(),
            text: "review only the diff, not the whole tree".into(),
            mandatory: true,
        });
    inputs
        .project
        .entries
        .push(crate::instruction_compiler::ProjectInstruction {
            id: "review-scope-restrict".into(),
            text: "review only the diff, not the whole tree; skip generated files".into(),
            mandatory: true,
        });
    InstructionCompiler::new()
        .compile(&inputs)
        .expect("reviewer fixture compiles")
}

/// Run the same fixture through both adapters and assert semantic
/// equivalence. Returns the renders for inspection.
pub fn run_portability_check() -> (RenderedPrompt, RenderedPrompt) {
    let instructions = compile_reviewer_fixture();
    let a = SectionedRenderer.render(&instructions);
    let b = FlatRenderer.render(&instructions);
    assert_eq!(
        a.semantic_fingerprint(),
        b.semantic_fingerprint(),
        "adapters must preserve the semantic contract"
    );
    (a, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_adapters_preserve_the_semantic_contract() {
        let (a, b) = run_portability_check();
        assert_ne!(a.adapter, b.adapter);
        assert_ne!(
            a.sections, b.sections,
            "layouts must differ to prove portability"
        );
        assert_eq!(
            a.semantic_fingerprint(),
            b.semantic_fingerprint(),
            "semantic fingerprints must match across adapters"
        );
    }

    #[test]
    fn fingerprint_covers_every_compiled_fragment() {
        let instructions = compile_reviewer_fixture();
        let (a, _) = run_portability_check();
        assert_eq!(
            a.semantic_fingerprint().len(),
            instructions.len(),
            "every fragment must survive rendering"
        );
    }

    #[test]
    fn fingerprint_order_matches_compiler_order() {
        // The compiler's deterministic order (strength desc, key asc)
        // must be preserved by both adapters — layout changes must not
        // reorder semantics.
        let instructions = compile_reviewer_fixture();
        let expected: Vec<String> = instructions
            .sections
            .iter()
            .map(|f| f.semantic_key.clone())
            .collect();
        let (a, b) = run_portability_check();
        let keys_a: Vec<String> = a
            .semantic_fingerprint()
            .into_iter()
            .map(|(k, _, _)| k)
            .collect();
        let keys_b: Vec<String> = b
            .semantic_fingerprint()
            .into_iter()
            .map(|(k, _, _)| k)
            .collect();
        assert_eq!(keys_a, expected);
        assert_eq!(keys_b, expected);
    }

    #[test]
    fn agent_profile_carries_no_provider_transport_data() {
        // AX-S4's second question: provider-specific data must live
        // outside AgentProfile. The struct has only semantic ceilings
        // (name/description/stability/side-effect/authority/targets) —
        // no model name, temperature, token budget, or endpoint fields.
        // Pin that shape: constructing a profile from semantic data only
        // must be possible and the struct must not grow transport
        // fields (this test fails if someone adds them without a
        // dedicated port).
        let profile = crate::agent_profile::AgentProfile {
            name: "reviewer".into(),
            description: "reviews diffs".into(),
            allowed_stabilities: vec![crate::command_spec::Stability::Stable],
            allowed_side_effects: vec![crate::command_spec::SideEffectClass::Pure],
            required_authority: crate::command_spec::AuthorityRequirement::None,
            allowed_targets: vec!["verify".into()],
        };
        assert_eq!(profile.name, "reviewer");
        // Provider transport data belongs to the renderer port instead:
        assert_eq!(SectionedRenderer.name(), "sectioned-fake");
        assert_eq!(FlatRenderer.name(), "flat-fake");
    }
}
