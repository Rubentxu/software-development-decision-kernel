//! Falsador de la relación entre una release ref y una version de producto.
//!
//! # Qué se está midiendo
//!
//! Que la relación entre una referencia y una versión se comprueba por
//! **construcción**, no por recorte. Hay una sola forma de que dos cosas
//! coincidan aquí: la naming construye el nombre que la versión tendría y las
//! dos cadenas se comparan. Ninguna función quita texto de una referencia para
//! sacarle una versión.
//!
//! Los cuatro mutantes de abajo son las cuatro formas en que eso se deshace, y
//! las cuatro son **defectos que se han dado de verdad** en código de
//! producción o en herramientas parecidas:
//!
//! - `M1 contains`: que la versión esté *dentro* del nombre de la referencia.
//!   Coincidiría con `v1.2.3-rc2` y con `release-1.2.3`, y autorizaría una
//!   candidata como si fuera estable.
//! - `M2 strip-suffix`: recortar el sufijo de candidata para que las dos
//!   cadenas coincidan. Es literalmente la coerción que el bloque prohíbe.
//! - `M3 optional-prefix`: que el prefijo declarado sea opcional. Parece
//!   inofensivo y no lo es: es la diferencia entre una convención y una
//!   sugerencia, y una sugerencia es lo que no autoriza nada.
//! - `M4 sequence-ignored`: leer la secuencia del nombre en vez de comparar la
//!   declarada, de modo que la segunda candidata nombre a la primera.
//!
//! Y una quinta, `M5 revision-is-version`, que **no** es un mutante: la ley que
//! protege —una revisión de código no es una versión de producto— se sostiene
//! en las firmas, no en una comparación, y eso se mide mirando el código igual
//! que cualquier otro fitness. Por qué se salió del molde de mutante está
//! escrito en su sección.

use sddk_domain::channel::ReleaseChannel;
use sddk_domain::release_ref::{
    BindOutcome, CandidateSequence, ReleaseRef, SourceRevision, VersionNaming, binds,
};
use sddk_domain::version_authority::ProductVersion;

fn v(value: &str) -> ProductVersion {
    ProductVersion::new(value).expect("version valida en el test")
}

fn stable(name: &str) -> ReleaseRef {
    ReleaseRef::new(name, ReleaseChannel::Stable)
}

fn candidate(name: &str, sequence: CandidateSequence) -> ReleaseRef {
    ReleaseRef::candidate(name, ReleaseChannel::Candidate, sequence)
}

/// Una fila: la pregunta y lo que la ley exige.
struct Case {
    name: &'static str,
    reference: ReleaseRef,
    version: ProductVersion,
    naming: VersionNaming,
    /// Lo que la ley dice que tiene que pasar.
    expected_bound: bool,
}

