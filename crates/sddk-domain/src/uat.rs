//! UAT (User Acceptance Testing) domain types — data-driven YAML model
//! (ADR-012/ADR-013): agents produce YAML artifacts, a deterministic renderer
//! turns them into self-contained HTML dashboards.

//! UAT (User Acceptance Testing) domain types — data-driven YAML model
//! (ADR-012/ADR-013): agents produce YAML artifacts, a deterministic renderer
//! turns them into self-contained HTML dashboards.
//!
//! `#![allow(missing_docs)]` — the schema itself is the contract for v2 types;
//! the canonical doc lives in the `uat-plan` skill and
//! `PLAN-uat-scenario-v2-context.md` in the knowledge vault.
#![allow(missing_docs)]

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Evidence types (ADR-0016): universal Evidence model, UAT specialization.
// Re-export from canonical evidence.rs. The UatEvidence* names (EvidenceKind,
// EvidenceArtifact, EvidenceBundle, etc.) are preserved here so downstream
// code (sddk-gateway, oracles, CLI) keeps compiling without changes.
// ---------------------------------------------------------------------------

pub use crate::evidence::{
    EvidenceArtifact, EvidenceAutomationStatus, EvidenceBlastRadius, EvidenceBundle,
    EvidenceEnvironment, EvidenceExecution, EvidenceExpectedCheck, EvidenceKind, EvidenceKindItem,
    EvidenceOrigin, EvidenceRiskClassification,
};

/// Backward-compat aliases — new code should use the Evidence* names directly.
pub type UatExpectedCheck = EvidenceExpectedCheck;
pub type UatEvidenceKind = EvidenceKind;
pub type UatEvidenceKindItem = EvidenceKindItem;
pub type UatAutomationStatus = EvidenceAutomationStatus;
pub type UatBlastRadius = EvidenceBlastRadius;
pub type UatOrigin = EvidenceOrigin;
pub type UatRiskClassification = EvidenceRiskClassification;

// ---------------------------------------------------------------------------
// Scenario v2 extensions (ADR-012 §4, §7 + ISO/IEC/IEEE 29119-3 alignment).
//
// All v2 fields are `Option<...>` or `Vec<...>` with `#[serde(default)]` so a
// v1 plan round-trips through `UatPlan` unchanged. Schema is bumped to v2
// only when at least one v2 field is used; the renderer degrades gracefully
// when v2 fields are absent (per the uat-guided-mode skill contract).
// ---------------------------------------------------------------------------

/// Closed vocabulary for the kind of a step (v2). Drives wizard rendering.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UatStepKind {
    #[default]
    Shell,
    Ui,
    Api,
    File,
    Manual,
}

/// Structured evidence specification for a scenario (v2).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatEvidenceSpec {
    #[serde(default = "default_true")]
    pub required: bool,
    #[serde(default)]
    pub kinds: Vec<UatEvidenceKindItem>,
    #[serde(default = "default_retention_days")]
    pub retention_days: u32,
}
fn default_retention_days() -> u32 {
    90
}

/// One piece of deterministic test data (v2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatTestDataItem {
    pub kind: String,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Workspace description (v2).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatWorkspace {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shell: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub browser: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editor: Option<String>,
    #[serde(default)]
    pub files_open: Vec<String>,
    #[serde(default)]
    pub external_urls: Vec<String>,
}

/// Timing metadata (v2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatTiming {
    #[serde(default = "default_window")]
    pub window: String,
    #[serde(default)]
    pub parallel_safe: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_hint: Option<String>,
    #[serde(default = "default_timeout_min")]
    pub timeout_min: u32,
}

fn default_window() -> String {
    "smoke".into()
}
fn default_timeout_min() -> u32 {
    10
}

impl Default for UatTiming {
    fn default() -> Self {
        Self {
            window: default_window(),
            parallel_safe: false,
            order_hint: None,
            timeout_min: default_timeout_min(),
        }
    }
}

/// Help block (v2).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatHelp {
    #[serde(default)]
    pub docs: Vec<String>,
    #[serde(default)]
    pub slack: Vec<String>,
    #[serde(default)]
    pub contacts: Vec<String>,
    #[serde(default)]
    pub related_adrs: Vec<String>,
    #[serde(default)]
    pub related_specs: Vec<String>,
}

/// Failure protocol (v2).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatFailureProtocol {
    #[serde(default)]
    pub on_fail: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_defect_template: Option<String>,
}

/// Risk classification for a scenario (v2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatRisk {
    pub classification: UatRiskClassification,
    pub blast_radius: UatBlastRadius,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mitigation: Option<String>,
}

/// Automation hook metadata (v2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatAutomation {
    pub status: UatAutomationStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ci_job: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
}

// ---------------------------------------------------------------------------
// UAT v3 — Human-Governed AI Quality Control Plane (ADR-014)
//
// Un scenario v3 declara CUATRO EJES independientes en vez del campo único
// `automation.status` (v2). Dominio puro: solo modelos y reglas de negocio;
// las implementaciones de executors/oracles viven en `sddk-gateway`
// (adaptadores de puertos definidos aquí) — arquitectura hexagonal.
// ---------------------------------------------------------------------------

/// Execution result of a scenario (v3). `PASSED != ACCEPTED` (REQ-RF-023):
/// este estado solo describe qué pasó en la ejecución, nunca la aceptación.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum UatExecutionResult {
    #[default]
    Passed,
    Failed,
    Blocked,
    Error,
    Skipped,
}

/// Machine assessment of the evidence against the oracles (v3).
/// El veredicto de la máquina NUNCA equivale a aceptación.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UatMachineAssessment {
    #[default]
    SupportedPass,
    SupportedFail,
    Uncertain,
    Conflicting,
}

/// Human decision on a scenario (v3). Única fuente de aceptación de negocio.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UatHumanDecision {
    #[default]
    Pending,
    Approved,
    Rejected,
    Waived,
}

/// Final acceptance status of a scenario (v3). `ACCEPTED != PASSED`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UatAcceptanceStatus {
    #[default]
    Pending,
    Accepted,
    Rejected,
    Conditional,
}

/// Who executes a scenario (v3, eje 1). El executor produce evidencia y
/// NUNCA decide el veredicto global (regla dura ADR-014 §3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UatExecutorKind {
    /// Local command line (runner tipado, sin shell — SDDK-601).
    Cli,
    /// HTTP/API interaction.
    Api,
    /// Script file (automation.ref v2 migra aquí).
    Script,
    /// Browser automation (Playwright) — sensor + actuador, nunca juez.
    Playwright,
    /// Computer-use agent (Fara) — observe→think→act con trajectory.
    ComputerUse,
    /// Human via the guided wizard / matrix view.
    Human,
}

/// Executor specification (v3, eje 1). Puertos: la implementación efectiva
/// de cada kind es un adaptador en `sddk-gateway` (DIP).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatExecutorSpec {
    pub kind: UatExecutorKind,
    /// Command line (cli/script): typed argv, first token = program.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Target URL (api/playwright/computer_use).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Goal for agentic executors (computer_use) — semantic journey.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goal: Option<String>,
    /// Model identifier (computer_use).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Wall-clock timeout in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

/// Evidence to capture for a scenario (v3, eje 2). Cada artefacto del bundle
/// es content-addressable (sha256); `trace > video` como evidencia primaria.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatEvidenceBundleSpec {
    #[serde(default)]
    pub screenshots: bool,
    #[serde(default)]
    pub playwright_trace: bool,
    #[serde(default)]
    pub console: bool,
    /// Capture network — `failures_only` si no se declara explícito.
    #[serde(default)]
    pub network: bool,
    #[serde(default)]
    pub accessibility: bool,
    #[serde(default)]
    pub geometry: bool,
    #[serde(default)]
    pub video: bool,
    /// Computer-use trajectory (Fara observe→think→act).
    #[serde(default)]
    pub trajectory: bool,
}

/// Backward-compat aliases for the universal Evidence model (ADR-0016).
///
/// These were historically duplicated structs in this module; the canonical
/// definitions now live in [`crate::evidence`]. Aliases keep downstream
/// consumers compiling while new code uses the `Evidence*` names directly.
pub type UatEvidenceArtifact = crate::evidence::EvidenceArtifact;
pub type UatEvidenceEnvironment = crate::evidence::EvidenceEnvironment;
pub type UatEvidenceExecution = crate::evidence::EvidenceExecution;
pub type UatEvidenceBundle = crate::evidence::EvidenceBundle;

/// Oracle kinds (v3, eje 3). Deterministas miden sin IA; semánticos evalúan
/// preliminarmente con confidence; `human` es la única autoridad de
/// aceptación. (Open/closed: añadir un kind es extensión, no modificación.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UatOracleKind {
    ExitCode,
    Http,
    Text,
    JsonSchema,
    Dom,
    Geometry,
    Accessibility,
    VisualDiff,
    VisualAi,
    LlmRubric,
    Human,
}

/// Oracle verdict — resultado de evaluar la evidencia contra un criterio.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UatOracleVerdict {
    #[default]
    Pass,
    Fail,
    Uncertain,
    Conflicting,
}

/// Oracle specification (v3, eje 3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatOracleSpec {
    pub kind: UatOracleKind,
    /// Criterio estructurado (json schema, expect body, selector DOM, ...).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expect: Option<serde_json::Value>,
    /// Rúbrica para oracles semánticos (visual_ai / llm_rubric).
    #[serde(default)]
    pub rubric: Vec<String>,
    /// Severity para accessibility (WCAG level / axe severity).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,
    /// Si es bloqueante para la aceptación.
    #[serde(default = "default_true")]
    pub blocking: bool,
}

/// Oracle assessment — resultado de evaluar un oracle contra la evidencia.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatOracleAssessment {
    pub oracle: UatOracleSpec,
    pub verdict: UatOracleVerdict,
    /// Confianza 0..1 (1 para oracles deterministas).
    #[serde(default)]
    pub confidence: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

/// Review policy kinds (v3, eje 4).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UatReviewPolicyKind {
    /// Todo escenario pasa por humano.
    Always,
    /// Ninguno (solo oracles deterministas).
    Never,
    #[default]
    /// Reglas `require_human_when` + sampling.
    RiskBased,
}

/// Trigger que obliga a revisión humana (v3, review policy).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UatReviewTrigger {
    BusinessCriticalityHigh,
    LowAiConfidence,
    OracleConflict,
    FirstExecution,
    SignificantVisualChange,
    HighHistoricalFailureRate,
}

/// Review policy (v3, eje 4): cuándo interviene el humano. El humano no
/// revisa un porcentaje fijo — risk-based + sampling empírico (REQ-RF-022).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatReviewPolicy {
    #[serde(default)]
    pub kind: UatReviewPolicyKind,
    #[serde(default)]
    pub require_human_when: Vec<UatReviewTrigger>,
    /// Fracción aleatoria 0..1 de machine-PASS que el humano también revisa.
    #[serde(default)]
    pub sampling: f64,
}

/// Por qué un scenario entra en la Human Review Queue (REQ-RF-022).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UatReviewReason {
    /// Obligatorio por política: P0, review Always, o trigger crítico.
    Required,
    /// Seleccionado por muestreo estadístico (sampling 1-5%).
    Sampled,
    /// Conflicto entre oracles (machine PASS vs FAIL en paralelo).
    OracleConflict,
    /// Confidence de la máquina por debajo del umbral de confianza.
    LowAiConfidence,
}

/// Item de la Human Review Queue: escenario + motivo + veredicto machine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatReviewItem {
    /// Scenario id (matches plan).
    pub scenario_id: String,
    /// Motivo de entrada en la cola.
    pub reason: UatReviewReason,
    /// Veredicto machine (PASS/FAIL/Uncertain del report).
    #[serde(default)]
    pub machine_verdict: UatOracleVerdict,
    /// Confidence de la máquina (0..1).
    #[serde(default)]
    pub machine_confidence: f64,
}

/// Desacuerdo humano vs máquina (REQ-RF-022): se persiste como dataset de
/// aprendizaje local para estimar falsos positivos/negativos de la IA.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatDisagreement {
    /// Scenario id.
    pub scenario_id: String,
    /// Veredicto machine original.
    pub machine_verdict: UatOracleVerdict,
    /// Confidence de la máquina.
    pub machine_confidence: f64,
    /// Veredicto humano (Accepted/Rejected).
    pub human_verdict: UatAcceptanceStatus,
    /// Categoría del desacuerdo (usability, bug, spec_drift, false_positive,
    /// false_negative, other).
    pub reason_category: String,
    /// Explicación del humano.
    pub explanation: String,
    /// Referencias de evidencia (sha256).
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    /// RFC 3339 timestamp.
    pub recorded_at: String,
}

/// Construye la Human Review Queue desde el plan + el report agregado.
///
/// Reglas (REQ-RF-022):
/// 1. `Required`: P0, review Always, trigger BusinessCriticalityHigh.
/// 2. `OracleConflict`: assessments con Fail y Pass para el mismo scenario.
/// 3. `LowAiConfidence`: mejor confidence < 0.7 en los assessments.
/// 4. `Sampled`: muestra determinista (hash scenario+seed) de los
///    machine-PASS que quedan, proporcional a `sampling` (default 0.02).
///
/// El muestreo es determinista dado `seed` — reproducible entre runs.
pub fn build_review_queue(
    plan: &UatPlan,
    report: &UatReport,
    sampling: f64,
    seed: &str,
) -> Vec<UatReviewItem> {
    use std::collections::HashSet;

    let sampling = if sampling.is_finite() && (0.0..=1.0).contains(&sampling) {
        sampling
    } else {
        0.02
    };
    let mut items: Vec<UatReviewItem> = Vec::new();
    let mut sampled_ids: HashSet<String> = HashSet::new();

    // Rollup por scenario del report.
    let mut rollup: std::collections::HashMap<&str, &UatScenarioRollup> =
        std::collections::HashMap::new();
    for feature in &report.features {
        for scenario in &feature.scenarios {
            rollup.insert(&scenario.scenario_id, scenario);
        }
    }

    for feature in &plan.features {
        for scenario in &feature.scenarios {
            let required = scenario.priority == UatPriority::P0
                || scenario
                    .review
                    .as_ref()
                    .map(|r| {
                        r.kind == UatReviewPolicyKind::Always
                            || r.require_human_when
                                .contains(&UatReviewTrigger::BusinessCriticalityHigh)
                    })
                    .unwrap_or(false);
            let roll = rollup.get(scenario.id.as_str());
            // Oracle conflict / low confidence desde los assessments.
            let (conflict, low_conf) = roll
                .and_then(|r| r.oracle_verdicts.as_ref())
                .map(|assessments| {
                    let has_pass = assessments
                        .iter()
                        .any(|a| a.verdict == UatOracleVerdict::Pass);
                    let has_fail = assessments
                        .iter()
                        .any(|a| a.verdict == UatOracleVerdict::Fail);
                    let best_conf = assessments
                        .iter()
                        .map(|a| a.confidence)
                        .fold(0.0_f64, f64::max);
                    (has_pass && has_fail, best_conf < 0.7)
                })
                .unwrap_or((false, false));

            if required {
                items.push(UatReviewItem {
                    scenario_id: scenario.id.clone(),
                    reason: UatReviewReason::Required,
                    machine_verdict: UatOracleVerdict::Pass,
                    machine_confidence: 1.0,
                });
            } else if conflict {
                items.push(UatReviewItem {
                    scenario_id: scenario.id.clone(),
                    reason: UatReviewReason::OracleConflict,
                    machine_verdict: UatOracleVerdict::Conflicting,
                    machine_confidence: 0.0,
                });
            } else if low_conf {
                items.push(UatReviewItem {
                    scenario_id: scenario.id.clone(),
                    reason: UatReviewReason::LowAiConfidence,
                    machine_verdict: UatOracleVerdict::Uncertain,
                    machine_confidence: 0.5,
                });
            } else if !sampled_ids.contains(&scenario.id) {
                // Muestreo determinista: hash(scenario + seed) % 100 < pct.
                let digest = sha256_hex(format!("{}::{seed}", scenario.id).as_bytes());
                let bucket = digest.chars().take(8).fold(0u64, |acc, c| {
                    acc.wrapping_mul(16) + c.to_digit(16).unwrap_or(0) as u64
                });
                let pct = (sampling * 100.0).round() as u64;
                if bucket % 100 < pct {
                    items.push(UatReviewItem {
                        scenario_id: scenario.id.clone(),
                        reason: UatReviewReason::Sampled,
                        machine_verdict: UatOracleVerdict::Pass,
                        machine_confidence: 1.0,
                    });
                    sampled_ids.insert(scenario.id.clone());
                }
            }
        }
    }
    items
}

