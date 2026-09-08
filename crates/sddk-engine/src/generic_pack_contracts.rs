//! Generic Pack Contracts
//!
//! Locks the closed-set taxonomy of pack kinds + the shape of
//! `PackManifest` + a plain-data validator that every pack (SDD, UAT,
//! Incident) must satisfy. Read-only with respect to canonical state.
//!
//! Pattern: P-a (struct + trait + plain-data evaluator).

// ── Audit guard constants ────────────────────────────────────────────────

/// Closed-set size of [`PackKind`]. Bump when a variant is added.
pub const PACK_KIND_VARIANT_COUNT: usize = 3;

/// Closed-set size of [`EvidenceFormat`]. Bump when a variant is added.
pub const EVIDENCE_FORMAT_VARIANT_COUNT: usize = 4;

// ── Version ──────────────────────────────────────────────────────────────

/// Minimal semver-like version. We don't pull in the `semver` crate
/// to keep the kernel lean.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl RuntimeVersion {
    #[must_use]
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        let major = parts[0].parse().ok()?;
        let minor = parts[1].parse().ok()?;
        let patch = parts[2].parse().ok()?;
        Some(Self {
            major,
            minor,
            patch,
        })
    }
}

impl core::fmt::Display for RuntimeVersion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

// ── Pack kind + evidence format ─────────────────────────────────────────

/// Closed-set taxonomy of pack kinds.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PackKind {
    Sdd,
    Uat,
    Incident,
}

impl PackKind {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Sdd => "sdd",
            Self::Uat => "uat",
            Self::Incident => "incident",
        }
    }
}

/// Closed-set taxonomy of evidence formats.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceFormat {
    JsonSchemaDraft07,
    CycloneDx15,
    OpenTelemetryProto1,
    Custom { label: String, schema_ref: String },
}

impl EvidenceFormat {
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::JsonSchemaDraft07 => "json_schema_draft_07",
            Self::CycloneDx15 => "cyclonedx_1_5",
            Self::OpenTelemetryProto1 => "opentelemetry_proto_1",
            Self::Custom { .. } => "custom",
        }
    }
}

// ── Manifest + error ────────────────────────────────────────────────────

/// Manifest that every pack declares.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackManifest {
    pub pack_id: String,
    pub kind: PackKind,
    pub evidence_format: EvidenceFormat,
    pub min_runtime_version: RuntimeVersion,
    pub max_runtime_version: RuntimeVersion,
    pub schema_ref: String,
    pub recorded_at: String,
}

/// Errors emitted by the manifest validator.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackContractError {
    UnknownKind,
    IncompatibleRuntime {
        pack_id: String,
        runtime: String,
        required: String,
    },
    MissingSchemaRef,
    InvalidEvidenceFormat,
    DuplicatePackId,
}

// ── Trait + default validator ──────────────────────────────────────────