fn cases() -> Vec<Case> {
    vec![
        // ── El caso que esta regla protege ──
        Case {
            name: "la naming por defecto y su prefijo",
            reference: stable("v1.2.3"),
            version: v("1.2.3"),
            naming: VersionNaming::v_prefixed(),
            expected_bound: true,
        },
        // Una candidata NO nombra a su version con la naming por defecto. Si
        // esta fila.binding == true, es que alguien recorta el sufijo.
        Case {
            name: "una candidata no nombra a la version de su producto",
            reference: candidate("v1.2.3-rc2", CandidateSequence::nth(2).unwrap()),
            version: v("1.2.3"),
            naming: VersionNaming::v_prefixed(),
            expected_bound: false,
        },
        // Y el caso espejo: con una naming que TIENE sitio para la secuencia,
        // si y solo si coincide.
        Case {
            name: "una candidata con naming declarada y la misma secuencia",
            reference: candidate("v1.2.3-rc2", CandidateSequence::nth(2).unwrap()),
            version: v("1.2.3"),
            naming: VersionNaming::PrefixedCandidate {
                prefix: "v".into(),
                separator: "-".into(),
                marker: "rc".into(),
            },
            expected_bound: true,
        },
        Case {
            name: "una candidata cuya secuencia no es la del nombre",
            reference: candidate("v1.2.3-rc2", CandidateSequence::nth(3).unwrap()),
            version: v("1.2.3"),
            naming: VersionNaming::PrefixedCandidate {
                prefix: "v".into(),
                separator: "-".into(),
                marker: "rc".into(),
            },
            expected_bound: false,
        },
        // El prefijo declarado es obligatorio. Antes era opcional de facto.
        Case {
            name: "la naming declara el prefijo y es obligatorio",
            reference: stable("1.2.3"),
            version: v("1.2.3"),
            naming: VersionNaming::v_prefixed(),
            expected_bound: false,
        },
        // Con `Exact` la cosa es exacta en las dos direcciones.
        Case {
            name: "exacta nombra a la version sin prefijo",
            reference: stable("1.2.3"),
            version: v("1.2.3"),
            naming: VersionNaming::Exact,
            expected_bound: true,
        },
        Case {
            name: "exacta NO nombra a la version con prefijo",
            reference: stable("v1.2.3"),
            version: v("1.2.3"),
            naming: VersionNaming::Exact,
            expected_bound: false,
        },
        // Un prefijo declarado mas largo es otro nombre, no una equivalencia.
        Case {
            name: "un prefijo declarado mas largo no es un superprefijo",
            reference: stable("release-1.2.3"),
            version: v("1.2.3"),
            naming: VersionNaming::VPrefixed { prefix: "v".into() },
            expected_bound: false,
        },
        Case {
            name: "y con el prefijo que SI declara, funciona",
            reference: stable("release-1.2.3"),
            version: v("1.2.3"),
            naming: VersionNaming::VPrefixed {
                prefix: "release-".into(),
            },
            expected_bound: true,
        },
        // Y una version distinta no se parece lo suficiente.
        Case {
            name: "una version distinto no es la version",
            reference: stable("v1.2.30"),
            version: v("1.2.3"),
            naming: VersionNaming::v_prefixed(),
            expected_bound: false,
        },
    ]
}

fn bound(reference: &ReleaseRef, version: &ProductVersion, naming: &VersionNaming) -> bool {
    binds(reference, version, naming).is_bound()
}

// ── M1: la versión está dentro del nombre ─────────────────────────────────

/// Que la versión aparezca **dentro** del nombre de la referencia.
///
/// Coincidiría con `v1.2.3-rc2` —porque `1.2.3` está dentro— y con
/// `release-1.2.3`, y con `1.2.3-hotfix`. Es la comprobación más fácil de
/// escribir y la que más rápido convierte una candidata en una release
/// estable.
fn reference_contains_version(reference: &ReleaseRef, version: &ProductVersion) -> bool {
    reference.name().contains(version.as_str())
}

#[test]
fn m1_contains_murió() {
    assert_kills(
        "contains",
        |r, ver, _n| reference_contains_version(r, ver),
        "una candidata no nombra a la version de su producto",
    );
}

// ── M2: recortar el sufijo de candidata ──────────────────────────────────

/// Recortar el sufijo `-rcN` para que las dos cadenas coincidan.
///
/// Es la coerción exacta que el bloque prohíbe, y es la que estaba implícita
/// mientras la comparación era «quitar la `v` y ver si queda lo mismo»: quitar
/// la `v` y quitar `-rc2` son el mismo gesto hecho con dos reglas distintas.
fn strip_candidate_suffix(reference: &ReleaseRef, version: &ProductVersion) -> bool {
    if let Some(sequence) = reference.sequence() {
        let marker = format!("-rc{sequence}");
        return reference.name().trim_end_matches(&marker) == version.as_str();
    }
    reference
        .name()
        .strip_prefix('v')
        .unwrap_or(reference.name())
        == version.as_str()
}

#[test]
fn m2_strip_candidate_suffix_murió() {
    assert_kills(
        "strip-candidate-suffix",
        |r, ver, _n| strip_candidate_suffix(r, ver),
        "una candidata con naming declarada y la misma secuencia",
    );
}

// ── M3: el prefijo declarado es opcional ─────────────────────────────────