/// Testability report (REQ-RF-021): qué tan automatizable es un scenario.
/// Advisory — el humano/plan decide el executor final, nunca el agente.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatTestabilityReport {
    #[serde(default)]
    pub deterministic: f64,
    #[serde(default)]
    pub browser_automatable: f64,
    #[serde(default)]
    pub agentic_automatable: f64,
    #[serde(default)]
    pub requires_human_judgement: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recommended_executor: Option<UatExecutorKind>,
    #[serde(default)]
    pub recommended_oracles: Vec<UatOracleKind>,
    #[serde(default)]
    pub human_review_recommended: bool,
    #[serde(default)]
    pub reasons: Vec<String>,
}

/// Provenance for a scenario (v2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatProvenance {
    pub author: String,
    pub created_at: String,
    pub last_modified_at: String,
    pub origin: UatOrigin,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin_ref: Option<String>,
}

/// Full context block for a scenario (v2).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatScenarioContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_story: Option<String>,
    #[serde(default)]
    pub preconditions: Vec<String>,
    #[serde(default)]
    pub test_data: Vec<UatTestDataItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<UatWorkspace>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timing: Option<UatTiming>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help: Option<UatHelp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_protocol: Option<UatFailureProtocol>,
    #[serde(default)]
    pub postconditions: Vec<String>,
}

/// Closed vocabulary for scenario priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum UatPriority {
    /// Critical path: blocks release without human verdict.
    P0,
    /// High value: normally covered by UAT.
    P1,
    /// Lower priority: optional coverage.
    #[default]
    P2,
}

/// Closed vocabulary for scenario assignee role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UatAssignee {
    /// Functional flow validation.
    #[default]
    Developer,
    /// Design/UX/consistency validation.
    Architect,
}

/// Closed vocabulary for per-step execution status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum UatStatus {
    /// Scenario was not executed or lacks the evidence required by the plan.
    #[default]
    #[serde(rename = "NOT_RUN")]
    NotRun,
    /// Scenario passed.
    Pass,
    /// Scenario failed (defect found).
    Fail,
    /// Scenario could not run (blocked).
    Blocked,
    /// Scenario partially passed.
    Partial,
}

/// Closed vocabulary for who executed a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UatExecutor {
    /// Executed by a human tester.
    #[default]
    Human,
    /// Executed by the visual agent (pre-flight).
    Fara,
    /// Mixed human + agent execution.
    Mixed,
    /// Executed by the local auto-runner (`uat run`) from `automation.ref`.
    Automated,
}

/// Closed vocabulary for the global release verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UatVerdict {
    /// All critical scenarios pass.
    Ready,
    /// Passes with documented risks.
    ReadyWithRisks,
    /// Blocking defects found.
    NotReady,
}

/// One guided step of a scenario (plain language, junior-friendly).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatStep {
    /// Plain-language instruction (e.g. "Abre http://localhost:3000/login").
    pub action: String,
    /// Renderer paints a copy button when true.
    #[serde(default)]
    pub copy_hint: bool,
    /// Expected observable outcome of this step.
    pub expected: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<UatStepKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vs_expected_check: Option<UatExpectedCheck>,
}

// ---------------------------------------------------------------------------
// UAT Form DSL (ADR-015, REQ-RF-025) — vocabulario cerrado para el Guided
// Runner v3. Los agentes generan ESTA spec (nunca HTML/JS); el renderer
// determinista la compila. Todo valor fuera de los enums es rechazado por
// `uat validate`.
// ---------------------------------------------------------------------------

/// Input humano que el wizard debe recolectar (vocabulario cerrado).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UatFormInputKind {
    Confirm,
    YesNo,
    PassFail,
    SingleChoice,
    MultiChoice,
    Text,
    Textarea,
    Number,
    Rating,
    Date,
    Duration,
    Select,
    Checkbox,
    Checklist,
    BlindObservation,
}

/// Evidencia que el runner captura/adjunta (vocabulario cerrado).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UatFormEvidenceKind {
    Screenshot,
    Video,
    File,
    Annotation,
    BrowserTrace,
    Console,
    Network,
    Log,
    Url,
    Clipboard,
}

/// Validación automática (oracle) disponible en un check del formulario.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UatFormOracleKind {
    Http,
    Json,
    Text,
    Dom,
    Aria,
    Geometry,
    VisualDiff,
    VisualAi,
    Accessibility,
    Performance,
    Cli,
    Database,
    CustomScript,
}

/// Elemento informativo del formulario (no requiere input).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UatFormInfoKind {
    Instruction,
    Warning,
    ExpectedResult,
    Tip,
    Reference,
    Image,
    Code,
    Link,
    Example,
}

/// Control de flujo del wizard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UatFormFlowKind {
    Next,
    Previous,
    Skip,
    Block,
    Retry,
    Branch,
    Repeat,
    Goto,
    Stop,
}

/// Visibilidad de un check (blind checks: expected oculto — REQ-RF-026).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UatFormVisibility {
    #[default]
    Visible,
    Hidden,
    Blind,
}

/// Check del formulario: un punto de validación con reglas (REQ-RF-025).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatFormCheck {
    /// Qué recoge el check.
    pub kind: UatFormInputKind,
    /// Pregunta/instrucción mostrada al humano.
    pub prompt: String,
    /// Oracle automático opcional (máquina pre-verifica).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oracle: Option<UatFormOracleKind>,
    /// Visibilidad del expected (blind check cuando `blind`).
    #[serde(default)]
    pub visibility: UatFormVisibility,
    /// Si el check es obligatorio.
    #[serde(default = "default_true")]
    pub required: bool,
    /// Si un fail del check bloquea el escenario.
    #[serde(default = "default_true")]
    pub blocking: bool,
    /// Confidence mínima para aceptar el oracle automático (0..1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence_requirement: Option<f64>,
    /// Evidencia exigida (screenshot obligatorio, etc.).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_requirement: Vec<UatFormEvidenceKind>,
    /// Condición para obligar comentario: `always|on_fail|never`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment_required_when: Option<String>,
    /// Opciones para single_choice / multi_choice / select / checklist.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
    /// Expected (oculto si visibility=blind).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
}

/// Un elemento del formulario: check, informativo, flujo o checkpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatFormItem {
    /// Tipo de elemento.
    pub kind: UatFormElementKind,
    /// ID único del item (para referencias en goto/checkpoint). Si no se declara,
    /// la posición ordinal actúa como ID implícito.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Check cuando kind == Check.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub check: Option<UatFormCheck>,
    /// Texto para informativos / instrucciones.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Flujo cuando kind == Flow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flow: Option<UatFormFlowKind>,
    /// Destino de branch/goto (id de item).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// Checkpoint cuando kind == Checkpoint (REQ-RF-027).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkpoint: Option<UatCheckpoint>,
}

/// Clasificador del elemento de formulario.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UatFormElementKind {
    Check,
    Info,
    Flow,
    /// Checkpoint que pausa el wizard y requiere approve/reject (REQ-RF-027).
    Checkpoint,
}

/// Spec Form DSL de un escenario (ADR-015): pasos → items de formulario.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatFormSpec {
    /// Version del DSL (1).
    #[serde(default = "default_dsl_version")]
    pub dsl_version: u32,
    /// Items del formulario en orden de render.
    #[serde(default)]
    pub items: Vec<UatFormItem>,
    /// Completion policy for the scenario (REQ-RF-025).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completion: Option<UatCompletionPolicy>,
}

fn default_dsl_version() -> u32 {
    1
}

// ---------------------------------------------------------------------------
// Guided Runner F13 — Domain Types (REQ-RF-024..028)
// ---------------------------------------------------------------------------

/// Modo de ejecución del Runner (REQ-RF-028): Designer (edición),
/// Runner (wizard/evidence/checkpoints), Reviewer (sign-off/acceptance).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UatRunnerMode {
    Designer,
    Runner,
    Reviewer,
}

/// Machine-readable evidence summary shown at a checkpoint (REQ-RF-027).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatEvidenceSummary {
    #[serde(default)]
    pub machine_passed: u32,
    #[serde(default)]
    pub machine_total: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fara_assessment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fara_confidence: Option<f64>,
    #[serde(default)]
    pub anomalies: Vec<String>,
}

/// Completion policy mode for a form scenario (REQ-RF-025).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UatCompletionMode {
    /// All items must pass.
    All,
    /// Majority threshold required.
    Majority,
}

/// Policy that determines when a form scenario is considered complete (REQ-RF-025).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatCompletionPolicy {
    pub mode: UatCompletionMode,
    /// Threshold for Majority mode (1..n). None for All.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threshold: Option<u32>,
}

impl UatCompletionPolicy {
    /// Validate the completion policy. Returns errors if invalid.
    pub fn validate(policy: &UatCompletionPolicy) -> Vec<String> {
        let mut errors = Vec::new();
        if policy.threshold == Some(0) {
            errors.push("completion.threshold must be >= 1".into());
        }
        errors
    }
}

/// Checkpoint block that pauses the wizard for human approve/reject (REQ-RF-027).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatCheckpoint {
    #[serde(default)]
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default)]
    pub evidence_summary: UatEvidenceSummary,
    /// Item ids that belong to this checkpoint block.
    #[serde(default)]
    pub items: Vec<String>,
}

/// Diagnostics report produced when a scenario fails (REQ-RF-027).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatDiagnosticsReport {
    pub scenario_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkpoint_id: Option<String>,
    #[serde(default)]
    pub collected_evidence: Vec<UatEvidenceKindItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cause: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested_defect: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
}

/// Decision on an acceptance record (REQ-RF-028).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UatAcceptanceDecision {
    Accepted,
    AcceptedConditional,
    Rejected,
}

/// Immutable acceptance record with sha256 snapshots (REQ-RF-028).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatAcceptanceRecord {
    pub decision: UatAcceptanceDecision,
    /// Actor who signed off, e.g. `user:421`.
    pub actor: String,
    /// RFC 3339 timestamp.
    pub timestamp: String,
    /// SHA-256 of the plan at sign-off time (format: `sha256:<hex>`).
    pub plan_version_sha256: String,
    /// SHA-256 of the evidence manifest snapshot (format: `sha256:<hex>`).
    pub evidence_snapshot_sha256: String,
    #[serde(default)]
    pub outstanding_findings: Vec<String>,
    pub justification: String,
}

impl UatAcceptanceRecord {
    /// Validate the acceptance record. Returns errors if invalid.
    pub fn validate(record: &UatAcceptanceRecord) -> Vec<String> {
        let mut errors = Vec::new();
        if !record.plan_version_sha256.starts_with("sha256:") {
            errors.push("plan_version_sha256 must start with sha256:".into());
        }
        if !record.evidence_snapshot_sha256.starts_with("sha256:") {
            errors.push("evidence_snapshot_sha256 must start with sha256:".into());
        }
        errors
    }
}

/// Kind of staleness change detected (REQ-RF-024).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UatStalenessChangeKind {
    SelectorChanged,
    TextContentChanged,
    AttributeChanged,
    ElementRemoved,
    ElementAdded,
}

/// One scenario affected by UI staleness (REQ-RF-024).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatStalenessScenario {
    pub scenario_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkpoint_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selector: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_content: Option<String>,
    pub previous_fingerprint: String,
    pub current_fingerprint: String,
    pub change_kind: UatStalenessChangeKind,
}

/// One detected difference in fingerprint between snapshot and current UI (REQ-RF-024).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatStalenessDiff {
    pub scenario_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkpoint_id: Option<String>,
    pub field: String,
    pub previous: String,
    pub current: String,
}

/// Staleness advisory report (REQ-RF-024).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatStalenessReport {
    pub release: String,
    pub assessed_at: String,
    #[serde(default)]
    pub affected_scenarios: Vec<UatStalenessScenario>,
    #[serde(default)]
    pub fingerprint_diffs: Vec<UatStalenessDiff>,
}

/// Staleness status of a scenario (REQ-RF-024).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UatScenarioStaleness {
    Fresh,
    Stale,
}

impl UatScenarioStaleness {
    /// Maps the UAT-specific staleness to the universal state (SPEC-012, Phase 6).
    /// `Stale` maps to `PossiblyStale` conservatively: UAT fingerprints cannot
    /// distinguish semantic invalidation, so universal derivation may escalate.
    pub fn to_universal(self) -> crate::staleness::StalenessState {
        match self {
            UatScenarioStaleness::Fresh => crate::staleness::StalenessState::Fresh,
            UatScenarioStaleness::Stale => crate::staleness::StalenessState::PossiblyStale,
        }
    }
}

