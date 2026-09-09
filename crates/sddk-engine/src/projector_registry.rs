// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// projector_registry.rs — T-07 (M1 arch-spec-001 CA-003)
//
// Registry of projectors with required metadata:
//   - source_streams: Vec<StreamId>
//   - version: SemVer
//   - rebuild: RebuildBehavior
//
// Registration fails closed when any of these is missing.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct StreamId(pub String);

impl StreamId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct SemVer(pub String);

impl SemVer {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RebuildBehavior {
    Full,
    Incremental,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProjectorRegistryError {
    #[error("projector {name} must declare at least one source stream")]
    NoSourceStreams { name: String },
    #[error("projector {name} version is empty")]
    EmptyVersion { name: String },
    #[error("projector {name} is already registered")]
    AlreadyRegistered { name: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Projector {
    pub name: String,
    pub source_streams: Vec<StreamId>,
    pub version: SemVer,
    pub rebuild: RebuildBehavior,
}

impl Projector {
    pub fn validate(&self) -> Result<(), ProjectorRegistryError> {
        if self.source_streams.is_empty() {
            return Err(ProjectorRegistryError::NoSourceStreams {
                name: self.name.clone(),
            });
        }
        if self.version.as_str().is_empty() {
            return Err(ProjectorRegistryError::EmptyVersion {
                name: self.name.clone(),
            });
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectorRegistry {
    projectors: BTreeMap<String, Projector>,
}

impl ProjectorRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, projector: Projector) -> Result<(), ProjectorRegistryError> {
        projector.validate()?;
        if self.projectors.contains_key(&projector.name) {
            return Err(ProjectorRegistryError::AlreadyRegistered {
                name: projector.name.clone(),
            });
        }
        self.projectors.insert(projector.name.clone(), projector);
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&Projector> {
        self.projectors.get(name)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Projector)> {
        self.projectors.iter()
    }

    pub fn len(&self) -> usize {
        self.projectors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.projectors.is_empty()
    }
}

/// Cross-check with AuthorityEngine: returns Err when the projector
/// declares no source stream AND claims to be the sole evidence for
/// an irreversible side effect.
pub fn project_can_serve_as_authority_evidence(
    projector: &Projector,
    claims_authority: bool,
) -> Result<(), ProjectorRegistryError> {
    if claims_authority && projector.source_streams.is_empty() {
        return Err(ProjectorRegistryError::NoSourceStreams {
            name: projector.name.clone(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn projector(name: &str, streams: Vec<&str>, version: &str, rb: RebuildBehavior) -> Projector {
        Projector {
            name: name.into(),
            source_streams: streams.into_iter().map(StreamId::new).collect(),
            version: SemVer::new(version),
            rebuild: rb,
        }
    }

    #[test]
    fn register_rejects_missing_source_streams() {
        let mut reg = ProjectorRegistry::new();
        let p = projector("p1", vec![], "1.0.0", RebuildBehavior::Full);
        let err = reg.register(p).unwrap_err();
        assert_eq!(
            err,
            ProjectorRegistryError::NoSourceStreams { name: "p1".into() }
        );
    }

    #[test]
    fn register_rejects_empty_version() {
        let mut reg = ProjectorRegistry::new();
        let mut p = projector("p1", vec!["stream:a"], "", RebuildBehavior::Full);
        p.version = SemVer::new("");
        let err = reg.register(p).unwrap_err();
        assert_eq!(
            err,
            ProjectorRegistryError::EmptyVersion { name: "p1".into() }
        );
    }

    #[test]
    fn register_rejects_duplicate_name() {
        let mut reg = ProjectorRegistry::new();
        reg.register(projector("p1", vec!["s:a"], "1.0.0", RebuildBehavior::Full))
            .unwrap();
        let err = reg
            .register(projector(
                "p1",
                vec!["s:b"],
                "1.0.1",
                RebuildBehavior::Incremental,
            ))
            .unwrap_err();
        assert_eq!(
            err,
            ProjectorRegistryError::AlreadyRegistered { name: "p1".into() }
        );
    }

    #[test]
    fn register_succeeds_with_full_metadata() {
        let mut reg = ProjectorRegistry::new();
        reg.register(projector("p1", vec!["s:a"], "1.0.0", RebuildBehavior::Full))
            .expect("register ok");
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn rebuild_behavior_is_explicit() {
        // Two distinct registered values for `rebuild` — cannot be silently
        // defaulted from one to the other.
        let p_full = projector("p1", vec!["s:a"], "1.0.0", RebuildBehavior::Full);
        let p_inc = projector("p2", vec!["s:a"], "1.0.0", RebuildBehavior::Incremental);
        assert_ne!(p_full.rebuild, p_inc.rebuild);
    }

    #[test]
    fn projector_without_streams_cannot_serve_authority_evidence() {
        let p = projector("p1", vec![], "1.0.0", RebuildBehavior::Full);
        let err = project_can_serve_as_authority_evidence(&p, true).unwrap_err();
        match err {
            ProjectorRegistryError::NoSourceStreams { name } => {
                assert_eq!(name, "p1");
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn projector_with_streams_can_serve_authority_evidence() {
        let p = projector(
            "p1",
            vec!["s:canonical_fact_log"],
            "1.0.0",
            RebuildBehavior::Full,
        );
        assert!(project_can_serve_as_authority_evidence(&p, true).is_ok());
    }
}
