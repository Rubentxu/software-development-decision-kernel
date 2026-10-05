//! Fitness for the version path of the engine: the crate that decides names
//! no technology.
//!
//! # Why this exists, and why a comment would not do
//!
//! ADR-0153 moved the manifest names out of `ensure_version_lockstep` and into
//! a data registry, and left a gate that checked the *reader* did not name
//! them. That was the right check for the design of the time, and it is now
//! strictly weaker than the property: the registry itself sat inside the crate
//! that decides, and a registry is a list of `Cargo.toml`, `package.json`,
//! `gradle.properties`... A gate that scans the reader while the registry sits
//! two hundred lines up in the same file certifies a distinction nobody draws
//! at the call site.
//!
//! So the check moves up a level: **the module that resolves a product's
//! version does not name a concrete technology at all.** Not in code, not in a
//! registry, not in a comment. The names now live in
//! `crates/sddk-gateway/src/version_provider.rs`, which is allowed to know them
//! because its job is to ask a concrete tool a concrete question.
//!
//! # Why it is a source scan and not a behaviour test
//!
//! Because a behaviour test cannot see it. A provider that hardcoded a
//! preference — reading one file first and falling back to another — behaves
//! *identically* on every fixture built from the registry, and differs only on
//! a layout nobody wrote down. The only thing that catches it is reading the
//! source, and a fitness that is only a comment is a comment that will be
//! wrong in six months.

use sddk_domain::channel::ReleaseChannel;
use sddk_domain::release_ref::{CandidateSequence, ReleaseRef, VersionNaming, binds};
use sddk_domain::version_authority::{
    ProductVersion, ReleaseTarget, VersionAuthority, VersionResolverRegistry,
};
use sddk_engine::version::{VersionLockstepError, ensure_release_ref_lockstep, version};

/// La versión del test, por el constructor que declara la identidad.
fn product_version(value: &str) -> ProductVersion {
    ProductVersion::new(value).expect("version valida en el test")
}

/// ¿El módulo que decide fabrica alguna convención de nombres?
///
/// Un default no es una función que **reciba** una naming y la use: es una
/// función que la **devuelve** sin que nadie se la pidiera. Se busca la
/// segunda, que es la que hace que la primera sea opcional en la practica.
fn el_modulo_fabrica_una_naming() -> bool {
    include_str!("../src/version.rs")
        .lines()
        .map(str::trim)
        .any(fabrica_una_naming)
}

/// Una línea declara una fábrica de namings cuando su tipo de retorno **es**
/// `VersionNaming`, y no una cosa que lo contenga.
///
/// El tipo se mira **desenvuelto** y no con un `contains`: la primera versión
/// buscaba `-> VersionNaming` en la línea entera, y eso marcaba también
/// `-> Option<VersionNaming>` — que es una función capaz de decir «no tengo
/// ninguna», justo lo **opuesto** de tener un default. Es el segundo falso
/// positivo de la misma familia en este mismo bloque, y el motivo de que los
/// dos escáneres comparen el tipo y no la línea.
fn fabrica_una_naming(linea: &str) -> bool {
    if !linea.contains("pub fn ") && !linea.contains("const fn ") {
        return false;
    }
    let Some((_, retorno)) = linea.split_once("->") else {
        return false;
    };
    // Se corta en la llave de apertura del cuerpo: `-> VersionNaming {` y
    // `-> VersionNaming` son el mismo tipo escrito de dos maneras.
    let retorno = retorno.trim();
    let retorno = retorno.split('{').next().unwrap_or(retorno).trim();
    retorno.starts_with("VersionNaming")
}

