//! Deterministic compiler from legacy `WorkflowManifest` to kernel-pure `WorkflowIR`.
//!
//! This module is **pure**: no LLM, no I/O, no wall-clock. Same `(manifest, template)`
//! always produces the same `WorkflowIR` byte-stream and content hash.
//!
//! ## 8-stage pipeline
//!
//! Stages compose via explicit `?`-propagation; no visitor, no fold. The stage set is
//! closed and heterogeneous — a visitor trait would add a dispatch layer with no extension
//! point. Only S3/S5 recurse, and only over `Operator::referenced_ids()`.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use sha2::{Digest, Sha256};

use crate::workflow::WorkflowManifest;
use crate::workflow_ir::{
    Budgets, CapabilityId, CompileError, GuardExpr, IrId, Operator, OperatorId, Provenance,
    SCHEMA_VERSION, TemplateRef, WorkflowIR, WorkflowTemplate,
};

/// The SDDK workflow compiler — deterministic, LLM-free manifest → IR translation.
///
/// # Example
///
/// ```ignore
/// let compiler = WorkflowCompiler;
/// let ir = compiler.compile(&manifest, &template)?;
/// ```
#[derive(Debug, Clone, Default)]
pub struct WorkflowCompiler;

impl WorkflowCompiler {
    /// Compiles a legacy `WorkflowManifest` into a kernel-pure `WorkflowIR`.
    ///
    /// This is a pure function: same inputs always produce the same `ir.compute_content_hash()`.
    pub fn compile(
        &self,
        manifest: &WorkflowManifest,
        template: &WorkflowTemplate,
    ) -> Result<WorkflowIR, CompileError> {
        self.compile_with_known_digests(manifest, template, &BTreeSet::new())
    }

    /// Like [`compile`](Self::compile) but checks for hash collisions against a caller-supplied
    /// set of known digests.
    ///
    /// This is the total entry point; `compile` is a convenience wrapper that delegates here
    /// with an empty `known` set.
    pub fn compile_with_known_digests(
        &self,
        manifest: &WorkflowManifest,
        template: &WorkflowTemplate,
        known: &BTreeSet<crate::workflow_ir::ContentHash>,
    ) -> Result<WorkflowIR, CompileError> {
        let mut ctx = CompileCtx::new(manifest, template);
        self.run_pipeline(&mut ctx, known)?;
        Ok(ctx.into_ir())
    }

    // ── Pipeline stages ──────────────────────────────────────────────────────

    fn run_pipeline(
        &self,
        ctx: &mut CompileCtx,
        known: &BTreeSet<crate::workflow_ir::ContentHash>,
    ) -> Result<(), CompileError> {
        // S1: preflight — schema + empty allowlist check
        self.s1_preflight(ctx)?;
        // S2: capability reconciliation
        self.s2_reconcile_capabilities(ctx)?;
        // S3: phase → operator mapping
        self.s3_map_phases(ctx)?;
        // S4: synthesize edges (root + guards)
        self.s4_synthesize_edges(ctx)?;
        // S5: cycle detection
        self.s5_detect_cycles(ctx)?;
        // S6: budget clamping
        self.s6_clamp_budgets(ctx)?;
        // S7: provenance
        self.s7_emit_provenance(ctx)?;
        // S8: content addressing
        self.s8_content_address(ctx, known)?;
        Ok(())
    }

    /// S1 — schema version + template validation.
    fn s1_preflight(&self, ctx: &mut CompileCtx) -> Result<(), CompileError> {
        // Manifest schema version must be 1
        if ctx.manifest.schema_version != 1 {
            return Err(CompileError::UnsupportedSchemaVersion {
                got: ctx.manifest.schema_version as u32,
                want: SCHEMA_VERSION,
            });
        }
        // Template schema version must be SCHEMA_VERSION
        if ctx.template.schema_version != SCHEMA_VERSION {
            return Err(CompileError::UnsupportedSchemaVersion {
                got: ctx.template.schema_version,
                want: SCHEMA_VERSION,
            });
        }
        // Empty allowlist is rejected
        if ctx.template.capability_allowlist.is_empty() {
            return Err(CompileError::EmptyCapabilityAllowlist);
        }
        // All expansion permissions must be valid (closed set)
        for perm in &ctx.template.expansion_permissions {
            if !perm.is_known_permission() {
                return Err(CompileError::ExpansionNotAllowed);
            }
        }
        // Template budgets must fit within hard limits
        if !ctx.template.budgets.fits_within(&Budgets::hard_limits()) {
            return Err(CompileError::BudgetExceedsLimit);
        }
        Ok(())
    }

