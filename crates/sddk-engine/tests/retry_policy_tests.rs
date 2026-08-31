//! Integration tests for `RetryPolicy`.
//!
//! Tests cover: success-on-first-attempt, success-after-N-retries,
//! exhausted-after-max-attempts scenarios for each of the 4 strategies.

use sddk_domain::{TaskError, TaskExecutor, TaskOutput};
use sddk_engine::retry::{RetryPolicy, RngCore};
use serde_json::Value;
use std::collections::BTreeMap;

// ── Deterministic RNG for jitter tests ───────────────────────────────────────

#[derive(Debug, Clone)]
struct StepRng {
    value: u64,
    step: u64,
}

impl StepRng {
    fn new(seed: u64, step: u64) -> Self {
        Self { value: seed, step }
    }
}

impl RngCore for StepRng {
    fn next_u64(&mut self) -> u64 {
        let current = self.value;
        self.value = self.value.saturating_add(self.step);
        current
    }
}

// ── Fixed strategy ───────────────────────────────────────────────────────────

fn always_succeed() -> impl TaskExecutor {
    struct AlwaysSucceed;
    impl TaskExecutor for AlwaysSucceed {
        fn execute(
            &self,
            _: &str,
            _: &BTreeMap<String, Value>,
        ) -> std::result::Result<TaskOutput, TaskError> {
            Ok(TaskOutput {
                outputs: Default::default(),
            })
        }
    }
    AlwaysSucceed
}

#[test]
fn fixed_success_on_first_attempt() {
    let policy = RetryPolicy::Fixed {
        delay_ms: 10,
        max_attempts: 3,
    };
    let _executor = always_succeed();
    let mut rng = StepRng::new(0, 1);
    // First attempt should succeed, no delay.
    assert_eq!(policy.delay_ms(1, &mut rng), 0);
}

#[test]
fn fixed_success_after_2_retries() {
    let policy = RetryPolicy::Fixed {
        delay_ms: 10,
        max_attempts: 3,
    };
    let mut rng = StepRng::new(0, 1);
    // Attempts 1: 0ms, 2: 10ms, 3: 10ms
    assert_eq!(policy.delay_ms(1, &mut rng), 0);
    assert_eq!(policy.delay_ms(2, &mut rng), 10);
    assert_eq!(policy.delay_ms(3, &mut rng), 10);
}

#[test]
fn fixed_exhausted_after_max_attempts() {
    let policy = RetryPolicy::Fixed {
        delay_ms: 10,
        max_attempts: 3,
    };
    assert_eq!(policy.max_attempts(), 3);
    let mut rng = StepRng::new(0, 1);
    // After 3 failed attempts, we should have waited 10ms twice (attempts 2 and 3).
    let total_delay: u64 = (2..=3).map(|a| policy.delay_ms(a, &mut rng)).sum();
    assert_eq!(total_delay, 20);
}

// ── Linear strategy ─────────────────────────────────────────────────────────

#[test]
fn linear_success_on_first_attempt() {
    let policy = RetryPolicy::Linear {
        base_ms: 10,
        max_backoff_ms: 100,
        max_attempts: 5,
    };
    let mut rng = StepRng::new(0, 1);
    assert_eq!(policy.delay_ms(1, &mut rng), 0); // first try: no delay
}

#[test]
fn linear_success_after_3_retries() {
    let policy = RetryPolicy::Linear {
        base_ms: 10,
        max_backoff_ms: 100,
        max_attempts: 5,
    };
    let mut rng = StepRng::new(0, 1);
    // attempt 1: 0, 2: 10, 3: 20, 4: 30
    assert_eq!(policy.delay_ms(1, &mut rng), 0);
    assert_eq!(policy.delay_ms(2, &mut rng), 10);
    assert_eq!(policy.delay_ms(3, &mut rng), 20);
    assert_eq!(policy.delay_ms(4, &mut rng), 30);
}