/// Que el prefijo declarado se pueda quitar.
///
/// El mismo comportamiento observable que antes de este bloque, y por eso el
/// cambio de gate es real y no cosmético. Con el prefijo opcional, la naming
/// deja de ser una convención y pasa a ser una sugerencia, y una sugerencia no
/// puede rechazar nada.
///
/// ## Por qué la primera versión de este mutante estaba mal
///
/// La primera escribía «el nombre es la versión desnuda, o la versión con `v`»
/// — y por eso reventaba la fila de la candidata, que es la fila de M2. Ese
/// mutante no rompía la optionalidad del prefijo: rompía *todo lo que la
/// naming declara*, y por eso moría antes de llegar a su propia fila. La ley
/// que este mutante tiene que romper es una sola — *quitar el prefijo declarado
/// sigue siendo aceptable* — y para que rompa esa y solo esa, tiene que seguir
/// respetando la secuencia, el marcador y el separador.
fn prefix_is_optional(
    reference: &ReleaseRef,
    version: &ProductVersion,
    naming: &VersionNaming,
) -> bool {
    let Some(declared) = naming.name_for(version, reference.sequence()) else {
        // Sin sitio para candidatas no hay nombre que Compare ni con prefijo ni
        // sin el: la mutacion no crea sitio donde la naming no lo declaro.
        return false;
    };
    if declared == reference.name() {
        return true;
    }
    let prefix = match naming {
        VersionNaming::VPrefixed { prefix } | VersionNaming::PrefixedCandidate { prefix, .. } => {
            prefix.as_str()
        }
        // `Exact` no declara prefijo, luego no hay prefijo que pueda ser
        // opcional. Convertirla en opcional seria inventar una convencion.
        VersionNaming::Exact => return false,
    };
    match declared.strip_prefix(prefix) {
        Some(sin_prefijo) => sin_prefijo == reference.name(),
        None => false,
    }
}

#[test]
fn m3_optional_prefix_murió() {
    assert_kills(
        "optional-prefix",
        prefix_is_optional,
        "la naming declara el prefijo y es obligatorio",
    );
}

// ── M4: la secuencia se ignora ──────────────────────────────────────────

/// Que la secuencia no se compare, y que la segunda candidata nombre a la
/// primera.
///
/// Con `v1.2.3-rc2` y una secuencia declarada de 3, la respuesta honesta es
/// que no nombra a esa version. Aqui la secuencia sale del propio nombre, que
/// es la unica fuente que el nucleo no deberia leer: la entrada declarada
/// existe precisamente para que el nombre no tenga que decirla.
///
/// ## Por qué la primera versión de este mutante estaba mal
///
/// La primera construia `v{version}-rc2` siempre, con lo cual reventaba la
/// primera fila —un release estable, donde la ley dice que si nombra— antes de
/// llegar a la fila de la secuencia. Volvia a romper dos leyes en vez de una.
/// Este lee la secuencia del texto, que es romper *solo* la de comparar la
/// secuencia declarada.
fn sequence_ignored(
    reference: &ReleaseRef,
    version: &ProductVersion,
    naming: &VersionNaming,
) -> bool {
    match naming.name_for(version, sequence_in_text(reference, naming)) {
        Some(expected) => expected == reference.name(),
        None => false,
    }
}

/// La secuencia que el nombre declara, leida del texto.
///
/// Unico uso: el mutante de arriba. El codigo de produccion no tiene esta
/// funcion y no deberia tenerla — una referencia guarda su secuencia porque se
/// la dieron, y releerla del nombre seria devolver el problema.
fn sequence_in_text(reference: &ReleaseRef, naming: &VersionNaming) -> Option<CandidateSequence> {
    let (separator, marker) = match naming {
        VersionNaming::PrefixedCandidate {
            separator, marker, ..
        } => (separator, marker),
        _ => return None,
    };
    let (_, cola) = reference.name().rsplit_once(separator.as_str())?;
    let digitos = cola.strip_prefix(marker.as_str())?;
    CandidateSequence::nth(digitos.parse().ok()?)
}

#[test]
fn m4_sequence_ignored_murió() {
    assert_kills(
        "sequence-ignored",
        sequence_ignored,
        "una candidata cuya secuencia no es la del nombre",
    );
}

// ── M5: una revisión de código no es una versión ────────────────────────