/// El control: el escaner ve una fábrica si la hay, y no la ve si no lo es.
///
/// Sin el caso positivo, el `!el_modulo_fabrica_una_naming()` de arriba podría
/// pasar porque el escaner no mira nada. Sin el **negativo**, no demuestra que
/// sepa cuándo NO disparar — y un guard que solo tiene el caso positivo es la
/// forma mas comun de que un fitness de source-level se apague sin que nadie lo
/// note.
#[test]
fn el_escaner_de_naming_distingue_fabricar_de_devolver_opcional() {
    assert!(
        fabrica_una_naming("pub fn default_version_naming() -> VersionNaming {"),
        "una fabrica de defaults tiene que verse: es justo lo que este \
         criterio prohibe"
    );
    assert!(
        !fabrica_una_naming("pub fn naming_de(&self) -> Option<VersionNaming> {"),
        "devolver Option<VersionNaming> es poder no tener ninguna, que es lo \
         contrario que tener un default"
    );
    assert!(!fabrica_una_naming("pub fn usa(naming: &VersionNaming) {"));
}

/// Names that must never appear in the module that decides.
///
/// Literal substrings, on purpose: the failure this guards against is somebody
/// writing the name once in a comment and nobody objecting. `manifest` is
/// deliberately absent — it is an ordinary English word, and a fitness that
/// banned ordinary English would push people to evade it rather than comply.
///
/// The list is shared in substance with the domain's fitness. It is written out
/// again rather than imported because a shared constant would let one crate
/// weaken the other's check by editing it once, and the two crates are the two
/// ends of the property being asserted.
const FORBIDDEN: &[&str] = &[
    "rust",
    "python",
    "kotlin",
    "groovy",
    "java",
    "typescript",
    "javascript",
    "golang",
    "csharp",
    "gradle",
    "maven",
    "cargo",
    "npm",
    "yarn",
    "pnpm",
    "bazel",
    "cmake",
    "msbuild",
    "dotnet",
    "pip",
    "poetry",
    "cargo.toml",
    "package.json",
    "pyproject.toml",
    "gradle.properties",
    "build.gradle",
    "pom.xml",
    "directory.build.props",
    "cmakelists.txt",
    "go.mod",
    "module.bazel",
    "version-source",
    "jvm_gradle",
    "cpp_cmake",
];

/// ## La ÚNICA excepción, y por qué está escrita
///
/// `CARGO_PKG_VERSION` es una variable de compilación **de esta
/// herramienta**, no una descripción de un proyecto que se publica. El
/// fitness no puede forbiddingla sinecke poder decir la verdad sobre de dónde
/// sale la versión del propio binario, y un guard que prohíbe sin dejar salida
/// empuja al primero que llega a escribir una falsehood — la lección que
/// ADR-0155 ya escribió para otro puerto, y que aquí se repite porque el
/// patrón es el mismo.
///
/// No es una puerta trasera porque la lista de excepciones **no puede crecer**:
/// el test de abajo exige que toda entrada sea una variable de compilación de
/// este crate. Añadir `Cargo.toml` ahí es tan imposible como saltarse el
/// fitness, y sale a la vista.
const ALLOWED: &[&str] = &["CARGO_PKG_VERSION"];

/// Busca `needle` como PALABRA, no como subcadena.
///
/// ## El defecto que esto arregla, medido
///
/// La primera versión de este escáner usaba `contains`, yMEDIDO sobre el
/// propio módulo: marcaba `rust` dentro de **«trusted»**, `pip` dentro de
/// **«pipeline»** y `cargo` dentro del nombre de la macro de compilación. Tres
/// rojos que no eran del defecto que el guard vigila — y un guard que produce
/// rojos falsos entrena a su lector a ignorarlo, que es como un guard
/// desactivado se parece a uno que pasa.
///
/// El mismo defecto estaba en el escáner del dominio, escrito en el mismo
/// bloque. Los dos están corregidos con la misma regla.
fn find_word(haystack: &str, needle: &str) -> bool {
    let bytes = haystack.as_bytes();
    let target = needle.as_bytes();
    if target.is_empty() || bytes.len() < target.len() {
        return false;
    }
    for start in 0..=(bytes.len() - target.len()) {
        if &bytes[start..start + target.len()] != target {
            continue;
        }
        let before_ok = start == 0 || !is_word_byte(bytes[start - 1]);
        let after = start + target.len();
        let after_ok = after == bytes.len() || !is_word_byte(bytes[after]);
        if before_ok && after_ok {
            return true;
        }
    }
    false
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'.' || byte == b'-'
}

