//! Deterministic gate that validates `WorkflowIR` before runtime execution.
//!
//! This module is **pure**: no I/O, no side effects. Same `WorkflowIR` always yields
//! the same `Ok(())` or `Err(...)` outcome.

use std::collections::BTreeSet;
use std::sync::LazyLock;

use regex::Regex;

use crate::workflow_ir::{
    Budgets, Operator, OperatorId, ValidateError, WorkflowIR, WorkflowTemplate,
};

/// The SDDK workflow validator.
#[derive(Debug, Clone, Default)]
pub struct WorkflowValidator;

impl WorkflowValidator {
    /// Validates an IR in isolation (G1–G5, G7).
    ///
    /// G6 is a stub in this mode — full capability allowlist checking requires
    /// a template. Use [`validate_with_template`](Self::validate_with_template) for full validation.
    pub fn validate(&self, ir: &WorkflowIR) -> Result<(), ValidateError> {
        self.g1_schema(ir)?;
        self.g2_operators(ir)?;
        self.g3_cycle_free(ir)?;
        self.g4_guards(ir)?;
        self.g5_budgets(ir)?;
        self.g7_expansion_perms(ir)?;
        Ok(())
    }

    /// Validates an IR against a template (all 7 gates).
    ///
    /// Use this when the template's `capability_allowlist` is available.
    pub fn validate_with_template(
        &self,
        ir: &WorkflowIR,
        template: &WorkflowTemplate,
    ) -> Result<(), ValidateError> {
        self.g1_schema(ir)?;
        self.g2_operators(ir)?;
        self.g3_cycle_free(ir)?;
        self.g4_guards(ir)?;
        self.g5_budgets(ir)?;
        self.g6_capabilities_with_template(ir, template)?;
        self.g7_expansion_perms(ir)?;
        Ok(())
    }

    // ── Gates ────────────────────────────────────────────────────────────────

    /// G1 — schema version check.
    fn g1_schema(&self, ir: &WorkflowIR) -> Result<(), ValidateError> {
        if ir.schema_version != crate::workflow_ir::SCHEMA_VERSION {
            return Err(ValidateError::UnsupportedSchemaVersion {
                got: ir.schema_version,
                want: crate::workflow_ir::SCHEMA_VERSION,
            });
        }
        Ok(())
    }

    /// G2 — every `referenced_ids()` in every operator exists in `ir.operators`.
    fn g2_operators(&self, ir: &WorkflowIR) -> Result<(), ValidateError> {
        for op in ir.operators.values() {
            for ref_id in op.referenced_ids() {
                if !ir.operators.contains_key(&ref_id) {
                    return Err(ValidateError::OperatorNotFound(ref_id));
                }
            }
        }
        Ok(())
    }

    /// G3 — iterative DFS cycle detection (white/grey/black coloring).
    fn g3_cycle_free(&self, ir: &WorkflowIR) -> Result<(), ValidateError> {
        if ir.operators.is_empty() {
            return Ok(());
        }

        // Find roots: operators never referenced by any other operator
        let all_ids: BTreeSet<OperatorId> = ir.operators.keys().cloned().collect();
        let referenced_ids: BTreeSet<OperatorId> = ir
            .operators
            .values()
            .flat_map(|op| op.referenced_ids())
            .collect();
        let roots: Vec<OperatorId> = all_ids.difference(&referenced_ids).cloned().collect();

        if roots.is_empty() && !ir.operators.is_empty() {
            return Err(ValidateError::CycleDetected);
        }

        // DFS with color marking: white=0, grey=1, black=2
        let mut color: std::collections::BTreeMap<OperatorId, u8> =
            std::collections::BTreeMap::new();
        let mut stack: Vec<OperatorId> = roots;

        while let Some(current) = stack.pop() {
            let c = color.entry(current.clone()).or_insert(0);
            if *c == 2 {
                continue;
            }
            if *c == 1 {
                return Err(ValidateError::CycleDetected);
            }
            *c = 1;

            if let Some(op) = ir.operators.get(&current) {
                for child_id in op.referenced_ids() {
                    if ir.operators.contains_key(&child_id) {
                        stack.push(child_id);
                    }
                }
            }

            *color.get_mut(&current).unwrap() = 2;
        }

        Ok(())
    }