    /// S2 — capability reconciliation: derive required capabilities from phases.
    fn s2_reconcile_capabilities(
        &self,
        ctx: &mut CompileCtx,
    ) -> Result<BTreeMap<CapabilityId, crate::workflow::CapabilityDef>, CompileError> {
        let mut required: BTreeSet<CapabilityId> = BTreeSet::new();
        for path_def in ctx.manifest.paths.values() {
            for phase_str in &path_def.phases {
                if let Some(cap) = phase_str_to_capability(phase_str) {
                    required.insert(cap);
                }
            }
        }
        // Verify all required capabilities are in the template allowlist
        for cap in &required {
            if !ctx.template.capability_allowlist.contains(cap) {
                return Err(CompileError::CapabilityNotInAllowlist(cap.clone()));
            }
        }
        let cap_defs = BTreeMap::new();
        Ok(cap_defs)
    }

    /// S3 — map phases to operators, converting manifest HashMap → BTreeMap for determinism.
    fn s3_map_phases(
        &self,
        ctx: &mut CompileCtx,
    ) -> Result<BTreeMap<String, Vec<(OperatorId, Operator)>>, CompileError> {
        // Deterministic iteration: collect manifest HashMap keys into BTreeMap
        let mut paths: BTreeMap<String, &crate::workflow::PathDef> = BTreeMap::new();
        for (k, v) in &ctx.manifest.paths {
            paths.insert(k.clone(), v);
        }

        let mut result: BTreeMap<String, Vec<(OperatorId, Operator)>> = BTreeMap::new();

        for (path_name, path_def) in paths {
            let mut ops = Vec::new();
            for (i, phase_str) in path_def.phases.iter().enumerate() {
                let Some(cap) = phase_str_to_capability(phase_str) else {
                    return Err(CompileError::OperatorNotAllowed(CapabilityId(
                        phase_str.clone(),
                    )));
                };
                let op_id = OperatorId(format!("{path_name}#{i:04}"));
                let op = Operator::Task {
                    capability: cap,
                    inputs: BTreeMap::new(),
                };
                ops.push((op_id, op));
            }
            result.insert(path_name, ops);
        }

        ctx.operators_by_path = Some(result);
        Ok(ctx.operators_by_path.take().unwrap())
    }

    /// S4 — synthesize edges: root Sequence (1 path) or Choice (N paths), attach guards.
    fn s4_synthesize_edges(&self, ctx: &mut CompileCtx) -> Result<(), CompileError> {
        let paths = ctx.operators_by_path.take().unwrap_or_default();
        let path_names: BTreeSet<String> = paths.keys().cloned().collect();

        if paths.is_empty() {
            // Empty manifest
            let root_id = OperatorId("root#seq".to_string());
            ctx.ir
                .operators
                .insert(root_id.clone(), Operator::Sequence { body: vec![] });
            ctx.root_id = Some(root_id);
            return Ok(());
        }

        if path_names.len() == 1 {
            // Single path → root is the Sequence of that path
            let only_path = path_names.iter().next().unwrap();
            let seq_id = OperatorId(format!("{only_path}#seq"));
            let task_ids: Vec<OperatorId> = paths
                .get(only_path)
                .unwrap()
                .iter()
                .map(|(id, _)| id.clone())
                .collect();
            ctx.ir
                .operators
                .insert(seq_id.clone(), Operator::Sequence { body: task_ids });
            ctx.root_id = Some(seq_id);
        } else {
            // Multiple paths → root is Choice over path Sequences
            let mut choice_branches: BTreeMap<String, OperatorId> = BTreeMap::new();
            for path_name in &path_names {
                let seq_id = OperatorId(format!("{path_name}#seq"));
                let task_ids: Vec<OperatorId> = paths
                    .get(path_name)
                    .unwrap()
                    .iter()
                    .map(|(id, _)| id.clone())
                    .collect();
                ctx.ir
                    .operators
                    .insert(seq_id.clone(), Operator::Sequence { body: task_ids });
                choice_branches.insert(path_name.clone(), seq_id);
            }
            let root_id = OperatorId("root#choice".to_string());
            ctx.ir.operators.insert(
                root_id.clone(),
                Operator::Choice {
                    branches: choice_branches,
                },
            );
            ctx.root_id = Some(root_id);
        }

        // Attach guards from transitions
        for transition in &ctx.manifest.transitions {
            if transition.requires.is_empty() {
                continue;
            }
            let Some(to_phase) = &transition.to.phase else {
                continue;
            };
            let phase_str = phase_to_string(to_phase);

            // Find operator ID for the target phase in the last path containing it
            let target_op_id: Option<OperatorId> = paths
                .iter()
                .rev() // deterministic: last path wins
                .find_map(|(_path_name, ops)| {
                    ops.iter()
                        .find(|(_, op)| {
                            if let Operator::Task { capability, .. } = op {
                                capability.0 == phase_str
                            } else {
                                false
                            }
                        })
                        .map(|(id, _)| id.clone())
                });

            if let Some(op_id) = target_op_id {
                let expr_parts: Vec<String> = transition
                    .requires
                    .iter()
                    .map(|r| match r {
                        crate::workflow::Requirement::Simple(name) => {
                            format!("requires({name})")
                        }
                        crate::workflow::Requirement::Structured { kind, name } => {
                            format!("requires({kind}:{name})")
                        }
                    })
                    .collect();
                let expr = expr_parts.join(",");
                ctx.ir.guards.insert(op_id, GuardExpr { expr });
            }
        }

        ctx.operators_by_path = Some(paths);
        Ok(())
    }

