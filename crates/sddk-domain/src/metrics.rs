//! Telemetry metrics types for SDDK cycles (Levels A-E and loop costs).

use std::collections::HashMap;

/// Per-cycle metrics record (metrics-schema.md Levels A-E + L1-L6 costs).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MetricsRecord {
    /// Stable cycle identifier, e.g. `p-xxxx/telemetry-analytics-loop`.
    pub cycle_id: String,
    /// Workflow path taken: `b-direct | a-min | a-lite | a-full`.
    pub path: String,
    /// Context quality at triage: C0..C3.
    pub context_quality: String,
    /// Phase durations in seconds, keyed by phase name.
    #[serde(default)]
    pub phase_durations_sec: HashMap<String, u64>,
    /// Coherence scores for transitions that ran a check.
    #[serde(default)]
    pub coherence_scores: Vec<u8>,
    /// Number of apply<->verify correction cycles.
    #[serde(default)]
    pub correction_cycles: u8,
    /// Estimated tokens used.
    #[serde(default)]
    pub tokens_used: u64,
    /// Estimated cost in USD.
    #[serde(default)]
    pub cost_estimate_usd: f64,
    /// Whether the first verification attempt passed.
    pub first_pass_success: bool,
    /// Verification verdict: PASS | PW | FAIL.
    pub verify_verdict: String,
    /// Whether the change was merged to main.
    pub merged_to_main: bool,
    /// Semantic version tag when released.
    pub tag_version: Option<String>,
    /// Lead time in hours (goal received -> main merge).
    pub lead_time_hours: Option<f64>,
    /// Teleological coherence percentage (Level E).
    pub teleological_coherence_pct: Option<f64>,
    /// Loop costs keyed by loop id (L1..L6).
    #[serde(default)]
    pub costs: HashMap<String, f64>,
    /// RFC 3339 timestamp when the record was written.
    pub recorded_at: String,
}

/// Rolling aggregate over a time window (7d/30d).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MetricsAggregate {
    /// Window size in days.
    pub window_days: u16,
    /// First-pass success rate (0..=1).
    pub first_pass_success_rate: f64,
    /// Median lead time in hours.
    pub median_lead_time_hours: Option<f64>,
    /// Median cost in USD.
    pub median_cost_usd: Option<f64>,
    /// Phase with the highest median duration.
    pub top_bottleneck_phase: Option<String>,
    /// Path distribution: `{ "a-lite": 3, ... }`.
    pub path_distribution: HashMap<String, u32>,
    /// Verdict distribution: `{ "PASS": 5, "PW": 1, ... }`.
    pub verdict_distribution: HashMap<String, u32>,
    /// Number of records in the window.
    pub sample_size: u32,
}

/// F3 self-tuning recommendation block (advisory, never silent override).
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct F3Tuning {
    /// Phases recommended to skip in the next cycle.
    #[serde(default)]
    pub recommended_skip: Vec<String>,
    /// Phases recommended to deepen in the next cycle.
    #[serde(default)]
    pub recommended_deepen: Vec<String>,
    /// Lenses recommended for the next cycle.
    #[serde(default)]
    pub recommended_lens: Vec<String>,
    /// Path bias for a context quality, e.g. `A-min` for `C1`.
    pub path_bias: Option<String>,
    /// Circuit breaker failure threshold recommendation.
    pub circuit_threshold: Option<u32>,
    /// Per-task max attempts recommendation.
    pub per_task_max_attempts: Option<u32>,
}

impl MetricsAggregate {
    /// Create an empty aggregate for a window.
    pub fn empty(window_days: u16) -> Self {
        Self {
            window_days,
            first_pass_success_rate: 0.0,
            median_lead_time_hours: None,
            median_cost_usd: None,
            top_bottleneck_phase: None,
            path_distribution: HashMap::new(),
            verdict_distribution: HashMap::new(),
            sample_size: 0,
        }
    }
}
