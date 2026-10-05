//! Autofalsador del reducer de version authority (VA1).
//!
//! # QUE PREGUNTA, Y QUE NO
//!
//! El exit gate pide mutaciones que maten cuatro cosas concretas:
//!
//! 1. `first observation wins`
//! 2. `Invalid -> continue`
//! 3. `Undeclared -> fake default`
//! 4. dos versiones distintas aceptadas como una sola
//!
//! ## Por qué esto es DIFERENCIAL y no una mutación de fuente
//!
//! La técnica habitual —copiar el módulo, cambiar una línea, recompilar—
//! mide cobertura de mutaciones. Aquí se mide algo más fuerte para este
//! caso: **que la tabla de verdad del reducer REAL distingue al mutante del
//! código correcto**. Un mutante que la tabla no distingue es un mutante que
//! la tabla dejaría pasar, y eso es la pregunta.
//!
//! Cada mutante se implementa aqui como una reducer alternativa que recibe las
//! mismas observaciones, y la tabla le pasa la verdad real. Un mutante que
//! sobrevive es un agujero en la tabla, y sale por el mismo camino que un test
//! rojo.
//!
//! ## Lo que NO afirma
//!
//! No afirma cobertura exhaustiva de mutaciones del módulo. Afirma que los
//! cuatro modos de fallo que el exit gate nombra son distinguibles con la tabla
//! que hay. Anyadir un quinto mutante que la tabla no distinga es trabajo que
//! este fichero no evita: lo hace visible.

use sddk_domain::version_authority::{
    reduce, ProductVersion, VersionAuthority, VersionEvidence, VersionObservation, VersionProbe,
};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn v(value: &str) -> ProductVersion {
    ProductVersion::new(value).expect("version valida en el test")
}

fn ev() -> VersionEvidence {
    VersionEvidence {
        source_kind: "fixture".to_owned(),
        digest: Some(format!("sha256:{}", "0".repeat(64))),
        location: None,
    }
}

fn obs(provider: &str, probe: VersionProbe) -> VersionObservation {
    VersionObservation {
        provider_id: provider.to_owned(),
        provider_version: "1.0.0".to_owned(),
        capability: "product-version.observation/v1".to_owned(),
        probe,
    }
}

fn decl(provider: &str, version: &str) -> VersionObservation {
    obs(
        provider,
        VersionProbe::Declared {
            version: v(version),
            evidence: ev(),
        },
    )
}

fn und(provider: &str) -> VersionObservation {
    obs(
        provider,
        VersionProbe::Undeclared {
            reason: "no declara".to_owned(),
        },
    )
}

fn na(provider: &str) -> VersionObservation {
    obs(
        provider,
        VersionProbe::NotApplicable {
            reason: "no es asunto mio".to_owned(),
        },
    )
}

fn bad(provider: &str) -> VersionObservation {
    obs(
        provider,
        VersionProbe::Invalid {
            reason: "existe y no se pudo leer".to_owned(),
        },
    )
}

/// Un caso: nombre, observaciones, y si el reducer real lo distingue del
/// mutante.
struct Case {
    name: &'static str,
    observations: Vec<VersionObservation>,
}

/// La tabla. Cada caso tiene un nombre que dice qué separa del mutante.
fn table() -> Vec<Case> {
    vec![
        Case {
            name: "dos providers declaran lo mismo",
            observations: vec![decl("alpha", "1.0.0"), decl("bravo", "1.0.0")],
        },
        Case {
            name: "dos providers declaran distinto",
            observations: vec![decl("alpha", "1.0.0"), decl("bravo", "2.0.0")],
        },
        Case {
            name: "uno declara y otro no declara",
            observations: vec![decl("alpha", "1.0.0"), und("bravo")],
        },
        Case {
            name: "uno declara y otro no aplica",
            observations: vec![decl("alpha", "1.0.0"), na("bravo")],
        },
        Case {
            name: "uno declara y otro es invalido",
            observations: vec![decl("alpha", "1.0.0"), bad("bravo")],
        },
        Case {
            name: "solo invalidos",
            observations: vec![bad("alpha"), bad("bravo")],
        },
        Case {
            name: "nadie declara",
            observations: vec![und("alpha"), na("bravo")],
        },
        Case {
            name: "el mismo provider dos veces",
            observations: vec![decl("alpha", "1.0.0"), decl("alpha", "1.0.0")],
        },
    ]
}

/// Corre el reducer real y el mutante sobre TODA la tabla, y exige que el
/// mutante sea **distinguible en al menos un caso**.
///
/// ## La asercion va al reves de la habitual, y el error que casi se cuela
///
/// Matar un mutante significa que la tabla lo RECHAZA, no que lo rechace en
/// todas las filas. Una primera version de esta funcion exigia que real y
/// mutante difirieran en CADA caso, y con eso los seis mutantes
/// "sobrevivieron": la funcion estaba reportando un defecto del reducer
/// cuando lo roto era el instrumento. La version correcta pide un unico caso
/// que los distinga, y nombra cual fue —un falsador que no dice por que caso
/// mata a un mutante obliga a volver a la tabla a averiguarlo—.
fn assert_kills(name: &str, mutant: fn(Vec<VersionObservation>) -> VersionAuthority) {
    let mut distinguishing: Vec<&str> = Vec::new();
    let mut identical: Vec<&str> = Vec::new();
    for case in table() {
        let real = reduce(case.observations.clone());
        let mutated = mutant(case.observations.clone());
        if real == mutated {
            identical.push(case.name);
        } else {
            distinguishing.push(case.name);
        }
    }
    assert!(
        !distinguishing.is_empty(),
        "el mutante `{name}` SOBREVIVE: la tabla no lo distingue en NINGUN caso. \
         Coinciden en {identical:?}. O la tabla se queda corta, o el reducer real \
         comparte el defecto."
    );
    println!("  `{name}` muere en: {distinguishing:?}");
}

