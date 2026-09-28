use super::super::bundle_manifest::{
    BundleManifest, BundleManifestError, ContentsSection, parse_bundle_manifest,
    verify_bundle_compat, verify_manifest_anchor, write_bundle_manifest,
};
use sha2::Digest;
use std::path::PathBuf;

fn tmp_dir(name: &str) -> PathBuf {
    let base = std::env::temp_dir();
    let path = base.join(format!(
        "sddk-bundle-manifest-test-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn write_then_parse_round_trips() {
    let root = tmp_dir("roundtrip");
    let path = write_bundle_manifest(
        &root,
        "1.62.0",
        "1.62.0",
        "1.62.0",
        ContentsSection {
            agents_count: 142,
            skills_count: 87,
            prompts_count: 18,
            assets_count: 5,
            manifest_sha256: Some("sha256:abc".to_owned()),
        },
    )
    .unwrap();
    assert!(path.is_file());

    let parsed = parse_bundle_manifest(&path).unwrap();
    assert_eq!(parsed.bundle.version, "1.62.0");
    assert_eq!(parsed.bundle.binary_min_version, "1.62.0");
    assert_eq!(parsed.bundle.binary_max_version, "1.62.0");
    assert_eq!(parsed.contents.agents_count, 142);
    assert_eq!(parsed.contents.skills_count, 87);
}

#[test]
fn verify_compat_exact_match() {
    let m = BundleManifest {
        bundle: super::super::bundle_manifest::BundleSection {
            schema_version: 2,
            version: "1.62.0".to_owned(),
            binary_min_version: "1.62.0".to_owned(),
            binary_max_version: "1.62.0".to_owned(),
        },
        contents: ContentsSection::default(),
    };
    assert!(verify_bundle_compat(&m, "1.62.0").is_ok());
    assert!(verify_bundle_compat(&m, "v1.62.0").is_ok());
}

#[test]
fn verify_compat_rejects_older() {
    let m = BundleManifest {
        bundle: super::super::bundle_manifest::BundleSection {
            schema_version: 2,
            version: "1.62.0".to_owned(),
            binary_min_version: "1.62.0".to_owned(),
            binary_max_version: "1.62.0".to_owned(),
        },
        contents: ContentsSection::default(),
    };
    let err = verify_bundle_compat(&m, "1.61.0").unwrap_err();
    match err {
        BundleManifestError::IncompatibleBinary { .. } => {}
        other => panic!("expected IncompatibleBinary, got {other:?}"),
    }
}

#[test]
fn verify_compat_accepts_range() {
    let m = BundleManifest {
        bundle: super::super::bundle_manifest::BundleSection {
            schema_version: 2,
            version: "1.62.0".to_owned(),
            binary_min_version: "1.61.0".to_owned(),
            binary_max_version: "1.63.5".to_owned(),
        },
        contents: ContentsSection::default(),
    };
    assert!(verify_bundle_compat(&m, "1.61.0").is_ok());
    assert!(verify_bundle_compat(&m, "1.62.0").is_ok());
    assert!(verify_bundle_compat(&m, "1.63.5").is_ok());
    assert!(verify_bundle_compat(&m, "1.60.0").is_err());
    assert!(verify_bundle_compat(&m, "1.64.0").is_err());
}

#[test]
fn parse_missing_file_errors() {
    let path = std::path::Path::new("/nonexistent/BUNDLE.toml");
    let err = parse_bundle_manifest(path).unwrap_err();
    assert!(matches!(err, BundleManifestError::NotFound(_)));
}

#[test]
fn parse_unsupported_schema_rejected() {
    let root = tmp_dir("schema");
    let manifest = "[bundle]\nschema_version = 999\nversion = \"1.62.0\"\nbinary_min_version = \"1.62.0\"\nbinary_max_version = \"1.62.0\"\n";
    let path = root.join("BUNDLE.toml");
    std::fs::write(&path, manifest).unwrap();
    let err = parse_bundle_manifest(&path).unwrap_err();
    assert!(matches!(err, BundleManifestError::UnsupportedSchema { .. }));
}

// ─── manifest anchor (INC-DEBT-025 part 2, session-30) ───────────────────────
//
// The `manifest_sha256` field was written and parsed but never compared, so the
// anchor it declares was inert. These tests pin the three real cases plus the
// one deliberately permissive case (no declaration = no claim = no rejection).

/// Writes a MANIFEST.sha256 whose content is deterministic, and returns the
/// `sha256:<hex>` string that a correct BUNDLE.toml must declare for it.
fn seed_manifest_and_hash(root: &std::path::Path) -> String {
    let body = "aaaa  agents/one.md\nbbbb  skills/two.md\n";
    std::fs::write(root.join("MANIFEST.sha256"), body).unwrap();
    format!("sha256:{:x}", sha2::Sha256::digest(body.as_bytes()))
}

fn manifest_with_sha(version: &str, sha: Option<String>) -> BundleManifest {
    BundleManifest {
        bundle: super::super::bundle_manifest::BundleSection {
            schema_version: super::super::bundle_manifest::BUNDLE_MANIFEST_SCHEMA,
            version: version.to_owned(),
            binary_min_version: version.to_owned(),
            binary_max_version: version.to_owned(),
        },
        contents: ContentsSection {
            manifest_sha256: sha,
            ..Default::default()
        },
    }
}

#[test]
fn manifest_anchor_accepts_a_matching_hash() {
    let root = tmp_dir("anchor-match");
    let declared = seed_manifest_and_hash(&root);
    let m = manifest_with_sha("2.2.4", Some(declared.clone()));
    assert!(
        verify_manifest_anchor(&root, &m).is_ok(),
        "a bundle whose declared hash matches its manifest must validate"
    );
}

#[test]
fn manifest_anchor_rejects_a_rewritten_manifest() {
    // The whole point: rewriting MANIFEST.sha256 wholesale used to leave the
    // bundle validating, because nothing compared the two.
    let root = tmp_dir("anchor-mismatch");
    let declared = seed_manifest_and_hash(&root);

    // Attacker (or a bad publish) rewrites the manifest to match a changed file.
    std::fs::write(
        root.join("MANIFEST.sha256"),
        "cccc  agents/one.md\nevil  skills/two.md\n",
    )
    .unwrap();

    let m = manifest_with_sha("2.2.4", Some(declared));
    let err = verify_manifest_anchor(&root, &m).unwrap_err();
    match err {
        BundleManifestError::ManifestShaMismatch { declared, actual } => {
            assert!(declared.starts_with("sha256:"));
            assert!(actual.starts_with("sha256:"));
            assert_ne!(declared, actual, "the two hashes must actually differ");
        }
        other => panic!("expected ManifestShaMismatch, got {other:?}"),
    }
}

#[test]
fn manifest_anchor_rejects_a_declared_hash_with_no_manifest() {
    let root = tmp_dir("anchor-missing");
    // No MANIFEST.sha256 written at all.
    let m = manifest_with_sha("2.2.4", Some("sha256:deadbeef".to_owned()));
    let err = verify_manifest_anchor(&root, &m).unwrap_err();
    assert!(matches!(err, BundleManifestError::ManifestMissing { .. }));
}

#[test]
fn manifest_anchor_is_permissive_when_no_hash_is_declared() {
    // Backwards compatibility: bundles published before the anchor existed must
    // stay installable, so an absent declaration is not a claim and not a
    // rejection.
    let root = tmp_dir("anchor-absent");
    let m = manifest_with_sha("2.0.1", None);
    assert!(verify_manifest_anchor(&root, &m).is_ok());
}

#[test]
fn manifest_anchor_treats_a_blank_declaration_as_absent() {
    let root = tmp_dir("anchor-blank");
    seed_manifest_and_hash(&root);
    let m = manifest_with_sha("2.0.1", Some("   ".to_owned()));
    assert!(verify_manifest_anchor(&root, &m).is_ok());
}