/// La frontera entre los tres conceptos.
///
/// Una revision identifica **donde estaba el codigo**, una version **que es el
/// producto**, y una referencia **como se llama este release**. Un build cuyo
/// codigo se movio y cuya version no es un hecho normal, y una herramienta que
/// trate la revision como version no puede representarlo: o autoriza un build
/// distinto, o rechaza uno legitimo.
///
/// ## Por qué esto no es un mutante
///
/// La primera version de esta seccion defini aqui mismo una funcion
/// `revision_is_version` que comparaba las dos cadenas, y luego se exigio que
/// devolviera `false`. No habia nada que falsar: la funcion era del test, no
/// del modulo, y `ProductVersion::new("1.2.3")` tambien es una version valida,
/// luego comparar las dos cadenas **si** da `true` y el test solo se estaba
/// contradiciendo a si mismo.
///
/// La ley real no es de comparacion: es que **este modulo no fabrica
/// versiones**. Consume una version —`binds` y `name_for` la reciben— y no
/// devuelve ninguna. Una revision no puede convertirse en version aqui porque
/// aqui no se produce ninguna version, y esa es una propiedad de las firmas,
/// no una funcion que se pueda mutar en caliente.
///
/// Lo que si se puede falsar es la existencia de la conversion, y para eso hace
/// falta mirar el codigo, igual que el resto de los fitness del repositorio.
/// La frontera entre los tres conceptos se sostiene en el sistema de tipos, y
/// el sistema de tipos no se ejercita con un `assert!`: o el crate compila, o
/// no.
mod revision_is_not_a_version {
    use super::*;

    /// Lo que este modulo no puede tener.
    ///
    /// Una conversion es el unico camino por el que una revision pasaria a
    /// ser una version sin que nadie lo note: misma forma, mismo `.as_str()`,
    /// y a partir de ahi los dos son intercambiables. Se busca por posicion —
    /// `From<`, `Into<`, `TryFrom<`, `TryInto<` — y no por la palabra suelta,
    /// porque el nombre del tipo aparece legitimamente en docstrings.
    const CONVERSIONES: &[&str] = &[
        "From<SourceRevision",
        "From<ProductVersion",
        "Into<SourceRevision",
        "Into<ProductVersion",
        "TryFrom<SourceRevision",
        "TryFrom<ProductVersion",
        "TryInto<SourceRevision",
        "TryInto<ProductVersion",
    ];

    fn es_conversion(fragmento: &str) -> bool {
        CONVERSIONES.iter().any(|c| fragmento.contains(c))
    }

    /// Este modulo recibe versiones y no las produce.
    ///
    /// Lo que se mira es el tipo de **retorno** —lo que va despues del `->`— y
    /// no si el nombre aparece en la linea. La primera version de este escaner
    /// hacia justo eso, y marco `BindOutcome::message`, que **recibe** una
    /// version para escribir el mensaje del desajuste y devuelve un `String`.
    /// El defecto estaba en el instrumento: preguntar «la linea menciona una
    /// version» no es «la linea fabrica una version», y un guard que se dispara
    /// sobre algo legitimo entrena a su lector a saltarselo.
    ///
    /// Una referencia prestada al retour (`-> &ProductVersion`) **si** cuenta:
    /// devolver una referencia no fabrica nada, asi que se excluye
    /// explicitamente en vez de por accidente de formato.
    fn fabrica_una_version(linea: &str) -> bool {
        let Some((_, retorno)) = linea.split_once("->") else {
            return false;
        };
        let retorno = retorno.trim();
        retorno.contains("ProductVersion") && !retorno.starts_with('&')
    }

    #[test]
    fn ninguna_firma_devuelve_una_version() {
        let fuente = include_str!("../src/release_ref.rs");
        let firmas: Vec<&str> = fuente
            .lines()
            .map(str::trim)
            .filter(|linea| linea.contains("fn ") && fabrica_una_version(linea))
            .collect();
        assert!(
            firmas.is_empty(),
            "este modulo devuelve una ProductVersion, y devolver una version es \
             el unico camino por el que una revision pasaria a ser una: {firmas:?}"
        );
    }