// ---------------------------------------------------------------------------
// M1 — first observation wins
// ---------------------------------------------------------------------------

fn first_wins(mut observations: Vec<VersionObservation>) -> VersionAuthority {
    observations.sort_by(|a, b| a.provider_id.cmp(&b.provider_id));
    let winner = observations
        .iter()
        .find(|o| o.probe.declares())
        .or_else(|| observations.iter().find(|o| matches!(o.probe, VersionProbe::Invalid { .. })))
        .or_else(|| {
            observations
                .iter()
                .find(|o| matches!(o.probe, VersionProbe::Undeclared { .. }))
        });
    match winner.map(|o| &o.probe) {
        Some(VersionProbe::Declared { version, .. }) => VersionAuthority::Resolved {
            version: version.clone(),
            observations,
        },
        Some(VersionProbe::Invalid { reason }) => VersionAuthority::Invalid {
            failures: vec![("elegido".to_owned(), reason.clone())],
            observations,
        },
        _ => VersionAuthority::Unresolved { observations },
    }
}

#[test]
fn m1_first_observation_wins_murió() {
    assert_kills("first-wins", first_wins);
}

// ---------------------------------------------------------------------------
// M2 — Invalid se salta y se sigue
// ---------------------------------------------------------------------------

fn invalid_continues(observations: Vec<VersionObservation>) -> VersionAuthority {
    let usable: Vec<VersionObservation> = observations
        .iter()
        .filter(|o| !o.probe.is_fatal())
        .cloned()
        .collect();
    if usable.is_empty() {
        return VersionAuthority::Unresolved { observations };
    }
    reduce(usable)
}

#[test]
fn m2_invalid_continue_murió() {
    assert_kills("invalid-continue", invalid_continues);
}

// ---------------------------------------------------------------------------
// M3 — Undeclared se convierte en un valor inventado
// ---------------------------------------------------------------------------

fn undeclared_invents(observations: Vec<VersionObservation>) -> VersionAuthority {
    let invented = observations
        .into_iter()
        .map(|o| match o.probe {
            VersionProbe::Undeclared { reason } => VersionObservation {
                probe: VersionProbe::Declared {
                    version: v("0.0.0"),
                    evidence: VersionEvidence {
                        source_kind: reason,
                        digest: None,
                        location: None,
                    },
                },
                ..o
            },
            other => VersionObservation { probe: other, ..o },
        })
        .collect();
    reduce(invented)
}

#[test]
fn m3_undeclared_fake_default_murió() {
    assert_kills("undeclared-invents", undeclared_invents);
}

// ---------------------------------------------------------------------------
// M4 — dos versiones distintas fusionadas en una
// ---------------------------------------------------------------------------

fn distinct_collapse(observations: Vec<VersionObservation>) -> VersionAuthority {
    let mut first_version: Option<ProductVersion> = None;
    let mut agreeing: Vec<VersionObservation> = Vec::new();
    for o in &observations {
        if let VersionProbe::Declared { version, .. } = &o.probe {
            match &first_version {
                None => {
                    first_version = Some(version.clone());
                    agreeing.push(o.clone());
                }
                Some(seen) if seen == version => agreeing.push(o.clone()),
                // DIFERENTE: se descarta en silencio, que es el defecto.
                Some(_) => {}
            }
        }
    }
    match first_version {
        Some(version) => VersionAuthority::CrossValidated {
            version,
            observations: agreeing,
        },
        None => VersionAuthority::Unresolved { observations },
    }
}

#[test]
fn m4_distinct_collapsed_into_one_murió() {
    assert_kills("distinct-collapse", distinct_collapse);
}

// ---------------------------------------------------------------------------
// M5 — la identidad del provider como prioridad
// ---------------------------------------------------------------------------

fn identity_is_priority(observations: Vec<VersionObservation>) -> VersionAuthority {
    // Ordena por "autoridad" segun el nombre: un provider cuyo identificador
    // ordena antes gana. Es la prioridad disfrazada de reducer.
    let mut declared: Vec<VersionObservation> = observations
        .iter()
        .filter(|o| o.probe.declares())
        .cloned()
        .collect();
    declared.sort_by(|a, b| b.provider_id.cmp(&a.provider_id));
    match declared.into_iter().next() {
        Some(o) => match o.probe {
            VersionProbe::Declared { version, .. } => VersionAuthority::Resolved {
                version,
                observations: observations.into_iter().collect(),
            },
            _ => VersionAuthority::Unresolved {
                observations: observations.into_iter().collect(),
            },
        },
        None => VersionAuthority::Unresolved {
            observations: observations.into_iter().collect(),
        },
    }
}

#[test]
fn m5_provider_identity_as_priority_murió() {
    assert_kills("identity-priority", identity_is_priority);
}

// ---------------------------------------------------------------------------
// M6 — el mismo provider contado como dos votos
// ---------------------------------------------------------------------------

fn self_corroborates(observations: Vec<VersionObservation>) -> VersionAuthority {
    let mut declared: Vec<VersionObservation> = observations
        .iter()
        .filter(|o| o.probe.declares())
        .cloned()
        .collect();
    declared.sort_by(|a, b| a.provider_id.cmp(&b.provider_id));
    if declared.len() >= 2 {
        let version = match &declared[0].probe {
            VersionProbe::Declared { version, .. } => version.clone(),
            _ => unreachable!("filtrado por declares"),
        };
        VersionAuthority::CrossValidated {
            version,
            observations: declared,
        }
    } else {
        reduce(observations)
    }
}

#[test]
fn m6_self_corroboration_murió() {
    assert_kills("self-corroborates", self_corroborates);
}
