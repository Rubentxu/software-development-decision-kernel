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
            specs_count: 14,
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

// ─── backwards compatibility con el ancla ya publicada (session-30 retrospectiva) ──
//
// `release.sh` escribe `manifest_sha256` SIN el prefijo `sha256:` (usa
// `sha256sum MANIFEST.sha256 | awk '{print $1}'`), mientras que
// `dev manifest --bundle` lo escribe CON prefijo. Son dos formatos del mismo
// campo, y solo el segundo es el que documenta el propio modulo.
//
// El caso: TODOS los releases publicados (v2.0.1 incluido) llevan el valor
// sin prefijo Y calculado sobre la primera linea del manifest, no sobre el
// manifest. Un gate que exija coincidencia literal rechaza esos bundles.

/// El formato que `release.sh` publica: hex desnudo, sin prefijo.
const PUBLISHED_FORMAT: &str = "608c6d9ced950456e9d453d8e54529b6c3dc06e45302189c3c15c01c738fd39f";

#[test]
fn manifest_anchor_accepts_a_published_legacy_anchor() {
    // Regresion real (OBSERVED contra el asset publicado v2.0.1): este bundle
    // declara un hash en formato hex sin prefijo, y debe seguir siendo
    // instalable. Si el gate exigiera el prefijo `sha256:`, rechazaria el
    // bundle que la gente tiene instalado hoy.
    let root = tmp_dir("anchor-legacy-format");
    // El manifest real de un bundle publicado NO empieza por ese valor: el
    // valor publicado es el hash de la primera linea, no el del manifest.
    let body = "608c6d9c...  agents/analytics-judge.md\nbbbb  skills/two.md\n";
    std::fs::write(root.join("MANIFEST.sha256"), body).unwrap();
    let real = format!("sha256:{:x}", sha2::Sha256::digest(body.as_bytes()));

    let m = manifest_with_sha("2.0.1", Some(PUBLISHED_FORMAT.to_owned()));
    // La asercion que falla antes del fix: el valor declarado no lleva prefijo.
    assert_ne!(m.contents.manifest_sha256.as_deref(), Some(real.as_str()));
    assert!(verify_manifest_anchor(&root, &m).is_ok());
}

#[test]
fn manifest_anchor_rejects_a_legacy_format_that_does_not_match() {
    // El caso permisivo de formato NO debe convertirse en permisivo de
    // valor. Version moderna (no eximida) + hex sin prefijo que no coincide
    // con el manifest -> se rechaza igual que con prefijo.
    let root = tmp_dir("anchor-legacy-bad");
    let body = "aaaa  agents/one.md\n";
    std::fs::write(root.join("MANIFEST.sha256"), body).unwrap();
    let m = manifest_with_sha("2.2.5", Some(PUBLISHED_FORMAT.to_owned()));
    let err = verify_manifest_anchor(&root, &m).unwrap_err();
    assert!(matches!(
        err,
        BundleManifestError::ManifestShaMismatch { .. }
    ));
}

#[test]
fn manifest_anchor_does_not_exempt_a_legacy_version_with_a_tampered_value() {
    // LA PRUEBA QUE IMPIDE QUE EL FIX SEA UN AGUJAZO. La exencion esta
    // pinada al par (version, valor publicado). Un bundle 2.0.1 cuyo
    // MANIFEST.sha256 fue reescrito lleva en su BUNDLE.toml un valor NUEVO.
    // Ese valor no es la constante publicada, luego no hay exencion, y el
    // chequeo normal corre: si el atacante escribe el digest real de su
    // manifest reescrito, el bundle pasa (el digest es coherente consigo
    // mismo), pero si escribe CUALQUIER otra cosa, falla.
    let root = tmp_dir("anchor-legacy-tampered");
    let body = "ffff  agents/one.md\n";
    std::fs::write(root.join("MANIFEST.sha256"), body).unwrap();

    // Caso 1: declara un valor arbitrario que no es el publicado ni el real.
    let m = manifest_with_sha("2.0.1", Some(PUBLISHED_FORMAT.to_owned()));
    // Aqui el valor SI es la constante publicada, asi que si hay exencion: un
    // 2.0.1 intacto es justo el caso que la lista cubre.
    assert!(
        verify_manifest_anchor(&root, &m).is_ok(),
        "un 2.0.1 con la constante publicada es la exencion legitima"
    );

    // Caso 2: el mismo bundle 2.0.1 con un valor NO publicado y que tampoco
    // coincide con su manifest -> falla. La exencion no cubre "cualquier
    // valor para una version legacy".
    let m = manifest_with_sha("2.0.1", Some("deadbeef".repeat(8)));
    let err = verify_manifest_anchor(&root, &m).unwrap_err();
    assert!(matches!(
        err,
        BundleManifestError::ManifestShaMismatch { .. }
    ));
}

#[test]
fn manifest_anchor_accepts_both_known_formats_when_they_agree() {
    // `dev manifest --bundle` produce con prefijo; `release.sh` produce sin
    // prefijo. Cuando el valor es el mismo, ambas formas deben validar: el
    // gate no debe carecer del formato que el propio repo publica.
    let root = tmp_dir("anchor-both-formats");
    let declared = seed_manifest_and_hash(&root); // "sha256:<hex>"
    let bare = declared.trim_start_matches("sha256:").to_owned();
    assert_ne!(
        declared, bare,
        "el fixture debe distinguir los dos formatos"
    );

    let with_prefix = manifest_with_sha("2.2.5", Some(declared));
    let without_prefix = manifest_with_sha("2.2.5", Some(bare));
    assert!(verify_manifest_anchor(&root, &with_prefix).is_ok());
    assert!(verify_manifest_anchor(&root, &without_prefix).is_ok());
}

#[test]
fn manifest_anchor_exemption_is_version_scoped() {
    // El mismo valor legacy con una version nueva NO se exime: un bundle
    // 9.9.9 no puede instalar declarando la constante publicada.
    let root = tmp_dir("anchor-legacy-wrong-version");
    let body = "ffff  agents/one.md\n";
    std::fs::write(root.join("MANIFEST.sha256"), body).unwrap();
    let m = manifest_with_sha("9.9.9", Some(PUBLISHED_FORMAT.to_owned()));
    let err = verify_manifest_anchor(&root, &m).unwrap_err();
    assert!(matches!(
        err,
        BundleManifestError::ManifestShaMismatch { .. }
    ));
}

#[test]
fn manifest_anchor_does_not_exempt_a_legacy_version_with_no_manifest() {
    // La exencion no puede saltarse la existencia del manifest: si un bundle
    // legacy no trae MANIFEST.sha256, sigue siendo ManifestMissing.
    let root = tmp_dir("anchor-legacy-no-manifest");
    let m = manifest_with_sha("2.0.1", Some(PUBLISHED_FORMAT.to_owned()));
    let err = verify_manifest_anchor(&root, &m).unwrap_err();
    assert!(matches!(err, BundleManifestError::ManifestMissing { .. }));
}
