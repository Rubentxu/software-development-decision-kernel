use super::super::bundle_manifest::{
    BundleManifest, BundleManifestError, ContentsSection, parse_bundle_manifest,
    verify_bundle_compat, write_bundle_manifest,
};
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