/// Validates a pack manifest against the generic contract.
pub trait PackManifestValidator {
    fn validate(
        &self,
        manifest: &PackManifest,
        runtime_version: &RuntimeVersion,
        known_pack_ids: &[String],
    ) -> Result<(), PackContractError>;
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DefaultPackManifestValidator;

impl PackManifestValidator for DefaultPackManifestValidator {
    fn validate(
        &self,
        manifest: &PackManifest,
        runtime_version: &RuntimeVersion,
        known_pack_ids: &[String],
    ) -> Result<(), PackContractError> {
        // Duplicate id.
        if known_pack_ids.iter().any(|id| id == &manifest.pack_id) {
            return Err(PackContractError::DuplicatePackId);
        }
        // Schema ref non-empty.
        if manifest.schema_ref.is_empty() {
            return Err(PackContractError::MissingSchemaRef);
        }
        // Runtime in range.
        if runtime_version < &manifest.min_runtime_version
            || runtime_version > &manifest.max_runtime_version
        {
            return Err(PackContractError::IncompatibleRuntime {
                pack_id: manifest.pack_id.clone(),
                runtime: runtime_version.to_string(),
                required: format!(
                    ">={} <= {}",
                    manifest.min_runtime_version, manifest.max_runtime_version
                ),
            });
        }
        // Custom evidence format requires both label and schema_ref.
        if let EvidenceFormat::Custom { label, schema_ref } = &manifest.evidence_format
            && (label.is_empty() || schema_ref.is_empty())
        {
            return Err(PackContractError::InvalidEvidenceFormat);
        }
        Ok(())
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn validator() -> DefaultPackManifestValidator {
        DefaultPackManifestValidator
    }

    fn manifest(id: &str, min: (u32, u32, u32), max: (u32, u32, u32)) -> PackManifest {
        PackManifest {
            pack_id: id.to_string(),
            kind: PackKind::Sdd,
            evidence_format: EvidenceFormat::JsonSchemaDraft07,
            min_runtime_version: RuntimeVersion::new(min.0, min.1, min.2),
            max_runtime_version: RuntimeVersion::new(max.0, max.1, max.2),
            schema_ref: "schema://valid".to_string(),
            recorded_at: "t0".to_string(),
        }
    }

    // ── S-1: valid manifest passes ───────────────────────────────────

    #[test]
    fn s1_valid_manifest_passes() {
        let m = manifest("p1", (1, 0, 0), (2, 0, 0));
        let r = validator().validate(&m, &RuntimeVersion::new(1, 5, 0), &[]);
        assert_eq!(r, Ok(()));
    }

    // ── S-2: duplicate pack_id rejected ──────────────────────────────

    #[test]
    fn s2_duplicate_pack_id_rejected() {
        let m = manifest("p1", (1, 0, 0), (2, 0, 0));
        let r = validator().validate(&m, &RuntimeVersion::new(1, 0, 0), &["p1".to_string()]);
        assert_eq!(r, Err(PackContractError::DuplicatePackId));
    }

    // ── S-3: empty schema_ref rejected ───────────────────────────────

    #[test]
    fn s3_empty_schema_ref_rejected() {
        let mut m = manifest("p1", (1, 0, 0), (2, 0, 0));
        m.schema_ref = String::new();
        let r = validator().validate(&m, &RuntimeVersion::new(1, 0, 0), &[]);
        assert_eq!(r, Err(PackContractError::MissingSchemaRef));
    }

    // ── S-4: incompatible runtime version ────────────────────────────

    #[test]
    fn s4_incompatible_runtime() {
        let m = manifest("p1", (1, 0, 0), (1, 5, 0));
        let r = validator().validate(&m, &RuntimeVersion::new(2, 0, 0), &[]);
        match r {
            Err(PackContractError::IncompatibleRuntime {
                pack_id,
                runtime,
                required,
            }) => {
                assert_eq!(pack_id, "p1");
                assert_eq!(runtime, "2.0.0");
                assert!(required.contains(">=1.0.0"));
                assert!(required.contains("1.5.0"));
            }
            other => panic!("expected IncompatibleRuntime, got {other:?}"),
        }
    }

    // ── S-5: closed-set audit ────────────────────────────────────────

    #[test]
    fn s5_closed_set_audit() {
        assert_eq!(PACK_KIND_VARIANT_COUNT, 3);
        let kinds = [PackKind::Sdd, PackKind::Uat, PackKind::Incident];
        assert_eq!(kinds.len(), PACK_KIND_VARIANT_COUNT);

        assert_eq!(EVIDENCE_FORMAT_VARIANT_COUNT, 4);
        let formats = [
            EvidenceFormat::JsonSchemaDraft07,
            EvidenceFormat::CycloneDx15,
            EvidenceFormat::OpenTelemetryProto1,
            EvidenceFormat::Custom {
                label: "x".to_string(),
                schema_ref: "y".to_string(),
            },
        ];
        assert_eq!(formats.len(), EVIDENCE_FORMAT_VARIANT_COUNT);
    }

    // ── S-6: determinism ──────────────────────────────────────────────

    #[test]
    fn s6_deterministic_output() {
        let m = manifest("p1", (1, 0, 0), (2, 0, 0));
        let v = RuntimeVersion::new(1, 5, 0);
        let r1 = validator().validate(&m, &v, &[]);
        let r2 = validator().validate(&m, &v, &[]);
        assert_eq!(r1, r2);
    }

    // ── Bonus: RuntimeVersion parse + Display + ord ───────────────────

    #[test]
    fn s7_runtime_version_parse_display_ord() {
        let v = RuntimeVersion::parse("1.2.3").expect("parse");
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
        assert_eq!(v.to_string(), "1.2.3");
        let v2 = RuntimeVersion::new(1, 2, 4);
        assert!(v2 > v);
        assert!(RuntimeVersion::parse("1.2").is_none());
        assert!(RuntimeVersion::parse("a.b.c").is_none());
    }

    // ── Bonus: Custom evidence format with empty label rejected ─────

    #[test]
    fn s8_custom_format_empty_label_rejected() {
        let mut m = manifest("p1", (1, 0, 0), (2, 0, 0));
        m.evidence_format = EvidenceFormat::Custom {
            label: String::new(),
            schema_ref: "ref://x".to_string(),
        };
        let r = validator().validate(&m, &RuntimeVersion::new(1, 0, 0), &[]);
        assert_eq!(r, Err(PackContractError::InvalidEvidenceFormat));
    }
}
