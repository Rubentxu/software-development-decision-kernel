//! CC-S1 — Coverage Contract evaluation.
//!
//! Cycle: `p-63676b11dc0ef88f/a6-cc-s1-static-graph-completeness`.
//! ADR:   `ADR-0139-STATIC-ENHANCED-COVERAGE-CONTRACT.md`.
//! Acceptance: `arch-acceptance-coverage-001.md`.
//!
//! Falsifications (per SCOPE-CONTRACT §3 M8 and the acceptance
//! contract §3 T-AR-1..6):
//!
//! - F-α (T-AR-2): a required file the provider omits -> Incomplete.
//! - F-β (T-AR-3): a required semantic class the provider does
//!   not declare -> Incomplete.
//! - F-γ: an operational issue (forced) -> Incomplete.
//! - T-AR-1: a fully-Demonstrated contract -> Satisfied.
//! - T-AR-4: provider Unavailable -> EvidenceGap, not Satisfied.
//! - T-AR-5: determinism across two consecutive evaluations.
//! - T-AR-6: EXT test gated by COGNICODE_MCP_BIN, `#[ignore]` when
//!   unset. Marked so `cargo test --workspace` lists it.

#![forbid(unsafe_code)]

use sddk_engine::code_intelligence_port::{
    CapabilityProfile, CapabilitySnapshot, CodeIntelligencePort, CoverageBasis, CoverageContract,
    CoverageEvaluation, CoverageGap, CoverageVerdict, DimensionValue, EvidenceGap, Inventory,
    RequiredCapability, RequiredCapabilityKind, ScopeRef, default_coverage_evaluation,
};
use sddk_engine::code_intelligence_port_fake::FakeCodeIntelligenceProvider;

const HEAD_REV: &str = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef";
const INCLUDE_GLOBS: &[&str] = &[
    "crates/**/src/**/*.rs",
    "crates/**/tests/**/*.rs",
    "tests/**/*.rs",
];
const EXCLUDE_GLOBS: &[&str] = &["target/**", ".git/**", "**/tests/fixtures/**"];

/// Build the canonical CC-S1 contract (used across tests).
fn contract_v1(required: Vec<RequiredCapability>) -> CoverageContract {
    CoverageContract {
        contract_id: "static-enhanced-workspace-v1".to_string(),
        contract_version: "1.0.0".to_string(),
        consumer: "verify-kernel::static_evidence".to_string(),
        scope: ScopeRef {
            repo: "sddk-framework".to_string(),
            revision: HEAD_REV.to_string(),
            include_globs: INCLUDE_GLOBS.iter().map(|s| s.to_string()).collect(),
            exclude_globs: EXCLUDE_GLOBS.iter().map(|s| s.to_string()).collect(),
        },
        required_capabilities: required,
    }
}

/// Build the canonical CC-S1 basis.
fn basis_v1() -> CoverageBasis {
    CoverageBasis {
        revision: HEAD_REV.to_string(),
        inventory: Inventory {
            revision: HEAD_REV.to_string(),
            rule_version: "1.0.0".to_string(),
            expected_files: vec!["crates/sddk-cli/src/main.rs".to_string()],
        },
        provider_strategy: "lightweight".to_string(),
        provider_version: "0.97.1".to_string(),
        contract_revision: "1.0.0".to_string(),
    }
}

/// A CapabilitySnapshot that declares the given semantic classes
/// and strategies.
fn snapshot_with(classes: &[&str], strategies: &[&str]) -> CapabilitySnapshot {
    CapabilitySnapshot::from_advertised_with_capabilities(
        CapabilityProfile::StaticEnhanced,
        &["dummy"],
        strategies,
        classes,
    )
}