/// Valida que todos los valores de la spec pertenezcan al vocabulario
/// cerrado (REQ-RF-025 + REQ-RF-027). Devuelve lista de errores estables; vacío = válida.
///
/// Validates:
/// - Closed vocabulary for check items
/// - Branching referencial: every `goto` target resolves to an existing item id
/// - Cycle detection: goto graph must be acyclic (DFS coloring)
/// - CompletionPolicy: mode ∈ {all, majority}, threshold 1..n
/// - Checkpoint: all referenced item ids must exist
pub fn validate_form_dsl(spec: &UatFormSpec) -> Vec<String> {
    let mut errors = Vec::new();

    // Build item id → position index.
    // ID resolution order: explicit `item.id` field first, then fallback to position string.
    let n = spec.items.len();
    let id_to_pos: std::collections::HashMap<&str, usize> = spec
        .items
        .iter()
        .enumerate()
        .filter_map(|(pos, item)| {
            // Explicit id takes precedence
            item.id.as_ref().map(|id| (id.as_str(), pos))
        })
        .collect();

    // For targets that don't resolve via explicit id, try position-based fallback.
    fn resolve_target(
        target: &str,
        items: &[UatFormItem],
        id_to_pos: &std::collections::HashMap<&str, usize>,
    ) -> Option<usize> {
        // First try explicit id index
        if let Some(&pos) = id_to_pos.get(target) {
            return Some(pos);
        }
        // Then try position as string
        if let Ok(pos) = target.parse::<usize>()
            && pos < items.len()
        {
            return Some(pos);
        }
        None
    }

    // First pass: basic item validation.
    for (i, item) in spec.items.iter().enumerate() {
        match item.kind {
            UatFormElementKind::Check => match &item.check {
                None => errors.push(format!("item[{i}]: kind=check sin `check` block")),
                Some(check) => {
                    if check
                        .comment_required_when
                        .as_deref()
                        .is_some_and(|v| !matches!(v, "always" | "on_fail" | "never"))
                    {
                        errors.push(format!(
                            "item[{i}]: comment_required_when={:?} no está en {{always,on_fail,never}}",
                            check.comment_required_when
                        ));
                    }
                    if check
                        .confidence_requirement
                        .is_some_and(|c| !(0.0..=1.0).contains(&c))
                    {
                        errors.push(format!(
                            "item[{i}]: confidence_requirement={:?} fuera de [0,1]",
                            check.confidence_requirement
                        ));
                    }
                    if matches!(
                        check.kind,
                        UatFormInputKind::SingleChoice
                            | UatFormInputKind::MultiChoice
                            | UatFormInputKind::Select
                            | UatFormInputKind::Checklist
                    ) && check.options.is_empty()
                    {
                        errors.push(format!(
                            "item[{i}]: check {:?} necesita `options` no vacío",
                            check.kind
                        ));
                    }
                }
            },
            UatFormElementKind::Info => {
                if item.text.is_none() {
                    errors.push(format!("item[{i}]: kind=info sin `text`"));
                }
            }
            UatFormElementKind::Flow => {
                if item.flow.is_none() {
                    errors.push(format!("item[{i}]: kind=flow sin `flow`"));
                }
            }
            UatFormElementKind::Checkpoint => {
                if item.checkpoint.is_none() {
                    errors.push(format!("item[{i}]: kind=checkpoint sin `checkpoint` block"));
                }
            }
        }
    }

    // Completion policy validation.
    if let Some(ref completion) = spec.completion {
        errors.extend(
            UatCompletionPolicy::validate(completion)
                .into_iter()
                .map(|e| format!("completion: {e}")),
        );
    }

    // Branching referencial: every goto target must resolve to an existing item.
    for (i, item) in spec.items.iter().enumerate() {
        if item.kind == UatFormElementKind::Flow && item.flow == Some(UatFormFlowKind::Goto) {
            if let Some(target) = &item.target {
                if resolve_target(target, &spec.items, &id_to_pos).is_none() {
                    errors.push(format!(
                        "scenario=X item[{i}]: goto target '{target}' not found"
                    ));
                }
            } else {
                errors.push(format!("item[{i}]: kind=flow with goto but no target"));
            }
        }
    }

    // Cycle detection using DFS three-color marking: 0=unvisited, 1=visiting, 2=done.
    let mut color = vec![0u8; n];
    let mut cycle_path: Vec<usize> = Vec::new();

    fn dfs(
        pos: usize,
        items: &[UatFormItem],
        id_to_pos: &std::collections::HashMap<&str, usize>,
        color: &mut [u8],
        path: &mut Vec<usize>,
        errors: &mut Vec<String>,
    ) {
        if color[pos] == 2 {
            return; // already fully processed
        }
        if color[pos] == 1 {
            // Cycle detected — record the cycle path.
            let cycle_start = path.iter().position(|&p| p == pos).unwrap_or(0);
            let cycle: Vec<String> = path[cycle_start..]
                .iter()
                .chain(std::iter::once(&pos))
                .map(|&p| {
                    let item = &items[p];
                    let label = item.id.as_deref().or(item.target.as_deref()).unwrap_or("*");
                    format!("item[{p}](goto:{label})")
                })
                .collect();
            errors.push(format!("goto cycle detected: {}", cycle.join(" → ")));
            return;
        }

        color[pos] = 1;
        path.push(pos);

        let item = &items[pos];
        if item.kind == UatFormElementKind::Flow
            && item.flow == Some(UatFormFlowKind::Goto)
            && let Some(target) = &item.target
            && let Some(target_pos) = resolve_target(target, items, id_to_pos)
        {
            dfs(target_pos, items, id_to_pos, color, path, errors);
        }

        path.pop();
        color[pos] = 2;
    }

    for start in 0..n {
        if color[start] == 0 {
            dfs(
                start,
                &spec.items,
                &id_to_pos,
                &mut color,
                &mut cycle_path,
                &mut errors,
            );
        }
    }

    // Checkpoint item references must point to existing items.
    for (i, item) in spec.items.iter().enumerate() {
        if item.kind == UatFormElementKind::Checkpoint
            && let Some(cp) = &item.checkpoint
        {
            for cp_item in &cp.items {
                if resolve_target(cp_item, &spec.items, &id_to_pos).is_none() {
                    errors.push(format!(
                        "item[{i}]: checkpoint references nonexistent item '{cp_item}'"
                    ));
                }
            }
        }
    }

    errors
}

fn default_true() -> bool {
    true
}

/// One acceptance scenario of a feature.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatScenario {
    /// Stable scenario id, e.g. `S-1`.
    pub id: String,
    /// Human-readable scenario title.
    pub title: String,
    #[serde(default)]
    /// Scenario priority (P0..P2).
    pub priority: UatPriority,
    #[serde(default)]
    /// Role assigned to validate.
    pub assignee: UatAssignee,
    /// Preconditions the tester must ensure before starting.
    #[serde(default)]
    pub preconditions: Vec<String>,
    /// Junior guided view: one step per screen.
    #[serde(default)]
    pub plain_steps: Vec<UatStep>,
    /// Senior matrix view: technical shorthand (optional).
    #[serde(default)]
    pub technical_steps: Vec<String>,
    /// "Why this matters" — written by uat-guide.
    #[serde(default)]
    pub rationale: Option<String>,
    /// What evidence the tester must capture (screenshot, log, note).
    #[serde(default)]
    pub evidence_prompt: Option<String>,
    /// Semantic flags from a closed vocabulary: smoke|warning|optional|data-verify.
    #[serde(default)]
    pub flags: Vec<String>,
    /// Estimated execution time in minutes.
    #[serde(default)]
    pub est_minutes: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<UatScenarioContext>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<UatEvidenceSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk: Option<UatRisk>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub automation: Option<UatAutomation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<UatProvenance>,
    // --- UAT v3 (ADR-014) — campos opcionales, aditivos sobre v2 ---
    /// Eje 1: quién ejecuta (v3). Sustituye a `automation` en planes v3.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executor: Option<UatExecutorSpec>,
    /// Eje 2: qué evidencia capturar (v3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_bundle: Option<UatEvidenceBundleSpec>,
    /// Eje 3: oracles que juzgan la evidencia (v3). Vacío en v2.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub oracles: Vec<UatOracleSpec>,
    /// Eje 4: política de revisión humana (v3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review: Option<UatReviewPolicy>,
    /// Estado de aceptación (v3). `ACCEPTED != PASSED` (REQ-RF-023).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acceptance: Option<UatAcceptanceStatus>,
    /// Form DSL (v3, ADR-015/REQ-RF-025): spec declarativa del Guided
    /// Runner. Opcional — los escenarios v2 siguen usando plain_steps.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub form: Option<UatFormSpec>,
    // --- UAT v4 (F13) — Guided Runner fields (REQ-RF-024..028) ---
    /// Form checkpoint that marks the end of a block (REQ-RF-027).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub form_checkpoint: Option<UatCheckpoint>,
    /// Form-level completion policy (REQ-RF-025).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub form_completion: Option<UatCompletionPolicy>,
    /// Scenario-level completion policy (REQ-RF-025).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completion: Option<UatCompletionPolicy>,
    /// Staleness status (REQ-RF-024).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub staleness: Option<UatScenarioStaleness>,
}

/// One feature under test, grouping its scenarios.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatFeature {
    /// Stable feature id, e.g. `F-01`.
    pub id: String,
    /// Feature display name.
    pub name: String,
    /// PRD requirement reference for the traceability view (e.g. `RF-016`).
    #[serde(default)]
    pub requirement_ref: Option<String>,
    /// Related design reference (e.g. `ADR-012-§7`) — v2.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub design_ref: Option<String>,
    #[serde(default)]
    /// Feature priority (P0..P2).
    pub priority: UatPriority,
    #[serde(default)]
    /// Scenarios of this feature.
    pub scenarios: Vec<UatScenario>,
}

/// Canonical acceptance plan artifact (`uat-plan.yaml`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatPlan {
    /// Schema version of this plan (renderer supports N versions).
    pub schema_version: u32,
    /// Candidate tag under test and aggregation window.
    pub release: UatPlanRelease,
    /// Which agent generated the plan.
    #[serde(default)]
    pub generated_by: String,
    /// RFC 3339 generation timestamp.
    pub generated_at: String,
    #[serde(default)]
    /// Features under test.
    pub features: Vec<UatFeature>,
    /// Runner mode hint (v4, REQ-RF-028): designer/runner/reviewer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runner_mode: Option<UatRunnerMode>,
    /// Human approval record (v4, E14.5): present when a human reviewed
    /// and approved the plan. Auto/CI runs omit this field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval: Option<UatPlanApproval>,
}

/// Approval record for a plan: who approved and when.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatPlanApproval {
    /// Internal ID of the approver (e.g. "T-0001").
    pub id: String,
    /// Display name of the approver.
    pub display: String,
    /// RFC3339 timestamp of the approval decision.
    pub approved_at: String,
}

/// Release context of a plan: features aggregated since the last UAT'd tag.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatPlanRelease {
    /// Candidate tag, e.g. `v1.5.0`.
    pub candidate: String,
    /// Project identifier (adopted project id or repo basename).
    #[serde(default)]
    pub project: Option<String>,
    /// Last release that went through UAT; features are aggregated from here.
    #[serde(default)]
    pub last_uat_release: Option<String>,
}

/// One executed session (`uat-session.yaml`): per-scenario results + evidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatSession {
    pub schema_version: u32,
    pub session_id: String,
    pub plan_ref: String,
    pub release: String,
    #[serde(default)]
    pub executor: UatExecutor,
    #[serde(default)]
    pub executed_by: Option<String>,
    pub started_at: String,
    #[serde(default)]
    pub finished_at: Option<String>,
    #[serde(default)]
    pub results: Vec<UatScenarioResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<UatSessionMetadata>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_version: Option<u32>,
}

/// Anonymous tester reference (v2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatTesterRef {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

/// Environment fingerprint (v2).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatEnvFingerprint {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub os: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shell: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workdir: Option<String>,
}

/// Build metadata (v2).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatBuild {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(default)]
    pub dirty: Option<bool>,
}

/// Session metadata (v2).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatSessionMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tester: Option<UatTesterRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env_fingerprint: Option<UatEnvFingerprint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build: Option<UatBuild>,
}

/// Latest supported session schema version.
pub const LATEST_SESSION_SCHEMA_VERSION: u32 = 2;

/// Per-scenario result inside a session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatScenarioResult {
    pub scenario_id: String,
    #[serde(default)]
    pub status: UatStatus,
    #[serde(default)]
    pub comment: Option<String>,
    #[serde(default)]
    pub evidence: Vec<UatEvidence>,
    #[serde(default)]
    pub duration_minutes: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verdict_at: Option<String>,
    #[serde(default)]
    pub verdict_duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tester_notes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linked_defect: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repro_command: Option<String>,
    /// Oracle assessments (v3, eje 3): veredictos deterministas/semánticos
    /// sobre la evidencia capturada. Vacío en sesiones v2.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub oracle_assessments: Vec<UatOracleAssessment>,
}

/// Evidence captured for a scenario result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatEvidence {
    #[serde(default)]
    pub kind: UatEvidenceKind,
    /// `sha256:<hash>` of the evidence payload.
    pub r#ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub captured_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub match_mode: Option<UatExpectedCheck>,
}

/// Aggregated report (`uat-report.yaml`) with the global verdict.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatReport {
    /// Schema version of this report.
    pub schema_version: u32,
    /// Candidate tag under test.
    pub release: String,
    /// Plan reference this report aggregates.
    pub plan_ref: String,
    /// Session ids aggregated into this report.
    #[serde(default)]
    /// Session ids aggregated into this report.
    pub sessions: Vec<String>,
    /// Numeric rollup of the report.
    pub summary: UatReportSummary,
    /// Per-feature rollup for the traceability view.
    #[serde(default)]
    pub features: Vec<UatFeatureRollup>,
    /// Recommendation, not an order: READY | READY_WITH_RISKS | NOT_READY.
    /// Recommendation: READY | READY_WITH_RISKS | NOT_READY.
    pub verdict: UatVerdict,
    /// Scenarios blocking a READY verdict (with reasons).
    #[serde(default)]
    pub not_ready_blockers: Vec<String>,
    /// Scenarios que requieren aceptación humana y no la tienen (v3,
    /// REQ-RF-023). Vacío = no hay acceptance pendiente.
    #[serde(default)]
    pub acceptance_blockers: Vec<String>,
}

/// Numeric rollup of a report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatReportSummary {
    /// Total scenarios in the plan.
    pub total_scenarios: u32,
    /// Passed count.
    pub passed: u32,
    /// Failed count.
    pub failed: u32,
    /// Blocked count.
    pub blocked: u32,
    /// Partial count.
    pub partial: u32,
    /// Scenarios that were not executed or could not be proven.
    #[serde(default)]
    pub not_run: u32,
    /// Coverage percentage (0..=100).
    pub coverage_pct: f64,
    /// Functional defects found.
    pub defects: u32,
    /// UX issues observed.
    pub ux_issues: u32,
    #[serde(default)]
    /// Total human minutes spent across sessions.
    pub uat_duration_minutes: u32,
}

/// Per-feature rollup: coverage + scenario statuses (traceability view).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatFeatureRollup {
    /// Feature id (matches plan).
    pub id: String,
    /// Feature display name.
    pub name: String,
    /// Percentage of scenarios covered.
    pub coverage_pct: f64,
    #[serde(default)]
    /// Scenario statuses rolled up for this feature.
    pub scenarios: Vec<UatScenarioRollup>,
}

/// Scenario status within a feature rollup.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatScenarioRollup {
    /// Scenario id (matches plan).
    pub scenario_id: String,
    /// Rolled-up status.
    pub status: UatStatus,
    /// Executor that produced this status (last writer wins).
    #[serde(default)]
    pub executor: Option<UatExecutor>,
    /// Acceptance status (v3, REQ-RF-023: PASSED != ACCEPTED). `None`
    /// significa que el escenario no requiere aceptación humana.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acceptance: Option<UatAcceptanceStatus>,
    /// Si el escenario exige aceptación humana para liberar (P0 o review
    /// policy que lo pide). Deriva del plan en el agregador.
    #[serde(default)]
    pub acceptance_required: bool,
    /// Oracle assessments agregados (v3, eje 3) — para el análisis de
    /// conflictos y confidence en la Human Review Queue (REQ-RF-022).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oracle_verdicts: Option<Vec<UatOracleAssessment>>,
}

// ---------------------------------------------------------------------------
// Per-project UAT config (XDG-resident, ADR-0011 compliant)
// ---------------------------------------------------------------------------

/// What the `release-uat-approved` gate does for a given release type.
/// Default policy: major=Required, minor=Required, patch=Skip.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReleaseGateAction {
    /// The gate blocks; the release cannot proceed without a human UAT verdict.
    #[default]
    Required,
    /// The gate is bypassed.
    Skip,
    /// The gate is recorded but does not block (advisory only).
    Advisory,
}

/// Type of a release — derived by semver diff against the previous tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReleaseType {
    /// Major version bump (breaking change).
    Major,
    /// Minor version bump (new features, backwards-compatible).
    Minor,
    /// Patch version bump (bug fixes).
    Patch,
}