#[test]
fn the_version_resolution_module_names_no_concrete_technology() {
    let mut source = include_str!("../src/version.rs").to_lowercase();
    // Las excepciones se retiran ANTES de escanear, y se retiran enteras: un
    // nombre permitido no puede dejar un resto pegado que se detecte despues
    // como si fuera tecnologia.
    for allowed in ALLOWED {
        source = source.replace(&allowed.to_lowercase(), " ");
    }
    let hits: Vec<&str> = FORBIDDEN
        .iter()
        .copied()
        .filter(|needle| find_word(&source, needle))
        .collect();
    assert!(
        hits.is_empty(),
        "el modulo que resuelve la version de un producto nombra tecnologia \
         concreta: {hits:?}. El kernel hace la pregunta; los providers saben \
         donde mirar. Si el nombre hace falta, el provider lo dice, no el motor."
    );
}

/// La excepción es una, y es una variable de compilación de este crate.
///
/// Sin este criterio, `ALLOWED` es un agujero con forma de lista: la vía más
/// fácil de apagar el fitness es añadirle el nombre que molesta. Aquí eso
/// exige que el nombre **acabe en `_PKG_VERSION`**, y decir por qué.
#[test]
fn la_excepcion_no_puede_crecer_hasta_ser_un_agujero() {
    for allowed in ALLOWED {
        assert!(
            allowed.ends_with("_PKG_VERSION"),
            "`{allowed}` esta en la lista de excepciones del fitness y no es una \
             variable de compilacion de este crate. La lista existe para que el \
             modulo pueda decir de donde sale la version DEL TOOL, no para que \
             pueda nombrar la tecnologia de un producto."
        );
    }
}

/// ## El fitness tiene que poder ver un nombre
///
/// Un escáner que no puede fallar no es un guard: es una decoracion. Este
/// test lee su propia lista, la busca en un texto que la contiene, y exige que
/// la encuentre. Si alguien vacía la lista —por ejemplo para que el rojo
/// desaparezca— este test se cae antes que el otro, y dice por qué.
#[test]
fn the_fitness_scanner_can_actually_see_a_name() {
    let decoy = "una linea que menciona Cargo.toml y gradle.properties y go.mod";
    let lowered = decoy.to_lowercase();
    let seen: Vec<&str> = FORBIDDEN
        .iter()
        .copied()
        .filter(|needle| find_word(&lowered, needle))
        .collect();
    assert!(
        seen.contains(&"cargo.toml")
            && seen.contains(&"gradle.properties")
            && seen.contains(&"go.mod"),
        "el escaner no ve los nombres que dice vigilar, y por tanto el criterio \
         anterior no mide nada: solo encontro {seen:?}"
    );
    // Y que la coincidencia sea de PALABRA, no de subcadena: este texto no
    // nombra ninguna tecnologia, y un escaner de subcadenas lo declararia
    // infringement.
    let innocent = "el pipeline trustworthy comprueba el trust de la release";
    let false_positives: Vec<&str> = FORBIDDEN
        .iter()
        .copied()
        .filter(|needle| find_word(innocent, needle))
        .collect();
    assert!(
        false_positives.is_empty(),
        "el escaner marca palabras inglesas como si fueran tecnologia: \
         {false_positives:?}"
    );
}

