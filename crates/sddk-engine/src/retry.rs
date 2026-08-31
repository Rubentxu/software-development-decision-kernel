//! Retry policy for task execution.
//!
//! `RetryPolicy` wraps `Task::evaluate` in a retry loop with configurable
//! backoff strategies: `Fixed`, `Linear`, `Exponential`, and `ExponentialWithJitter`.
//!
//! The `Clock` trait provides time readings for delay computation.
//! Production code uses `WallClock`; tests inject a mock `MockClock` for determinism.

/// Retry policy enumeration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetryPolicy {
    /// No retries — execute once.
    None,
    /// Fixed delay between attempts.
    Fixed {
        /// Delay in milliseconds.
        delay_ms: u64,
        /// Maximum number of attempts.
        max_attempts: u32,
    },
    /// Linear backoff: delay = base_ms * (attempt - 1).
    Linear {
        /// Base delay in milliseconds.
        base_ms: u64,
        /// Maximum backoff delay in milliseconds.
        max_backoff_ms: u64,
        /// Maximum number of attempts.
        max_attempts: u32,
    },
    /// Exponential backoff without jitter.
    Exponential {
        /// Base delay in milliseconds.
        base_ms: u64,
        /// Maximum backoff delay in milliseconds.
        max_backoff_ms: u64,
        /// Maximum number of attempts.
        max_attempts: u32,
    },
    /// Exponential backoff with uniform jitter.
    ExponentialWithJitter {
        /// Base delay in milliseconds.
        base_ms: u64,
        /// Maximum backoff delay in milliseconds.
        max_backoff_ms: u64,
        /// Jitter range: delay is sampled from [base, base + jitter_ms].
        jitter_ms: u64,
        /// Maximum number of attempts.
        max_attempts: u32,
    },
}

impl RetryPolicy {
    /// Returns `true` if this policy performs no retries.
    pub fn is_none(&self) -> bool {
        matches!(self, RetryPolicy::None)
    }

    /// Returns the maximum number of attempts.
    pub fn max_attempts(&self) -> u32 {
        match self {
            RetryPolicy::None => 1,
            RetryPolicy::Fixed { max_attempts, .. } => *max_attempts,
            RetryPolicy::Linear { max_attempts, .. } => *max_attempts,
            RetryPolicy::Exponential { max_attempts, .. } => *max_attempts,
            RetryPolicy::ExponentialWithJitter { max_attempts, .. } => *max_attempts,
        }
    }

    /// Computes the delay in milliseconds for a given attempt (1-indexed).
    ///
    /// Returns `0` for attempt 1 (first try — no delay before it).
    pub fn delay_ms(&self, attempt: u32, rng: &mut dyn RngCore) -> u64 {
        match self {
            RetryPolicy::None => 0,
            RetryPolicy::Fixed { delay_ms, .. } => {
                if attempt <= 1 {
                    0
                } else {
                    *delay_ms
                }
            }
            RetryPolicy::Linear {
                base_ms,
                max_backoff_ms,
                ..
            } => {
                if attempt <= 1 {
                    0
                } else {
                    let delay = base_ms.saturating_mul((attempt - 1) as u64);
                    delay.min(*max_backoff_ms)
                }
            }
            RetryPolicy::Exponential {
                base_ms,
                max_backoff_ms,
                ..
            } => {
                if attempt <= 1 {
                    0
                } else {
                    let delay = base_ms * 2u64.saturating_pow(attempt - 2);
                    delay.min(*max_backoff_ms)
                }
            }
            RetryPolicy::ExponentialWithJitter {
                base_ms,
                max_backoff_ms,
                jitter_ms,
                ..
            } => {
                if attempt <= 1 {
                    0
                } else {
                    let exp_delay = base_ms * 2u64.saturating_pow(attempt - 2);
                    let capped = exp_delay.min(*max_backoff_ms);
                    // Uniform jitter in [capped, capped + jitter_ms].
                    let jitter_range = jitter_ms.saturating_add(1);
                    let jitter = rng.next_u64() % jitter_range;
                    capped.saturating_add(jitter)
                }
            }
        }
    }