impl ReleaseType {
    /// String form (`"major"`/`"minor"`/`"patch"`).
    pub fn as_str(&self) -> &'static str {
        match self {
            ReleaseType::Major => "major",
            ReleaseType::Minor => "minor",
            ReleaseType::Patch => "patch",
        }
    }
}

/// Default gate policy (matches the RNF-010 spec in the knowledge vault):
/// major and minor require human verdict, patches do not.
/// Per-release-type gate policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ReleaseGateMap {
    /// Policy for major releases (default: required).
    #[serde(default = "default_required")]
    pub major: ReleaseGateAction,
    /// Policy for minor releases (default: required).
    #[serde(default = "default_required")]
    pub minor: ReleaseGateAction,
    /// Policy for patch releases (default: skip).
    #[serde(default = "default_skip")]
    pub patch: ReleaseGateAction,
}

impl Default for ReleaseGateMap {
    fn default() -> Self {
        Self {
            major: ReleaseGateAction::Required,
            minor: ReleaseGateAction::Required,
            patch: ReleaseGateAction::Skip,
        }
    }
}

fn default_required() -> ReleaseGateAction {
    ReleaseGateAction::Required
}
fn default_skip() -> ReleaseGateAction {
    ReleaseGateAction::Skip
}

/// Which human roles are available to validate the UAT (controls the
/// orchestrator's activation function).
/// Which human roles can validate UAT for this project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct HumanAvailability {
    /// Whether a developer is available to validate functional flows.
    #[serde(default = "default_true")]
    pub developer: bool,
    /// Whether an architect is available to validate design/UX/consistency.
    #[serde(default = "default_true")]
    pub architect: bool,
}

impl Default for HumanAvailability {
    fn default() -> Self {
        Self {
            developer: true,
            architect: true,
        }
    }
}

/// Thresholds for the orchestrator's activation function (ADR-012): when is
/// a release worth the human's time?
/// Thresholds for the orchestrator's activation function (ADR-012).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ActivationThresholds {
    /// Minimum number of features required to activate UAT.
    #[serde(default = "default_three")]
    pub min_features: u32,
    /// Minimum diff lines required to activate UAT.
    #[serde(default = "default_two_hundred")]
    pub min_diff_lines: u32,
    /// Domain keywords (e.g. "auth", "payments") that trigger UAT activation.
    #[serde(default)]
    pub critical_domains: Vec<String>,
}

fn default_three() -> u32 {
    3
}
fn default_two_hundred() -> u32 {
    200
}

impl Default for ActivationThresholds {
    fn default() -> Self {
        Self {
            min_features: 3,
            min_diff_lines: 200,
            critical_domains: Vec::new(),
        }
    }
}

/// Per-project UAT configuration (XDG: `~/.local/share/sddk/projects/<id>/uat.toml`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatConfig {
    /// Per-release-type gate policy (major/minor/patch).
    #[serde(default)]
    pub release_gate: ReleaseGateMap,
    /// Which human roles are available to validate UAT.
    #[serde(default)]
    pub human: HumanAvailability,
    /// Thresholds for the orchestrator's activation function.
    #[serde(default)]
    pub activation: ActivationThresholds,
}

/// Evaluate the gate for a given release type under a config.
pub fn evaluate_release_gate(config: &UatConfig, release_type: ReleaseType) -> ReleaseGateAction {
    match release_type {
        ReleaseType::Major => config.release_gate.major,
        ReleaseType::Minor => config.release_gate.minor,
        ReleaseType::Patch => config.release_gate.patch,
    }
}

/// Derive a release type from the semver diff of two tags (`v1.5.2` vs `v1.4.0`).
/// Returns None when tags can't be parsed or are equal.
pub fn release_type_from_diff(current: &str, previous: &str) -> Option<ReleaseType> {
    let parse = |t: &str| -> Option<(u64, u64, u64)> {
        let s = t.trim_start_matches(|c: char| !c.is_ascii_digit());
        let mut parts = s.split('.');
        Some((
            parts.next()?.parse().ok()?,
            parts.next()?.parse().ok()?,
            parts.next()?.parse().ok()?,
        ))
    };
    let (cmaj, cmin, cpat) = parse(current)?;
    let (pmaj, pmin, ppat) = parse(previous)?;
    if cmaj > pmaj {
        Some(ReleaseType::Major)
    } else if cmin > pmin {
        Some(ReleaseType::Minor)
    } else if cpat > ppat {
        Some(ReleaseType::Patch)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Plan migration (v1 → v2)
// ---------------------------------------------------------------------------

/// Migration status of a plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UatMigrationAction {
    AlreadyV2,
    Migrated,
    /// Plan ya estaba en v3 (idempotente).
    AlreadyV3,
    /// Migrado v1/v2 → v3.
    MigratedToV3,
}

/// Result of migrating a plan between schema versions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatMigrationReport {
    pub action: UatMigrationAction,
    pub from_version: u32,
    pub to_version: u32,
    pub features_touched: u32,
    pub scenarios_touched: u32,
    pub evidence_promoted: u32,
    pub risk_promoted: u32,
    pub timing_promoted: u32,
    /// Escenarios con 4 ejes v3 completos (ADR-014) tras la migración.
    #[serde(default)]
    pub scenarios_v3: u32,
    /// Escenarios con oracles deterministas asignados (v3).
    #[serde(default)]
    pub oracles_assigned: u32,
    /// Escenarios con review policy asignada (v3).
    #[serde(default)]
    pub reviews_assigned: u32,
}

pub const LATEST_PLAN_SCHEMA_VERSION: u32 = 4;

/// Migrate a `UatPlan` from v1 to v2 in an additive, idempotent way.
pub fn migrate_plan_v1_to_v2(plan: &mut UatPlan) -> UatMigrationReport {
    let from_version = plan.schema_version;
    let mut features_touched = 0u32;
    let mut scenarios_touched = 0u32;
    let mut evidence_promoted = 0u32;
    let mut risk_promoted = 0u32;
    let mut timing_promoted = 0u32;

    if plan.schema_version >= 2 {
        return UatMigrationReport {
            action: UatMigrationAction::AlreadyV2,
            from_version,
            to_version: plan.schema_version,
            features_touched,
            scenarios_touched,
            evidence_promoted,
            risk_promoted,
            timing_promoted,
            scenarios_v3: 0,
            oracles_assigned: 0,
            reviews_assigned: 0,
        };
    }

    plan.schema_version = 2;

    for feature in &mut plan.features {
        features_touched += 1;
        if feature.design_ref.is_none() {
            feature.design_ref = None;
        }
        for scenario in &mut feature.scenarios {
            scenarios_touched += 1;
            let timing_window = scenario
                .context
                .as_ref()
                .and_then(|c| c.timing.as_ref())
                .map(|t| t.window.clone())
                .unwrap_or_else(|| "smoke".into());
            let timing_timeout = scenario
                .context
                .as_ref()
                .and_then(|c| c.timing.as_ref())
                .map(|t| t.timeout_min)
                .unwrap_or_else(|| scenario.est_minutes.max(10));
            let timing_parallel = scenario
                .context
                .as_ref()
                .and_then(|c| c.timing.as_ref())
                .map(|t| t.parallel_safe)
                .unwrap_or(false);
            let timing_order = scenario
                .context
                .as_ref()
                .and_then(|c| c.timing.as_ref())
                .and_then(|t| t.order_hint.clone());
            let context = scenario
                .context
                .get_or_insert_with(UatScenarioContext::default);
            context.timing = Some(UatTiming {
                window: timing_window,
                parallel_safe: timing_parallel,
                order_hint: timing_order,
                timeout_min: timing_timeout,
            });
            timing_promoted += 1;

            if scenario.evidence.is_none()
                && let Some(prompt) = &scenario.evidence_prompt
                && !prompt.trim().is_empty()
            {
                scenario.evidence = Some(UatEvidenceSpec {
                    required: true,
                    kinds: vec![UatEvidenceKindItem {
                        kind: UatEvidenceKind::Note,
                        r#ref: None,
                        match_mode: None,
                        expected_value: None,
                        min_bytes: None,
                    }],
                    retention_days: 90,
                });
                evidence_promoted += 1;
            }

            if scenario.risk.is_none() {
                let (classification, blast) = match scenario.priority {
                    UatPriority::P0 => (
                        UatRiskClassification::Critical,
                        UatBlastRadius::ReleaseBlocker,
                    ),
                    UatPriority::P1 => {
                        (UatRiskClassification::High, UatBlastRadius::ReleaseBlocker)
                    }
                    UatPriority::P2 => (
                        UatRiskClassification::Medium,
                        UatBlastRadius::FeatureBlocker,
                    ),
                };
                scenario.risk = Some(UatRisk {
                    classification,
                    blast_radius: blast,
                    mitigation: None,
                });
                risk_promoted += 1;
            }

            if scenario.automation.is_none() {
                scenario.automation = Some(UatAutomation {
                    status: UatAutomationStatus::Manual,
                    r#ref: None,
                    ci_job: None,
                    when: None,
                });
            }

            if scenario.provenance.is_none() {
                scenario.provenance = Some(UatProvenance {
                    author: plan.generated_by.clone(),
                    created_at: plan.generated_at.clone(),
                    last_modified_at: plan.generated_at.clone(),
                    origin: UatOrigin::Spec,
                    origin_ref: feature.requirement_ref.clone(),
                });
            }
        }
    }

    UatMigrationReport {
        action: UatMigrationAction::Migrated,
        from_version,
        to_version: 2,
        features_touched,
        scenarios_touched,
        evidence_promoted,
        risk_promoted,
        timing_promoted,
        scenarios_v3: 0,
        oracles_assigned: 0,
        reviews_assigned: 0,
    }
}

// ---------------------------------------------------------------------------
// UAT v3 — migración v2 → v3 (ADR-014)
// ---------------------------------------------------------------------------

/// Migrate a `UatPlan` to v3 in an additive, idempotent way.
///
/// v3 desacopla `automation.status` en cuatro ejes por scenario:
/// `executor` (eje 1), `evidence_bundle` (eje 2), `oracles[]` (eje 3) y
/// `review` (eje 4), más `acceptance` (REQ-RF-023). La migración es
/// heurística y conservadora:
/// - `automation.status: scripted|automated` con `ref` → executor
///   `cli|script` + oracle determinista `exit_code`.
/// - `automation.status: manual` o sin automation → executor `human` +
///   review policy que exige humano.
/// - Prioridad P0/P1 eleva la review policy a risk-based con trigger de
///   criticidad de negocio.
pub fn migrate_plan_v2_to_v3(plan: &mut UatPlan) -> UatMigrationReport {
    let from_version = plan.schema_version;
    let mut features_touched = 0u32;
    let mut scenarios_touched = 0u32;
    let mut scenarios_v3 = 0u32;
    let mut oracles_assigned = 0u32;
    let mut reviews_assigned = 0u32;

    if plan.schema_version >= 3 {
        return UatMigrationReport {
            action: UatMigrationAction::AlreadyV3,
            from_version,
            to_version: plan.schema_version,
            features_touched: 0,
            scenarios_touched: 0,
            evidence_promoted: 0,
            risk_promoted: 0,
            timing_promoted: 0,
            scenarios_v3,
            oracles_assigned,
            reviews_assigned,
        };
    }

    plan.schema_version = 3;

    for feature in &mut plan.features {
        features_touched += 1;
        for scenario in &mut feature.scenarios {
            scenarios_touched += 1;

            // Eje 1 — executor desde automation (si existe).
            let automation = scenario.automation.take();
            let executor = match &automation {
                Some(a) if a.status == UatAutomationStatus::Scripted => UatExecutorSpec {
                    kind: UatExecutorKind::Script,
                    command: a.r#ref.clone(),
                    url: None,
                    goal: None,
                    model: None,
                    timeout_ms: None,
                },
                Some(a) if a.status == UatAutomationStatus::Automated => UatExecutorSpec {
                    kind: UatExecutorKind::Cli,
                    command: a.r#ref.clone(),
                    url: None,
                    goal: None,
                    model: None,
                    timeout_ms: None,
                },
                _ => UatExecutorSpec {
                    kind: UatExecutorKind::Human,
                    command: None,
                    url: None,
                    goal: None,
                    model: None,
                    timeout_ms: None,
                },
            };
            scenario.executor = Some(executor);
            scenarios_v3 += 1;

            // Eje 3 — oracle determinista para executors no humanos.
            let automated = matches!(
                scenario.executor.as_ref().map(|e| e.kind),
                Some(UatExecutorKind::Cli | UatExecutorKind::Script)
            );
            if automated && scenario.oracles.is_empty() {
                scenario.oracles.push(UatOracleSpec {
                    kind: UatOracleKind::ExitCode,
                    expect: Some(serde_json::json!({ "code": 0 })),
                    rubric: Vec::new(),
                    severity: None,
                    blocking: true,
                });
                oracles_assigned += 1;
            }

            // Eje 4 — review policy según riesgo.
            if scenario.review.is_none() {
                let critical = matches!(scenario.priority, UatPriority::P0);
                scenario.review = Some(UatReviewPolicy {
                    kind: if automated && !critical {
                        UatReviewPolicyKind::Never
                    } else {
                        UatReviewPolicyKind::RiskBased
                    },
                    require_human_when: if critical {
                        vec![UatReviewTrigger::BusinessCriticalityHigh]
                    } else {
                        Vec::new()
                    },
                    sampling: if critical { 0.02 } else { 0.0 },
                });
                reviews_assigned += 1;
            }

            // Aceptación inicial pendiente (REQ-RF-023).
            if scenario.acceptance.is_none() {
                scenario.acceptance = Some(UatAcceptanceStatus::Pending);
            }
        }
    }

    UatMigrationReport {
        action: UatMigrationAction::MigratedToV3,
        from_version,
        to_version: 3,
        features_touched,
        scenarios_touched,
        evidence_promoted: 0,
        risk_promoted: 0,
        timing_promoted: 0,
        scenarios_v3,
        oracles_assigned,
        reviews_assigned,
    }
}

// ---------------------------------------------------------------------------
// Manifest + integrity verification (v2)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatManifest {
    pub schema_version: u32,
    pub project_id: String,
    pub generated_at: String,
    #[serde(default)]
    pub entries: Vec<UatManifestEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatManifestEntry {
    pub sha256: String,
    pub path: String,
    pub size_bytes: u64,
    pub captured_at: String,
    pub scenario_id: String,
    pub session_id: String,
    pub kind: UatEvidenceKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime: Option<String>,
}

