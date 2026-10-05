//! Falsador de los roles de release.
//!
//! # Qué se está midiendo
//!
//! Que un rol es una **declaración de responsabilidad**, y que su techo se
//! expresa en el retículo de canales que ya existía en vez de en unas reglas
//! nuevas.
//!
//! El riesgo de este bloque no es que el rol exista: es que se convierta en una
//! **segunda autoridad de promoción** al lado de `can_promote`. Dos respuestas a
//! la misma pregunta en el mismo árbol es la clase de defecto que este
//! repositorio ha encontrado siete veces, y por eso dos de los mutantes de abajo
//! atacan precisamente esa duplicación.
//!
//! # Las dos preguntas, y por qué están en tablas separadas
//!
//! Un rol tiene dos preguntas distintas, y **no** se pueden medir en una sola
//! tabla. MEDIDO al escribirlas: la primera versión las mezcló, y el techo
//! resultó **infalsable** por la vía de `may_publish` —porque `publishes()` solo
//! es cierto para el rol cuyo techo es `Stable`, luego quitar el techo de esa
//! función no cambia ninguna respuesta—. Eso no era un defecto del código: era
//! una fila que no distinguía la ley que decía medir. Por eso:
//!
//! - [`may_promote`] es la pregunta por un **paso**: ¿puede este actor dar este
//!   paso? Aquí vive el techo, y aquí es falsable.
//! - [`may_publish`] es la pregunta por la **publicación**: ¿puede este actor
//!   publicar? Aquí vive «solo publica quien publica».
//!
//! Las dos se miden en su tabla, cada mutante muere en la fila que ejerce su
//! ley, y ninguna se miente sobre lo que la otra cubre.
//!
//! Los tres mutantes, y los tres son defectos que se darían de verdad:
//!
//! - `M1 ceiling-ignores-role`: que el techo no mire el rol. Es lo que pasa si
//!   el techo se implementa como «todos pueden» y la distinción se queda en el
//!   nombre de la variante.
//! - `M2 lattice-ignored`: que el rol pueda saltarse el retículo. El productor
//!   que llega a `Stable` porque su rol «sí puede» es la forma exacta de que un
//!   productor publique.
//! - `M3 publishing-ignored`: que llegar baste para publicar. Es la confusión de
//!   las dos mitades, y un certificador que publica es un certificador que
//!   certifica su propio trabajo.
//!
//! Y tres leyes más que **no son mutantes**, porque la función que las sostiene no
//! se puede mutar en caliente y por eso tienen otra forma —cada una con su
//! nombre, para que un gate pueda citarla por su nombre y una reescritura no se
//! las lleve por delante sin darse cuenta—: `producir_es_un_final`,
//! `llegar_no_es_publicar` y `el_sobre_lleva_lo_que_hace_falta`.

use sddk_domain::channel::ReleaseChannel;
use sddk_domain::release_ref::{CandidateSequence, ReleaseRef, SourceRevision};
use sddk_domain::release_role::{
    CandidateHandoff, HandoffArtifact, ReleaseRole, RoleRefusal, may_promote, may_publish,
    promotion_distance,
};
use sddk_domain::version_authority::{ProductVersion, ReleaseTarget};

/// La versión del test.
fn v(value: &str) -> ProductVersion {
    ProductVersion::new(value).expect("version valida en el test")
}

/// El target del test, que no se mira para ninguna de estas leyes y por eso
/// puede ser el mismo en todas las filas.
fn target() -> ReleaseTarget {
    ReleaseTarget::at_root("runtime")
}

// ── Tabla 1: la pregunta por un PASO ─────────────────────────────────────

/// Una fila de la pregunta por un paso.
struct StepCase {
    name: &'static str,
    role: ReleaseRole,
    from: ReleaseChannel,
    to: ReleaseChannel,
    gates_ok: bool,
    /// Lo que la ley dice: `true` es «el paso es suyo».
    expected: bool,
}