    /// Returns a policy for testing: `Fixed(10ms, 3 attempts)`.
    #[cfg(test)]
    pub fn fixed_10ms_3attempts() -> Self {
        RetryPolicy::Fixed {
            delay_ms: 10,
            max_attempts: 3,
        }
    }
}

/// Backing store for `RetryPolicy::delay_ms` RNG parameter.
pub trait RngCore {
    /// Returns the next random u64.
    fn next_u64(&mut self) -> u64;
}

// ── Clock trait ─────────────────────────────────────────────────────────────

/// Time source for retry delay computation.
pub trait Clock: Send + Sync {
    /// Returns the current time in milliseconds since some epoch.
    fn now_ms(&self) -> u64;
}

/// Wall-clock implementation using `std::time::SystemTime`.
#[derive(Debug, Clone, Copy, Default)]
pub struct WallClock;

impl Clock for WallClock {
    fn now_ms(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}

/// Mock clock for deterministic testing.
#[derive(Debug, Clone)]
pub struct MockClock {
    now_ms: u64,
}

impl MockClock {
    /// Creates a mock clock fixed at the given timestamp.
    pub fn new(now_ms: u64) -> Self {
        Self { now_ms }
    }

    /// Advances the clock by `delta_ms` milliseconds.
    pub fn advance(&mut self, delta_ms: u64) {
        self.now_ms = self.now_ms.saturating_add(delta_ms);
    }
}

impl Clock for MockClock {
    fn now_ms(&self) -> u64 {
        self.now_ms
    }
}

// ── Mock RNG for tests ───────────────────────────────────────────────────────

/// Deterministic mock RNG for testing retry jitter.
#[cfg(test)]
pub mod mock {
    use super::RngCore;

    /// A simple step RNG for deterministic testing.
    #[derive(Debug, Clone)]
    pub struct StepRng {
        value: u64,
        step: u64,
    }

    impl StepRng {
        /// Creates a `StepRng` starting at `seed` with the given `step`.
        pub fn new(seed: u64, step: u64) -> Self {
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
}

#[cfg(test)]
mod tests {
    use super::{RetryPolicy, mock::StepRng};

    // ── Fixed ─────────────────────────────────────────────────────────────────

    #[test]
    fn fixed_policy_attempt_1_has_zero_delay() {
        let policy = RetryPolicy::Fixed {
            delay_ms: 100,
            max_attempts: 3,
        };
        let mut rng = StepRng::new(0, 1);
        assert_eq!(policy.delay_ms(1, &mut rng), 0);
    }

    #[test]
    fn fixed_policy_attempt_2_returns_delay() {
        let policy = RetryPolicy::Fixed {
            delay_ms: 100,
            max_attempts: 3,
        };
        let mut rng = StepRng::new(0, 1);
        assert_eq!(policy.delay_ms(2, &mut rng), 100);
    }

    #[test]
    fn fixed_policy_is_none_false() {
        let policy = RetryPolicy::fixed_10ms_3attempts();
        assert!(!policy.is_none());
    }

    // ── Linear ────────────────────────────────────────────────────────────────

    #[test]
    fn linear_policy_attempt_1_zero_delay() {
        let policy = RetryPolicy::Linear {
            base_ms: 10,
            max_backoff_ms: 1000,
            max_attempts: 5,
        };
        let mut rng = StepRng::new(0, 1);
        assert_eq!(policy.delay_ms(1, &mut rng), 0);
    }

    #[test]
    fn linear_policy_attempt_2_base_delay() {
        let policy = RetryPolicy::Linear {
            base_ms: 10,
            max_backoff_ms: 1000,
            max_attempts: 5,
        };
        let mut rng = StepRng::new(0, 1);
        // attempt 2: base_ms * (2-1) = 10
        assert_eq!(policy.delay_ms(2, &mut rng), 10);
    }

