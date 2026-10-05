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

use sddk_engine::version::{product_version_of_release_ref, version};

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

/// La convención de la release reference es un prefijo, y solo un prefijo.
///
/// Este criterio no va de tecnología sino de lo que más se parece: una
/// **coerción**. Un recorte es cómo dos valores distintos acaban presentándose
/// como el mismo, y un release autorizado sobre un número que no es el del
/// proyecto es la peor clase de verde. Se fija aquí, en el sitio donde vive,
/// para que nadie añada un segundo recorte al lado sin notar que hay uno.
#[test]
fn la_convencion_de_la_release_ref_es_un_prefijo_y_nada_mas() {
    assert_eq!(product_version_of_release_ref("v1.2.3"), "1.2.3");
    assert_eq!(product_version_of_release_ref("1.2.3"), "1.2.3");

    // Una referencia de candidata: se le quita el prefijo como a cualquier
    // otra, y el `-rc2` se queda. Ese es el punto — una candidata NO autoriza
    // una release estable, y la unica forma de que lo hiciera seria recortar
    // el sufijo para que las dos cadenas coincidieran.
    assert_eq!(product_version_of_release_ref("v1.2.3-rc2"), "1.2.3-rc2");
    assert_ne!(
        product_version_of_release_ref("v1.2.3-rc2"),
        "1.2.3",
        "una candidata no puede autorizarse como la version del producto"
    );

    // Lo que NO empieza por `v` no se toca, ni se busca un `v` por dentro.
    for untouched in ["1.2.3+build.7", "release-1.2.3", "1.2.3v", "V1.2.3"] {
        assert_eq!(
            product_version_of_release_ref(untouched),
            untouched,
            "«{untouched}» no es la version con un prefijo `v`: recortarlo seria \
             decidir que dos valores distintos son el mismo"
        );
    }

    // Y la ley que de verdad importa, sobre todo lo que lleva una `v` cerca y
    // no es la version: NINGUNA se convierte en `1.2.3`. Un recorte mas —un
    // sufijo de candidata, un `=`, espacios— haria que dos valores distintos
    // se presentaran como el mismo, y eso autoriza un release sobre un numero
    // que no es el del proyecto.
    for not_the_version in [
        "v1.2.3-rc2",
        "v1.2.3+build.7",
        "v1.2.4",
        "v 1.2.3",
        "vv1.2.3",
        "v1.2",
    ] {
        assert_ne!(
            product_version_of_release_ref(not_the_version),
            "1.2.3",
            "«{not_the_version}» no es la version del producto y no puede \
            (authorizarse) como si lo fuera"
        );
    }
    // El prefijo doble deja un `v` que NO se vuelve a quitar: la convencion es
    // una, no una normalizacion.
    assert_eq!(product_version_of_release_ref("vv1.2.3"), "v1.2.3");
}

#[test]
fn el_version_del_tool_no_es_el_del_producto() {
    // Documentado, no comparado: este módulo nunca confronta la versión de
    // la herramienta con la del producto que se publica.
    assert!(!version().is_empty());
    assert_eq!(version(), env!("CARGO_PKG_VERSION"));
}
