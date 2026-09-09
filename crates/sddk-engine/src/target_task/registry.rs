//! `TargetRegistry` — lookup `target_name -> Target`.
//!
//! The registry holds the typed map of declared targets. Built-in
//! targets ship with the engine (this cycle); user-declared targets
//! land in a later cycle.

#![allow(missing_docs)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::dag::resolve_dag;
use super::{Target, TargetResolution};

/// Lookup index over declared targets.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetRegistry {
    by_name: BTreeMap<String, Target>,
}

impl TargetRegistry {
    /// Construct an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a target. Returns the previous binding, if any.
    pub fn register(&mut self, target: Target) -> Option<Target> {
        self.by_name.insert(target.name.clone(), target)
    }

    /// Number of registered targets.
    pub fn len(&self) -> usize {
        self.by_name.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }

    /// Iterate over registered target names in lexicographic order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.by_name.keys().map(String::as_str)
    }

    /// Look up a target by name without resolving the DAG.
    pub fn get(&self, name: &str) -> Option<&Target> {
        self.by_name.get(name)
    }

    /// Resolve a target by name to a topologically-ordered DAG.
    ///
    /// Unknown target names produce a [`TargetResolution`] with
    /// `dag = None` and `validation` describing the error — this
    /// shape is machine-readable and serializable, which keeps the
    /// engine↔CLI contract uniform.
    pub fn resolve(&self, name: &str) -> TargetResolution {
        let Some(target) = self.by_name.get(name) else {
            return TargetResolution {
                target: name.to_string(),
                dag: None,
                validation: format!("unknown target `{name}`"),
            };
        };
        match resolve_dag(target) {
            Ok(dag) => TargetResolution {
                target: target.name.clone(),
                dag: Some(dag),
                validation: "ok".to_string(),
            },
            Err(e) => TargetResolution {
                target: target.name.clone(),
                dag: None,
                validation: e.to_string(),
            },
        }
    }

    /// Construct a registry pre-populated with the built-in targets.
    pub fn with_builtins() -> Self {
        let mut r = Self::new();
        for target in super::builtin::builtin_targets() {
            r.register(target);
        }
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::target_task::{
        AuthorityRequirement, Cacheability, Determinism, EvidenceContract, MemoryEffect,
        RetryPolicy, SideEffectClass, Task,
    };

    fn minimal_target(name: &str) -> Target {
        Target {
            name: name.into(),
            about: format!("target {name}"),
            tasks: vec![Task {
                id: "only".into(),
                inputs: vec![],
                outputs: vec![],
                depends_on: vec![],
                side_effect_class: SideEffectClass::Pure,
                authority_requirement: AuthorityRequirement::None,
                determinism: Determinism::Deterministic,
                cacheability: Cacheability::Cacheable,
                retry_policy: RetryPolicy::NoRetry,
                evidence_contract: EvidenceContract::None,
                memory_effects: MemoryEffect::None,
            }],
        }
    }

    #[test]
    fn register_and_get_roundtrip() {
        let mut r = TargetRegistry::new();
        assert!(r.register(minimal_target("a")).is_none());
        assert!(r.get("a").is_some());
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn register_overwrites_with_previous_returned() {
        let mut r = TargetRegistry::new();
        r.register(minimal_target("a"));
        let prev = r.register(minimal_target("a"));
        assert!(prev.is_some());
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn resolve_unknown_target_returns_resolution_with_no_dag() {
        let r = TargetRegistry::new();
        let res = r.resolve("nope");
        assert_eq!(res.target, "nope");
        assert!(res.dag.is_none());
        assert!(res.validation.contains("unknown target"));
    }

    #[test]
    fn resolve_known_target_returns_dag() {
        let mut r = TargetRegistry::new();
        r.register(minimal_target("a"));
        let res = r.resolve("a");
        assert_eq!(res.target, "a");
        let dag = res.dag.expect("dag");
        assert_eq!(dag.order, vec!["only"]);
        assert_eq!(res.validation, "ok");
    }

    #[test]
    fn names_iterates_in_lexicographic_order() {
        let mut r = TargetRegistry::new();
        r.register(minimal_target("z"));
        r.register(minimal_target("a"));
        r.register(minimal_target("m"));
        let names: Vec<&str> = r.names().collect();
        assert_eq!(names, vec!["a", "m", "z"]);
    }

    #[test]
    fn with_builtins_populates_four_targets() {
        let r = TargetRegistry::with_builtins();
        assert_eq!(r.len(), 4);
        assert!(r.get("status").is_some());
        assert!(r.get("run").is_some());
        assert!(r.get("ship").is_some());
        assert!(r.get("recover").is_some());
    }
}