fn step_cases() -> Vec<StepCase> {
    use ReleaseChannel::{Candidate, Dev, Edge, Stable};
    vec![
        // El positivo: el rol que puede dar el paso con las puertas abiertas.
        StepCase {
            name: "el que publica todo da el paso con las puertas abiertas",
            role: ReleaseRole::FullPublisher,
            from: Candidate,
            to: Stable,
            gates_ok: true,
            expected: true,
        },
        // M1. Aqui el techo es lo UNICO que lo para: el reticulo lo permite y
        // las puertas estan abiertas, luego lo unico que puede rechazarlo es el
        // techo del productor.
        StepCase {
            name: "el techo del productor lo para aunque todo lo demas lo permita",
            role: ReleaseRole::CandidateProducer,
            from: Candidate,
            to: Stable,
            gates_ok: true,
            expected: false,
        },
        // Y su espejo, que es la parte que hace que el techo signifique algo:
        // el productor SI puede dar el paso que si es suyo.
        StepCase {
            name: "y el paso que si es suyo lo puede dar",
            role: ReleaseRole::CandidateProducer,
            from: Edge,
            to: Candidate,
            gates_ok: true,
            expected: true,
        },
        // M2. Aqui el reticulo es lo unico que lo para: el rol puede llegar a
        // stable, luego lo unico que puede rechazarlo es el reticulo.
        StepCase {
            name: "el reticulo no lo permite a NADIE, ni al que publica todo",
            role: ReleaseRole::FullPublisher,
            from: Dev,
            to: Stable,
            gates_ok: true,
            expected: false,
        },
        // Y las puertas cerradas, que son un tercer motivo con un arreglo
        // distinto: no es que no pueda, es que todavia no.
        StepCase {
            name: "las puertas cerradas son un motivo propio",
            role: ReleaseRole::FullPublisher,
            from: Candidate,
            to: Stable,
            gates_ok: false,
            expected: false,
        },
    ]
}

/// Corre el codigo CORRECTO por las filas y exige lo declarado.
///
/// Sin esto un mutante podria morir por un motivo equivocado y el falsador daria
/// verde habiendose roto a si mismo — que es lo que le ha pasado a tres
/// instrumentos de este mismo bloque.
#[test]
fn la_tabla_de_pasos_declara_lo_que_la_ley_exige() {
    for case in step_cases() {
        assert_eq!(
            may_promote(case.role, case.from, case.to, case.gates_ok),
            case.expected,
            "fila «{}»",
            case.name
        );
    }
}

// ── Tabla 2: la pregunta por la PUBLICACIÓN ─────────────────────────────

/// Una fila de la pregunta por la publicación.
struct PublishCase {
    name: &'static str,
    role: ReleaseRole,
    from: ReleaseChannel,
    to: ReleaseChannel,
    gates_ok: bool,
    /// Lo que la ley dice: `None` es «se puede publicar».
    expected: Option<&'static str>,
}

fn publish_cases() -> Vec<PublishCase> {
    use ReleaseChannel::{Candidate, Dev, Stable};
    vec![
        PublishCase {
            name: "el que publica todo publica, con las puertas abiertas",
            role: ReleaseRole::FullPublisher,
            from: Candidate,
            to: Stable,
            gates_ok: true,
            expected: None,
        },
        // M3. Aqui «publica» es lo unico que lo para: el reticulo lo permite, el
        // techo lo alcanza, y aun asi no es el publicador.
        PublishCase {
            name: "un certificador no publica: llegar a stable no es publicar",
            role: ReleaseRole::Certifier,
            from: Candidate,
            to: Stable,
            gates_ok: true,
            expected: Some("NotThisRolesStep"),
        },
        PublishCase {
            name: "y un promotor tampoco, que promover no es publicar",
            role: ReleaseRole::Promoter,
            from: Candidate,
            to: Stable,
            gates_ok: true,
            expected: Some("NotThisRolesStep"),
        },
        // El techo del productor. Llega al mismo sitio que el certificador por
        // el techo, y por eso tiene su propia fila: son dos razones
        // distintas con dos arreglos distintos.
        PublishCase {
            name: "un productor se queda en su techo, que es donde termina",
            role: ReleaseRole::CandidateProducer,
            from: Candidate,
            to: Stable,
            gates_ok: true,
            expected: Some("BeyondCeiling"),
        },
        // Y el reticulo, que no depende de quien pregunta.
        PublishCase {
            name: "el reticulo no lo permite a NADIE",
            role: ReleaseRole::FullPublisher,
            from: Dev,
            to: Stable,
            gates_ok: true,
            expected: Some("StepNotInLattice"),
        },
    ]
}