    /// Y ninguna conversion implicita entre los dos tipos.
    #[test]
    fn no_hay_conversion_entre_revision_y_version() {
        let fuente = include_str!("../src/release_ref.rs");
        let encontradas: Vec<&str> = fuente
            .split_whitespace()
            .filter(|token| es_conversion(token))
            .collect();
        assert!(
            encontradas.is_empty(),
            "este modulo declara una conversion entre SourceRevision y \
             ProductVersion, y con ella la frontera entre revision y version \
             deja de ser una garantia: {encontradas:?}"
        );
    }

    /// El control: el escaner ve una conversion si aparece una.
    ///
    /// Sin esto, `encontradas.is_empty()` podria pasar porque el escaner no
    /// mira nada — que es la forma mas comun de que un fitness de source-level
    /// se apague sin que nadie lo note.
    #[test]
    fn el_escaner_ve_una_conversion_si_aparece() {
        let sintetico = "impl From<SourceRevision> for ProductVersion { }";
        assert!(
            es_conversion(sintetico),
            "el escaner no ve una conversion escrita de forma normal"
        );
        assert!(
            !es_conversion("fn esto_no_es_una_conversion(x: &str) -> &str"),
            "el escaner marca como conversion algo que no lo es"
        );
    }

    /// Y el control del control: distinguir **fabricar** una version de
    /// **recibirla**.
    ///
    /// Es la distincion que la primera version del escaner no hacia, y por la
    /// que se disparo sobre `BindOutcome::message`. Un fitness que no tiene
    /// este caso negativo no demuestra que sepa cuando NO disparar.
    #[test]
    fn el_escaner_de_fabrica_distingue_recibir_de_devolver() {
        // Fabricar: la version es lo que sale.
        assert!(
            fabrica_una_version("fn la_deja() -> ProductVersion { todo() }"),
            "una firma que devuelve una version tiene que marcarse"
        );
        // Recibir: la version entra y sale otra cosa. Este caso salio de un
        // falso positivo real, asi que esta aqui para que no vuelva.
        assert!(
            !fabrica_una_version("pub fn message(&self, version: &ProductVersion) -> String {"),
            "recibir una version no es fabricarla: esta firma devuelve un String"
        );
        // Referencia prestada de retour: tampoco fabrica nada.
        assert!(
            !fabrica_una_version("fn la_devuelve(&self) -> &ProductVersion {"),
            "devolver una referencia a una version no fabrica una version"
        );
        // Y una firma que no devuelve nada.
        assert!(!fabrica_una_version("fn esto() { nada(); }"));
    }

    /// Lo que si se puede observar en caliente: la revision es opaca y viva.
    ///
    /// Un revision vacia no es una revision, y una revision se conserva tal
    /// cual porque el nucleo no sabe de control de versiones. Estas dos si son
    /// afirmacion sobre el codigo, y por eso se comprueban aqui y no mas
    /// arriba: son las unicas partes de M5 que se pueden ejercer sin el
    /// compilador.
    #[test]
    fn una_revision_vacia_no_es_una_revision_y_se_conserva_tal_cual() {
        assert!(
            SourceRevision::new("   ").is_none(),
            "una revision vacia no es una revision"
        );
        assert_eq!(
            SourceRevision::new("abc1234").expect("valida").as_str(),
            "abc1234",
            "y se conserva tal cual, porque el nucleo no sabe de control de versiones"
        );
    }
}

// ---------------------------------------------------------------------------
// El veredicto
// ---------------------------------------------------------------------------

/// Corre el codigo CORRECTO por las nueve filas y exige lo declarado.
///
/// Sin esta comprobación, un mutante podría morir por un motivo equivocado y el
/// falsador daria verde habiendose roto a si mismo.
#[test]
fn la_tabla_declara_lo_que_la_ley_exige() {
    for case in cases() {
        let actual = bound(&case.reference, &case.version, &case.naming);
        assert_eq!(
            actual, case.expected_bound,
            "fila «{}»: la ley dice {} y el codigo dice {actual}",
            case.name, case.expected_bound
        );
    }
}