impl UatManifest {
    pub const SCHEMA_VERSION: u32 = 1;
    pub fn new(project_id: impl Into<String>, generated_at: impl Into<String>) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION,
            project_id: project_id.into(),
            generated_at: generated_at.into(),
            entries: Vec::new(),
        }
    }
    pub fn upsert(&mut self, entry: UatManifestEntry) {
        if let Some(slot) = self.entries.iter_mut().find(|e| e.sha256 == entry.sha256) {
            *slot = entry;
        } else {
            self.entries.push(entry);
        }
    }
    pub fn lookup(&self, sha256_ref: &str) -> Option<&UatManifestEntry> {
        let key = sha256_ref.strip_prefix("sha256:").unwrap_or(sha256_ref);
        self.entries
            .iter()
            .find(|e| e.sha256 == key || e.sha256.strip_prefix("sha256:") == Some(key))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatIntegrityFinding {
    pub scenario_id: String,
    pub sha256: String,
    pub kind: UatEvidenceKind,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_size_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_size_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatIntegrityReport {
    pub session_id: String,
    pub project_id: String,
    pub verified_at: String,
    pub total_evidence: u32,
    pub findings: Vec<UatIntegrityFinding>,
    pub verdict: String,
}

impl UatIntegrityReport {
    pub fn compute_verdict(findings: &[UatIntegrityFinding]) -> &'static str {
        if findings.is_empty() {
            return "ok";
        }
        let has_fail = findings.iter().any(|f| {
            matches!(
                f.status.as_str(),
                "missing" | "hash_mismatch" | "size_mismatch" | "value_mismatch"
            )
        });
        let has_partial = findings.iter().any(|f| f.status == "no_payload");
        match (has_fail, has_partial) {
            (true, _) => "fail",
            (false, true) => "partial",
            (false, false) => "ok",
        }
    }
}

/// Return true when captured evidence satisfies every requirement declared by
/// the scenario. Optional evidence never blocks a result.
pub fn evidence_satisfies_spec(spec: Option<&UatEvidenceSpec>, evidence: &[UatEvidence]) -> bool {
    let Some(spec) = spec else {
        return true;
    };
    if !spec.required {
        return true;
    }
    if evidence.is_empty() {
        return false;
    }
    if spec.kinds.is_empty() {
        return true;
    }

    spec.kinds.iter().all(|required| {
        evidence.iter().any(|actual| {
            if actual.kind != required.kind || actual.r#ref.trim().is_empty() {
                return false;
            }
            if let Some(min_bytes) = required.min_bytes
                && actual.size_bytes.unwrap_or(0) < min_bytes
            {
                return false;
            }
            let Some(expected) = required.expected_value.as_deref() else {
                return true;
            };
            let Some(observed) = actual.observed_value.as_deref() else {
                return false;
            };
            match required.match_mode.unwrap_or(UatExpectedCheck::ExactMatch) {
                UatExpectedCheck::ExactMatch | UatExpectedCheck::ExitCode => observed == expected,
                UatExpectedCheck::Contains => observed.contains(expected),
                UatExpectedCheck::Regex => regex_lite_match(expected, observed).unwrap_or(false),
                UatExpectedCheck::JsonPath => false,
            }
        })
    })
}