#[test]
fn linear_exhausted_max_attempts() {
    let policy = RetryPolicy::Linear {
        base_ms: 10,
        max_backoff_ms: 100,
        max_attempts: 3,
    };
    assert_eq!(policy.max_attempts(), 3);
    let mut rng = StepRng::new(0, 1);
    // Total delay over all retries (excluding first attempt): 10 + 20 = 30ms
    let total: u64 = (2..=3).map(|a| policy.delay_ms(a, &mut rng)).sum();
    assert_eq!(total, 30);
}

// ── Exponential strategy ─────────────────────────────────────────────────────

#[test]
fn exponential_success_on_first_attempt() {
    let policy = RetryPolicy::Exponential {
        base_ms: 100,
        max_backoff_ms: 1000,
        max_attempts: 4,
    };
    let mut rng = StepRng::new(0, 1);
    assert_eq!(policy.delay_ms(1, &mut rng), 0);
}

#[test]
fn exponential_success_after_2_retries() {
    let policy = RetryPolicy::Exponential {
        base_ms: 100,
        max_backoff_ms: 1000,
        max_attempts: 4,
    };
    let mut rng = StepRng::new(0, 1);
    // attempt 1: 0ms, 2: 100ms, 3: 200ms
    assert_eq!(policy.delay_ms(1, &mut rng), 0);
    assert_eq!(policy.delay_ms(2, &mut rng), 100);
    assert_eq!(policy.delay_ms(3, &mut rng), 200);
}

#[test]
fn exponential_exhausted_with_backoff() {
    let policy = RetryPolicy::Exponential {
        base_ms: 100,
        max_backoff_ms: 150, // cap low to test capping
        max_attempts: 5,
    };
    assert_eq!(policy.max_attempts(), 5);
    let mut rng = StepRng::new(0, 1);
    // attempt 2: 100ms, 3: 200ms→cap 150, 4: 400ms→cap 150, 5: 800ms→cap 150
    assert_eq!(policy.delay_ms(2, &mut rng), 100);
    assert_eq!(policy.delay_ms(3, &mut rng), 150); // capped
    assert_eq!(policy.delay_ms(4, &mut rng), 150); // capped
    assert_eq!(policy.delay_ms(5, &mut rng), 150); // capped
}

// ── ExponentialWithJitter strategy ─────────────────────────────────────────

#[test]
fn jitter_success_on_first_attempt() {
    let policy = RetryPolicy::ExponentialWithJitter {
        base_ms: 100,
        max_backoff_ms: 200,
        jitter_ms: 50,
        max_attempts: 4,
    };
    let mut rng = StepRng::new(0, 1);
    assert_eq!(policy.delay_ms(1, &mut rng), 0);
}

#[test]
fn jitter_delay_bounded() {
    let policy = RetryPolicy::ExponentialWithJitter {
        base_ms: 100,
        max_backoff_ms: 200,
        jitter_ms: 50,
        max_attempts: 3,
    };
    // With StepRng(seed=0): jitter=0 → delay=100
    let mut rng1 = StepRng::new(0, 1);
    let d1 = policy.delay_ms(2, &mut rng1);
    assert!(
        (100..=150).contains(&d1),
        "delay {d1} should be in [100, 150]"
    );

    // With StepRng(seed=100): jitter=100 → capped at 200
    let mut rng2 = StepRng::new(100, 1);
    let d2 = policy.delay_ms(2, &mut rng2);
    assert!(
        (100..=200).contains(&d2),
        "delay {d2} should be in [100, 200]"
    );
}

#[test]
fn jitter_exhausted_deterministic() {
    let policy = RetryPolicy::ExponentialWithJitter {
        base_ms: 10,
        max_backoff_ms: 20,
        jitter_ms: 5,
        max_attempts: 3,
    };
    assert_eq!(policy.max_attempts(), 3);
    let mut rng = StepRng::new(7, 1); // fixed seed for determinism
    // Attempt 2: exp=10, jitter_range=6, jitter=1 (7%6=1) → 10+1=11
    assert_eq!(policy.delay_ms(2, &mut rng), 11);
}
