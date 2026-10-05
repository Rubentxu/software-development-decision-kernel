//! Falsador de la selección de target.
//!
//! Cada mutante es una versión de `select_target` con una ley rota, y la
//! tabla es la que lo distingue. Sin esto, las doce pruebas de aceptación
//! afirmarían que existe una función que decide, no que decide bien.
//!
//! # Los tres modos, y por qué son tres y no uno
//!
//! 1. **Ante varios, gana el primero.** El defecto más obvio y el que más
//!    daño hace en silencio, porque el resultado es un target *válido*: un
//!    `read_dir` no ordenado haría que el producto publicado dependiera del
//!    sistema de ficheros.
//! 2. **Un nombre que no casa cae al único que hay.** Convierte el nombre en
//!    decoracion, y es el modo por el que un monorepo acabaría publicando la
//!    versión de un producto书写 al que se quería nombrar con otro nombre.
//! 3. **Lo desconocido se degrada a ambiguo.** Parece cosmético y no lo es:
//!    quien recibe «hay varios» busca un segundo producto, y quien recibe
//!    «ese no existe» corrige el nombre. Son dos reparaciones distintas, y la
//!    que se pierde es la de escribir bien el nombre.

use sddk_domain::version_authority::{
    ReleaseTarget, TargetSelectionError, TargetSelector, select_target,
};

type Selection = Result<&'static ReleaseTarget, TargetSelectionError>;

/// Targets que viven lo justo lo que la tabla necesita, y nada mas.
struct Fixture {
    all: Vec<ReleaseTarget>,
}

impl Fixture {
    fn new() -> Self {
        Self { all: Vec::new() }
    }

    fn with(mut self, id: &'static str, root: &'static str) -> Self {
        self.all.push(ReleaseTarget::at(id, root));
        self
    }
}

/// Una fila: qué se pregunta y qué se espera.
///
/// El resultado esperado se declara como texto, y la comprobación de que la
/// mutación ha muerto compara contra la fila que dice qué respuesta es la
/// **correcta** —la de la ley—, no contra la que produce el código roto. Si
/// una mutación coincidiera con lo correcto, no sería una mutación.
struct Case {
    name: &'static str,
    setup: fn() -> Fixture,
    selector: fn() -> TargetSelector,
    expected: &'static str,
}

fn empty_fixture() -> Fixture {
    Fixture::new()
}

fn one_target() -> Fixture {
    Fixture::new().with("runtime", "packages/runtime")
}

fn two_targets() -> Fixture {
    Fixture::new()
        .with("alpha", "packages/alpha")
        .with("beta", "packages/beta")
}

fn duplicated_identity() -> Fixture {
    Fixture::new()
        .with("runtime", "packages/a")
        .with("runtime", "packages/b")
}

fn unsolicited() -> TargetSelector {
    TargetSelector::unsolicited()
}

fn named_alpha() -> TargetSelector {
    TargetSelector::named("alpha")
}

fn named_runtime() -> TargetSelector {
    TargetSelector::named("runtime")
}

fn named_typo() -> TargetSelector {
    TargetSelector::named("runtine")
}

fn cases() -> Vec<Case> {
    vec![
        Case {
            name: "cero targets",
            setup: empty_fixture,
            selector: unsolicited,
            expected: "NoTarget",
        },
        Case {
            name: "un target sin pedir",
            setup: one_target,
            selector: unsolicited,
            expected: "target",
        },
        Case {
            name: "dos targets sin pedir",
            setup: two_targets,
            selector: unsolicited,
            expected: "Ambiguous",
        },
        Case {
            name: "dos targets, uno nombrado",
            setup: two_targets,
            selector: named_alpha,
            expected: "target",
        },
        Case {
            name: "un target, un nombre que no casa",
            setup: one_target,
            selector: named_typo,
            expected: "Unknown",
        },
        // Estas dos filas son las que separan M3 de «todo se degrada a
        // ambiguo»: una pide que un nombre inexistente se distinga de varios,
        // y la otra que un nombre que casa no se degrade.
        Case {
            name: "un target, su propio nombre",
            setup: one_target,
            selector: named_runtime,
            expected: "target",
        },
        Case {
            name: "identidad repetida, nombre correcto",
            setup: duplicated_identity,
            selector: named_runtime,
            expected: "Ambiguous",
        },
    ]
}