    #[test]
    fn linear_policy_attempt_3_doubles() {
        let policy = RetryPolicy::Linear {
            base_ms: 10,
            max_backoff_ms: 1000,
            max_attempts: 5,
        };
        let mut rng = StepRng::new(0, 1);
        // attempt 3: base_ms * (3-1) = 20
        assert_eq!(policy.delay_ms(3, &mut rng), 20);
    }

    // ── Exponential ────────────────────────────────────────────────────────────

    #[test]
    fn exponential_policy_attempt_1_zero_delay() {
        let policy = RetryPolicy::Exponential {
            base_ms: 100,
            max_backoff_ms: 1000,
            max_attempts: 4,
        };
        let mut rng = StepRng::new(0, 1);
        assert_eq!(policy.delay_ms(1, &mut rng), 0);
    }

    #[test]
    fn exponential_policy_attempt_2_base_delay() {
        let policy = RetryPolicy::Exponential {
            base_ms: 100,
            max_backoff_ms: 1000,
            max_attempts: 4,
        };
        let mut rng = StepRng::new(0, 1);
        assert_eq!(policy.delay_ms(2, &mut rng), 100);
    }

    #[test]
    fn exponential_policy_attempt_3_doubles() {
        let policy = RetryPolicy::Exponential {
            base_ms: 100,
            max_backoff_ms: 1000,
            max_attempts: 5,
        };
        let mut rng = StepRng::new(0, 1);
        // attempt 3: base_ms * 2^(3-2) = 100 * 2 = 200
        assert_eq!(policy.delay_ms(3, &mut rng), 200);
    }

    #[test]
    fn exponential_policy_respects_max_backoff() {
        let policy = RetryPolicy::Exponential {
            base_ms: 100,
            max_backoff_ms: 150, // cap at 150
            max_attempts: 10,
        };
        let mut rng = StepRng::new(0, 1);
        // attempt 5: 100 * 2^3 = 800 → capped at 150
        assert_eq!(policy.delay_ms(5, &mut rng), 150);
    }

    // ── ExponentialWithJitter ────────────────────────────────────────────────

    #[test]
    fn jitter_policy_attempt_1_zero_delay() {
        let policy = RetryPolicy::ExponentialWithJitter {
            base_ms: 100,
            max_backoff_ms: 200,
            jitter_ms: 50,
            max_attempts: 3,
        };
        let mut rng = StepRng::new(0, 1);
        assert_eq!(policy.delay_ms(1, &mut rng), 0);
    }

    #[test]
    fn jitter_policy_delay_in_expected_range() {
        let policy = RetryPolicy::ExponentialWithJitter {
            base_ms: 100,
            max_backoff_ms: 200,
            jitter_ms: 50,
            max_attempts: 3,
        };
        // StepRng(seed=0, step=1): next_u64 returns 0, then 1, etc.
        // attempt 2: exp=100, jitter_range=51, jitter=0 → 100+0=100
        let mut rng1 = StepRng::new(0, 1);
        let d1 = policy.delay_ms(2, &mut rng1);
        assert!(
            (100..=150).contains(&d1),
            "delay {d1} should be in [100, 150]"
        );

        // StepRng(seed=50, step=1): next_u64 returns 50, jitter=50 → 100+50=150
        let mut rng2 = StepRng::new(50, 1);
        let d2 = policy.delay_ms(2, &mut rng2);
        assert!(
            (100..=150).contains(&d2),
            "delay {d2} should be in [100, 150]"
        );
    }

    // ── max_attempts ─────────────────────────────────────────────────────────

    #[test]
    fn none_policy_max_attempts_is_1() {
        let policy = RetryPolicy::None;
        assert_eq!(policy.max_attempts(), 1);
    }

    #[test]
    fn fixed_policy_max_attempts() {
        let policy = RetryPolicy::fixed_10ms_3attempts();
        assert_eq!(policy.max_attempts(), 3);
    }
}