#[test]
fn la_tabla_de_publicacion_declara_lo_que_la_ley_exige() {
    for case in publish_cases() {
        let actual = match may_publish(case.role, case.from, case.to, case.gates_ok) {
            Ok(()) => None,
            Err(error) => Some(rejection_name(&error)),
        };
        assert_eq!(
            actual, case.expected,
            "fila «{}»: la ley dice {:?} y el codigo dice {actual:?}",
            case.name, case.expected
        );
    }
}

/// El nombre del rechazo, para comparar sin arrastrar el resto del detalle.
fn rejection_name(error: &RoleRefusal) -> &'static str {
    match error {
        RoleRefusal::BeyondCeiling { .. } => "BeyondCeiling",
        RoleRefusal::StepNotInLattice { .. } => "StepNotInLattice",
        RoleRefusal::NotThisRolesStep { .. } => "NotThisRolesStep",
    }
}

// ── M1: el techo no mira el rol ─────────────────────────────────────────

/// Que el techo no se mire: cualquier actor llega a cualquier canal.
///
/// Es lo que pasa si el techo se implementa como «todos pueden» y la distinción
/// se queda en el nombre de la variante. El enum tiene cuatro valores, cuatro
/// nombres distintos y un comportamiento: un tipo que dice cuatro cosas y hace
/// una.
fn m1_ceiling_ignores_role(from: ReleaseChannel, to: ReleaseChannel, gates_ok: bool) -> bool {
    sddk_domain::channel::can_promote(from, to, gates_ok)
}

#[test]
fn m1_ceiling_ignores_role_murio() {
    assert_step_kills(
        "ceiling-ignores-role",
        |_role, from, to, gates| m1_ceiling_ignores_role(from, to, gates),
        "el techo del productor lo para aunque todo lo demas lo permita",
    );
}

// ── M2: el retículo no se mira ──────────────────────────────────────────

/// Que el rol pueda saltarse el retículo.
///
/// El productor que llega a `Stable` porque «su rol sí puede» es la forma exacta
/// de que un productor publique. Y es el mutante que **mas dano haria** de los
/// tres, porque suena a criterio: «el rol lo permite». Lo que no dice es quién
/// puso ese retículo, y la respuesta es que ya estaba y no es suyo.
fn m2_lattice_ignored(role: ReleaseRole, to: ReleaseChannel) -> bool {
    role.may_reach(to)
}

#[test]
fn m2_lattice_ignored_murio() {
    assert_step_kills(
        "lattice-ignored",
        |role, _from, to, _gates| m2_lattice_ignored(role, to),
        "el reticulo no lo permite a NADIE, ni al que publica todo",
    );
}

// ── M3: llegar es publicar ──────────────────────────────────────────────

/// Que el techo baste para publicar.
///
/// Es la confusión de las dos mitades: `may_reach` dice hasta dónde, y alguien
/// lo lee como «y por lo tanto puede». Un certificador llega a `Stable` —tiene
/// que llegar, para poder certificar dentro— y de ahí salta la conclusión
/// falsa de que puede publicarlo.
fn m3_publishing_ignored(role: ReleaseRole, from: ReleaseChannel, to: ReleaseChannel) -> bool {
    sddk_domain::channel::can_promote(from, to, true) && role.may_reach(to)
}