fn shape(selection: Result<&ReleaseTarget, TargetSelectionError>) -> String {
    match selection {
        Ok(target) => format!("target {}", target.id()),
        Err(TargetSelectionError::NoTarget) => "NoTarget".to_owned(),
        Err(TargetSelectionError::AmbiguousTarget { .. }) => "Ambiguous".to_owned(),
        Err(TargetSelectionError::UnknownTarget { .. }) => "Unknown".to_owned(),
    }
}

fn correct(setup: fn() -> Fixture, selector: fn() -> TargetSelector) -> String {
    let fixture = setup();
    shape(select_target(&fixture.all, &selector()))
}

/// Ejecuta la mutación. Cada una reproduce la firma de `select_target` para que
/// el error de compilación sea imposible de confundir con un fallo de lógica.
type Mutant = fn(&[ReleaseTarget], &TargetSelector) -> Selection;

/// Devuelve una referencia con vida estatica a un clon del target.
///
/// La firma de los mutantes lo exige (`&'static`), y la indireccion es la
/// misma que tendria el defecto en codigo de produccion: el codigo real
/// devuelve una referencia a algo que ya existe; un mutante que devuelve la
/// primera tiene que fabricarse una.
fn leaked(target: &ReleaseTarget) -> &'static ReleaseTarget {
    Box::leak(Box::new(target.clone()))
}

// ── M1: ante varios, gana el primero ──────────────────────────────────────

/// Rompe **una** ley, y solo esa.
///
/// La primera versión de este mutante rompía dos: además de devolver el
/// primero cuando hay varios, convertía «un target sin pedir» en ambiguo. Eso
/// lo hacía morir en la fila equivocada, y el `assert_kills` —que exige morir
/// en la fila que ejerce SU ley— lo detectó. Un mutante impreciso no es un
/// mutante: es ruido que desplaza la medición.
fn first_target_wins(
    targets: &[ReleaseTarget],
    selector: &TargetSelector,
) -> Result<&'static ReleaseTarget, TargetSelectionError> {
    if selector.requested().is_none() && targets.len() > 1 {
        return Ok(leaked(&targets[0]));
    }
    select_target(targets, selector).map(leaked)
}

#[test]
fn m1_first_target_wins_murió() {
    assert_kills(
        "first-target-wins",
        first_target_wins as Mutant,
        "dos targets sin pedir",
    );
}

// ── M2: un nombre que no casa cae al único ────────────────────────────────

fn named_falls_back_to_the_only_one(
    targets: &[ReleaseTarget],
    selector: &TargetSelector,
) -> Result<&'static ReleaseTarget, TargetSelectionError> {
    if let (Some(wanted), [only]) = (selector.requested(), targets) {
        // El defecto: «no hay otro, así que será este». Convierte el nombre en
        // decoración.
        if only.id() != wanted {
            return Ok(leaked(only));
        }
    }
    select_target(targets, selector).map(leaked)
}

#[test]
fn m2_named_falls_back_to_the_only_one_murió() {
    assert_kills(
        "named-falls-back-to-the-only-one",
        named_falls_back_to_the_only_one as Mutant,
        "un target, un nombre que no casa",
    );
}

// ── M3: lo desconocido se degrada a ambiguo ───────────────────────────────