    /// S5 — iterative DFS cycle detection (white/grey/black coloring).
    fn s5_detect_cycles(&self, ctx: &mut CompileCtx) -> Result<(), CompileError> {
        if ctx.ir.operators.is_empty() {
            return Ok(());
        }

        // Find roots: operators never referenced by any other operator
        let all_ids: BTreeSet<OperatorId> = ctx.ir.operators.keys().cloned().collect();
        let referenced_ids: BTreeSet<OperatorId> = ctx
            .ir
            .operators
            .values()
            .flat_map(|op| op.referenced_ids())
            .collect();
        let roots: Vec<OperatorId> = all_ids.difference(&referenced_ids).cloned().collect();

        if roots.is_empty() && !ctx.ir.operators.is_empty() {
            return Err(CompileError::CycleDetected);
        }

        // DFS with color marking: white=0, grey=1, black=2
        let mut color: BTreeMap<OperatorId, u8> = BTreeMap::new();
        let mut stack: Vec<OperatorId> = roots;

        while let Some(current) = stack.pop() {
            let c = color.entry(current.clone()).or_insert(0);
            if *c == 2 {
                continue;
            }
            if *c == 1 {
                return Err(CompileError::CycleDetected);
            }
            *c = 1;

            if let Some(op) = ctx.ir.operators.get(&current) {
                for child_id in op.referenced_ids() {
                    if ctx.ir.operators.contains_key(&child_id) {
                        stack.push(child_id);
                    }
                }
            }

            *color.get_mut(&current).unwrap() = 2;
        }

        Ok(())
    }

    /// S6 — clamp template budgets against hard limits.
    fn s6_clamp_budgets(&self, ctx: &mut CompileCtx) -> Result<(), CompileError> {
        let hard = Budgets::hard_limits();
        let clamped = Budgets {
            max_wall_ms: ctx.template.budgets.max_wall_ms.min(hard.max_wall_ms),
            max_tokens: ctx.template.budgets.max_tokens.min(hard.max_tokens),
            max_cost_micros: ctx
                .template
                .budgets
                .max_cost_micros
                .min(hard.max_cost_micros),
            max_depth: ctx.template.budgets.max_depth.min(hard.max_depth),
            max_nodes: ctx.template.budgets.max_nodes.min(hard.max_nodes),
            remaining_tokens: ctx.template.budgets.remaining_tokens,
            no_progress_threshold: ctx.template.budgets.no_progress_threshold,
        };

        if !clamped.fits_within(&hard) {
            return Err(CompileError::BudgetExceedsLimit);
        }

        ctx.ir.budgets = clamped;
        Ok(())
    }