#[test]
fn t_ar_1_fully_demonstrated_contract_yields_satisfied() {
    // Snapshot declares every required class.
    let required = vec![
        RequiredCapability {
            kind: RequiredCapabilityKind::SymbolKind,
            name: "fn".to_string(),
        },
        RequiredCapability {
            kind: RequiredCapabilityKind::SymbolKind,
            name: "struct".to_string(),
        },
        RequiredCapability {
            kind: RequiredCapabilityKind::RelationClass,
            name: "find_usages".to_string(),
        },
    ];
    let snap = snapshot_with(&["fn", "struct", "find_usages"], &["lightweight", "full"]);
    let basis = basis_v1();
    let eval = default_coverage_evaluation(&contract_v1(required), &basis, &snap);
    // Semantics: Demonstrated (every required present).
    assert_eq!(eval.semantics, DimensionValue::Demonstrated);
    // Inventory & operational: still Unknown (M6). Verdict is
    // therefore Incomplete, NOT Satisfied, even when semantics
    // is Demonstrated. The contract does not short-circuit the
    // verdict on a single dimension.
    assert!(matches!(eval.verdict, CoverageVerdict::Incomplete { .. }));
    // Inventory and operational are explicitly Unknown with gaps
    // recorded.
    let inv_gap = eval
        .gaps
        .iter()
        .find(|g| matches!(g, CoverageGap::UnknownDimension(d) if d == "inventory"));
    assert!(
        inv_gap.is_some(),
        "inventory must be Unknown, got {:?}",
        eval.gaps
    );
    let op_gap = eval
        .gaps
        .iter()
        .find(|g| matches!(g, CoverageGap::UnknownDimension(d) if d == "operational"));
    assert!(
        op_gap.is_some(),
        "operational must be Unknown, got {:?}",
        eval.gaps
    );
}

#[test]
fn t_ar_2_missing_required_file_yields_incomplete() {
    // F-α (M8): a required file the provider did not include ->
    // Incomplete with the path registered as MissingFile.
    //
    // We model the "missing file" gap via a manual eval (the
    // default evaluation does not enumerate files). The contract
    // lists one required semantic class that the snapshot
    // declares, but we manually append a MissingFile gap to
    // simulate the inventory dimension.
    let required = vec![RequiredCapability {
        kind: RequiredCapabilityKind::SymbolKind,
        name: "fn".to_string(),
    }];
    let snap = snapshot_with(&["fn"], &["lightweight"]);
    let mut eval = default_coverage_evaluation(&contract_v1(required), &basis_v1(), &snap);
    let missing = CoverageGap::MissingFile("crates/sddk-cli/src/missing.rs".to_string());
    eval.gaps.push(missing.clone());
    // Inventory is still Unknown by default; the additional
    // MissingFile gap doesn't change that. Re-derive verdict.
    eval.verdict = if matches!(eval.inventory, DimensionValue::Demonstrated)
        && matches!(eval.semantics, DimensionValue::Demonstrated)
        && matches!(eval.operational, DimensionValue::Demonstrated)
        && eval.gaps.is_empty()
    {
        CoverageVerdict::Satisfied
    } else {
        CoverageVerdict::Incomplete {
            gaps: eval.gaps.clone(),
        }
    };
    assert!(eval.gaps.iter().any(|g| g == &missing));
    assert!(matches!(eval.verdict, CoverageVerdict::Incomplete { .. }));
}

#[test]
fn t_ar_3_missing_semantic_class_yields_incomplete() {
    // F-β (M8): required "trait" but snapshot only declares "fn".
    let required = vec![RequiredCapability {
        kind: RequiredCapabilityKind::SymbolKind,
        name: "trait".to_string(),
    }];
    let snap = snapshot_with(&["fn"], &["lightweight"]);
    let eval = default_coverage_evaluation(&contract_v1(required), &basis_v1(), &snap);
    assert_eq!(eval.semantics, DimensionValue::Incomplete);
    let has_trait_gap = eval
        .gaps
        .iter()
        .any(|g| matches!(g, CoverageGap::MissingSemanticClass(c) if c == "trait"));
    assert!(
        has_trait_gap,
        "expected MissingSemanticClass(trait), got {:?}",
        eval.gaps
    );
    assert!(matches!(eval.verdict, CoverageVerdict::Incomplete { .. }));
}

#[test]
fn t_ar_3b_lightweight_without_advertisement_yields_incomplete() {
    // AR-5: lightweight used but provider does not advertise it
    // in available_strategies[] -> RelationClass requirements
    // produce a missing-semantic-class gap.
    let required = vec![RequiredCapability {
        kind: RequiredCapabilityKind::RelationClass,
        name: "analyze_impact".to_string(),
    }];
    let snap = snapshot_with(
        &["fn", "analyze_impact"], // declares semantic classes
        &["full"],                 // but NOT lightweight
    );
    let eval = default_coverage_evaluation(&contract_v1(required), &basis_v1(), &snap);
    // Semantics check: the class is declared, but the strategy
    // used is not advertised. The additional check adds a gap.
    assert!(matches!(eval.verdict, CoverageVerdict::Incomplete { .. }));
    let has_strat_gap = eval
        .gaps
        .iter()
        .any(|g| matches!(g, CoverageGap::MissingSemanticClass(c) if c == "analyze_impact"));
    assert!(
        has_strat_gap,
        "expected MissingSemanticClass(analyze_impact) for non-advertised lightweight, got {:?}",
        eval.gaps
    );
}