/// Un mutante tiene que morir **en la fila que ejerce su ley**, y no antes.
///
/// Es lo que se aprendio del falsador de targets, y el motivo es el mismo: un
/// mutante rompe una ley y en las demas filas tiene que coincidir con el codigo
/// correcto. Exigir que se diferencie en todas las filas es exigir que rompa
/// leyes que no estaba aims romper.
fn assert_kills(
    label: &str,
    mutant: impl Fn(&ReleaseRef, &ProductVersion, &VersionNaming) -> bool,
    expected_row: &str,
) {
    let rows = cases();
    let mut died_at: Option<&str> = None;

    for case in &rows {
        let expected = bound(&case.reference, &case.version, &case.naming);
        let actual = mutant(&case.reference, &case.version, &case.naming);
        if actual == expected {
            continue;
        }
        assert_eq!(
            case.name,
            expected_row,
            "el mutante «{label}» revienta la fila «{row}», que no es la suya: \\
             la ley dice {expected} y el mutante contesta {actual}.",
            row = case.name
        );
        died_at = Some(case.name);
        break;
    }

    assert!(
        died_at.is_some(),
        "el mutante «{label}» sobrevive a todas las filas: contesta como el codigo \\
         correcto en todas, luego no hay fila que distinga la ley que rompio"
    );
}

/// La contrapartida de un rechazo: hay salida, y se declara.
///
/// ## Por qué esto vive en el falsador y no solo en la tabla
///
/// La tabla de arriba ya tiene filas de candidata y filas `Exact` —y las
/// recorre `assert_kills`—, pero una fila que nadie nombra puede desaparecer en
/// una reescritura y el falsador sigue verde. Nombrarla la convierte en
/// criterio, y los gates de ADR la pueden citar por nombre.
///
/// Y el nombre importa porque es lo que hace que el rechazo no sea una trampa:
/// una candidata no nombra a la versión de su producto bajo la naming de
/// fábrica, **y** hay una convención declarada bajo la cual sí lo hace.
#[test]
fn una_candidata_no_nombra_a_la_version_de_su_producto() {
    let version = v("1.2.3");
    let candidata = candidate("v1.2.3-rc2", CandidateSequence::nth(2).unwrap());

    let con_prefijo = binds(&candidata, &version, &VersionNaming::v_prefixed());
    assert!(
        !con_prefijo.is_bound(),
        "una candidata no nombra a la version de su producto: {}",
        con_prefijo.message(&version)
    );

    // Y la salida: el mismo tag, bajo la naming que si tiene sitio para
    // candidatas, si nombra a la version. El rechazo anterior no era una
    // trampa: era una convencion que faltaba declarar.
    let declarada = VersionNaming::PrefixedCandidate {
        prefix: "v".into(),
        separator: "-".into(),
        marker: "rc".into(),
    };
    assert!(
        binds(&candidata, &version, &declarada).is_bound(),
        "declarada la convencion de candidatas, la misma referencia si nombra \
         a la version"
    );
}

/// El resultado del desajuste dice **los dos lados**, siempre.
///
/// Un rechazo que solo dice «no» manda a quien lo lee a adivinar cuál de los dos
/// valores está mal. Con los dos, corregir es mecánico.
#[test]
fn un_desajuste_dice_los_dos_lados() {
    let outcome = binds(
        &stable("v1.2.3-rc2"),
        &v("1.2.3"),
        &VersionNaming::v_prefixed(),
    );
    match &outcome {
        BindOutcome::NamingHasNoRoomForCandidates => {
            let message = outcome.message(&v("1.2.3"));
            assert!(
                message.contains("candidate") && message.contains("naming"),
                "este caso dice que la naming no tiene sitio, que es una \\
                 reparacion distinta de «el nombre esta mal»: {message}"
            );
        }
        BindOutcome::DoesNotBind { expected, found } => {
            assert_eq!(expected, "v1.2.3");
            assert_eq!(found, "v1.2.3-rc2");
        }
        BindOutcome::Binds => panic!("una candidata no puede nombrar a su version: {outcome:?}"),
    }

    let mismatch = binds(&stable("v9.9.9"), &v("1.2.3"), &VersionNaming::v_prefixed());
    let message = mismatch.message(&v("1.2.3"));
    assert!(message.contains("v1.2.3"), "{message}");
    assert!(message.contains("v9.9.9"), "{message}");
}
