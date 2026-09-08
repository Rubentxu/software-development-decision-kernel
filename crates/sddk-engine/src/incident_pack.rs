//! Incident Pack on Canonical Runtime
//!
//! Plain-data incident pack (manifest + severity + runbooks +
//! post-mortems) validated against the canonical runtime contract.
//! No pack-specific kernel special cases.
//!
//! Pattern: P-a (struct + trait + plain-data evaluator).

use crate::generic_pack_contracts::{
    DefaultPackManifestValidator, PackContractError, PackKind, PackManifest, PackManifestValidator,
    RuntimeVersion,
};

// ── Audit guard constant ────────────────────────────────────────────────

/// Closed-set size of [`IncidentSeverity`]. Bump when a variant is added.
pub const INCIDENT_SEVERITY_VARIANT_COUNT: usize = 4;

// ── Enums + records ────────────────────────────────────────────────────

/// Closed-set taxonomy of incident severity.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IncidentSeverity {
    Sev1,
    Sev2,
    Sev3,
    Sev4,
}

impl IncidentSeverity {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Sev1 => "sev1",
            Self::Sev2 => "sev2",
            Self::Sev3 => "sev3",
            Self::Sev4 => "sev4",
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncidentRunbook {
    pub runbook_id: String,
    pub title: String,
    /// Sorted.
    pub steps: Vec<String>,
    /// Sorted.
    pub escalation: Vec<String>,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncidentPostMortem {
    pub post_mortem_id: String,
    pub root_cause: String,
    /// Sorted.
    pub remediation: Vec<String>,
    pub recorded_at: String,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncidentPack {
    pub manifest: PackManifest,
    pub severity: IncidentSeverity,
    /// Sorted by `runbook_id`.
    pub runbooks: Vec<IncidentRunbook>,
    /// Sorted by `post_mortem_id`.
    pub post_mortems: Vec<IncidentPostMortem>,
    pub installed_at: String,
}

// ── Error + installer ──────────────────────────────────────────────────

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IncidentPackError {
    NotIncidentKind,
    EmptyRunbooks,
    DuplicateRunbookId,
    RunbookMissingSteps { runbook_id: String },
    PostMortemMissingRootCause { post_mortem_id: String },
    ManifestRejected(PackContractError),
    IncompatibleRuntime(String),
}

pub trait IncidentPackInstaller {
    fn install(
        &self,
        runtime_version: &RuntimeVersion,
        pack: &IncidentPack,
        known_pack_ids: &[String],
    ) -> Result<(), IncidentPackError>;
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DefaultIncidentPackInstaller;

impl IncidentPackInstaller for DefaultIncidentPackInstaller {
    fn install(
        &self,
        runtime_version: &RuntimeVersion,
        pack: &IncidentPack,
        known_pack_ids: &[String],
    ) -> Result<(), IncidentPackError> {
        if pack.manifest.kind != PackKind::Incident {
            return Err(IncidentPackError::NotIncidentKind);
        }
        if pack.runbooks.is_empty() {
            return Err(IncidentPackError::EmptyRunbooks);
        }
        let mut sorted_runbooks = pack.runbooks.clone();
        sorted_runbooks.sort_by(|a, b| a.runbook_id.cmp(&b.runbook_id));
        for w in sorted_runbooks.windows(2) {
            if w[0].runbook_id == w[1].runbook_id {
                return Err(IncidentPackError::DuplicateRunbookId);
            }
        }
        for rb in &sorted_runbooks {
            if rb.steps.is_empty() {
                return Err(IncidentPackError::RunbookMissingSteps {
                    runbook_id: rb.runbook_id.clone(),
                });
            }
        }
        for pm in &pack.post_mortems {
            if pm.root_cause.is_empty() {
                return Err(IncidentPackError::PostMortemMissingRootCause {
                    post_mortem_id: pm.post_mortem_id.clone(),
                });
            }
        }
        let validator = DefaultPackManifestValidator;
        validator
            .validate(&pack.manifest, runtime_version, known_pack_ids)
            .map_err(IncidentPackError::ManifestRejected)?;
        if runtime_version < &pack.manifest.min_runtime_version
            || runtime_version > &pack.manifest.max_runtime_version
        {
            return Err(IncidentPackError::IncompatibleRuntime(
                runtime_version.to_string(),
            ));
        }
        Ok(())
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generic_pack_contracts::EvidenceFormat;

    fn manifest(id: &str) -> PackManifest {
        PackManifest {
            pack_id: id.to_string(),
            kind: PackKind::Incident,
            evidence_format: EvidenceFormat::JsonSchemaDraft07,
            min_runtime_version: RuntimeVersion::new(1, 0, 0),
            max_runtime_version: RuntimeVersion::new(2, 0, 0),
            schema_ref: "schema://incident".to_string(),
            recorded_at: "t0".to_string(),
        }
    }

    fn runbook(id: &str) -> IncidentRunbook {
        let mut steps = vec!["detect".to_string(), "mitigate".to_string()];
        steps.sort();
        IncidentRunbook {
            runbook_id: id.to_string(),
            title: format!("Runbook {id}"),
            steps,
            escalation: vec![],
        }
    }

    fn pack(id: &str, runbooks: Vec<IncidentRunbook>) -> IncidentPack {
        IncidentPack {
            manifest: manifest(id),
            severity: IncidentSeverity::Sev2,
            runbooks,
            post_mortems: vec![],
            installed_at: "t0".to_string(),
        }
    }

    fn installer() -> DefaultIncidentPackInstaller {
        DefaultIncidentPackInstaller
    }

    #[test]
    fn s1_valid_pack_installs() {
        let p = pack("inc-1", vec![runbook("rb1")]);
        let r = installer().install(&RuntimeVersion::new(1, 5, 0), &p, &[]);
        assert_eq!(r, Ok(()));
    }

    #[test]
    fn s2_non_incident_kind_rejected() {
        let mut p = pack("inc-1", vec![runbook("rb1")]);
        p.manifest.kind = PackKind::Sdd;
        let r = installer().install(&RuntimeVersion::new(1, 0, 0), &p, &[]);
        assert_eq!(r, Err(IncidentPackError::NotIncidentKind));
    }

    #[test]
    fn s3_empty_runbooks_rejected() {
        let p = pack("inc-1", vec![]);
        let r = installer().install(&RuntimeVersion::new(1, 0, 0), &p, &[]);
        assert_eq!(r, Err(IncidentPackError::EmptyRunbooks));
    }

    #[test]
    fn s4_duplicate_runbook_id_rejected() {
        let p = pack("inc-1", vec![runbook("rb1"), runbook("rb1")]);
        let r = installer().install(&RuntimeVersion::new(1, 0, 0), &p, &[]);
        assert_eq!(r, Err(IncidentPackError::DuplicateRunbookId));
    }

    #[test]
    fn s5_runbook_missing_steps_rejected() {
        let mut rb = runbook("rb1");
        rb.steps = vec![];
        let p = pack("inc-1", vec![rb]);
        let r = installer().install(&RuntimeVersion::new(1, 0, 0), &p, &[]);
        assert_eq!(
            r,
            Err(IncidentPackError::RunbookMissingSteps {
                runbook_id: "rb1".to_string()
            })
        );
    }

    #[test]
    fn s6_closed_set_audit() {
        assert_eq!(INCIDENT_SEVERITY_VARIANT_COUNT, 4);
        let all = [
            IncidentSeverity::Sev1,
            IncidentSeverity::Sev2,
            IncidentSeverity::Sev3,
            IncidentSeverity::Sev4,
        ];
        assert_eq!(all.len(), INCIDENT_SEVERITY_VARIANT_COUNT);
    }

    #[test]
    fn s7_deterministic() {
        let p = pack("inc-1", vec![runbook("rb1")]);
        let v = RuntimeVersion::new(1, 0, 0);
        let r1 = installer().install(&v, &p, &[]);
        let r2 = installer().install(&v, &p, &[]);
        assert_eq!(r1, r2);
    }

    #[test]
    fn s8_post_mortem_missing_root_cause_rejected() {
        let p = IncidentPack {
            manifest: manifest("inc-1"),
            severity: IncidentSeverity::Sev1,
            runbooks: vec![runbook("rb1")],
            post_mortems: vec![IncidentPostMortem {
                post_mortem_id: "pm1".to_string(),
                root_cause: String::new(),
                remediation: vec![],
                recorded_at: "t0".to_string(),
            }],
            installed_at: "t0".to_string(),
        };
        let r = installer().install(&RuntimeVersion::new(1, 0, 0), &p, &[]);
        assert_eq!(
            r,
            Err(IncidentPackError::PostMortemMissingRootCause {
                post_mortem_id: "pm1".to_string()
            })
        );
    }
}