#[test]
fn m3_publishing_ignored_murio() {
    let rows = publish_cases();
    let mut died_at: Option<&str> = None;
    for case in &rows {
        let expected = may_publish(case.role, case.from, case.to, case.gates_ok).is_ok();
        let actual = m3_publishing_ignored(case.role, case.from, case.to);
        if actual == expected {
            continue;
        }
        assert_eq!(
            case.name,
            "un certificador no publica: llegar a stable no es publicar",
            "el mutante «publishing-ignored» revienta la fila «{row}», que no es \
             la suya: la ley dice {expected} y el mutante contesta {actual}.",
            row = case.name
        );
        died_at = Some(case.name);
        break;
    }
    assert!(
        died_at.is_some(),
        "el mutante «publishing-ignored» sobrevive a todas las filas: contesta \
         como el codigo correcto en todas, luego no hay fila que distinga la ley \
         que rompio"
    );
}

// ── Ley con nombre: producir es un final ────────────────────────────────

/// Que la producción sea un final, y que eso se pueda decir.
///
/// ## Por qué esto no es un mutante
///
/// La primera versión lo escribio como un mutante que devolvia `false` y se
/// comparaba con `terminates_here()` para «roles que no son productor» —donde
/// los dos dan `false` y el mutante no se diferenciaba de nadie, y el assert se
/// caia. Un mutante tiene que **reemplazar** la respuesta, no llevar la
/// verdadera en un `&&` al lado: si la lleva, no puede discrepar nunca.
///
/// Y aqui no hay función que mutar en caliente: `terminates_here` es una
/// propiedad del tipo. Por eso la ley se escribe con nombre, para que un gate
/// pueda citarla y una reescritura no se la lleve sin querer.
///
/// ## Por qué la ley importa
///
/// Es el fallo **silencioso** del bloque. El handoff se emite, y si el flujo se
/// queda esperando una certificacion externa que este actor no controla, el
/// trabajo que si podia hacer se detiene con el. Terminar bien y terminar
/// incompleto se ven igual desde fuera si nadie escribe la ley.
#[test]
fn producir_es_un_final() {
    assert!(
        ReleaseRole::CandidateProducer.terminates_here(),
        "producir material candidato termina en el handoff: el productor no \
         espera a una certificacion que no controla, porque esperar convierte \
         una entrega en una dependencia y detiene el trabajo independiente"
    );
    for role in [
        ReleaseRole::Certifier,
        ReleaseRole::Promoter,
        ReleaseRole::FullPublisher,
    ] {
        assert!(
            !role.terminates_here(),
            "«{}» no produce material candidato, luego no termina en la \
             produccion: hay pasos despues",
            role.name()
        );
    }
}

// ── Ley con nombre: llegar no es publicar ───────────────────────────────

/// Que «hasta dónde llega» y «qué hace con lo que llega» sean dos cosas.
///
/// El techo no separa a un certificador de un publicador —los dos llegan igual
/// de lejos—, y por eso el techo **no basta** y hace falta una segunda pregunta.
/// Sin esta fila, `may_reach` sería la respuesta entera y bastaría, que es
/// exactamente el error.
#[test]
fn llegar_no_es_publicar() {
    assert_eq!(
        ReleaseRole::Certifier.ceiling(),
        ReleaseRole::FullPublisher.ceiling(),
        "el techo no es lo que separa a un certificador de un publicador: por eso \
         el techo no basta y hace falta la segunda pregunta"
    );
    assert!(!ReleaseRole::Certifier.publishes());
    assert!(ReleaseRole::FullPublisher.publishes());

    // Y el productor, que tiene el techo más bajo, tampoco publica. Que su techo
    // y su capacidad a publicar coincidan en la respuesta **no** significa que
    // sean la misma pregunta: se separan en la fila del techo.
    assert!(!ReleaseRole::CandidateProducer.publishes());
    assert_eq!(
        ReleaseRole::CandidateProducer.ceiling(),
        ReleaseChannel::Candidate
    );
}

