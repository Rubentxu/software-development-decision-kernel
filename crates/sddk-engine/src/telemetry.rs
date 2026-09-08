//! Provider telemetry — strict subset of SPEC-032 §Per Attempt metrics.

use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::circuit_breaker::ProviderIdentity;
use crate::run_view::ActionKind;

/// Per-attempt usage record. Unknown fields remain `None`; the kernel MUST
/// NOT synthesize unavailable metrics (SPEC-032).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageRecord {
    pub identity: ProviderIdentity,
    pub action_kind: ActionKind,
    pub attempts: u32,
    pub failovers: u32,
    pub wall_ms: u64,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cost: Option<f64>,
    pub currency: Option<String>,
    pub result: ProviderResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderResult {
    Ok,
    RetryableFailure,
    TerminalFailure,
}

/// Sink for usage records. Implementations MUST be `Send + Sync`.
pub trait TelemetrySink: Send + Sync {
    fn record(&self, record: UsageRecord);
}

/// No-op sink. Default for production hosts that do not aggregate yet.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullTelemetrySink;

impl TelemetrySink for NullTelemetrySink {
    fn record(&self, _record: UsageRecord) {}
}

/// In-memory sink for tests and the COCKPIT-002 dashboards.
#[derive(Debug, Default)]
pub struct InMemoryTelemetrySink {
    inner: Mutex<Vec<UsageRecord>>,
}

impl InMemoryTelemetrySink {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn records(&self) -> Vec<UsageRecord> {
        self.inner.lock().expect("poisoned").clone()
    }

    pub fn len(&self) -> usize {
        self.inner.lock().expect("poisoned").len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl TelemetrySink for InMemoryTelemetrySink {
    fn record(&self, record: UsageRecord) {
        self.inner.lock().expect("poisoned").push(record);
    }
}