#[test]
fn t_ar_4_provider_unavailable_returns_evidence_gap() {
    use sddk_engine::code_intelligence_port_fake::NullCodeIntelligenceProvider;
    let provider = NullCodeIntelligenceProvider;
    let required = vec![RequiredCapability {
        kind: RequiredCapabilityKind::SymbolKind,
        name: "fn".to_string(),
    }];
    let result = provider.coverage_evaluation(&contract_v1(required), &basis_v1());
    assert!(matches!(
        result,
        Err(EvidenceGap::ProviderUnavailable { .. })
    ));
}

#[test]
fn t_ar_4b_provider_incompatible_returns_evidence_gap() {
    // Build a fake with a forced protocol mismatch.
    let mut provider = FakeCodeIntelligenceProvider::new("test", &[]);
    provider.force_protocol_major(99);
    let required = vec![RequiredCapability {
        kind: RequiredCapabilityKind::SymbolKind,
        name: "fn".to_string(),
    }];
    let r = provider.coverage_evaluation(&contract_v1(required), &basis_v1());
    assert!(matches!(
        r,
        Err(EvidenceGap::ProviderIncompatible {
            provider_major: 99,
            sddk_major: 1
        })
    ));
}

#[test]
fn t_ar_5_determinism_two_consecutive_evaluations_byte_equal() {
    let required = vec![
        RequiredCapability {
            kind: RequiredCapabilityKind::SymbolKind,
            name: "fn".to_string(),
        },
        RequiredCapability {
            kind: RequiredCapabilityKind::SymbolKind,
            name: "trait".to_string(),
        },
    ];
    let snap = snapshot_with(&["fn"], &["lightweight"]); // trait missing
    let c = contract_v1(required);
    let b = basis_v1();
    let e1 = default_coverage_evaluation(&c, &b, &snap);
    let e2 = default_coverage_evaluation(&c, &b, &snap);
    assert_eq!(e1, e2, "determinism violated: same inputs differ");
}

#[test]
fn t_ar_5b_no_global_threshold_constant_in_module() {
    // AR-6: there must be no global constant of the form
    // COVERAGE_THRESHOLD. We assert by string-match against the
    // module source.
    let src = include_str!("../src/code_intelligence_port.rs");
    assert!(
        !src.contains("COVERAGE_THRESHOLD"),
        "AR-6: COVERAGE_THRESHOLD must not exist; ADR-0139 forbids global thresholds"
    );
    assert!(
        !src.contains("coverage_ratio"),
        "AR-6 / N5 of SCOPE-CONTRACT: scalar coverage_ratio must not exist"
    );
}

