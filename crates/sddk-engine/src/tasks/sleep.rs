//! `sleep` capability task.
//!
//! `SleepTask` suspends execution for a specified duration using `std::thread::sleep`
//! (runs inside `spawn_blocking` when called from a tokio runtime context).

use serde_json::Value;
use std::collections::BTreeMap;

/// Task operator for the `sleep` capability.
///
/// # Capability
/// `sleep` — blocks the current thread for the specified duration.
///
/// # Inputs
/// - `ms` (number, required): Duration to sleep in milliseconds.
///
/// # Outputs
/// - `elapsed_ms` (number): Actual elapsed time in milliseconds.
#[derive(Debug, Clone)]
pub struct SleepTask {
    /// Duration to sleep in milliseconds.
    pub ms: u64,
}

impl SleepTask {
    /// Returns the capability identifier.
    pub const fn capability() -> &'static str {
        "sleep"
    }

    /// Builds the capability inputs map.
    pub fn inputs(&self) -> BTreeMap<String, Value> {
        let mut m = BTreeMap::new();
        m.insert(
            "ms".into(),
            Value::Number(serde_json::Number::from(self.ms)),
        );
        m
    }

    /// Returns `(capability, inputs)` tuple for use with `TaskExecutor::execute`.
    pub fn capability_and_inputs(&self) -> (&'static str, BTreeMap<String, Value>) {
        (Self::capability(), self.inputs())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RealTaskExecutor;
    use sddk_domain::TaskExecutor;
    use std::time::Instant;

    #[test]
    fn sleep_task_capability_is_sleep() {
        let _task = SleepTask { ms: 10 };
        assert_eq!(SleepTask::capability(), "sleep");
    }

    #[test]
    fn sleep_task_inputs_contains_ms() {
        let task = SleepTask { ms: 20 };
        let inputs = task.inputs();
        assert_eq!(inputs.get("ms").and_then(|v| v.as_u64()), Some(20));
    }

    #[test]
    fn sleep_task_actually_sleeps() {
        let executor = RealTaskExecutor::new();
        let mut inputs = BTreeMap::new();
        inputs.insert("ms".into(), Value::Number(serde_json::Number::from(30u64)));

        let start = Instant::now();
        let result = <RealTaskExecutor as TaskExecutor>::execute(&executor, "sleep", &inputs);
        let elapsed = start.elapsed().as_millis() as u64;

        assert!(result.is_ok(), "sleep should succeed: {:?}", result);
        assert!(
            elapsed >= 25,
            "sleep(30ms) should have elapsed at least 25ms, got {elapsed}ms"
        );
    }
}