/// Compute sha256 of a byte slice and return as `sha256:<lowercase-hex>`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let digest = sha2_digest(bytes);
    let mut out = String::with_capacity(7 + 64);
    out.push_str("sha256:");
    for byte in digest {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

fn sha2_digest(bytes: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h = [
        0x6a09e667u32,
        0xbb67ae85,
        0x3c6ef372,
        0xa54ff53a,
        0x510e527f,
        0x9b05688c,
        0x1f83d9ab,
        0x5be0cd19,
    ];
    let bit_len = (bytes.len() as u64) * 8;
    let mut msg = bytes.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[4 * i],
                chunk[4 * i + 1],
                chunk[4 * i + 2],
                chunk[4 * i + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let mj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(mj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    let mut out = [0u8; 32];
    for (i, word) in h.iter().enumerate() {
        out[4 * i..4 * i + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

fn regex_lite_match(pattern: &str, haystack: &str) -> Result<bool, regex::Error> {
    regex::Regex::new(pattern).map(|re| re.is_match(haystack))
}

/// Verify one evidence entry against the manifest + on-disk file.
pub fn verify_evidence(
    entry: &UatEvidence,
    manifest_entry: Option<&UatManifestEntry>,
    evidence_bytes: Option<&[u8]>,
) -> UatIntegrityFinding {
    let sha256 = entry.r#ref.clone();
    let mut finding = UatIntegrityFinding {
        scenario_id: String::new(),
        sha256: sha256.clone(),
        kind: entry.kind,
        status: "ok".into(),
        expected_size_bytes: entry.size_bytes,
        observed_size_bytes: None,
        message: None,
    };

    match entry.kind {
        UatEvidenceKind::Assertion | UatEvidenceKind::Metric => {
            if entry.observed_value.is_none() {
                finding.status = "no_payload".into();
                finding.message = Some(format!(
                    "{:?} evidence has no observed_value; cannot verify the run",
                    entry.kind
                ));
            } else if let (Some(observed), Some(expected)) =
                (&entry.observed_value, &entry.expected_value)
            {
                let matches = match entry.match_mode.unwrap_or(UatExpectedCheck::ExactMatch) {
                    UatExpectedCheck::ExactMatch => observed == expected,
                    UatExpectedCheck::Contains => observed.contains(expected),
                    UatExpectedCheck::Regex => {
                        matches!(regex_lite_match(expected, observed), Ok(true))
                    }
                    _ => true,
                };
                if !matches {
                    finding.status = "value_mismatch".into();
                    finding.message = Some(format!(
                        "observed {:?} did not match expected {:?} (mode {:?})",
                        observed, expected, entry.match_mode
                    ));
                }
            }
            return finding;
        }
        UatEvidenceKind::Note => return finding,
        _ => {}
    }

    if let Some(bytes) = evidence_bytes {
        let computed = sha256_hex(bytes);
        if computed != entry.r#ref {
            finding.status = "hash_mismatch".into();
            finding.message = Some(format!(
                "computed {} does not match recorded {}",
                computed, entry.r#ref
            ));
        } else if let Some(expected) = entry.size_bytes {
            if bytes.len() as u64 != expected {
                finding.status = "size_mismatch".into();
                finding.observed_size_bytes = Some(bytes.len() as u64);
                finding.message = Some(format!("size {} != recorded {}", bytes.len(), expected));
            }
        } else {
            finding.observed_size_bytes = Some(bytes.len() as u64);
        }
        return finding;
    }

    match manifest_entry {
        Some(m) => {
            if let Some(expected) = entry.size_bytes
                && m.size_bytes != expected
            {
                finding.status = "size_mismatch".into();
                finding.observed_size_bytes = Some(m.size_bytes);
                finding.message = Some(format!(
                    "manifest size {} != evidence size {}",
                    m.size_bytes, expected
                ));
                return finding;
            }
            finding.expected_size_bytes = Some(m.size_bytes);
            let key = entry.r#ref.strip_prefix("sha256:").unwrap_or(&entry.r#ref);
            if m.sha256 != key && m.sha256 != entry.r#ref {
                finding.status = "hash_mismatch".into();
                finding.message =
                    Some(format!("manifest {} != evidence {}", m.sha256, entry.r#ref));
            }
            finding
        }
        None => {
            finding.status = "no_payload".into();
            finding.message =
                Some("no manifest entry and no embedded payload; cannot verify hash".into());
            finding
        }
    }
}

// ---------------------------------------------------------------------------
// P2: scenario-context suggester
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatContextSuggestion {
    pub scenario_id: String,
    pub field: String,
    pub kind: String,
    pub reason: String,
    pub proposed: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatScenarioSuggestions {
    pub scenario_id: String,
    pub feature_id: String,
    pub scenario_title: String,
    pub populated_fields: u32,
    pub missing_fields: u32,
    pub suggestions: Vec<UatContextSuggestion>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatSuggestionsReport {
    pub plan_ref: String,
    pub plan_version: u32,
    pub total_scenarios: u32,
    pub fully_populated: u32,
    pub partial: u32,
    pub suggestions_count: u32,
    pub scenarios: Vec<UatScenarioSuggestions>,
}

fn count_populated_fields(scenario: &UatScenario) -> u32 {
    let mut n = 0u32;
    if let Some(ctx) = &scenario.context {
        if ctx.user_story.is_some() {
            n += 1;
        }
        if !ctx.preconditions.is_empty() {
            n += 1;
        }
        if !ctx.test_data.is_empty() {
            n += 1;
        }
        if ctx.workspace.is_some() {
            n += 1;
        }
        if ctx.timing.is_some() {
            n += 1;
        }
        if ctx.help.is_some() {
            n += 1;
        }
        if ctx.failure_protocol.is_some() {
            n += 1;
        }
        if !ctx.postconditions.is_empty() {
            n += 1;
        }
    }
    if scenario.evidence.is_some() {
        n += 1;
    }
    if scenario.risk.is_some() {
        n += 1;
    }
    if scenario.automation.is_some() {
        n += 1;
    }
    if scenario.provenance.is_some() {
        n += 1;
    }
    if !scenario
        .rationale
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        n += 1;
    }
    n
}

pub fn suggest_scenario_context(plan: &UatPlan) -> UatSuggestionsReport {
    let mut scenarios = Vec::new();
    let mut total_suggestions = 0u32;
    let mut fully = 0u32;
    let mut partial = 0u32;
    for feature in &plan.features {
        for scenario in &feature.scenarios {
            let mut out = Vec::new();
            let sid = scenario.id.clone();
            let populated = count_populated_fields(scenario);

            let has_user_story = scenario
                .context
                .as_ref()
                .and_then(|c| c.user_story.as_ref())
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false);
            if !has_user_story {
                out.push(UatContextSuggestion {
                    scenario_id: sid.clone(),
                    field: "context.user_story".into(),
                    kind: "missing".into(),
                    reason: format!("no user story; derive from title: {}", scenario.title),
                    proposed: serde_json::Value::String(String::new()),
                });
            }

            let needs_bash = scenario
                .plain_steps
                .iter()
                .any(|s| matches!(s.kind, Some(UatStepKind::Shell)));
            let has_preconditions = scenario
                .context
                .as_ref()
                .map(|c| !c.preconditions.is_empty())
                .unwrap_or(false);
            if !has_preconditions && !scenario.plain_steps.is_empty() {
                let mut preconditions: Vec<String> = Vec::new();
                if needs_bash {
                    preconditions.push("bash (or zsh) in PATH".into());
                }
                if !scenario.plain_steps.is_empty() {
                    preconditions.push(format!(
                        "plan file {} present in cwd",
                        plan.release.candidate
                    ));
                }
                if !preconditions.is_empty() {
                    out.push(UatContextSuggestion {
                        scenario_id: sid.clone(),
                        field: "context.preconditions".into(),
                        kind: "missing".into(),
                        reason: "implied by step count".into(),
                        proposed: serde_json::to_value(preconditions)
                            .unwrap_or(serde_json::Value::Null),
                    });
                }
            }

            let has_timing = scenario
                .context
                .as_ref()
                .and_then(|c| c.timing.as_ref())
                .is_some();
            if !has_timing {
                let window = if scenario.flags.iter().any(|f| f == "smoke") {
                    "smoke"
                } else {
                    "regression"
                };
                let timeout_min = std::cmp::max(scenario.est_minutes.saturating_mul(2), 5);
                out.push(UatContextSuggestion {
                    scenario_id: sid.clone(),
                    field: "context.timing".into(),
                    kind: "missing".into(),
                    reason: "derive from est_minutes".into(),
                    proposed: serde_json::json!({"window": window, "parallel_safe": scenario.priority == UatPriority::P0, "timeout_min": timeout_min}),
                });
            }

            let has_workspace = scenario
                .context
                .as_ref()
                .and_then(|c| c.workspace.as_ref())
                .is_some();
            if !has_workspace {
                out.push(UatContextSuggestion {
                    scenario_id: sid.clone(),
                    field: "context.workspace".into(),
                    kind: "missing".into(),
                    reason: "defaults".into(),
                    proposed: serde_json::json!({"shell": "bash", "cwd": "<REPO_ROOT>"}),
                });
            }

            let has_help = scenario
                .context
                .as_ref()
                .and_then(|c| c.help.as_ref())
                .is_some();
            if !has_help {
                out.push(UatContextSuggestion {
                    scenario_id: sid.clone(),
                    field: "context.help".into(),
                    kind: "missing".into(),
                    reason: "linked to req + ADRs".into(),
                    proposed: serde_json::json!({"related_adrs": ["ADR-012"], "related_specs": [feature.requirement_ref]}),
                });
            }

            let has_failure = scenario
                .context
                .as_ref()
                .and_then(|c| c.failure_protocol.as_ref())
                .is_some();
            if !has_failure {
                out.push(UatContextSuggestion {
                    scenario_id: sid.clone(),
                    field: "context.failure_protocol".into(),
                    kind: "missing".into(),
                    reason: "default".into(),
                    proposed: serde_json::json!({"on_fail": ["ping @qa-lead"]}),
                });
            }

            let has_evidence = scenario
                .evidence
                .as_ref()
                .map(|e| !e.kinds.is_empty())
                .unwrap_or(false);
            let has_prompt = scenario
                .evidence_prompt
                .as_deref()
                .map(|p| !p.trim().is_empty())
                .unwrap_or(false);
            if !has_evidence && !has_prompt {
                out.push(UatContextSuggestion {
                    scenario_id: sid.clone(),
                    field: "evidence".into(),
                    kind: "missing".into(),
                    reason: "default to Note".into(),
                    proposed: serde_json::json!({"kinds": [{"kind": "note"}], "retention_days": 90}),
                });
            }

            if scenario.risk.is_none() {
                let (cls, blast) = match scenario.priority {
                    UatPriority::P0 => ("critical", "release_blocker"),
                    UatPriority::P1 => ("high", "release_blocker"),
                    UatPriority::P2 => ("medium", "feature_blocker"),
                };
                out.push(UatContextSuggestion {
                    scenario_id: sid.clone(),
                    field: "risk".into(),
                    kind: "missing".into(),
                    reason: "derive from priority".into(),
                    proposed: serde_json::json!({"classification": cls, "blast_radius": blast}),
                });
            }

            if scenario.automation.is_none() {
                out.push(UatContextSuggestion {
                    scenario_id: sid.clone(),
                    field: "automation".into(),
                    kind: "missing".into(),
                    reason: "default Manual".into(),
                    proposed: serde_json::json!({"status": "manual"}),
                });
            }

            if scenario.provenance.is_none() {
                out.push(UatContextSuggestion {
                    scenario_id: sid.clone(),
                    field: "provenance".into(),
                    kind: "missing".into(),
                    reason: "stamp from plan".into(),
                    proposed: serde_json::json!({"author": plan.generated_by, "origin_ref": feature.requirement_ref}),
                });
            }

            let missing = out.len() as u32;
            if missing == 0 {
                fully += 1;
            } else {
                partial += 1;
            }
            total_suggestions += missing;
            scenarios.push(UatScenarioSuggestions {
                scenario_id: scenario.id.clone(),
                feature_id: feature.id.clone(),
                scenario_title: scenario.title.clone(),
                populated_fields: populated,
                missing_fields: missing,
                suggestions: out,
            });
        }
    }
    UatSuggestionsReport {
        plan_ref: plan.release.candidate.clone(),
        plan_version: plan.schema_version,
        total_scenarios: scenarios.len() as u32,
        fully_populated: fully,
        partial,
        suggestions_count: total_suggestions,
        scenarios,
    }
}

pub fn apply_suggestion(scenario: &mut UatScenario, suggestion: &UatContextSuggestion) -> bool {
    match suggestion.field.as_str() {
        "context.user_story" => {
            if let Some(s) = suggestion.proposed.as_str()
                && !s.trim().is_empty()
            {
                let ctx = scenario
                    .context
                    .get_or_insert_with(UatScenarioContext::default);
                ctx.user_story = Some(s.to_string());
                return true;
            }
            false
        }
        "context.preconditions" => {
            if let Some(arr) = suggestion.proposed.as_array() {
                let preconditions: Vec<String> = arr
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
                if !preconditions.is_empty() {
                    let ctx = scenario
                        .context
                        .get_or_insert_with(UatScenarioContext::default);
                    ctx.preconditions = preconditions;
                    return true;
                }
            }
            false
        }
        "context.timing" => {
            if let Some(obj) = suggestion.proposed.as_object() {
                let timing = UatTiming {
                    window: obj
                        .get("window")
                        .and_then(|v| v.as_str())
                        .unwrap_or("smoke")
                        .into(),
                    parallel_safe: obj
                        .get("parallel_safe")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    order_hint: None,
                    timeout_min: obj
                        .get("timeout_min")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(10) as u32,
                };
                let ctx = scenario
                    .context
                    .get_or_insert_with(UatScenarioContext::default);
                ctx.timing = Some(timing);
                return true;
            }
            false
        }
        "context.workspace" => {
            if let Some(obj) = suggestion.proposed.as_object() {
                let workspace = UatWorkspace {
                    shell: obj.get("shell").and_then(|v| v.as_str()).map(String::from),
                    cwd: obj.get("cwd").and_then(|v| v.as_str()).map(String::from),
                    ..Default::default()
                };
                let ctx = scenario
                    .context
                    .get_or_insert_with(UatScenarioContext::default);
                ctx.workspace = Some(workspace);
                return true;
            }
            false
        }
        "context.help" => {
            if let Some(obj) = suggestion.proposed.as_object() {
                let help = UatHelp {
                    related_adrs: obj
                        .get("related_adrs")
                        .and_then(|v| v.as_array())
                        .map(|a| {
                            a.iter()
                                .filter_map(|s| s.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default(),
                    related_specs: obj
                        .get("related_specs")
                        .and_then(|v| v.as_array())
                        .map(|a| {
                            a.iter()
                                .filter_map(|s| s.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default(),
                    ..Default::default()
                };
                let ctx = scenario
                    .context
                    .get_or_insert_with(UatScenarioContext::default);
                ctx.help = Some(help);
                return true;
            }
            false
        }
        "context.failure_protocol" => {
            if let Some(obj) = suggestion.proposed.as_object() {
                let on_fail: Vec<String> = obj
                    .get("on_fail")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|s| s.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let ctx = scenario
                    .context
                    .get_or_insert_with(UatScenarioContext::default);
                ctx.failure_protocol = Some(UatFailureProtocol {
                    on_fail,
                    expected_defect_template: None,
                });
                return true;
            }
            false
        }
        "evidence" => {
            if let Some(obj) = suggestion.proposed.as_object() {
                let kinds: Vec<UatEvidenceKindItem> = obj
                    .get("kinds")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|item| {
                                let obj = item.as_object()?;
                                let kind_str = obj.get("kind")?.as_str()?;
                                let kind = match kind_str {
                                    "file" => UatEvidenceKind::File,
                                    "screenshot" => UatEvidenceKind::Screenshot,
                                    "command_output" => UatEvidenceKind::CommandOutput,
                                    "assertion" => UatEvidenceKind::Assertion,
                                    "metric" => UatEvidenceKind::Metric,
                                    _ => UatEvidenceKind::Note,
                                };
                                Some(UatEvidenceKindItem {
                                    kind,
                                    r#ref: None,
                                    match_mode: None,
                                    expected_value: None,
                                    min_bytes: None,
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                scenario.evidence = Some(UatEvidenceSpec {
                    required: true,
                    kinds,
                    retention_days: 90,
                });
                return true;
            }
            false
        }
        "risk" => {
            if let Some(obj) = suggestion.proposed.as_object() {
                let classification = match obj
                    .get("classification")
                    .and_then(|v| v.as_str())
                    .unwrap_or("medium")
                {
                    "critical" => UatRiskClassification::Critical,
                    "high" => UatRiskClassification::High,
                    _ => UatRiskClassification::Medium,
                };
                let blast_radius = match obj
                    .get("blast_radius")
                    .and_then(|v| v.as_str())
                    .unwrap_or("feature_blocker")
                {
                    "release_blocker" => UatBlastRadius::ReleaseBlocker,
                    "advisory" => UatBlastRadius::Advisory,
                    _ => UatBlastRadius::FeatureBlocker,
                };
                scenario.risk = Some(UatRisk {
                    classification,
                    blast_radius,
                    mitigation: None,
                });
                return true;
            }
            false
        }
        "automation" => {
            if suggestion.proposed.is_object() {
                scenario.automation = Some(UatAutomation {
                    status: UatAutomationStatus::Manual,
                    r#ref: None,
                    ci_job: None,
                    when: None,
                });
                return true;
            }
            false
        }
        "provenance" => {
            if let Some(obj) = suggestion.proposed.as_object() {
                scenario.provenance = Some(UatProvenance {
                    author: obj
                        .get("author")
                        .and_then(|v| v.as_str())
                        .unwrap_or("uat-planner")
                        .into(),
                    created_at: plan_gen_at_static(),
                    last_modified_at: plan_gen_at_static(),
                    origin: UatOrigin::Spec,
                    origin_ref: obj
                        .get("origin_ref")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                });
                return true;
            }
            false
        }
        _ => false,
    }
}

fn plan_gen_at_static() -> String {
    "2026-08-07T00:00:00Z".into()
}

pub fn apply_all_suggestions(plan: &mut UatPlan, report: &UatSuggestionsReport) -> u32 {
    let mut applied = 0u32;
    for scenario_report in &report.scenarios {
        let target_id = &scenario_report.scenario_id;
        for feature in &mut plan.features {
            for scenario in &mut feature.scenarios {
                if &scenario.id != target_id {
                    continue;
                }
                for suggestion in &scenario_report.suggestions {
                    if apply_suggestion(scenario, suggestion) {
                        applied += 1;
                    }
                }
            }
        }
    }
    applied
}

// ---------------------------------------------------------------------------
// P4: scenario history aggregation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatScenarioHistory {
    pub scenario_id: String,
    pub feature_id: String,
    pub scenario_title: String,
    pub runs_total: u32,
    pub runs_passing: u32,
    pub runs_failing: u32,
    pub runs_blocked: u32,
    #[serde(default)]
    pub runs_not_run: u32,
    pub success_rate: f64,
    pub flakiness_score: f64,
    pub first_run: Option<UatRunRef>,
    pub last_run: Option<UatRunRef>,
    pub defect_ids: Vec<String>,
    pub avg_duration_ms: Option<u64>,
    pub p95_duration_ms: Option<u64>,
    pub trend: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub struct UatRunRef {
    pub session_id: String,
    pub at: String,
    pub status: String,
    pub commit: Option<String>,
    pub tester_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatFeatureHistory {
    pub feature_id: String,
    pub feature_name: String,
    pub coverage_pct: f64,
    pub scenarios_total: u32,
    pub scenarios_passing: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UatHistoryReport {
    pub schema_version: u32,
    pub release: String,
    pub plan_ref: String,
    pub generated_at: String,
    pub sessions_total: u32,
    pub defects_total: u32,
    pub features: Vec<UatFeatureHistory>,
    pub scenarios: Vec<UatScenarioHistory>,
}

impl UatHistoryReport {
    pub const SCHEMA_VERSION: u32 = 1;
}

fn compute_trend(runs: &[&str]) -> &'static str {
    let weight = |s: &&str| -> f64 {
        match s.to_ascii_uppercase().as_str() {
            "PASS" => 1.0,
            "PARTIAL" => 0.5,
            _ => 0.0,
        }
    };
    if runs.len() < 4 {
        return "stable";
    }
    let recent = &runs[runs.len() - 3..];
    let prior = &runs[runs.len() - 6..runs.len() - 3];
    let recent_avg: f64 = recent.iter().map(weight).sum::<f64>() / recent.len() as f64;
    let prior_avg: f64 = prior.iter().map(weight).sum::<f64>() / prior.len() as f64;
    let delta = recent_avg - prior_avg;
    if delta > 0.2 {
        "improving"
    } else if delta < -0.2 {
        "degrading"
    } else {
        "stable"
    }
}

fn p95(durations: &[u64]) -> Option<u64> {
    if durations.is_empty() {
        return None;
    }
    let mut sorted = durations.to_vec();
    sorted.sort_unstable();
    let idx = ((sorted.len() as f64 * 0.95).ceil() as usize).saturating_sub(1);
    Some(sorted[idx.min(sorted.len() - 1)])
}

pub fn aggregate_history(
    plan: &UatPlan,
    sessions: &[UatSession],
    release: &str,
    generated_at: &str,
) -> UatHistoryReport {
    let mut runs_by_scenario: std::collections::BTreeMap<
        String,
        Vec<(&UatScenarioResult, &UatSession)>,
    > = std::collections::BTreeMap::new();
    for session in sessions {
        for result in &session.results {
            runs_by_scenario
                .entry(result.scenario_id.clone())
                .or_default()
                .push((result, session));
        }
    }

    let mut scenarios = Vec::new();
    let mut defects_total = 0u32;

    for feature in &plan.features {
        for scenario in &feature.scenarios {
            let runs = runs_by_scenario
                .get(&scenario.id)
                .cloned()
                .unwrap_or_default();
            let runs_total = runs.len() as u32;
            let mut runs_passing = 0u32;
            let mut runs_failing = 0u32;
            let mut runs_blocked = 0u32;
            let mut runs_not_run = 0u32;
            let mut statuses_for_trend: Vec<&str> = Vec::new();
            let mut defect_ids: Vec<String> = Vec::new();
            let mut durations: Vec<u64> = Vec::new();
            let mut first_run: Option<UatRunRef> = None;
            let mut last_run: Option<UatRunRef> = None;

            for (i, (result, session)) in runs.iter().enumerate() {
                let status = match result.status {
                    UatStatus::NotRun => "NOT_RUN",
                    UatStatus::Pass => "PASS",
                    UatStatus::Fail => "FAIL",
                    UatStatus::Blocked => "BLOCKED",
                    UatStatus::Partial => "PARTIAL",
                };
                statuses_for_trend.push(status);
                match result.status {
                    UatStatus::NotRun => runs_not_run += 1,
                    UatStatus::Pass => runs_passing += 1,
                    UatStatus::Fail => {
                        runs_failing += 1;
                        defects_total += 1;
                    }
                    UatStatus::Blocked => runs_blocked += 1,
                    UatStatus::Partial => {}
                }
                if let Some(d) = result.linked_defect.as_deref()
                    && !defect_ids.contains(&d.to_string())
                {
                    defect_ids.push(d.to_string());
                }
                if let Some(ms) = result.verdict_duration_ms {
                    durations.push(ms);
                } else if result.duration_minutes > 0 {
                    durations.push(result.duration_minutes as u64 * 60_000);
                }
                let commit = session
                    .metadata
                    .as_ref()
                    .and_then(|m| m.build.as_ref())
                    .and_then(|b| b.commit.clone());
                let tester_id = session
                    .metadata
                    .as_ref()
                    .and_then(|m| m.tester.as_ref())
                    .map(|t| t.id.clone());
                let at = result
                    .verdict_at
                    .clone()
                    .or_else(|| session.finished_at.clone())
                    .unwrap_or_else(|| session.started_at.clone());
                let run_ref = UatRunRef {
                    session_id: session.session_id.clone(),
                    at,
                    status: status.to_string(),
                    commit,
                    tester_id,
                };
                if i == 0 {
                    first_run = Some(run_ref.clone());
                }
                last_run = Some(run_ref);
            }

            let success_rate = if runs_total > 0 {
                runs_passing as f64 / runs_total as f64
            } else {
                0.0
            };
            let flakiness_score = if runs_total > 0 {
                1.0 - success_rate
            } else {
                0.0
            };
            let avg_duration_ms = if !durations.is_empty() {
                Some(durations.iter().sum::<u64>() / durations.len() as u64)
            } else {
                None
            };
            let p95_duration_ms = p95(&durations);
            let trend = compute_trend(&statuses_for_trend);

            scenarios.push(UatScenarioHistory {
                scenario_id: scenario.id.clone(),
                feature_id: feature.id.clone(),
                scenario_title: scenario.title.clone(),
                runs_total,
                runs_passing,
                runs_failing,
                runs_blocked,
                runs_not_run,
                success_rate,
                flakiness_score,
                first_run,
                last_run,
                defect_ids,
                avg_duration_ms,
                p95_duration_ms,
                trend: trend.to_string(),
            });
        }
    }

    let mut features = Vec::new();
    for feature in &plan.features {
        let total = feature.scenarios.len() as u32;
        let passing = scenarios
            .iter()
            .filter(|s| s.feature_id == feature.id && s.runs_passing > 0)
            .count() as u32;
        let coverage_pct = if total > 0 {
            100.0 * passing as f64 / total as f64
        } else {
            0.0
        };
        features.push(UatFeatureHistory {
            feature_id: feature.id.clone(),
            feature_name: feature.name.clone(),
            coverage_pct,
            scenarios_total: total,
            scenarios_passing: passing,
        });
    }

    UatHistoryReport {
        schema_version: UatHistoryReport::SCHEMA_VERSION,
        release: release.to_string(),
        plan_ref: plan.release.candidate.clone(),
        generated_at: generated_at.to_string(),
        sessions_total: sessions.len() as u32,
        defects_total,
        features,
        scenarios,
    }
}

#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn default_policy_blocks_major_minor_skips_patch() {
        let config = UatConfig::default();
        assert_eq!(
            evaluate_release_gate(&config, ReleaseType::Major),
            ReleaseGateAction::Required
        );
        assert_eq!(
            evaluate_release_gate(&config, ReleaseType::Minor),
            ReleaseGateAction::Required
        );
        assert_eq!(
            evaluate_release_gate(&config, ReleaseType::Patch),
            ReleaseGateAction::Skip
        );
    }

    #[test]
    fn custom_policy_overrides_defaults() {
        let toml = r#"
            [release_gate]
            major = "skip"
            minor = "advisory"
            patch = "required"
        "#;
        let config: UatConfig = toml::from_str(toml).unwrap();
        assert_eq!(
            evaluate_release_gate(&config, ReleaseType::Major),
            ReleaseGateAction::Skip
        );
        assert_eq!(
            evaluate_release_gate(&config, ReleaseType::Minor),
            ReleaseGateAction::Advisory
        );
        assert_eq!(
            evaluate_release_gate(&config, ReleaseType::Patch),
            ReleaseGateAction::Required
        );
    }

    #[test]
    fn release_type_from_diff_basic() {
        assert_eq!(
            release_type_from_diff("v1.5.2", "v1.4.0"),
            Some(ReleaseType::Minor)
        );
        assert_eq!(
            release_type_from_diff("v2.0.0", "v1.9.9"),
            Some(ReleaseType::Major)
        );
        assert_eq!(
            release_type_from_diff("v1.5.2", "v1.5.1"),
            Some(ReleaseType::Patch)
        );
        assert_eq!(release_type_from_diff("v1.5.2", "v1.5.2"), None);
        assert_eq!(release_type_from_diff("not-a-tag", "v1.0.0"), None);
    }

    #[test]
    fn value_mismatch_fails_integrity() {
        let finding = UatIntegrityFinding {
            scenario_id: "S-1".into(),
            sha256: "sha256:test".into(),
            kind: UatEvidenceKind::Assertion,
            status: "value_mismatch".into(),
            expected_size_bytes: None,
            observed_size_bytes: None,
            message: None,
        };
        assert_eq!(UatIntegrityReport::compute_verdict(&[finding]), "fail");
    }

    #[test]
    fn required_evidence_must_match_declared_kinds_and_values() {
        let spec = UatEvidenceSpec {
            required: true,
            kinds: vec![UatEvidenceKindItem {
                kind: UatEvidenceKind::Assertion,
                r#ref: None,
                match_mode: Some(UatExpectedCheck::ExactMatch),
                expected_value: Some("200".into()),
                min_bytes: None,
            }],
            retention_days: 90,
        };
        let mismatched = UatEvidence {
            kind: UatEvidenceKind::Assertion,
            r#ref: "sha256:test".into(),
            note: None,
            captured_at: None,
            size_bytes: None,
            mime: None,
            path: None,
            observed_value: Some("500".into()),
            expected_value: Some("200".into()),
            match_mode: Some(UatExpectedCheck::ExactMatch),
        };
        assert!(!evidence_satisfies_spec(Some(&spec), &[]));
        assert!(!evidence_satisfies_spec(Some(&spec), &[mismatched]));
    }

    #[test]
    fn not_run_has_stable_wire_value() {
        assert_eq!(
            serde_json::to_string(&UatStatus::NotRun).unwrap(),
            "\"NOT_RUN\""
        );
    }
}

#[cfg(test)]
mod uat_v3_tests {
    use super::*;

    fn v2_plan() -> UatPlan {
        serde_saphyr::from_str(
            r#"
schema_version: 2
release: { candidate: v2.1.0 }
generated_by: test
generated_at: "2026-08-10T00:00:00Z"
features:
  - id: F-1
    name: Feature
    priority: P0
    scenarios:
      - id: S-1
        title: Scripted
        priority: P0
        automation:
          status: scripted
          ref: ./scripts/s1.sh
      - id: S-2
        title: Manual
        priority: P2
        automation:
          status: manual
"#,
        )
        .unwrap()
    }

    #[test]
    fn migrate_v2_to_v3_sets_four_axes() {
        let mut plan = v2_plan();
        let report = migrate_plan_v2_to_v3(&mut plan);
        assert_eq!(report.action, UatMigrationAction::MigratedToV3);
        assert_eq!(report.from_version, 2);
        assert_eq!(report.to_version, 3);
        assert_eq!(plan.schema_version, 3);
        assert_eq!(report.scenarios_touched, 2);
        assert_eq!(report.scenarios_v3, 2);
        assert_eq!(report.oracles_assigned, 1);
        assert_eq!(report.reviews_assigned, 2);

        let s1 = &plan.features[0].scenarios[0];
        // Eje 1: scripted → executor script con ref como command.
        let executor = s1.executor.as_ref().unwrap();
        assert_eq!(executor.kind, UatExecutorKind::Script);
        assert_eq!(executor.command.as_deref(), Some("./scripts/s1.sh"));
        // Eje 3: oracle exit_code determinista.
        assert_eq!(s1.oracles.len(), 1);
        assert_eq!(s1.oracles[0].kind, UatOracleKind::ExitCode);
        assert!(s1.oracles[0].blocking);
        // Eje 4: P0 → risk_based con trigger de criticidad.
        let review = s1.review.as_ref().unwrap();
        assert_eq!(review.kind, UatReviewPolicyKind::RiskBased);
        assert!(
            review
                .require_human_when
                .contains(&UatReviewTrigger::BusinessCriticalityHigh)
        );
        // Aceptación pendiente inicial.
        assert_eq!(s1.acceptance, Some(UatAcceptanceStatus::Pending));
    }

    #[test]
    fn migrate_manual_scenario_uses_human_executor() {
        let mut plan = v2_plan();
        migrate_plan_v2_to_v3(&mut plan);
        let s2 = &plan.features[0].scenarios[1];
        assert_eq!(s2.executor.as_ref().unwrap().kind, UatExecutorKind::Human);
        assert!(s2.oracles.is_empty());
        assert_eq!(
            s2.review.as_ref().unwrap().kind,
            UatReviewPolicyKind::RiskBased
        );
    }

    #[test]
    fn migrate_v2_to_v3_is_idempotent() {
        let mut plan = v2_plan();
        let first = migrate_plan_v2_to_v3(&mut plan);
        assert_eq!(first.action, UatMigrationAction::MigratedToV3);
        let second = migrate_plan_v2_to_v3(&mut plan);
        assert_eq!(second.action, UatMigrationAction::AlreadyV3);
        // Idempotencia: no duplica oracles ni cambia executor.
        let s1 = &plan.features[0].scenarios[0];
        assert_eq!(s1.oracles.len(), 1);
        assert_eq!(s1.executor.as_ref().unwrap().kind, UatExecutorKind::Script);
    }

    #[test]
    fn v2_plan_still_parses_with_v3_fields_absent() {
        // Un plan v2 sin campos v3 debe seguir parseando (OCP: extensión aditiva).
        let plan: UatPlan = serde_saphyr::from_str(
            r#"
schema_version: 2
release: { candidate: v2.1.0 }
generated_by: test
generated_at: "2026-08-10T00:00:00Z"
features:
  - id: F-1
    name: Feature
    scenarios:
      - id: S-1
        title: Plain
"#,
        )
        .unwrap();
        let s = &plan.features[0].scenarios[0];
        assert!(s.executor.is_none());
        assert!(s.evidence_bundle.is_none());
        assert!(s.oracles.is_empty());
        assert!(s.review.is_none());
        assert!(s.acceptance.is_none());
        assert_eq!(plan.schema_version, 2);
    }

    #[test]
    fn v3_round_trip_preserves_four_axes() {
        let mut plan = v2_plan();
        migrate_plan_v2_to_v3(&mut plan);
        let yaml = serde_saphyr::to_string(&plan).unwrap();
        let reparsed: UatPlan = serde_saphyr::from_str(&yaml).unwrap();
        assert_eq!(reparsed.schema_version, 3);
        let s = &reparsed.features[0].scenarios[0];
        assert_eq!(s.executor.as_ref().unwrap().kind, UatExecutorKind::Script);
        assert_eq!(s.oracles[0].kind, UatOracleKind::ExitCode);
        assert!(s.review.is_some());
    }

    #[test]
    fn execution_result_differs_from_acceptance() {
        // REQ-RF-023: PASSED != ACCEPTED — el dominio los modela separados.
        let passed = UatExecutionResult::Passed;
        let assessment = UatMachineAssessment::SupportedPass;
        let decision = UatHumanDecision::Pending;
        let acceptance = UatAcceptanceStatus::Pending;
        assert_eq!(passed, UatExecutionResult::Passed);
        assert_eq!(assessment, UatMachineAssessment::SupportedPass);
        assert_eq!(decision, UatHumanDecision::Pending);
        assert_eq!(acceptance, UatAcceptanceStatus::Pending);
        // Wire values estables.
        assert_eq!(
            serde_json::to_string(&UatExecutionResult::Failed).unwrap(),
            "\"FAILED\""
        );
        assert_eq!(
            serde_json::to_string(&UatHumanDecision::Waived).unwrap(),
            "\"waived\""
        );
    }

    #[test]
    fn oracle_assessment_carries_confidence_and_details() {
        let assessment = UatOracleAssessment {
            oracle: UatOracleSpec {
                kind: UatOracleKind::Http,
                expect: Some(serde_json::json!({ "status": 200 })),
                rubric: vec![],
                severity: None,
                blocking: true,
            },
            verdict: UatOracleVerdict::Pass,
            confidence: 1.0,
            details: Some("GET /health → 200".into()),
        };
        assert_eq!(assessment.verdict, UatOracleVerdict::Pass);
        assert_eq!(assessment.confidence, 1.0);
    }

    #[test]
    fn form_dsl_valid_spec_passes_validation() {
        let spec: UatFormSpec = serde_saphyr::from_str(
            r#"
dsl_version: 1
items:
  - kind: info
    text: "Bienvenido"
  - kind: check
    check:
      kind: blind_observation
      prompt: "¿Qué ves en la pantalla?"
      visibility: blind
      blocking: true
      evidence_requirement: [screenshot]
      options: ["Un formulario", "Un error", "Nada"]
  - kind: check
    check:
      kind: rating
      prompt: "Calidad visual"
      required: true
      options: ["1", "2", "3", "4", "5"]
  - kind: flow
    flow: stop
"#,
        )
        .unwrap();
        assert!(validate_form_dsl(&spec).is_empty());
    }

    #[test]
    fn form_dsl_rejects_out_of_vocabulary() {
        // comment_required_when fuera del vocabulario -> error estable.
        let spec: UatFormSpec = serde_saphyr::from_str(
            r#"
dsl_version: 1
items:
  - kind: check
    check:
      kind: yes_no
      prompt: "¿OK?"
      comment_required_when: "sometimes"
"#,
        )
        .unwrap();
        let errors = validate_form_dsl(&spec);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("comment_required_when"));
    }

    #[test]
    fn form_dsl_choice_without_options_is_rejected() {
        let spec: UatFormSpec = serde_saphyr::from_str(
            r#"
dsl_version: 1
items:
  - kind: check
    check:
      kind: single_choice
      prompt: "¿Cuál?"
"#,
        )
        .unwrap();
        let errors = validate_form_dsl(&spec);
        assert!(!errors.is_empty());
        assert!(errors[0].contains("options"));
    }

    #[test]
    fn form_dsl_rejects_unknown_kind() {
        // `kind: magic` no está en el enum -> serde falla.
        let raw = r#"
dsl_version: 1
items:
  - kind: magic
"#;
        let result: Result<UatFormSpec, _> = serde_saphyr::from_str(raw);
        assert!(result.is_err());
    }

    fn review_plan(acceptance: Option<UatAcceptanceStatus>) -> UatPlan {
        serde_saphyr::from_str(&format!(
            r#"
schema_version: 3
release: {{ candidate: v1.7.0 }}
generated_by: test
generated_at: "2026-08-10T00:00:00Z"
features:
  - id: F-01
    name: Feature
    scenarios:
      - id: S-P0
        title: Critical
        priority: P0
        assignee: developer
        executor: {{ kind: cli, command: "echo ok" }}
        acceptance: {acc}
      - id: S-1
        title: Normal one
        assignee: developer
        executor: {{ kind: cli, command: "echo ok" }}
      - id: S-2
        title: Normal two
        assignee: developer
        executor: {{ kind: cli, command: "echo ok" }}
      - id: S-3
        title: Normal three
        assignee: developer
        executor: {{ kind: cli, command: "echo ok" }}
"#,
            acc = acceptance
                .map(|a| serde_json::to_string(&a).unwrap())
                .unwrap_or_else(|| "null".into())
        ))
        .unwrap()
    }

    fn review_report(plan: &UatPlan) -> UatReport {
        UatReport {
            schema_version: 3,
            release: plan.release.candidate.clone(),
            plan_ref: plan.release.candidate.clone(),
            sessions: vec!["s1".into()],
            summary: UatReportSummary {
                total_scenarios: 4,
                passed: 4,
                failed: 0,
                blocked: 0,
                partial: 0,
                not_run: 0,
                coverage_pct: 100.0,
                defects: 0,
                ux_issues: 0,
                uat_duration_minutes: 0,
            },
            features: vec![UatFeatureRollup {
                id: "F-01".into(),
                name: "Feature".into(),
                coverage_pct: 100.0,
                scenarios: vec![
                    UatScenarioRollup {
                        scenario_id: "S-P0".into(),
                        status: UatStatus::Pass,
                        executor: None,
                        acceptance: Some(UatAcceptanceStatus::Pending),
                        acceptance_required: true,
                        oracle_verdicts: None,
                    },
                    UatScenarioRollup {
                        scenario_id: "S-1".into(),
                        status: UatStatus::Pass,
                        executor: None,
                        acceptance: None,
                        acceptance_required: false,
                        oracle_verdicts: None,
                    },
                    UatScenarioRollup {
                        scenario_id: "S-2".into(),
                        status: UatStatus::Pass,
                        executor: None,
                        acceptance: None,
                        acceptance_required: false,
                        oracle_verdicts: None,
                    },
                    UatScenarioRollup {
                        scenario_id: "S-3".into(),
                        status: UatStatus::Pass,
                        executor: None,
                        acceptance: None,
                        acceptance_required: false,
                        oracle_verdicts: None,
                    },
                ],
            }],
            verdict: UatVerdict::Ready,
            not_ready_blockers: vec![],
            acceptance_blockers: vec![],
        }
    }

    #[test]
    fn review_queue_always_includes_p0() {
        let plan = review_plan(None);
        let report = review_report(&plan);
        let queue = build_review_queue(&plan, &report, 0.0, "seed");
        assert!(
            queue
                .iter()
                .any(|i| i.scenario_id == "S-P0" && i.reason == UatReviewReason::Required)
        );
    }

    #[test]
    fn review_queue_sampling_is_deterministic() {
        let plan = review_plan(None);
        let report = review_report(&plan);
        let a = build_review_queue(&plan, &report, 0.5, "seed-x");
        let b = build_review_queue(&plan, &report, 0.5, "seed-x");
        assert_eq!(a, b, "same seed must produce the same queue");
        // sampling 0 -> only P0 (required).
        let none = build_review_queue(&plan, &report, 0.0, "seed-x");
        assert_eq!(none.len(), 1);
        assert_eq!(none[0].scenario_id, "S-P0");
    }

    #[test]
    fn review_queue_sampling_scales_with_fraction() {
        let plan = review_plan(None);
        let report = review_report(&plan);
        let low = build_review_queue(&plan, &report, 0.0, "seed");
        let high = build_review_queue(&plan, &report, 1.0, "seed");
        // sampling 1.0 -> todos los no-required entran.
        let sampled = high
            .iter()
            .filter(|i| i.reason == UatReviewReason::Sampled)
            .count();
        assert_eq!(sampled, 3);
        assert!(low.len() <= high.len());
    }
}

#[cfg(test)]
mod guided_runner_f13_domain_tests {
    use super::*;

    // -------------------------------------------------------------------------
    // Task 1.1: UatRunnerMode
    // -------------------------------------------------------------------------

    #[test]
    fn uat_runner_mode_serde_designer() {
        let mode = UatRunnerMode::Designer;
        let yaml = serde_saphyr::to_string(&mode).unwrap();
        assert!(yaml.contains("designer"));
        let round: UatRunnerMode = serde_saphyr::from_str(&yaml).unwrap();
        assert_eq!(round, mode);
    }

    #[test]
    fn uat_runner_mode_serde_runner() {
        let mode = UatRunnerMode::Runner;
        let json = serde_json::to_string(&mode).unwrap();
        assert!(json.contains("runner"));
        let round: UatRunnerMode = serde_json::from_str(&json).unwrap();
        assert_eq!(round, mode);
    }

    #[test]
    fn uat_runner_mode_serde_reviewer() {
        let mode = UatRunnerMode::Reviewer;
        let yaml = serde_saphyr::to_string(&mode).unwrap();
        assert!(yaml.contains("reviewer"));
        let round: UatRunnerMode = serde_saphyr::from_str(&yaml).unwrap();
        assert_eq!(round, mode);
    }

    // -------------------------------------------------------------------------
    // Task 1.1: UatEvidenceSummary
    // -------------------------------------------------------------------------

    #[test]
    fn uat_evidence_summary_serde() {
        let summary = UatEvidenceSummary {
            machine_passed: 8,
            machine_total: 10,
            fara_assessment: Some("PASS".into()),
            fara_confidence: Some(0.95),
            anomalies: vec!["selector-changed".into(), "text-mismatch".into()],
        };
        let yaml = serde_saphyr::to_string(&summary).unwrap();
        assert!(yaml.contains("machine_passed"));
        assert!(yaml.contains("8"));
        let round: UatEvidenceSummary = serde_saphyr::from_str(&yaml).unwrap();
        assert_eq!(round.machine_passed, 8);
        assert_eq!(round.machine_total, 10);
        assert_eq!(round.anomalies.len(), 2);
    }

    #[test]
    fn uat_evidence_summary_default() {
        let summary = UatEvidenceSummary::default();
        assert_eq!(summary.machine_passed, 0);
        assert_eq!(summary.machine_total, 0);
        assert!(summary.fara_assessment.is_none());
        assert!(summary.anomalies.is_empty());
    }

    // -------------------------------------------------------------------------
    // Task 1.1: UatCompletionPolicy
    // -------------------------------------------------------------------------

    #[test]
    fn uat_completion_policy_mode_all() {
        let policy = UatCompletionPolicy {
            mode: UatCompletionMode::All,
            threshold: None,
        };
        let yaml = serde_saphyr::to_string(&policy).unwrap();
        assert!(yaml.contains("all"));
        let round: UatCompletionPolicy = serde_saphyr::from_str(&yaml).unwrap();
        assert_eq!(round.mode, UatCompletionMode::All);
    }

    #[test]
    fn uat_completion_policy_mode_majority_with_threshold() {
        let policy = UatCompletionPolicy {
            mode: UatCompletionMode::Majority,
            threshold: Some(5),
        };
        let json = serde_json::to_string(&policy).unwrap();
        assert!(json.contains("majority"));
        let round: UatCompletionPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(round.mode, UatCompletionMode::Majority);
        assert_eq!(round.threshold, Some(5));
    }

    #[test]
    fn uat_completion_policy_threshold_bounds() {
        // threshold 0 is invalid (must be 1..n)
        let policy = UatCompletionPolicy {
            mode: UatCompletionMode::Majority,
            threshold: Some(0),
        };
        let errors = UatCompletionPolicy::validate(&policy);
        assert!(!errors.is_empty());
        assert!(errors[0].contains("threshold"));

        // threshold 1 is valid
        let policy = UatCompletionPolicy {
            mode: UatCompletionMode::Majority,
            threshold: Some(1),
        };
        assert!(UatCompletionPolicy::validate(&policy).is_empty());
    }

    // -------------------------------------------------------------------------
    // Task 1.1: UatCheckpoint
    // -------------------------------------------------------------------------

    #[test]
    fn uat_checkpoint_serde_with_items() {
        let checkpoint = UatCheckpoint {
            id: "cp-1".into(),
            label: Some("After login flow".into()),
            evidence_summary: UatEvidenceSummary {
                machine_passed: 5,
                machine_total: 5,
                fara_assessment: None,
                fara_confidence: None,
                anomalies: vec![],
            },
            items: vec!["item-1".into(), "item-2".into(), "item-3".into()],
        };
        let yaml = serde_saphyr::to_string(&checkpoint).unwrap();
        assert!(yaml.contains("cp-1"));
        assert!(yaml.contains("item-1"));
        let round: UatCheckpoint = serde_saphyr::from_str(&yaml).unwrap();
        assert_eq!(round.id, "cp-1");
        assert_eq!(round.items.len(), 3);
    }

    #[test]
    fn uat_checkpoint_default() {
        let cp = UatCheckpoint::default();
        assert!(cp.id.is_empty());
        assert!(cp.items.is_empty());
    }

    // -------------------------------------------------------------------------
    // Task 1.1: UatDiagnosticsReport
    // -------------------------------------------------------------------------

    #[test]
    fn uat_diagnostics_report_serde() {
        let report = UatDiagnosticsReport {
            scenario_id: "S-1".into(),
            checkpoint_id: Some("cp-1".into()),
            collected_evidence: vec![
                UatEvidenceKindItem {
                    kind: UatEvidenceKind::Screenshot,
                    r#ref: None,
                    match_mode: None,
                    expected_value: None,
                    min_bytes: None,
                },
                UatEvidenceKindItem {
                    kind: UatEvidenceKind::Console,
                    r#ref: None,
                    match_mode: None,
                    expected_value: None,
                    min_bytes: None,
                },
            ],
            cause: Some("Element not found".into()),
            category: Some("ui_interaction".into()),
            suggested_defect: Some("Login button selector changed".into()),
            observed: Some("Button#login not present".into()),
            expected: Some("Button#login present and visible".into()),
        };
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("S-1"));
        assert!(json.contains("screenshot"));
        let round: UatDiagnosticsReport = serde_json::from_str(&json).unwrap();
        assert_eq!(round.scenario_id, "S-1");
        assert_eq!(round.collected_evidence.len(), 2);
        assert_eq!(round.cause.as_deref(), Some("Element not found"));
    }

    #[test]
    fn uat_diagnostics_report_minimal() {
        let report = UatDiagnosticsReport {
            scenario_id: "S-2".into(),
            checkpoint_id: None,
            collected_evidence: vec![],
            cause: None,
            category: None,
            suggested_defect: None,
            observed: None,
            expected: None,
        };
        let yaml = serde_saphyr::to_string(&report).unwrap();
        let round: UatDiagnosticsReport = serde_saphyr::from_str(&yaml).unwrap();
        assert_eq!(round.scenario_id, "S-2");
        assert!(round.cause.is_none());
    }

    // -------------------------------------------------------------------------
    // Task 1.1: UatAcceptanceRecord
    // -------------------------------------------------------------------------

    #[test]
    fn uat_acceptance_record_serde_with_sha256() {
        let record = UatAcceptanceRecord {
            decision: UatAcceptanceDecision::Accepted,
            actor: "user:421".into(),
            timestamp: "2026-08-11T10:00:00Z".into(),
            plan_version_sha256: "sha256:abc123def456".into(),
            evidence_snapshot_sha256: "sha256:789xyz123abc".into(),
            outstanding_findings: vec!["finding-1".into()],
            justification: "All critical paths verified".into(),
        };
        let yaml = serde_saphyr::to_string(&record).unwrap();
        assert!(yaml.contains("sha256:abc123def456"));
        assert!(yaml.contains("accepted"));
        let round: UatAcceptanceRecord = serde_saphyr::from_str(&yaml).unwrap();
        assert_eq!(round.plan_version_sha256, "sha256:abc123def456");
        assert_eq!(round.decision, UatAcceptanceDecision::Accepted);
        assert_eq!(round.outstanding_findings.len(), 1);
    }

    #[test]
    fn uat_acceptance_record_decision_variants() {
        let decisions = vec![
            UatAcceptanceDecision::Accepted,
            UatAcceptanceDecision::AcceptedConditional,
            UatAcceptanceDecision::Rejected,
        ];
        for decision in decisions {
            let json = serde_json::to_string(&decision).unwrap();
            let round: UatAcceptanceDecision = serde_json::from_str(&json).unwrap();
            assert_eq!(round, decision);
        }
    }

    #[test]
    fn uat_acceptance_record_sha256_format() {
        // sha256 prefix is required
        let record = UatAcceptanceRecord {
            decision: UatAcceptanceDecision::Accepted,
            actor: "user:1".into(),
            timestamp: "2026-08-11T00:00:00Z".into(),
            plan_version_sha256: "abc123".into(), // missing sha256: prefix
            evidence_snapshot_sha256: "sha256:xyz".into(),
            outstanding_findings: vec![],
            justification: "test".into(),
        };
        let errors = UatAcceptanceRecord::validate(&record);
        assert!(!errors.is_empty());
        assert!(errors[0].contains("sha256"));
    }

    // -------------------------------------------------------------------------
    // Task 1.1: UatStalenessReport
    // -------------------------------------------------------------------------

    #[test]
    fn uat_staleness_report_serde() {
        let report = UatStalenessReport {
            release: "v1.9.0".into(),
            assessed_at: "2026-08-11T12:00:00Z".into(),
            affected_scenarios: vec![UatStalenessScenario {
                scenario_id: "S-1".into(),
                checkpoint_id: None,
                selector: Some("button#submit".into()),
                text_content: Some("Submit".into()),
                previous_fingerprint: "fp-v1".into(),
                current_fingerprint: "fp-v2".into(),
                change_kind: UatStalenessChangeKind::SelectorChanged,
            }],
            fingerprint_diffs: vec![UatStalenessDiff {
                scenario_id: "S-2".into(),
                checkpoint_id: Some("cp-1".into()),
                field: "selector".into(),
                previous: "div.old".into(),
                current: "div.new".into(),
            }],
        };
        let yaml = serde_saphyr::to_string(&report).unwrap();
        assert!(yaml.contains("v1.9.0"));
        assert!(yaml.contains("button#submit"));
        let round: UatStalenessReport = serde_saphyr::from_str(&yaml).unwrap();
        assert_eq!(round.release, "v1.9.0");
        assert_eq!(round.affected_scenarios.len(), 1);
        assert_eq!(round.fingerprint_diffs.len(), 1);
    }

    #[test]
    fn uat_staleness_report_empty_is_valid() {
        let report = UatStalenessReport {
            release: "v2.0.0".into(),
            assessed_at: "2026-08-11T00:00:00Z".into(),
            affected_scenarios: vec![],
            fingerprint_diffs: vec![],
        };
        assert!(report.affected_scenarios.is_empty());
        assert!(report.fingerprint_diffs.is_empty());
        let yaml = serde_saphyr::to_string(&report).unwrap();
        let round: UatStalenessReport = serde_saphyr::from_str(&yaml).unwrap();
        assert_eq!(round.release, "v2.0.0");
    }

    #[test]
    fn uat_staleness_change_kind_variants() {
        let kinds = vec![
            UatStalenessChangeKind::SelectorChanged,
            UatStalenessChangeKind::TextContentChanged,
            UatStalenessChangeKind::AttributeChanged,
            UatStalenessChangeKind::ElementRemoved,
            UatStalenessChangeKind::ElementAdded,
        ];
        for kind in kinds {
            let json = serde_json::to_string(&kind).unwrap();
            let round: UatStalenessChangeKind = serde_json::from_str(&json).unwrap();
            assert_eq!(round, kind);
        }
    }

    // -------------------------------------------------------------------------
    // Task 1.3: Plan schema v4 — backward compat
    // -------------------------------------------------------------------------

    #[test]
    fn plan_v3_still_parses_identically() {
        // A v3 plan must parse without needing any v4 fields.
        let v3_plan: UatPlan = serde_saphyr::from_str(
            r#"
schema_version: 3
release: { candidate: v1.8.0 }
generated_by: test
generated_at: "2026-08-11T00:00:00Z"
features:
  - id: F-1
    name: Feature
    scenarios:
      - id: S-1
        title: Scenario
"#,
        )
        .unwrap();
        assert_eq!(v3_plan.schema_version, 3);
        // v4 fields are absent — must be None
        assert!(v3_plan.runner_mode.is_none());
    }

    #[test]
    fn plan_v4_with_all_new_fields() {
        let v4_plan: UatPlan = serde_saphyr::from_str(
            r#"
schema_version: 4
release: { candidate: v1.9.0 }
generated_by: test
generated_at: "2026-08-11T00:00:00Z"
runner_mode: runner
features:
  - id: F-1
    name: Feature
    scenarios:
      - id: S-1
        title: Scenario
        form_checkpoint:
          id: cp-1
          label: Checkpoint 1
          evidence_summary:
            machine_passed: 3
            machine_total: 3
            anomalies: []
          items: [item-1, item-2]
        completion:
          mode: all
        staleness: stale
"#,
        )
        .unwrap();
        assert_eq!(v4_plan.schema_version, 4);
        assert_eq!(v4_plan.runner_mode, Some(UatRunnerMode::Runner));
        let s = &v4_plan.features[0].scenarios[0];
        assert!(s.form_checkpoint.is_some());
        assert!(s.completion.is_some());
        assert!(s.staleness.is_some());
    }

    #[test]
    fn latest_plan_schema_version_is_4() {
        assert_eq!(LATEST_PLAN_SCHEMA_VERSION, 4);
    }

    #[test]
    fn scenario_staleness_variants() {
        let stale_scenario: UatScenario = serde_saphyr::from_str(
            r#"
id: S-1
title: Stale Scenario
staleness: stale
"#,
        )
        .unwrap();
        assert_eq!(stale_scenario.staleness, Some(UatScenarioStaleness::Stale));

        let fresh_scenario: UatScenario = serde_saphyr::from_str(
            r#"
id: S-2
title: Fresh Scenario
staleness: fresh
"#,
        )
        .unwrap();
        assert_eq!(fresh_scenario.staleness, Some(UatScenarioStaleness::Fresh));
    }

    // -------------------------------------------------------------------------
    // Task 1.2: Validator — branching referencial + completion
    // -------------------------------------------------------------------------

    #[test]
    fn validate_form_dsl_goto_target_must_exist() {
        let spec: UatFormSpec = serde_saphyr::from_str(
            r#"
dsl_version: 1
items:
  - kind: check
    check:
      kind: yes_no
      prompt: OK?
  - kind: flow
    flow: goto
    target: nonexistent-id
"#,
        )
        .unwrap();
        let errors = validate_form_dsl(&spec);
        assert!(!errors.is_empty());
        assert!(
            errors
                .iter()
                .any(|e| e.contains("nonexistent-id") && e.contains("goto"))
        );
    }

    #[test]
    fn validate_form_dsl_detects_cycle() {
        // A -> B -> C -> A cycle (using explicit item ids)
        let spec: UatFormSpec = serde_saphyr::from_str(
            r#"
dsl_version: 1
items:
  - id: item-a
    kind: flow
    flow: goto
    target: item-c
  - id: item-b
    kind: flow
    flow: goto
    target: item-a
  - id: item-c
    kind: flow
    flow: goto
    target: item-b
"#,
        )
        .unwrap();
        let errors = validate_form_dsl(&spec);
        assert!(!errors.is_empty());
        assert!(
            errors
                .iter()
                .any(|e| e.contains("cycle") || e.contains("cyclic")),
            "expected cycle error, got: {errors:?}"
        );
    }

    #[test]
    fn validate_form_dsl_completion_policy_mode_all_valid() {
        let spec: UatFormSpec = serde_saphyr::from_str(
            r#"
dsl_version: 1
items:
  - kind: flow
    flow: stop
completion:
  mode: all
"#,
        )
        .unwrap();
        let errors = validate_form_dsl(&spec);
        assert!(errors.is_empty(), "all mode should be valid: {errors:?}");
    }

    #[test]
    fn validate_form_dsl_completion_policy_mode_majority_valid() {
        let spec: UatFormSpec = serde_saphyr::from_str(
            r#"
dsl_version: 1
items:
  - kind: flow
    flow: stop
completion:
  mode: majority
  threshold: 5
"#,
        )
        .unwrap();
        let errors = validate_form_dsl(&spec);
        assert!(
            errors.is_empty(),
            "majority with threshold should be valid: {errors:?}"
        );
    }

    #[test]
    fn validate_form_dsl_completion_policy_threshold_zero_invalid() {
        let spec: UatFormSpec = serde_saphyr::from_str(
            r#"
dsl_version: 1
items:
  - kind: flow
    flow: stop
completion:
  mode: majority
  threshold: 0
"#,
        )
        .unwrap();
        let errors = validate_form_dsl(&spec);
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.contains("threshold")));
    }

    #[test]
    fn validate_form_dsl_checkpoint_must_reference_existing_items() {
        let spec: UatFormSpec = serde_saphyr::from_str(
            r#"
dsl_version: 1
items:
  - kind: check
    check:
      kind: yes_no
      prompt: OK?
  - kind: checkpoint
    checkpoint:
      id: cp-1
      items: [nonexistent-item]
"#,
        )
        .unwrap();
        let errors = validate_form_dsl(&spec);
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.contains("nonexistent-item")));
    }
}