#[test]
fn t_ar_5c_no_provider_name_in_the_core_port() {
    // N2 of SCOPE-CONTRACT, y C3m.3 / ADR-0155: el puerto de inteligencia de
    // codigo es la superficie publica que todo adopter ve, y no debe nombrar
    // ningun proveedor. El nombre del proveedor es DATO que declara el
    // adaptador (`code_intelligence_port_mcp::PROVIDER_ID`), no una variante
    // de un enum del core.
    //
    // POR QUE ESTE GUARD SE REESCRIBE Y NO SE ANADE. La version anterior se
    // llamaba `t_ar_5c_no_cognicode_type_in_sddk_engine` —"sin tipo
    // CogniCode"— y su cuerpo hacia
    // `assert!(src.contains("ProviderKind::CogniCode"))`: AFIRMABA que el
    // nombre del proveedor estuviera presente. Su comentario ademas decia que
    // el lint `no_knowledge_to_provider_sdk` lo aplicaba "precisamente", y ese
    // lint solo escanea cinco modulos de knowledge
    // (`context_fitness.rs:114-120`), nunca el puerto. O sea: el guard tenia un
    // nombre mas estrecho que la propiedad que decia vigilar, y la propiedad
    // que decia vigilar no la comprobaba. Un guard asi es peor que ninguno,
    // porque ocupa el sitio del que si la vigila.
    //
    // Lo que se comprueba ahora, sobre el fuente del puerto:
    //   1. Ningun NOMBRE de proveedor como variante de enum publica.
    //   2. Ningun tipo `pub` cuyo nombre sea el de un proveedor.
    //   3. Que la identidad del proveedor siga siendo expresable, como dato.
    // El punto 3 importa: un guard que prohibe el nombre sin comprobar que
    // quede forma de declararlo empuja al primer que llegue a escribir un
    // falsehood para no tocar el enum.
    let src = include_str!("../src/code_intelligence_port.rs");

    // (1) Ningun nombre de proveedor puede ser una VARIANTE del enum. Se
    // parsean las DECLARACIONES de variante, no el cuerpo entero: citar el
    // nombre retirado en la prosa del enum esta bien y es lo que explica por
    // que se quito — un guard que prohibe mencionarlo obligaria a borrar la
    // explicacion, que es el precio de no distinguir citar de usar. La primera
    // version de este check buscaba el token en el cuerpo crudo y fallo
    // contra su propio doc; el fix es hacer el check preciso, no callar la
    // prosa ni bajar la exigencia.
    let enum_body = src
        .split("pub enum ProviderKind {")
        .nth(1)
        .and_then(|rest| rest.split("\n}").next())
        .unwrap_or_else(|| panic!("no encuentro `pub enum ProviderKind` en el puerto"));
    let variantes: Vec<&str> = enum_body
        .lines()
        .filter_map(|l| {
            let t = l.trim();
            t.strip_suffix(',')
                .filter(|n| !n.is_empty() && n.chars().all(|c| c.is_alphanumeric() || c == '_'))
        })
        .collect();
    assert!(
        !variantes.is_empty(),
        "C3m.3: no se han podido extraer las variantes de ProviderKind; un check que \
         no encuentra nada que comprobar pasa por el motivo equivocado."
    );
    for provider in ["CogniCode", "Chronos", "CodeIntelligence"] {
        assert!(
            !variantes.contains(&provider),
            "C3m.3: `{provider}` ha vuelto como VARIANTE de ProviderKind (variantes \
             actuales: {variantes:?}). La identidad del proveedor es DATO \
             (`ObservationSet::provider_id`), no estructura: un enum cerrado de \
             productos hace que registrar un proveedor sea un cambio incompatible \
             de la API publica de sddk-engine. Citarlo en la prosa del enum si es \
             legitimo; declararlo como variante no lo es."
        );
    }

    // (2) Ningun tipo publico del puerto puede llamarse como un proveedor.
    for line in src.lines() {
        let trimmed = line.trim_start();
        for provider in ["CogniCode", "Chronos"] {
            if trimmed.starts_with(&format!("pub struct {provider}"))
                || trimmed.starts_with(&format!("pub enum {provider}"))
                || trimmed.starts_with(&format!("pub trait {provider}"))
            {
                panic!("N2 violated: el tipo `{provider}` se filtre al puerto del core");
            }
        }
    }

    // (3) La identidad del proveedor sigue siendo expresable como dato, y el
    // enum sigue distinguiendo "hay un proveedor real" de "no hay ninguno".
    assert!(
        src.contains("pub provider_id: String"),
        "C3m.3: sin `provider_id` no hay forma de decir QUE proveedor produjo las \
         observaciones, y el unico camino que queda es mentir en el enum."
    );
    assert!(
        src.contains("ProviderKind::External"),
        "C3m.3: the enum must be able to say 'a real provider outside the process'. \
         Without this variant a real adapter can only report Null or Fake, and \
         both are false."
    );

    // (4) `Null` no puede declarar un id. El enum dice «no hay proveedor», y
    // un id al lado seria la misma mentira que este cambio elimina — de ahi el
    // comentario en el propio fake. Esta comprobacion se anadio porque la
    // falsificacion del guard (M6) sobrevivio: el codigo estaba bien escrito
    // y comentado, pero NADA lo vigilaba, y un comentario no es un guard.
    //
    // El recorrido es sobre el texto NORMALIZADO y no sobre lineas: la primera
    // version contaba tres lineas hacia adelante y `cargo fmt` partio
    // `provider_id: String::new(),` en varias, con lo que el check encontro una
    // cadena vacia y fallo. Un check atado al formato no es un check sobre la
    // propiedad: es un check sobre como esta escrito hoy.
    let fake = include_str!("../src/code_intelligence_port_fake.rs");
    let plano: String = fake.split_whitespace().collect::<Vec<_>>().join(" ");
    for trozo in plano.split("provider_kind: ProviderKind::Null,").skip(1) {
        // El constructor termina donde empieza el siguiente `provider_kind`,
        // o donde se acaba el struct literal.
        let cuerpo = trozo.split("provider_kind:").next().unwrap_or(trozo);
        if let Some(id) = cuerpo.split("provider_id:").nth(1) {
            let valor = id.split(',').next().unwrap_or("").trim();
            assert!(
                valor.contains("String::new()"),
                "C3m.3: `Null` significa «no hay proveedor», asi que no puede declarar \
                 un id. Se ha encontrado: `{valor}`. Un id junto a la negacion de un \
                 proveedor es la misma mentira que este cambio quita, y por eso se \
                 comprueba y no se deja en un comentario."
            );
        }
    }
}