    /// G4 — guard expression well-formedness stub.
    ///
    /// v1.30.0: accepts any non-empty, balanced-parens expression.
    /// Full `GuardExpr` AST parsing deferred to cycle 3.
    fn g4_guards(&self, ir: &WorkflowIR) -> Result<(), ValidateError> {
        for (op_id, guard) in &ir.guards {
            let expr = guard.expr.trim();
            if expr.is_empty() {
                return Err(ValidateError::GuardFailed(format!(
                    "guard on {}: expression is empty",
                    op_id.0
                )));
            }
            let open = expr.matches('(').count();
            let close = expr.matches(')').count();
            if open != close {
                return Err(ValidateError::GuardFailed(format!(
                    "guard on {}: unbalanced parentheses (open={open}, close={close})",
                    op_id.0
                )));
            }
        }
        Ok(())
    }

    /// G5 — budget feasibility: fits within hard limits AND `consume(&zero())` doesn't underflow.
    fn g5_budgets(&self, ir: &WorkflowIR) -> Result<(), ValidateError> {
        if !ir.budgets.fits_within(&Budgets::hard_limits()) {
            return Err(ValidateError::BudgetExceedsLimit);
        }
        // Verify consume doesn't underflow on zero budget
        if ir.budgets.consume(&Budgets::zero()).is_err() {
            return Err(ValidateError::BudgetExceedsLimit);
        }
        Ok(())
    }

    /// G6 — capability closure (template-aware).
    fn g6_capabilities_with_template(
        &self,
        ir: &WorkflowIR,
        template: &WorkflowTemplate,
    ) -> Result<(), ValidateError> {
        static CAP_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^[a-z][a-z0-9]*(\.[a-z][a-z0-9_]*)+$").unwrap());

        for op in ir.operators.values() {
            if let Operator::Task { capability, .. } = op {
                // First: syntactic check (valid capability format)
                if !CAP_RE.is_match(&capability.0) {
                    return Err(ValidateError::CapabilityNotInAllowlist(capability.clone()));
                }
                // Second: allowlist membership
                if !template.capability_allowlist.contains(capability) {
                    return Err(ValidateError::CapabilityNotInAllowlist(capability.clone()));
                }
            }
        }
        Ok(())
    }

