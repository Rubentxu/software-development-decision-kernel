// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// state_class_lint.rs — T-06 (M1 arch-spec-001 CA-004)
//
// Persistent models MUST declare Fact/Object/Projection/Ephemeral
// before they register. The linter refuses registration when
// `state_class` is missing.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

/// Four-state classification (mirrors ADR-0095-FOUR-STATE-CLASSES).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum StateClass {
    Fact,
    Object,
    Projection,
    Ephemeral,
}

impl StateClass {
    pub fn domain_tag(&self) -> &'static str {
        match self {
            StateClass::Fact => "fact",
            StateClass::Object => "object",
            StateClass::Projection => "projection",
            StateClass::Ephemeral => "ephemeral",
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum StateClassError {
    #[error("model {model} must declare a state_class (Fact|Object|Projection|Ephemeral)")]
    MissingStateClass { model: String },
}

/// A persistent model declaration. `state_class` is a required field;
/// there is no `Option` here.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistentModel {
    pub id: String,
    pub state_class: StateClass,
    pub owner_crate: String,
    pub description: String,
}

impl PersistentModel {
    pub fn validate(&self) -> Result<(), StateClassError> {
        if self.id.is_empty() {
            return Err(StateClassError::MissingStateClass {
                model: "<empty id>".into(),
            });
        }
        Ok(())
    }
}

/// In-memory registry of declared persistent models.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistentModelRegistry {
    models: BTreeMap<String, PersistentModel>,
}

impl PersistentModelRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a model. `state_class` must be declared at the type level
    /// (compilation enforces this); runtime validation checks `id` not empty.
    pub fn register(&mut self, model: PersistentModel) -> Result<(), StateClassError> {
        model.validate()?;
        if self.models.contains_key(&model.id) {
            // Idempotent: duplicate registrations of the same id+state_class
            // are no-ops. Mismatched state_class is an error.
            if let Some(existing) = self.models.get(&model.id)
                && existing.state_class != model.state_class
            {
                return Err(StateClassError::MissingStateClass {
                    model: format!(
                        "{} redeclared with class {:?} (was {:?})",
                        model.id, model.state_class, existing.state_class
                    ),
                });
            }
            return Ok(());
        }
        self.models.insert(model.id.clone(), model);
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&PersistentModel> {
        self.models.get(id)
    }

    pub fn len(&self) -> usize {
        self.models.len()
    }

    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &PersistentModel)> {
        self.models.iter()
    }
}

/// Lint that surfaces models registered without a state_class. With the
/// current `PersistentModel` design, this is a static guarantee (the field
/// is required), but the function remains for the legacy attachment case
/// where state_class comes from configuration rather than the type.
pub fn lint_state_classes(proposed: &[Option<StateClass>]) -> Vec<StateClassError> {
    let mut errors = Vec::new();
    for (i, proposed) in proposed.iter().enumerate() {
        if proposed.is_none() {
            errors.push(StateClassError::MissingStateClass {
                model: format!("index-{i}"),
            });
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(id: &str, sc: StateClass) -> PersistentModel {
        PersistentModel {
            id: id.into(),
            state_class: sc,
            owner_crate: "sddk-engine".into(),
            description: format!("test model {id}"),
        }
    }

    #[test]
    fn register_accepts_fact_class() {
        let mut reg = PersistentModelRegistry::new();
        reg.register(model("fact_event_log", StateClass::Fact))
            .expect("fact class accepted");
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn register_accepts_object_class() {
        let mut reg = PersistentModelRegistry::new();
        reg.register(model("cas_object_store", StateClass::Object))
            .expect("object class accepted");
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn register_accepts_projection_class() {
        let mut reg = PersistentModelRegistry::new();
        reg.register(model("semantic_graph_view", StateClass::Projection))
            .expect("projection class accepted");
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn register_rejects_empty_id() {
        let mut reg = PersistentModelRegistry::new();
        let m = PersistentModel {
            id: "".into(),
            state_class: StateClass::Fact,
            owner_crate: "x".into(),
            description: "x".into(),
        };
        let err = reg.register(m).unwrap_err();
        match err {
            StateClassError::MissingStateClass { model } => {
                assert_eq!(model, "<empty id>");
            }
        }
    }

    #[test]
    fn register_rejects_state_class_drift() {
        let mut reg = PersistentModelRegistry::new();
        reg.register(model("m", StateClass::Fact)).unwrap();
        let conflict = model("m", StateClass::Object);
        let err = reg.register(conflict).unwrap_err();
        match err {
            StateClassError::MissingStateClass { model } => {
                assert!(model.contains("redeclared"));
            }
        }
    }

    #[test]
    fn lint_rejects_undeclared_class() {
        let proposed = vec![Some(StateClass::Fact), None, Some(StateClass::Object)];
        let errs = lint_state_classes(&proposed);
        assert_eq!(errs.len(), 1);
        match &errs[0] {
            StateClassError::MissingStateClass { model } => {
                assert_eq!(model, "index-1");
            }
        }
    }

    #[test]
    fn register_idempotent_for_same_id_class() {
        let mut reg = PersistentModelRegistry::new();
        reg.register(model("m", StateClass::Fact)).unwrap();
        reg.register(model("m", StateClass::Fact)).unwrap();
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn state_class_domain_tags_are_distinct() {
        assert_ne!(
            StateClass::Fact.domain_tag(),
            StateClass::Object.domain_tag()
        );
        assert_ne!(
            StateClass::Projection.domain_tag(),
            StateClass::Ephemeral.domain_tag()
        );
    }
}