/// La relación entre una release reference y una versión es **declarada**.
///
/// ## Qué cambió respecto a la versión anterior de este criterio
///
/// Antes decíamos que la convención «es un prefijo y solo un prefijo» y lo
/// medíamos con un recorte: `product_version_of_release_ref("v1.2.3") ==
/// "1.2.3"`. Esa función ha desaparecido, y su desaparición **es** el
/// criterio. Un recorte no es una convención, es una coerción: quita texto a
/// una referencia y llama al resultado «la versión», de modo que
/// `v1.2.3-rc2` y `v1.2.3`.authorizan la misma release.
///
/// Ahora la convención es un valor —`default_version_naming()`— y la
/// relación se comprueba por construcción: la naming **construye** el nombre
/// que la versión tendría, y las dos cadenas se comparan. Eso no solo quita el
/// recorte: además hace que el prefijo sea **obligatorio**, que antes era
/// opcional de facto. Un proyecto sin `v` que antes pasaba ahora tiene que
/// declarar `VersionNaming::Exact`.
///
/// ## Por qué aquí y no solo en el dominio
///
/// Porque la pieza que se puede quebrar en este crate no es la comparación —esa
/// vive en el dominio— sino el **valor por defecto** que el motor pasa. Si
/// `default_version_naming()` volviera a ser un recorte, el dominio seguiría
/// en verde y el motor volvería a coercionar. Este criterio mira las tres
/// cosas que el motor decide: que la convención **entre declarada** y no
/// impuesta, el canal que se le pasa a una referencia, y qué mensaje ve quien
/// recibe un `Err`.
#[test]
fn la_relacion_entre_referencia_y_version_es_declarada_y_no_recortada() {
    // 1. El motor NO tiene default. Que la relacion se compruebe por construccion
    //    solo es verdad si la convencion la trae quien llama: un default aqui
    //    seria el recorte de vuelta, con la diferencia de que ahora estaria
    //    invisible. Se mide por la ausencia de la firma, no por un valor.
    assert!(
        !el_modulo_fabrica_una_naming(),
        "el modulo de version vuelve a tener una naming por defecto: con ella \
         presente, «declarada» es una palabra y el recorte vuelve a existir, \
         solo que escondido en una constante"
    );

    // 2. Y lo que esa naming declara es un prefijo EXACTO, en las dos
    //    direcciones. La fila `1.2.3` es el cambio de comportamiento real: antes
    //    pasaba sin `v` porque el prefijo era opcional de facto; ahora hay que
    //    declararlo.
    let naming = VersionNaming::v_prefixed();
    assert!(
        binds(
            &ReleaseRef::new("v1.2.3", ReleaseChannel::Stable),
            &product_version("1.2.3"),
            &naming
        )
        .is_bound(),
        "un release estable con la convencion por defecto nombra a la version"
    );
    assert!(
        !binds(
            &ReleaseRef::new("1.2.3", ReleaseChannel::Stable),
            &product_version("1.2.3"),
            &naming
        )
        .is_bound(),
        "sin el prefijo declarado ya no nombra: un prefijo opcional es una \
         sugerencia, y una sugerencia no puede autorizar un release"
    );

    // 3. Y una candidata NO nombra a la version de su producto. Esto es lo que
    //    el recorte de sufijo hacia, y lo que hace que una candidata no pueda
    //    autorizarse como release estable. Antes esta fila la cubria `assert_ne`
    //    sobre una cadena recortada; ahora la cubre la relacion entera.
    let candidata = ReleaseRef::candidate(
        "v1.2.3-rc2",
        ReleaseChannel::Candidate,
        CandidateSequence::nth(2).expect("la segunda candidata existe"),
    );
    assert!(
        !binds(&candidata, &product_version("1.2.3"), &naming).is_bound(),
        "una candidata no nombra a la version de su producto: solo podria \
         hacerlo recortando el sufijo, que es la coercion que este criterio \
         lleva prohibiendo desde antes de existir la naming"
    );

    // 4. NINGUNA de estas se convierte en `1.2.3` bajo la naming por defecto.
    //    Mismo conjunto que antes, misma ley, y ahora sobre la relacion y no
    //    sobre una cadena recortada.
    for referencia in [
        "v1.2.3-rc2",
        "v1.2.3+build.7",
        "v1.2.4",
        "v 1.2.3",
        "vv1.2.3",
        "v1.2",
        "1.2.3",
        "release-1.2.3",
        "1.2.3v",
        "V1.2.3",
    ] {
        assert!(
            !binds(
                &ReleaseRef::new(referencia, ReleaseChannel::Stable),
                &product_version("1.2.3"),
                &naming
            )
            .is_bound(),
            "«{referencia}» no es la version del producto y no puede \
             autorizarse como si lo fuera"
        );
    }
}

