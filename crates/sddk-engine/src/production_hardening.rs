//! Production Hardening Budgets + Auditor
//!
//! Plain-data budgets (performance / retention / migration /
//! reliability) and a validator that asserts a runtime report
//! satisfies them.
//!
//! Pattern: P-a (struct + trait + plain-data evaluator).

use crate::generic_pack_contracts::RuntimeVersion;

// ── Audit guard constants ────────────────────────────────────────────────

/// Closed-set size of [`HardeningError`]. Bump when a variant is added.
pub const HARDENING_ERROR_VARIANT_COUNT: usize = 10;

/// Number of fields in [`PerfBudget`].
pub const PERF_BUDGET_FIELD_COUNT: usize = 3;

/// Number of fields in [`ReliabilityBudget`].
pub const RELIABILITY_BUDGET_FIELD_COUNT: usize = 3;

// ── Budgets ────────────────────────────────────────────────────────────

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerfBudget {
    pub max_p99_latency_ms: u32,
    pub max_throughput_rps: u32,
    pub max_memory_mb: u32,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionPolicy {
    pub keep_last_n: u32,
    pub max_age_days: u32,
    pub redaction_required: bool,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationPlan {
    pub from_version: RuntimeVersion,
    pub to_version: RuntimeVersion,
    /// Sorted, unique.
    pub steps: Vec<String>,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReliabilityBudget {
    pub max_error_rate_bps: u32,
    pub min_uptime_bps: u32,
    pub max_recovery_seconds: u32,
}

// ── Report ──────────────────────────────────────────────────────────────

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeReport {
    pub runtime_version: RuntimeVersion,
    pub measured_p99_latency_ms: u32,
    pub measured_throughput_rps: u32,
    pub measured_memory_mb: u32,
    pub measured_error_rate_bps: u32,
    pub measured_uptime_bps: u32,
    pub measured_recovery_seconds: u32,
    pub migration_in_flight: bool,
    pub collected_at: String,
}

// ── Error + auditor ────────────────────────────────────────────────────

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HardeningError {
    PerfLatencyExceeded,
    PerfThroughputUnder,
    PerfMemoryExceeded,
    ErrorRateExceeded,
    UptimeBelow,
    RecoveryTooSlow,
    MigrationPending,
    RetentionKeepLastNZero,
    EmptyMigrationSteps,
    DuplicateMigrationStep,
}

pub trait HardeningAuditor {
    fn audit(
        &self,
        report: &RuntimeReport,
        perf_budget: &PerfBudget,
        reliability_budget: &ReliabilityBudget,
        retention: &RetentionPolicy,
        migration: Option<&MigrationPlan>,
    ) -> Result<(), HardeningError>;
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DefaultHardeningAuditor;

impl HardeningAuditor for DefaultHardeningAuditor {
    fn audit(
        &self,
        report: &RuntimeReport,
        perf_budget: &PerfBudget,
        reliability_budget: &ReliabilityBudget,
        retention: &RetentionPolicy,
        migration: Option<&MigrationPlan>,
    ) -> Result<(), HardeningError> {
        // Retention.
        if retention.keep_last_n == 0 {
            return Err(HardeningError::RetentionKeepLastNZero);
        }
        // Migration plan validity.
        if let Some(m) = migration {
            if m.steps.is_empty() {
                return Err(HardeningError::EmptyMigrationSteps);
            }
            let mut sorted = m.steps.clone();
            sorted.sort();
            for w in sorted.windows(2) {
                if w[0] == w[1] {
                    return Err(HardeningError::DuplicateMigrationStep);
                }
            }
        }
        if report.migration_in_flight {
            return Err(HardeningError::MigrationPending);
        }
        // Perf.
        if report.measured_p99_latency_ms > perf_budget.max_p99_latency_ms {
            return Err(HardeningError::PerfLatencyExceeded);
        }
        if report.measured_throughput_rps < perf_budget.max_throughput_rps {
            return Err(HardeningError::PerfThroughputUnder);
        }
        if report.measured_memory_mb > perf_budget.max_memory_mb {
            return Err(HardeningError::PerfMemoryExceeded);
        }
        // Reliability.
        if report.measured_error_rate_bps > reliability_budget.max_error_rate_bps {
            return Err(HardeningError::ErrorRateExceeded);
        }
        if report.measured_uptime_bps < reliability_budget.min_uptime_bps {
            return Err(HardeningError::UptimeBelow);
        }
        if report.measured_recovery_seconds > reliability_budget.max_recovery_seconds {
            return Err(HardeningError::RecoveryTooSlow);
        }
        Ok(())
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn auditor() -> DefaultHardeningAuditor {
        DefaultHardeningAuditor
    }

    fn clean_report() -> RuntimeReport {
        RuntimeReport {
            runtime_version: RuntimeVersion::new(1, 0, 0),
            measured_p99_latency_ms: 50,
            measured_throughput_rps: 200,
            measured_memory_mb: 256,
            measured_error_rate_bps: 10,
            measured_uptime_bps: 9_999,
            measured_recovery_seconds: 60,
            migration_in_flight: false,
            collected_at: "t0".to_string(),
        }
    }

    fn perf() -> PerfBudget {
        PerfBudget {
            max_p99_latency_ms: 100,
            max_throughput_rps: 100,
            max_memory_mb: 512,
        }
    }
    fn reliability() -> ReliabilityBudget {
        ReliabilityBudget {
            max_error_rate_bps: 100,
            min_uptime_bps: 9_900,
            max_recovery_seconds: 120,
        }
    }
    fn retention() -> RetentionPolicy {
        RetentionPolicy {
            keep_last_n: 10,
            max_age_days: 365,
            redaction_required: true,
        }
    }

    #[test]
    fn s1_clean_report_passes() {
        let r = clean_report();
        assert_eq!(
            auditor().audit(&r, &perf(), &reliability(), &retention(), None),
            Ok(())
        );
    }

    #[test]
    fn s2_latency_exceeded() {
        let mut r = clean_report();
        r.measured_p99_latency_ms = 200;
        let v = auditor().audit(&r, &perf(), &reliability(), &retention(), None);
        assert_eq!(v, Err(HardeningError::PerfLatencyExceeded));
    }

    #[test]
    fn s3_throughput_under() {
        let mut r = clean_report();
        r.measured_throughput_rps = 50;
        assert_eq!(
            auditor().audit(&r, &perf(), &reliability(), &retention(), None),
            Err(HardeningError::PerfThroughputUnder)
        );
    }

    #[test]
    fn s4_error_rate_exceeded() {
        let mut r = clean_report();
        r.measured_error_rate_bps = 1_000;
        assert_eq!(
            auditor().audit(&r, &perf(), &reliability(), &retention(), None),
            Err(HardeningError::ErrorRateExceeded)
        );
    }

    #[test]
    fn s5_uptime_below() {
        let mut r = clean_report();
        r.measured_uptime_bps = 8_000;
        assert_eq!(
            auditor().audit(&r, &perf(), &reliability(), &retention(), None),
            Err(HardeningError::UptimeBelow)
        );
    }

    #[test]
    fn s6_migration_in_flight() {
        let mut r = clean_report();
        r.migration_in_flight = true;
        assert_eq!(
            auditor().audit(&r, &perf(), &reliability(), &retention(), None),
            Err(HardeningError::MigrationPending)
        );
    }

    #[test]
    fn s7_empty_migration_steps_rejected() {
        let m = MigrationPlan {
            from_version: RuntimeVersion::new(1, 0, 0),
            to_version: RuntimeVersion::new(1, 1, 0),
            steps: vec![],
        };
        assert_eq!(
            auditor().audit(
                &clean_report(),
                &perf(),
                &reliability(),
                &retention(),
                Some(&m)
            ),
            Err(HardeningError::EmptyMigrationSteps)
        );
    }

    #[test]
    fn s8_duplicate_migration_step_rejected() {
        let m = MigrationPlan {
            from_version: RuntimeVersion::new(1, 0, 0),
            to_version: RuntimeVersion::new(1, 1, 0),
            steps: vec!["a".to_string(), "a".to_string()],
        };
        assert_eq!(
            auditor().audit(
                &clean_report(),
                &perf(),
                &reliability(),
                &retention(),
                Some(&m)
            ),
            Err(HardeningError::DuplicateMigrationStep)
        );
    }

    #[test]
    fn s9_closed_set_audit() {
        assert_eq!(HARDENING_ERROR_VARIANT_COUNT, 10);
        assert_eq!(PERF_BUDGET_FIELD_COUNT, 3);
        assert_eq!(RELIABILITY_BUDGET_FIELD_COUNT, 3);
    }

    #[test]
    fn s10_deterministic() {
        let r = clean_report();
        assert_eq!(
            auditor().audit(&r, &perf(), &reliability(), &retention(), None),
            auditor().audit(&r, &perf(), &reliability(), &retention(), None)
        );
    }

    #[test]
    fn s11_retention_keep_last_n_zero() {
        let mut ret = retention();
        ret.keep_last_n = 0;
        assert_eq!(
            auditor().audit(&clean_report(), &perf(), &reliability(), &ret, None),
            Err(HardeningError::RetentionKeepLastNZero)
        );
    }

    #[test]
    fn s12_recovery_too_slow() {
        let mut r = clean_report();
        r.measured_recovery_seconds = 600;
        assert_eq!(
            auditor().audit(&r, &perf(), &reliability(), &retention(), None),
            Err(HardeningError::RecoveryTooSlow)
        );
    }
}