    /// S7 — provenance metadata.
    fn s7_emit_provenance(&self, ctx: &mut CompileCtx) -> Result<(), CompileError> {
        let prompt_bytes =
            serde_json::to_vec(&ctx.manifest).expect("WorkflowManifest is always serializable");
        let prompt_hash = format!("sha256:{:064x}", Sha256::digest(&prompt_bytes));

        let policy_bytes =
            serde_json::to_vec(&ctx.template.policies).expect("Policies is always serializable");
        let policy_hash = format!("sha256:{:064x}", Sha256::digest(&policy_bytes));

        ctx.ir.provenance = Provenance {
            generated_by: "sddk.kernel.compiler".to_string(),
            prompt_hash,
            model_hash: format!("sha256:{}", "0".repeat(64)),
            policy_hash,
        };

        Ok(())
    }

    /// S8 — content addressing with collision check.
    fn s8_content_address(
        &self,
        ctx: &mut CompileCtx,
        known: &BTreeSet<crate::workflow_ir::ContentHash>,
    ) -> Result<(), CompileError> {
        let hash = ctx.ir.compute_content_hash();
        if known.contains(&hash) {
            return Err(CompileError::HashCollision);
        }
        ctx.ir.ir_id = Some(IrId(hash));
        Ok(())
    }
}

// ── Compile context ─────────────────────────────────────────────────────────

/// Private accumulator threaded through the 8 compiler stages.
struct CompileCtx<'a> {
    manifest: &'a WorkflowManifest,
    template: &'a WorkflowTemplate,
    ir: WorkflowIR,
    operators_by_path: Option<BTreeMap<String, Vec<(OperatorId, Operator)>>>,
    root_id: Option<OperatorId>,
}

impl<'a> CompileCtx<'a> {
    fn new(manifest: &'a WorkflowManifest, template: &'a WorkflowTemplate) -> Self {
        Self {
            manifest,
            template,
            ir: WorkflowIR {
                ir_id: None,
                schema_version: SCHEMA_VERSION,
                template_ref: TemplateRef {
                    id: template.template_id.clone(),
                    version: template.version.clone(),
                },
                operators: BTreeMap::new(),
                guards: BTreeMap::new(),
                expansion_permissions: template.expansion_permissions.clone(),
                budgets: Budgets::default(),
                required_invariants: template.invariants.clone(),
                provenance: Provenance {
                    generated_by: String::new(),
                    prompt_hash: String::new(),
                    model_hash: String::new(),
                    policy_hash: String::new(),
                },
            },
            operators_by_path: None,
            root_id: None,
        }
    }

    fn into_ir(self) -> WorkflowIR {
        self.ir
    }
}

// ── Phase → Capability mapping ────────────────────────────────────────────────

/// Maps a YAML phase string (kebab-case) to a `CapabilityId`.
/// Exhaustive over all 9 Phase variants.
fn phase_str_to_capability(phase_str: &str) -> Option<CapabilityId> {
    match phase_str {
        "explore" => Some(CapabilityId("discover.intent".to_string())),
        "specify" => Some(CapabilityId("spec.draft".to_string())),
        "design" => Some(CapabilityId("design.shape".to_string())),
        "plan" => Some(CapabilityId("change.shape".to_string())),
        "build" => Some(CapabilityId("code.implement".to_string())),
        "verify" => Some(CapabilityId("change.verify".to_string())),
        "uat" => Some(CapabilityId("change.accept".to_string())),
        "release" => Some(CapabilityId("change.integrate".to_string())),
        "archive" => Some(CapabilityId("change.archive".to_string())),
        _ => None,
    }
}

/// Maps a `Phase` enum to its kebab-case string.
fn phase_to_string(phase: &crate::cycle::Phase) -> String {
    match phase {
        crate::cycle::Phase::Explore => "explore".to_string(),
        crate::cycle::Phase::Specify => "specify".to_string(),
        crate::cycle::Phase::Design => "design".to_string(),
        crate::cycle::Phase::Plan => "plan".to_string(),
        crate::cycle::Phase::Build => "build".to_string(),
        crate::cycle::Phase::Verify => "verify".to_string(),
        crate::cycle::Phase::Uat => "uat".to_string(),
        crate::cycle::Phase::Release => "release".to_string(),
        crate::cycle::Phase::Archive => "archive".to_string(),
    }
}

