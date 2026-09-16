// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// debverify_kernel/strategy_set.rs — A4-2: deterministic strategy registry.

use std::collections::BTreeMap;

use super::strategy::ChallengeStrategy;
use super::types::ReconciliationScope;

/// A deterministic, name-keyed set of [`ChallengeStrategy`] values.
///
/// Iteration order is always by name (BTreeMap), so strategy execution order
/// is independent of insertion order and stable across processes.
#[derive(Default)]
pub struct ChallengeStrategySet {
    strategies: BTreeMap<String, Box<dyn ChallengeStrategy>>,
}

impl ChallengeStrategySet {
    /// Create an empty set.
    pub fn new() -> Self {
        Self::default()
    }
    /// Register a strategy. If a strategy with the same name already exists,
    /// it is replaced.
    ///
    /// The strategy is taken by value. Zero-sized strategy structs (the
    /// common case) are stored behind a trait object without allocation.
    pub fn register<S: ChallengeStrategy + 'static>(mut self, strategy: S) -> Self {
        let name = strategy.name().to_string();
        let boxed: Box<dyn ChallengeStrategy> = Box::new(strategy);
        self.strategies.insert(name, boxed);
        // Stable, name-sorted iteration.
        self
    }
    /// Number of registered strategies.
    pub fn len(&self) -> usize {
        self.strategies.len()
    }
    /// True iff no strategies are registered.
    pub fn is_empty(&self) -> bool {
        self.strategies.is_empty()
    }
    /// Names of all registered strategies, in canonical order.
    pub fn names(&self) -> Vec<&'static str> {
        // BTreeMap iteration is sorted by String. We must return &'static str;
        // since `name()` returns `&'static str`, that's fine.
        self.strategies.values().map(|s| s.name()).collect()
    }
    /// Every strategy applicable to the given scope, in canonical order.
    pub fn applicable(&self, scope: &ReconciliationScope) -> Vec<&dyn ChallengeStrategy> {
        self.strategies
            .values()
            .filter(|s| s.applicable(scope))
            .map(|s| &**s as &dyn ChallengeStrategy)
            .collect()
    }
    /// Look up a strategy by name.
    pub fn lookup(&self, name: &str) -> Result<&dyn ChallengeStrategy, StrategyNotFoundError> {
        self.strategies
            .get(name)
            .map(|s| &**s as &dyn ChallengeStrategy)
            .ok_or_else(|| StrategyNotFoundError(name.to_string()))
    }
}

/// Returned when a named strategy does not exist in the set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StrategyNotFoundError(pub String);

impl std::fmt::Display for StrategyNotFoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "debverify_kernel: strategy not found: `{}`", self.0)
    }
}

impl std::error::Error for StrategyNotFoundError {}