fn unknown_becomes_ambiguous(
    targets: &[ReleaseTarget],
    selector: &TargetSelector,
) -> Result<&'static ReleaseTarget, TargetSelectionError> {
    match select_target(targets, selector) {
        Ok(target) => Ok(leaked(target)),
        // Solo esta degradación. La primera versión también degradaba
        // `NoTarget`, y con eso el mutante moría en la fila equivocada: quien
        // recibe «hay varios» busca un segundo producto, y quien recibe «no hay
        // ninguno» tiene que mirar el repositorio. Son reparaciones distintas y
        // una mutación que las mezcla no mide ninguna.
        Err(TargetSelectionError::UnknownTarget { .. }) => {
            Err(TargetSelectionError::AmbiguousTarget {
                candidates: targets.iter().map(|t| t.id().to_owned()).collect(),
                requested: selector.requested().map(str::to_owned),
            })
        }
        Err(other) => Err(other),
    }
}

#[test]
fn m3_unknown_becomes_ambiguous_murió() {
    assert_kills(
        "unknown-becomes-ambiguous",
        unknown_becomes_ambiguous as Mutant,
        "un target, un nombre que no casa",
    );
}

// ---------------------------------------------------------------------------
// El veredicto
// ---------------------------------------------------------------------------

/// Corre el código CORRECTO por las siete filas y exige lo declarado.
///
/// Sin esta comprobación previa, un mutante podría «morir» por un motivo
/// equivocado —porque la fila está mal escrita, no porque la ley se Rompió— y
/// el falsadorwould dar verde habiéndose roto a sí mismo. Es la tercera vez
/// que este repo encuentra esa forma de autoengaño en un falsador.
#[test]
fn la_tabla_declara_lo_que_la_ley_exige() {
    for case in cases() {
        let actual = correct(case.setup, case.selector);
        assert_eq!(
            actual.split(' ').next().unwrap_or(""),
            case.expected,
            "fila «{}»: la ley dice {} y el codigo dice {actual}",
            case.name,
            case.expected
        );
    }
}

/// ## Por qué el mutante tiene que morir en UNA fila, no en todas
///
/// La primera versión de esta función exigía que el mutante se diferenciara
/// en las siete filas. Es imposible: un mutante rompe **una** ley, y en las
/// filas que no ejercitan esa ley tiene que coincidir con el código correcto —
/// si no, estaría «roto» en una forma que nadie ha declarado. Un `assert_ne`
/// sobre todas las filas convierte un mutante real en un fallo de compilación
/// disfrazado de falsación.
///
/// Lo que sí se exige, y es más fuerte que «difiere en alguna fila»: que muera
/// **en la fila que ejerce su ley**, y que no muera antes. Si un mutante
/// revienta una fila distinta de la suya, la ley se ha movido y el falsador
/// tiene que enterarse, no reescribirse.
///
/// Es el mismo error que ya se corrigió una vez en el falsador del reducer, y
/// haberlo reintroducido deja claro que el defecto no era del falsador sino
/// del hábito de exigir más de lo que se sabe.
fn assert_kills(label: &str, mutant: Mutant, expected_row: &str) {
    let rows = cases();
    let mut died_at: Option<&str> = None;

    for case in &rows {
        let fixture = (case.setup)();
        let selector = (case.selector)();
        let expected = correct(case.setup, case.selector);
        let actual = shape(mutant(&fixture.all, &selector));
        if actual == expected {
            continue;
        }
        assert_eq!(
            case.name,
            expected_row,
            "el mutante «{label}» revienta la fila «{row}», que no es la suya. \
             Su ley se ejerce en «{expected_row}» (la ley dice {expected} y el \
             mutante contesta {actual}). O la mutacion cambio de ley, o la tabla \
             esta mal escrita — y en los dos casos el falsador no esta midiendo \
             lo que dice medir.",
            row = case.name,
        );
        died_at = Some(case.name);
        break;
    }

    let died_at = died_at.expect(
        "el mutante contesta lo mismo que el codigo correcto en las siete filas: \
         no hay ninguna que distinga la ley que rompio, luego esa ley no esta medida",
    );
    assert!(
        rows.iter().any(|c| c.name == died_at),
        "el mutante «{label}» murio en una fila que no esta en la tabla"
    );
}