/// El motor pasa un **canal declarado**, nunca uno leído del nombre.
///
/// Una referencia cuyo canal hay que adivinar por su texto es una referencia
/// cuyo canal nadie declaró, y el bloque entero consiste en no adivinar. Aquí
/// se mide la mitad observable: la función que decide recibe un `&ReleaseRef`,
/// y la de convenience construye ese `ReleaseRef` con un canal explícito en
/// lugar de deducido.
#[test]
fn el_motor_recibe_una_referencia_con_canal_y_no_una_cadena() {
    // La firma que decide es la que lleva `ReleaseRef`: canal y secuencia son
    // entradas, no cosas que se lean del nombre.
    let decide: fn(
        &VersionResolverRegistry,
        &ReleaseTarget,
        &ReleaseRef,
        &VersionNaming,
    ) -> Result<VersionAuthority, VersionLockstepError> = ensure_release_ref_lockstep;

    // Un release estable con la convencion por defecto es la unica combinacion
    // que la convenience admite, y la construye en vez de deducirla.
    let estable = ReleaseRef::new("v1.2.3", ReleaseChannel::Stable);
    assert_eq!(estable.channel(), ReleaseChannel::Stable);
    assert!(!estable.is_candidate());

    // Y una candidata entra por la otra firma, con su secuencia declarada. Si
    // estas dos funciones se confunden —si la convenience aceptara candidatas
    // como si fueran estables— el candidato pasaria por el camino que declara
    // `Stable` y la distinction entre candidata y estable seria solo textual.
    let candidata = ReleaseRef::candidate(
        "v1.2.3-rc2",
        ReleaseChannel::Candidate,
        CandidateSequence::nth(2).expect("la segunda candidata existe"),
    );
    assert_eq!(candidata.channel(), ReleaseChannel::Candidate);
    assert!(candidata.is_candidate());
    assert_eq!(
        candidata.sequence(),
        Some(CandidateSequence::nth(2).expect("existe")),
        "la secuencia se conserva porque se declaro, no porque este en el nombre"
    );

    // Las dos firmas coexisten y no se solapan: la primera acepta cualquier
    // referencia, la segunda no. Que ambas compilen es la prueba de que la
    // convenience sigue siendo un wrapper y no un atajo con reglas propias.
    let _ = decide;
}

/// El mensaje de un rechazo dice **los dos lados**.
///
/// Antes el `Err` traía `workspace_version` y `release_ref`, y quien lo leía
/// tenía que adivinar cuál de los dos estaba mal. Con la relación declarada, el
/// nombre que la naming da a la versión es calculable, así que el mensaje puede
/// enseñarlo: la corrección pasa de ser una adivinanza a ser mecánica.
#[test]
fn un_rechazo_dice_el_nombre_esperado_y_el_encontrado() {
    let naming = VersionNaming::v_prefixed();
    let version = product_version("1.2.3");
    let desajuste = binds(
        &ReleaseRef::new("v9.9.9", ReleaseChannel::Stable),
        &version,
        &naming,
    );
    let mensaje = desajuste.message(&version);
    assert!(
        mensaje.contains("v1.2.3"),
        "el mensaje dice el nombre que la naming daria a la version: {mensaje}"
    );
    assert!(
        mensaje.contains("v9.9.9"),
        "y dice el nombre que realmente traia la referencia: {mensaje}"
    );
}

#[test]
fn el_version_del_tool_no_es_el_del_producto() {
    // Documentado, no comparado: este módulo nunca confronta la versión de
    // la herramienta con la del producto que se publica.
    assert!(!version().is_empty());
    assert_eq!(version(), env!("CARGO_PKG_VERSION"));
}