    /// G7 — expansion permission closure.
    fn g7_expansion_perms(&self, ir: &WorkflowIR) -> Result<(), ValidateError> {
        for (id, op) in &ir.operators {
            if matches!(op, Operator::Map { .. }) {
                // Map operator requires explicit expansion permission
                let has_perm = ir
                    .expansion_permissions
                    .contains(&crate::workflow_ir::ExpansionPermission::Map);
                if !has_perm {
                    return Err(ValidateError::OperatorNotAllowed((*id).clone()));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_ir::{
        Budgets, CapabilityId, Operator, OperatorId, Provenance, SCHEMA_VERSION, TemplateRef,
        WorkflowIR,
    };
    use std::collections::BTreeMap;

    fn make_valid_ir() -> WorkflowIR {
        let mut operators = BTreeMap::new();
        operators.insert(
            OperatorId("task-0".to_string()),
            Operator::Task {
                capability: CapabilityId("discover.intent".to_string()),
                inputs: BTreeMap::new(),
            },
        );
        operators.insert(
            OperatorId("task-1".to_string()),
            Operator::Task {
                capability: CapabilityId("spec.draft".to_string()),
                inputs: BTreeMap::new(),
            },
        );
        operators.insert(
            OperatorId("root#seq".to_string()),
            Operator::Sequence {
                body: vec![
                    OperatorId("task-0".to_string()),
                    OperatorId("task-1".to_string()),
                ],
            },
        );

        WorkflowIR {
            ir_id: None,
            schema_version: SCHEMA_VERSION,
            template_ref: TemplateRef {
                id: "test".to_string(),
                version: "1.0.0".to_string(),
            },
            operators,
            guards: BTreeMap::new(),
            expansion_permissions: BTreeSet::new(),
            budgets: Budgets {
                max_wall_ms: 3_600_000,
                max_tokens: 1_000_000,
                max_cost_micros: 100_000_000,
                max_depth: 32,
                max_nodes: 1_000,
                remaining_tokens: Some(1_000_000),
                no_progress_threshold: u32::MAX,
            },
            required_invariants: BTreeSet::new(),
            provenance: Provenance {
                generated_by: "test".to_string(),
                prompt_hash: "sha256:0000".to_string(),
                model_hash: "sha256:0000".to_string(),
                policy_hash: "sha256:0000".to_string(),
            },
        }
    }

    fn make_template() -> WorkflowTemplate {
        let mut allowlist = BTreeSet::new();
        allowlist.insert(CapabilityId("discover.intent".to_string()));
        allowlist.insert(CapabilityId("spec.draft".to_string()));
        WorkflowTemplate {
            template_id: "test".to_string(),
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
                remaining_tokens: Some(1_000_000),
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
    fn validate_healthy_ir_passes() {
        let validator = WorkflowValidator;
        let ir = make_valid_ir();
        let template = make_template();
        assert!(validator.validate(&ir).is_ok());
        assert!(validator.validate_with_template(&ir, &template).is_ok());
    }

    #[test]
    fn g2_fails_on_dangling_reference() {
        let validator = WorkflowValidator;
        let mut ir = make_valid_ir();
        // Add a Sequence that references a non-existent operator
        ir.operators.insert(
            OperatorId("bad#seq".to_string()),
            Operator::Sequence {
                body: vec![OperatorId("nonexistent".to_string())],
            },
        );
        let err = validator.validate(&ir).unwrap_err();
        assert!(matches!(err, ValidateError::OperatorNotFound(_)));
    }

    #[test]
    fn g3_fails_on_self_loop() {
        let validator = WorkflowValidator;
        let mut ir = make_valid_ir();
        // Replace root#seq with a mutual-cycle: task-0 → task-1 → task-0
        ir.operators.remove(&OperatorId("root#seq".to_string()));
        // task-1's body points back to task-0
        ir.operators.insert(
            OperatorId("task-1".to_string()),
            Operator::Loop {
                max_iterations: 3,
                until: crate::workflow_ir::GuardExpr {
                    expr: "true".to_string(),
                },
                body: OperatorId("task-0".to_string()),
            },
        );
        // task-0 now points to task-1, forming a cycle
        ir.operators.insert(
            OperatorId("task-0".to_string()),
            Operator::Loop {
                max_iterations: 5,
                until: crate::workflow_ir::GuardExpr {
                    expr: "false".to_string(),
                },
                body: OperatorId("task-1".to_string()),
            },
        );
        let err = validator.g3_cycle_free(&ir).unwrap_err();
        assert!(matches!(err, ValidateError::CycleDetected));
    }

    #[test]
    fn g5_fails_on_over_budget() {
        let validator = WorkflowValidator;
        let mut ir = make_valid_ir();
        ir.budgets = Budgets {
            max_wall_ms: u64::MAX,
            max_tokens: u64::MAX,
            max_cost_micros: u64::MAX,
            max_depth: u64::MAX,
            max_nodes: u64::MAX,
            remaining_tokens: Some(u64::MAX),
            no_progress_threshold: u32::MAX,
        };
        let err = validator.g5_budgets(&ir).unwrap_err();
        assert!(matches!(err, ValidateError::BudgetExceedsLimit));
    }

    #[test]
    fn g6_fails_on_unknown_capability() {
        let validator = WorkflowValidator;
        let ir = make_valid_ir();
        let mut template = make_template();
        template.capability_allowlist.clear(); // Empty allowlist
        let err = validator
            .validate_with_template(&ir, &template)
            .unwrap_err();
        assert!(matches!(err, ValidateError::CapabilityNotInAllowlist(_)));
    }
}