/// El `Ord` derivado de los canales va **al revés** que la promoción, y por eso
/// no se usa para nada aquí.
///
/// ## La ley, y por qué necesita nombre
///
/// `ReleaseChannel` se declara `Stable, Candidate, Edge, Dev`, que es el orden
/// en que se **lee** la cadena de promoción, de su final a su principio. Su
/// `Ord` derivado es por tanto el **revés**: `Stable` ordena antes que
/// `Candidate`, y `Stable <= Candidate` es cierto.
///
/// MEDIDO: la primera versión de este bloque escribió el techo como
/// `channel <= ceiling`, y un `CandidateProducer` pasaba a poder llegar a
/// `Stable` — que es literalmente lo que el módulo existe para impedir. Lo
/// encontró esta fila, no la lectura del código.
///
/// Un orden derivado que contradice el retículo del dominio no es un detalle
/// al que hay que tener cuidado: es una trampa con sistema de tipos alrededor,
/// y por eso la comparación correcta camina [`promotion_distance`]. La fila de
/// abajo lo fija para que nadie la vuelva a escribir como `<=`.
#[test]
fn el_orden_derivado_de_los_canales_no_es_el_de_la_promocion() {
    // La trampa, escrita de la forma en que se escribe por costumbre.
    assert!(
        ReleaseChannel::Stable <= ReleaseChannel::Candidate,
        "el `Ord` derivado pone `Stable` antes que `Candidate`: por eso \
         `channel <= ceiling` daria verde a un productor llegando a stable"
    );

    // Y por eso el techo se lee por el retículo, no por el orden.
    assert_eq!(
        promotion_distance(ReleaseChannel::Stable, ReleaseChannel::Stable),
        Some(0)
    );
    assert_eq!(
        promotion_distance(ReleaseChannel::Dev, ReleaseChannel::Stable),
        Some(3)
    );
    assert_eq!(
        promotion_distance(ReleaseChannel::Stable, ReleaseChannel::Dev),
        None,
        "desde `stable` no se promociona a ninguna parte, luego nadie esta por \
         detras de el"
    );
    assert_eq!(
        promotion_distance(ReleaseChannel::Stable, ReleaseChannel::Candidate),
        None,
        "y un techo de `candidate` NO tiene por delante a `stable`: por eso un \
         productor no lo alcanza, que es la fila que el retículo inverted habia \
         dejado pasar"
    );

    // La consecuencia, escrita sobre el rol y no sobre el retículo, porque es
    // la que le importa a quien publica.
    assert!(
        !ReleaseRole::CandidateProducer.may_reach(ReleaseChannel::Stable),
        "un productor no alcanza `stable` por su techo"
    );
    assert!(ReleaseRole::CandidateProducer.may_reach(ReleaseChannel::Candidate));
    assert!(ReleaseRole::FullPublisher.may_reach(ReleaseChannel::Dev));
}

// ── Ley con nombre: el sobre lleva lo que hace falta ────────────────────