/// Returns the canonical `CapabilityId` for a given `Phase`.
pub fn phase_capability(phase: crate::cycle::Phase) -> CapabilityId {
    match phase {
        crate::cycle::Phase::Explore => CapabilityId("discover.intent".to_string()),
        crate::cycle::Phase::Specify => CapabilityId("spec.draft".to_string()),
        crate::cycle::Phase::Design => CapabilityId("design.shape".to_string()),
        crate::cycle::Phase::Plan => CapabilityId("change.shape".to_string()),
        crate::cycle::Phase::Build => CapabilityId("code.implement".to_string()),
        crate::cycle::Phase::Verify => CapabilityId("change.verify".to_string()),
        crate::cycle::Phase::Uat => CapabilityId("change.accept".to_string()),
        crate::cycle::Phase::Release => CapabilityId("change.integrate".to_string()),
        crate::cycle::Phase::Archive => CapabilityId("change.archive".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::{PathDef, Policies};
    use std::collections::HashMap;

    fn make_min_manifest() -> WorkflowManifest {
        let mut paths = HashMap::new();
        paths.insert(
            "a-min".to_string(),
            PathDef {
                description: "A-min path".to_string(),
                debt_verification: "false".to_string(),
                phases: vec!["explore".to_string(), "specify".to_string()],
            },
        );

        WorkflowManifest {
            schema_version: 1,
            workflow: crate::workflow::WorkflowDef {
                id: "test".to_string(),
                version: "0.1.0".to_string(),
                description: "Test manifest".to_string(),
            },
            statuses: vec![],
            phases: vec![],
            paths,
            policies: Policies::default(),
            transitions: vec![],
            artifacts: HashMap::new(),
            gates: HashMap::new(),
            forge: None,
            storage: None,
            project_identity: None,
        }
    }

    fn make_template(caps: Vec<&str>) -> WorkflowTemplate {
        let allowlist: BTreeSet<CapabilityId> = caps
            .into_iter()
            .map(|s| CapabilityId(s.to_string()))
            .collect();
        WorkflowTemplate {
            template_id: "test.template".to_string(),
            name: "Test".to_string(),
            version: "1.0.0".to_string(),
            intent: "Test".to_string(),
            capability_allowlist: allowlist,
            expansion_permissions: BTreeSet::new(),
            invariants: BTreeSet::new(),
            budgets: Budgets {
                max_wall_ms: 3_600_000,
                max_tokens: 1_000_000,
                max_cost_micros: 100_000_000,
                max_depth: 32,
                max_nodes: 1_000,
                remaining_tokens: None,
                no_progress_threshold: u32::MAX,
            },
            policies: BTreeMap::new(),
            convergence: crate::workflow_ir::ConvergenceSpec {
                max_iterations: 10,
                no_progress_signature: None,
            },
            schema_version: SCHEMA_VERSION,
        }
    }

    #[test]
    fn compile_a_min_returns_one_sequence() {
        let compiler = WorkflowCompiler;
        let manifest = make_min_manifest();
        let template = make_template(vec!["discover.intent", "spec.draft"]);

        let ir = compiler.compile(&manifest, &template).unwrap();
        assert_eq!(ir.schema_version, SCHEMA_VERSION);
        // One path → root is a Sequence
        let root_op = ir
            .operators
            .values()
            .find(|op| matches!(op, Operator::Sequence { .. }));
        assert!(root_op.is_some(), "Expected a Sequence operator");
    }

    #[test]
    fn compile_empty_allowlist_fails() {
        let compiler = WorkflowCompiler;
        let manifest = make_min_manifest();
        let template = make_template(vec![]);

        let err = compiler.compile(&manifest, &template).unwrap_err();
        assert!(matches!(err, CompileError::EmptyCapabilityAllowlist));
    }

    #[test]
    fn compile_unknown_capability_fails() {
        let compiler = WorkflowCompiler;
        let manifest = make_min_manifest();
        let template = make_template(vec!["discover.intent"]); // missing spec.draft

        let err = compiler.compile(&manifest, &template).unwrap_err();
        assert!(matches!(err, CompileError::CapabilityNotInAllowlist(_)));
    }

    #[test]
    fn compile_deterministic() {
        let compiler = WorkflowCompiler;
        let manifest = make_min_manifest();
        let template = make_template(vec!["discover.intent", "spec.draft"]);

        let ir1 = compiler.compile(&manifest, &template).unwrap();
        let ir2 = compiler.compile(&manifest, &template).unwrap();
        assert_eq!(ir1.compute_content_hash(), ir2.compute_content_hash());
    }
}