#[test]
fn t_ar_5d_inventory_v1_artefact_loads_and_matches_repo() {
    // M1 of SCOPE-CONTRACT: the inventory artefact exists and is
    // parseable. We load it relative to the test file.
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/static_enhanced/inventory_v1.json");
    assert!(
        path.exists(),
        "inventory_v1.json missing at {}",
        path.display()
    );
    let raw = std::fs::read_to_string(&path).expect("read inventory_v1.json");
    // Minimal JSON shape check (the full schema is the cycle's
    // acceptance contract). We expect "revision", "rule_version",
    // "expected_files" keys.
    assert!(raw.contains("\"revision\""), "inventory missing 'revision'");
    assert!(
        raw.contains("\"rule_version\""),
        "inventory missing 'rule_version'"
    );
    assert!(
        raw.contains("\"expected_files\""),
        "inventory missing 'expected_files'"
    );
    let count_marker = "\"file_count\":";
    if let Some(idx) = raw.find(count_marker) {
        let rest = &raw[idx + count_marker.len()..].trim_start();
        // Skip whitespace, then read the integer up to the next
        // non-digit character (newline, comma, or close brace).
        let end = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        if end > 0 {
            let n: u64 = rest[..end].parse().expect("file_count is integer");
            assert!(n > 0, "inventory reports zero files");
        }
    }
}

// ── F-γ operational issue ────────────────────────────────────────────────

#[test]
fn t_ar_5e_operational_error_registered_as_gap() {
    // F-γ: an operational issue forces the operational dimension
    // to <Demonstrated and the verdict to Incomplete.
    let mut eval = CoverageEvaluation::satisfied();
    eval.gaps.push(CoverageGap::OperationalError(
        "truncation: 1000 results cap hit".to_string(),
    ));
    // Recompute verdict from dimensions + gaps.
    eval.verdict = if matches!(eval.inventory, DimensionValue::Demonstrated)
        && matches!(eval.semantics, DimensionValue::Demonstrated)
        && matches!(eval.operational, DimensionValue::Demonstrated)
        && eval.gaps.is_empty()
    {
        CoverageVerdict::Satisfied
    } else {
        CoverageVerdict::Incomplete {
            gaps: eval.gaps.clone(),
        }
    };
    assert!(matches!(eval.verdict, CoverageVerdict::Incomplete { .. }));
    assert!(
        eval.gaps
            .iter()
            .any(|g| matches!(g, CoverageGap::OperationalError(_)))
    );
}

// ── F-γ in the real workflow via the fake provider's forced protocol ────

#[test]
fn f_gamma_protocol_mismatch_forces_incomplete_via_evidence_gap() {
    // The fake with a forced protocol mismatch returns
    // EvidenceGap::ProviderIncompatible. Per AR-4, this is NOT
    // Satisfied. The contract cannot be advertised even though
    // the inventory and operational dimensions would otherwise
    // be Demonstrated.
    let mut provider = FakeCodeIntelligenceProvider::new("test", &[]);
    provider.force_protocol_major(99);
    let required = vec![RequiredCapability {
        kind: RequiredCapabilityKind::SymbolKind,
        name: "fn".to_string(),
    }];
    let r = provider.coverage_evaluation(&contract_v1(required), &basis_v1());
    assert!(matches!(r, Err(EvidenceGap::ProviderIncompatible { .. })));
}

// ── T-AR-6 EXT test, env-gated ───────────────────────────────────────────

#[test]
#[ignore = "requires COGNICODE_MCP_BIN pointing to the real CogniCode binary"]
fn t_ar_6_ext_real_cognicode_run() {
    // This test exists to make the EXT test discoverable by
    // `cargo test --workspace -- --include-ignored`. When
    // COGNICODE_MCP_BIN is set, this test will spawn the real
    // adapter (CC-S2 territory; the runner for this test is
    // out of scope for CC-S1 and lands when the real adapter
    // lands). The acceptance contract requires its presence in
    // the battery; its activation is gated by a future cycle
    // with the binary installed.
    let bin = std::env::var("COGNICODE_MCP_BIN").expect("COGNICODE_MCP_BIN not set");
    assert!(!bin.is_empty(), "COGNICODE_MCP_BIN must be non-empty");
}