/// Lo que el sobre tiene que llevar, y que perder es perder.
///
/// Un sobre que no lleva la revisión ha perdido la única manera de decir que el
/// código se movió y la versión no. Dos builds con la misma versión y distinto
/// código son un hecho normal, y un sobre que los presenta como el mismo ha
/// borrado el dato que los distinguía — que es peor que no llevar nada, porque
/// parece que sí.
#[test]
fn el_sobre_lleva_lo_que_hace_falta_y_nada_mas() {
    let handoff = CandidateHandoff {
        target: target(),
        product_version: v("1.4.0"),
        reference: ReleaseRef::candidate(
            "v1.4.0-rc2",
            ReleaseChannel::Candidate,
            CandidateSequence::nth(2).expect("la segunda candidata existe"),
        ),
        sequence: Some(CandidateSequence::nth(2).expect("la segunda candidata existe")),
        source_revision: SourceRevision::new("abc1234").expect("revision valida"),
        artifacts: vec![HandoffArtifact {
            kind: "lo que el productor llame a esto".to_owned(),
            digest: format!("sha256:{}", "0".repeat(64)),
            path: Some("dist/candidate.bin".to_owned()),
        }],
        evidence_refs: vec![format!("sha256:{}", "1".repeat(64))],
        external_handoff_type: "el vocabulario del productor, no el nuestro".to_owned(),
        external_handoff_digest: format!("sha256:{}", "2".repeat(64)),
    };

    // 1. La revisión viaja, y es una revisión y no una versión.
    assert_eq!(handoff.source_revision.as_str(), "abc1234");
    assert_ne!(
        handoff.source_revision.as_str(),
        handoff.product_version.as_str(),
        "una revisión de codigo no es una version de producto, y el sobre las \
         lleva en campos distintos precisamente para que no se confundan"
    );

    // 2. El sobre lleva la REFERENCIA, no el nombre recortado, y con ella el
    //    canal y la secuencia. Version y referencia son dos hechos, dos campos.
    assert_eq!(handoff.reference.channel(), ReleaseChannel::Candidate);
    assert_eq!(handoff.reference.name(), "v1.4.0-rc2");
    assert!(
        handoff.sequence.is_some(),
        "una referencia candidata lleva su secuencia declarada, y en el sobre va \
         tambien para que quien lo consuma no tenga que leerla del nombre"
    );

    // 3. Y la clase de handoff externo es del PRODUCTOR, en sus palabras.
    assert_eq!(
        handoff.external_handoff_type, "el vocabulario del productor, no el nuestro",
        "el sobre lleva la clase de handoff como texto opaco: un nucleo que la \
         interpretara habria adoptado el vocabulario de un productor"
    );

    // 4. Y los artefactos son direccionables por contenido.
    for artifact in &handoff.artifacts {
        assert!(
            artifact.digest.starts_with("sha256:") && artifact.digest.len() == 71,
            "un artefacto sin digest direccionable por contenido no es una \
             evidencia, es un nombre: {}",
            artifact.digest
        );
    }
}

// ── El veredicto sobre los instrumentos ─────────────────────────────────

/// Un mutante tiene que morir **en la fila que ejerce su ley**, y no antes.
///
/// Es lo que se aprendio del falsador de targets y del de la naming: un mutante
/// rompe una ley y en las demas filas tiene que coincidir con el codigo correcto.
/// Exigir que se diferencie en todas las filas es exigir que rompa leyes que no
/// estaba aims romper, y eso es lo que hacia que M2 y M3 murieran en la fila
/// equivocada en su primera version.
fn assert_step_kills(
    label: &str,
    mutant: impl Fn(ReleaseRole, ReleaseChannel, ReleaseChannel, bool) -> bool,
    expected_row: &str,
) {
    let rows = step_cases();
    let mut died_at: Option<&str> = None;

    for case in &rows {
        let expected = may_promote(case.role, case.from, case.to, case.gates_ok);
        let actual = mutant(case.role, case.from, case.to, case.gates_ok);
        if actual == expected {
            continue;
        }
        assert_eq!(
            case.name,
            expected_row,
            "el mutante «{label}» revienta la fila «{row}», que no es la suya: \
             la ley dice {expected} y el mutante contesta {actual}.",
            row = case.name
        );
        died_at = Some(case.name);
        break;
    }

    assert!(
        died_at.is_some(),
        "el mutante «{label}» sobrevive a todas las filas: contesta como el codigo \
         correcto en todas, luego no hay fila que distinga la ley que rompio"
    );
}
