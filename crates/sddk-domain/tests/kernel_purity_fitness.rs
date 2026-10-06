//! Fitness del nucleo: la ley de pureza, vigilada en los 46 modulos.
//!
//! ## La ley
//!
//! *El nucleo define las preguntas; los providers saben obtener la evidencia.*
//! Un nombre concreto de lenguaje, build system o fichero de manifiesto dentro
//! del dominio ata el nucleo a un ecosistema, y el dano no aparece como error:
//! aparece como una frase en un doc-comment que nadie discute.
//!
//! ## Por que este fichero existe, y no una linea mas en el fitness anterior
//!
//! El fitness que hay escanea `include_str!("../src/version_authority.rs")`:
//! **un** modulo de **cuarenta y seis**. Los otros 45 no los miraba nadie, y el
//! defecto que este guard vigila ya se publico **dos veces** justamente asi: el
//! nombre aparecio en el fichero que se estaba discutiendo, y el fichero que se
//! estaba discutiendo era el unico vigilado. Haberlo escrito en otro sitio habria
//! sido invisible tanto como escribirlo en este.
//!
//! ## Las dos zonas, y por que el guard tiene que distinguirlas
//!
//! En `test_adapters.rs`, `test_apply.rs` y `test_model.rs` el vocabulario
//! aparece 41 veces mas, y **ninguna viola la ley**: estan dentro de
//! `#[cfg(test)]` y construyen el input rechazado o el fixture.
//!
//! - **Produccion (codigo, doc-comments, cadenas):** nombrar una herramienta
//!   **enseña**. El lector del nucleo aprende que hay un ecosistema. Viola.
//! - **`#[cfg(test)]`:** nombrar una herramienta **prueba el rechazo**. No entra
//!   en el artefacto y no decide nada. No viola.
//!
//! Ambas son «la palabra aparece». Solo una rompe la ley, asi que un guard que
//! no distingue solo puede callarse o gritar. Y gritar entrena a su lector a
//! ignorarlo.
//!
//! ## El fallo que este guard puede cometer, y con nombre
//!
//! **Una exclusion por zona es un agujero con forma de guard.** Si el criterio
//! de «esto es zona de test» admite un `#[cfg(test)]` pegado a un item de
//! produccion, el guard se desconecta **anadiendo una linea** y queda verde con
//! el veto desenchufado. El caso ya es real en el corpus: `test_apply.rs:33`
//! tiene `#[cfg(test)] use crate::test_select::ImpactPlannerV1;` — que **no es
//! un modulo**. La regla ingenua «del primer `#[cfg(test)]` al final del
//! fichero» habria eximido alli 592 lineas de produccion. Los controles de este
//! fichero existen, sobre todo, para que ese agujero no se pueda abrir sin que
//! algo se ponga rojo.
//!
//! ## Y por que hay un lexer aqui
//!
//! Por la misma razon, en negativo: el final de una zona se decide por llaves,
//! y las llaves hay que contarlas **fuera** de cadenas y comentarios. En el
//! corpus hay 379 lineas con llaves dentro de cadenas, raw strings que abren con
//! una llave en la misma linea (`let fixture_json = r#"{`), y 55 literales de
//! caracter con la ambigüedad de los lifetimes. Un conteo ingenuo se desincroniza
//! a la primera. Y **desincronizarse en la direccion de "creo que estoy dentro
//! de una cadena" es ceguera**, que es el unico fallo que este guard no puede
//! permitirse.
//!
//! La mascara que se calcula abajo conserva el numero de bytes de la entrada, de
//! modo que un indice en la mascara y un indice en el original son el mismo
//! indice, y una linea es la misma linea antes y despues de enmascarar.

use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// El vocabulario
// ---------------------------------------------------------------------------

/// Nombres que el modulo que decide nunca debe escribir.
///
/// Escritos como literales y no como un patron ingenioso, porque el fallo que
/// este guard vigila es que alguien escriba el nombre una vez en un comentario y
/// quien revisa no lo objete. `manifest` esta deliberadamente ausente: es una
/// palabra generica, y un fitness que prohibiera ingles corriente empujaria a
/// esquivarlo en vez de a cumplirlo.
///
/// Este listado es el unico: el fitness anterior traia su propia copia y dos
/// copias de un vocabulario son dos autoridades para la misma ley, luego se
/// elimino aquella en vez de dejar las dos.
const FORBIDDEN: &[&str] = &[
    // languages
    "rust",
    "python",
    "kotlin",
    "groovy",
    "java",
    "typescript",
    "javascript",
    "go_lang",
    "golang",
    "csharp",
    "c_plus_plus", // build systems
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
    "poetry", // concrete files
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
    // ecosystems as the kernel would name them
    "jvm_gradle",
    "cpp_cmake",
];

/// Un nombre encontrado por el guard: donde y cual.
#[derive(Debug, PartialEq, Eq)]
struct Hit {
    line: usize,
    needle: &'static str,
    context: String,
}

// ---------------------------------------------------------------------------
// 1. La mascara: donde NO se pueden contar llaves
// ---------------------------------------------------------------------------

/// Sustituye por espacios todo lo que no es codigo, **conservando la longitud**.
///
/// Que conserve la longitud es la propiedad de la que depende todo lo demas: con
/// mascaras de otra longitud, el indice de la mascara y el del original son
/// numeros distintos y la linea de un hallazgo deja de ser la linea donde esta
/// escrito. Aqui son el mismo indice, y por eso `line_of` vale igual sobre la
/// entrada y sobre la mascara.
///
/// Lo que se enmascara: comentarios de linea, comentarios de bloque (anidados,
/// que Rust permite), cadenas normales, cadenas crudas —cualquier numero de
/// `#`— y literales de caracter.
///
/// Lo que NO se enmascara, a proposito: los doc-comments. Nombrar una
/// herramienta en un doc-comment es justamente el defecto que este guard
/// vigila, asi que enmascararlos seria tapar el fallo.
fn mask(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut out = vec![b' '; bytes.len()];
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        // Comentarios de linea: `//`, `///`, `//!`
        if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        // Comentarios de bloque, anidados.
        if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            let mut depth = 1usize;
            i += 2;
            while i < bytes.len() && depth > 0 {
                if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
                    depth += 1;
                    i += 2;
                } else if bytes[i] == b'*' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                    depth -= 1;
                    i += 2;
                } else {
                    if bytes[i] != b'\n' {
                        out[i] = b'\n';
                    }
                    i += 1;
                }
            }
            continue;
        }
        // Cadenas crudas: r"", r#"..#, r##"..##
        if b == b'r' {
            let mut j = i + 1;
            let mut hashes = 0usize;
            while j < bytes.len() && bytes[j] == b'#' {
                hashes += 1;
                j += 1;
            }
            if j < bytes.len()
                && bytes[j] == b'"'
                && let Some(end) = close_raw(bytes, j + 1, hashes)
            {
                for (k, slot) in out.iter_mut().enumerate().take(end).skip(i) {
                    if bytes[k] != b'\n' {
                        *slot = b'\n';
                    }
                }
                i = end;
                continue;
            }
            out[i] = b;
            i += 1;
            continue;
        }
        // Cadenas normales con escapes.
        if b == b'"' {
            let mut j = i + 1;
            while j < bytes.len() {
                if bytes[j] == b'\\' {
                    j += 2;
                    continue;
                }
                if bytes[j] == b'"' {
                    j += 1;
                    break;
                }
                j += 1;
            }
            for (k, slot) in out.iter_mut().enumerate().take(j.min(bytes.len())).skip(i) {
                if bytes[k] != b'\n' {
                    *slot = b'\n';
                }
            }
            i = j;
            continue;
        }
        // Literales de caracter. La ambiguedad con los lifetimes se resuelve
        // mirando lo que viene despues: un lifetime sigue con un identificador,
        // un literal de caracter se cierra con una comilla.
        if b == b'\''
            && let Some(len) = char_literal_len(bytes, i)
        {
            for slot in out.iter_mut().take(i + len).skip(i) {
                *slot = b'x';
            }
            i += len;
            continue;
        }
        out[i] = b;
        i += 1;
    }
    // Los saltos de linea se conservan: son la unica estructura que `line_of`
    // necesita y no son codigo ni ruido. Se reconstruyen en una pasada aparte
    // para no tener que mutar la mascara en sitio, que en Rust exigiria `unsafe`.
    let mut final_bytes = Vec::with_capacity(bytes.len());
    for (k, masked_byte) in out.iter().enumerate() {
        final_bytes.push(if bytes[k] == b'\n' {
            b'\n'
        } else {
            *masked_byte
        });
    }
    String::from_utf8(final_bytes).expect("la mascara solo sustituye por ASCII")
}

/// Longitud del literal de caracter que empieza en `start`, o `None` si lo que
/// hay ahi es un lifetime (`'a`) y no un literal.
fn char_literal_len(bytes: &[u8], start: usize) -> Option<usize> {
    match bytes.get(start + 1) {
        Some(b'\\') => {
            // Escape: puede medir 2 (`'\\'`) o 4 (`'\u{1F}'`).
            if bytes.get(start + 2) == Some(&b'u') && bytes.get(start + 3) == Some(&b'{') {
                let close = bytes[start + 4..].iter().position(|b| *b == b'}')?;
                if bytes.get(start + 5 + close + 1) == Some(&b'\'') {
                    Some(5 + close + 2)
                } else {
                    None
                }
            } else if bytes.get(start + 3) == Some(&b'\'') {
                Some(4)
            } else {
                None
            }
        }
        Some(_) => {
            // Un byte (o varios UTF-8) y cierre.
            let mut j = start + 1;
            while j < bytes.len() && (bytes[j] & 0xC0) == 0x80 {
                j += 1;
            }
            if bytes.get(j + 1) == Some(&b'\'') {
                Some(j + 2 - start)
            } else {
                None
            }
        }
        None => None,
    }
}

/// Devuelve el indice justo despues del cierre de una cadena cruda que abre su
/// comilla en `quote`, con `hashes` almohadillas.
fn close_raw(bytes: &[u8], mut i: usize, hashes: usize) -> Option<usize> {
    while i < bytes.len() {
        if bytes[i] == b'"' {
            let mut seen = 0usize;
            while i + 1 + seen < bytes.len() && bytes[i + 1 + seen] == b'#' && seen < hashes {
                seen += 1;
            }
            if seen == hashes {
                return Some(i + 1 + hashes);
            }
        }
        i += 1;
    }
    None
}

// ---------------------------------------------------------------------------
// 2. Las zonas de test
// ---------------------------------------------------------------------------

/// Par de lineas (1-based, inclusivas) que cae dentro de un item `#[cfg(test)]`.
type Zone = (usize, usize);

/// Localiza las zonas marcadas `#[cfg(test)]`.
///
/// Se decide por **llaves balanceadas sobre la mascara**, no por «hasta el final
/// del fichero» ni por «hasta la proxima llave sin sangrar». Los dos atajos se
/// mediron y los dos fallan en este corpus:
///
/// - «hasta el final del fichero» se come 592 lineas de produccion en
///   `test_apply.rs`, porque su primer `#[cfg(test)]` (linea 33) es un `use`, no
///   un modulo.
/// - «hasta la proxima cosa sin sangrar» corta antes de tiempo en
///   `test_adapters.rs`, donde el cuerpo de un JSON en cadena cruda va a columna
///   0. Cortar antes es el fallo que delata, no el que ciega —pero produce rojos
///   falsos sobre un fixture, que es como se entrena a un lector a ignorar un
///   guard.
///
/// `Err` es una zona que no se pudo determinar. El guard lo trata como fallo, no
/// como zona vacia: no se puede eximir lo que no se ha sabido mirar.
fn test_zones(source: &str) -> Result<Vec<Zone>, String> {
    let masked = mask(source);
    let bytes = masked.as_bytes();
    let mut zones = Vec::new();
    let mut cursor = 0usize;
    while let Some(rel) = find_bytes(&masked[cursor..], b"#[cfg(test)]") {
        let attr_start = cursor + rel;
        let mut i = attr_start + b"#[cfg(test)]".len();
        // Saltar atributos adicionales pegados al mismo item (`#[allow(..)]`)
        // y el espacio entre ellos.
        loop {
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if bytes.get(i) == Some(&b'#') && bytes.get(i + 1) == Some(&b'[') {
                let mut depth = 0usize;
                while i < bytes.len() {
                    match bytes[i] {
                        b'[' => depth += 1,
                        b']' => {
                            depth -= 1;
                            if depth == 0 {
                                i += 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                    i += 1;
                }
                continue;
            }
            break;
        }
        // El item al que se pega el atributo. Antes de buscar su llave hay que
        // descartar que no tenga cuerpo: `#[cfg(test)] use crate::algo::Ayuda;`
        // es un item legitimo y corriente, y si se le buscara `{` a lo bruto se
        // adoptaria la llave de la **primera funcion de produccion que hubiera
        // despues**, eximiendo su cuerpo entero. Eso no es un falso positivo: es
        // ceguera, y es el fallo que este guard no puede permitirse.
        let mut terminator = None;
        let mut scan = i;
        while scan < bytes.len() {
            match bytes[scan] {
                b'{' | b';' | b'}' => {
                    terminator = Some(bytes[scan]);
                    break;
                }
                b'\n' if scan + 1 < bytes.len() && bytes[scan + 1] == b'\n' => break,
                _ => scan += 1,
            }
        }
        if terminator != Some(b'{') {
            // Sin cuerpo: no hay zona que eximir. Un `use`, un `const`, un
            // `struct` de una linea. Se avanza al siguiente atributo.
            cursor = i;
            continue;
        }
        // La llave de apertura del item.
        while i < bytes.len() && bytes[i] != b'{' {
            i += 1;
        }
        let open = i;
        let mut depth = 0usize;
        while i < bytes.len() {
            match bytes[i] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        if depth != 0 {
            return Err(format!(
                "la zona `#[cfg(test)]` de la linea {} no cierra",
                line_of(source, attr_start)
            ));
        }
        zones.push((line_of(source, open), line_of(source, i)));
        cursor = i + 1;
    }
    Ok(zones)
}

fn find_bytes(haystack: &str, needle: &[u8]) -> Option<usize> {
    haystack
        .as_bytes()
        .windows(needle.len())
        .position(|w| w == needle)
}

/// Linea (1-based) del byte `offset`.
fn line_of(source: &str, offset: usize) -> usize {
    source.as_bytes()[..offset.min(source.len())]
        .iter()
        .filter(|b| **b == b'\n')
        .count()
        + 1
}

// ---------------------------------------------------------------------------
// 3. El guard
// ---------------------------------------------------------------------------

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'.' || byte == b'-'
}

/// Busca `needle` como **PALABRA**, no como subcadena, y devuelve todas las
/// posiciones.
///
/// ## El defecto que esto arregla, medido en el escaner hermano
///
/// La primera version usaba `contains`. Al aplicarlo al modulo del motor —que si
/// tenia las tres palabras en su prosa— dio tres rojos que no eran del defecto
/// que el guard vigila: `rust` dentro de «trusted», `pip` dentro de «pipeline» y
/// `cargo` dentro del nombre de una variable de compilacion. El defecto estaba
/// en el instrumento, no en el codigo, y se anulo aqui antes de que apareciera
/// por azar.
///
/// Un guard que produce rojos falsos entrena a su lector a ignorarlo, que es
/// como un guard desactivado se parece a uno que pasa.
fn find_word_positions(haystack: &str, needle: &str) -> Vec<usize> {
    let bytes = haystack.as_bytes();
    let target = needle.as_bytes();
    let mut hits = Vec::new();
    if target.is_empty() || bytes.len() < target.len() {
        return hits;
    }
    for start in 0..=(bytes.len() - target.len()) {
        if &bytes[start..start + target.len()] != target {
            continue;
        }
        let before_ok = start == 0 || !is_word_byte(bytes[start - 1]);
        let after = start + target.len();
        let after_ok = after == bytes.len() || !is_word_byte(bytes[after]);
        if before_ok && after_ok {
            hits.push(start);
        }
    }
    hits
}

/// Los nombres prohibidos que aparecen **fuera** de toda zona `#[cfg(test)]`.
///
/// `Err` cuando una zona no se pudo determinar. Es deliberado: la ley se puede
/// eximir, pero solo en una zona que se ha podido leer entera.
fn violations(source: &str) -> Result<Vec<Hit>, String> {
    let zones = test_zones(source)?;
    // Se busca en la version en minusculas porque el nombre de una tecnologia
    // aparece en cualquier caja, y `Cargo.toml` en un doc-comment cuenta igual
    // que `cargo.toml` en codigo. Sin esto el guard se escapa de la mitad de las
    // escrituras —y se le escapaba: el primer fallo de este fichero fue
    // exactamente ese, y lo encontro su propio control.
    let haystack = source.to_lowercase();
    // `to_lowercase` puede cambiar la longitud en bytes (por ejemplo la `I`
    // acentuada), luego las posiciones se toman del original y la busqueda se
    // limita a los offsets que siguen siendo comparables. En la practica el
    // corpus es ASCII y los dos indices coinciden; lo que se garantiza aqui es
    // que un desajuste no produce un resultado silenciosamente equivocado.
    if haystack.len() != source.len() {
        return Err(
            "el fuente contiene caracteres cuyo tamano en minusculas cambia, luego \
                    sus posiciones ya no son comparables y el guard no puede certificarlo"
                .to_string(),
        );
    }
    let mut out = Vec::new();
    for needle in FORBIDDEN {
        for at in find_word_positions(&haystack, needle) {
            let line = line_of(source, at);
            if zones.iter().any(|(a, b)| line >= *a && line <= *b) {
                continue;
            }
            let lo = source[..at].rfind('\n').map_or(0, |i| i + 1);
            let hi = source[at..].find('\n').map_or(source.len(), |i| at + i);
            out.push(Hit {
                line,
                needle,
                context: source[lo..hi].trim().to_string(),
            });
        }
    }
    out.sort_by_key(|h| (h.line, h.needle));
    Ok(out)
}

/// Ruta de los fuentes del dominio.
fn domain_src() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

// ---------------------------------------------------------------------------
// 4. El guard sobre el dominio real
// ---------------------------------------------------------------------------

/// LA LEY. Los cuarenta y seis modulos, no uno.
#[test]
fn el_nucleo_no_nombra_tecnologia_concreta() {
    let src = domain_src();
    let mut files: Vec<_> = fs::read_dir(&src)
        .unwrap_or_else(|e| panic!("no se pudo leer {}: {e}", src.display()))
        .map(|entry| entry.expect("entrada ilegible").path())
        .filter(|p| p.extension().is_some_and(|e| e == "rs"))
        .collect();
    files.sort();

    let mut found: Vec<(String, Hit)> = Vec::new();
    let mut uncertified: Vec<String> = Vec::new();
    for path in &files {
        let source = fs::read_to_string(path).expect("fuente legible");
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        match violations(&source) {
            Ok(hits) => found.extend(hits.into_iter().map(|h| (name.clone(), h))),
            Err(why) => uncertified.push(format!("{name}: {why}")),
        }
    }

    assert!(
        uncertified.is_empty(),
        "el nucleo tiene zonas que no se han podido determinar, luego el guard no \
         puede certificarlas y no las presume limpias: {uncertified:?}"
    );

    let readable = found
        .iter()
        .map(|(name, hit)| format!("{name}:{} [{}] {}", hit.line, hit.needle, hit.context))
        .collect::<Vec<_>>()
        .join("\n  ");
    assert!(
        found.is_empty(),
        "el nucleo nombra tecnologia concreta fuera de la zona de test:\n  {readable}\n\
         El kernel define las preguntas; los providers saben obtener la evidencia."
    );
}

/// El guard mira el dominio entero, no el fichero que se esta discutiendo.
///
/// Sin este control, la extension a 46 modulos podria seguir siendo un
/// `include_str!` de uno solo y nadie lo veria: el resto de controles pasan
/// igual porque opera sobre cadenas sinteticas.
#[test]
fn el_guard_cubre_todos_los_modulos_del_dominio() {
    let files = fs::read_dir(domain_src())
        .expect("src legible")
        .filter(|e| {
            e.as_ref()
                .expect("entrada")
                .path()
                .extension()
                .is_some_and(|x| x == "rs")
        })
        .count();
    assert!(
        files >= 46,
        "se esperaban al menos 46 modulos del dominio y hay {files}: el guard dejaria de \
         cubrir parte del nucleo sin que ningun control se entere"
    );
}

// ---------------------------------------------------------------------------
// 5. Los controles: que el guard PUEDA fallar
// ---------------------------------------------------------------------------

/// Control de potencia positiva: ve un nombre en produccion.
///
/// Un control que no puede fallar no es un control. Este corre el MISMO
/// predicado que el guard de arriba sobre una cadena sintetica, de modo que un
/// `hits.is_empty()` futuro no pueda pasar porque el escaner dejo de ver.
#[test]
fn el_guard_ve_un_nombre_en_produccion() {
    let source = "/// Documenta el campo.\npub struct Fichero {\n    pub ruta: String,\n}\n";
    let mut fuente = source.to_string();
    fuente.push_str(&format!("// ver: {}\n", FORBIDDEN[3]));
    let hits = violations(&fuente).expect("zonas determinadas");
    assert!(
        hits.iter().any(|h| h.needle == FORBIDDEN[3]),
        "el guard dejo de ver un nombre en produccion: {hits:?}"
    );
}

/// El agujero con forma de guard, en su forma real.
///
/// `#[cfg(test)] use ...;` **no es un modulo**. Sin mirar el terminador del item,
/// el detector busca la primera llave que encuentre y se queda con ella: la de
/// la **primera funcion de produccion que hubiera despues**, y exime su cuerpo
/// entero. El caso real esta medido en `test_apply.rs`, donde ese `use` de la
/// linea 33 precede a cientos de lineas de produccion.
///
/// ## POR QUE EL NOMBRE PROHIBITO VA DENTRO DEL CUERPO DE LA FUNCION
///
/// Con el nombre en una linea suelta despues, el control **no discriminaba**:
/// el detector roto acaba la zona antes y la zona bien la acaba tambien, luego
/// en los dos casos el nombre queda fuera y se ve. Lo encontro su propio
/// falsador, que declaro la mutacion M3 sobreviviente. Para que el fallo se
/// note, el nombre tiene que estar **dentro** del cuerpo que el defecto exonera.
///
/// ## Y POR QUE NO HAY LINEA EN BLANCO ENTRE EL `use` Y LA FUNCION
///
/// MEDIDO al escribir esto: con la linea en blanco de por medio, M3 seguia
/// sobreviviendo, y no porque el detector fuera correcto sino porque el
/// detector tiene **una segunda defensa** —corta el barrido en una linea
/// vacia—. Es decir, el defecto que M3 intenta introducir no es observable en
/// esa forma.
///
/// Quitar la linea en blanco es lo que hace que el control vigile la logica de
/// terminador, que es lo que se quiere vigilar. La segunda defensa se queda
/// como lo que es: una red, no una sustitucion de mirar el terminador.
#[test]
fn un_cfg_test_sobre_un_use_no_exime_lo_que_viene_despues() {
    let fuente = format!(
        "\
#[cfg(test)]
use crate::algo::Ayuda;
/// La funcion de produccion.
pub fn helping() -> &'static str {{
    \"usa {}\"
}}
",
        FORBIDDEN[3]
    );
    let hits = violations(&fuente).expect("zonas determinadas");
    assert!(
        hits.iter().any(|h| h.needle == FORBIDDEN[3]),
        "un cfg(test) sobre un use exonero el cuerpo de la funcion de produccion que le \
         sigue, que es el agujero que este guard tiene que cerrar: {hits:?}"
    );
}

/// La excepcion funciona: dentro de la zona de test no hay infraccion.
///
/// Si este control fallara, el guard estaria gritando sobre los 41 sitios
/// legitimos del corpus — y un guard que grita se deja de mirar.
#[test]
fn la_zona_de_test_no_infringe_y_prueba_el_rechazo() {
    let source = r##"\
#[cfg(test)]
mod tests {
    #[test]
    fn rechaza() {
        assert!(rechaza(&["cargo test", "npm run"]));
    }
}
"##;
    let hits = violations(source).expect("zonas determinadas");
    assert!(
        hits.is_empty(),
        "nombrar lo prohibido dentro del test que lo prohibe es legitimo, y el guard lo \
         esta marcando: {hits:?}"
    );
}

/// Una cadena cruda con llaves NO cierra la zona antes de tiempo.
///
/// ## Por que la llave va **sin cerrar** y eso es lo que hace que el control
/// discrimine
///
/// La primera version de este control metia una cadena cruda con `{` y `}` —que
/// se compensan— y por eso **no podia distinguir un lexer roto de uno bien**:
/// con los dos problemas el conteo llegaba al mismo sitio. Lo encontro su propio
/// falsador (mutacion M4, que sobrevivio).
///
/// Y hay un segundo detalle, medido al intentar arreglarlo, que es el que
/// realmente discrimina: **lo que no es la llave impar sino el numero IMPAR de
/// comillas**. Una cadena cruda con un numero par de comillas es indistinguible
/// de una cadena normal —el camino de cadena normal las empareja dos a dos y
/// cierra en el mismo sitio—, luego desactivar el tratamiento de crudas no
/// producia ceguera sino ruido, que es el fallo que no importa.
///
/// Con un numero impar, en cambio, el camino de cadena normal **deja una cadena
/// abierta** que se come el resto del fichero: la zona nunca cierra, el guard
/// falla cerrado y el control cae. Eso si es ceguera, y es lo que el control
/// tiene que poder ver.
///
/// El resto del control tiene cuerpo de produccion despues por el mismo
/// motivo: un `pub struct Fichero;` sin llaves no distingue nada.
#[test]
fn una_cadena_cruda_con_llaves_no_cierra_la_zona() {
    let source = r##"\
#[cfg(test)]
mod tests {
    const FIXTURE: &str = r#"{"clave"#;
}

/// La funcion de produccion.
pub fn helping() -> &'static str {
    "algo"
}
"##;
    let mut fuente = source.to_string();
    fuente.push_str(&format!("// ver: {}\n", FORBIDDEN[3]));
    let hits = violations(&fuente).expect("zonas determinadas");
    assert!(
        hits.iter().any(|h| h.needle == FORBIDDEN[3]),
        "la zona se cerro dentro de una cadena cruda y la produccion posterior quedo \
         sin vigilar: {hits:?}"
    );
}

/// Una cadena normal con llaves tampoco, y tampoco un literal de caracter.
///
/// Llave impar en cada caso, por el mismo motivo que en el control anterior: una
/// llave compensada no desplaza nada y el control no mediria.
#[test]
fn una_cadena_normal_y_un_literal_de_caracter_no_cuentan_como_llaves() {
    let source = r##"\
#[cfg(test)]
mod tests {
    const ABRE: char = '{';
    const TEXTO: &str = "{\\"a\\": 1";
}

/// La funcion de produccion.
pub fn helping() -> &'static str {
    "algo"
}
"##;
    let mut fuente = source.to_string();
    fuente.push_str(&format!("// ver: {}\n", FORBIDDEN[3]));
    let hits = violations(&fuente).expect("zonas determinadas");
    assert!(
        hits.iter().any(|h| h.needle == FORBIDDEN[3]),
        "una llave dentro de una cadena o de un literal de caracter movio el conteo y la \
         zona se cerro antes de tiempo: {hits:?}"
    );
}

/// Un comentario con llaves tampoco.
///
/// Y aqui hay un segundo motivo, porque el comentario de linea **no** se
/// enmascara: el guard tiene que ver los doc-comments, que es donde vive el
/// defecto. Lo que no puede es contar sus llaves.
#[test]
fn un_comentario_con_llaves_no_cierra_la_zona() {
    let source = r##"\
#[cfg(test)]
mod tests {
    // ejemplo con llave: {
    /* y aqui otra: { */
}

/// La funcion de produccion.
pub fn helping() -> &'static str {
    "algo"
}
"##;
    let mut fuente = source.to_string();
    fuente.push_str(&format!("// ver: {}\n", FORBIDDEN[3]));
    let hits = violations(&fuente).expect("zonas determinadas");
    assert!(
        hits.iter().any(|h| h.needle == FORBIDDEN[3]),
        "un comentario con llaves movio el conteo de la zona: {hits:?}"
    );
}

/// El guard falla cerrado cuando una zona no cierra.
///
/// La excepcion es legitima; eximir lo que no se ha podido leer entero es como
/// no vigilar. Una llave que no se cierra es precisamente el caso en que un
/// recorte ingenuo declararia medio fichero limpio.
///
/// Nota de por que la entrada es una zona **sin cerrar** y no un `#[cfg(test)]`
/// pelado: un atributo que no abre item no tiene cuerpo, luego no hay nada que
/// eximir y saltarselo es lo correcto. Lo que no se puede determinar es una
/// zona que empieza y no se sabe donde acaba.
#[test]
fn el_guard_falla_cerrado_ante_una_zona_indeterminable() {
    let source = "#[cfg(test)]\nmod tests {\n    fn a() {}\n";
    let resultado = violations(source);
    assert!(
        resultado.is_err(),
        "una zona que no cierra tiene que ser un fallo, no una zona vacia: {:?}",
        resultado.ok()
    );
}

/// El vocabulario no se ha podrido.
///
/// Un `hits.is_empty()` puede pasar porque el listado se vacio, no porque el
/// nucleo este limpio. Este control exige que la lista siga teniendo nombres.
#[test]
fn el_vocabulario_no_se_ha_vaciado() {
    assert!(
        FORBIDDEN.len() >= 30 && FORBIDDEN.iter().any(|n| n.contains("gradle")),
        "el vocabulario tiene {} entradas y no cubre lo que dice cubrir: {FORBIDDEN:?}",
        FORBIDDEN.len()
    );
}

/// El escaner de palabras no se ha convertido en un `contains`.
///
/// Los tres falsos positivos medidos —`rust` en «trusted», `pip` en «pipeline»,
/// `cargo` en una variable— son la razon de que el limite de palabra exista.
///
/// El texto llega en minusculas porque es lo que hace `violations`, que es el
/// unico sitio donde se normaliza la caja. Este control prueba el limite de
/// palabra; la normalizacion la prueba el control de al lado.
#[test]
fn el_escaner_de_palabras_no_es_una_subcadena() {
    for (texto, needle, esperado) in [
        ("una variable trusted", "rust", false),
        ("un pipeline de trabajo", "pip", false),
        ("release_target_path", "cargo", false),
        ("un fichero cargo.toml aqui", "cargo.toml", true),
        ("un fichero cargo.toml aqui", "cargo", false),
    ] {
        assert_eq!(
            !find_word_positions(texto, needle).is_empty(),
            esperado,
            "frase {texto:?} con needle {needle:?}: el limite de palabra no se comporta \
             como debe"
        );
    }
}
/// La caja no es una salida: un nombre con mayuscula tambien infringe.
///
/// El defecto real que este bloque encontro en el corpus venia escrito como
/// `Cargo.toml`, con mayuscula inicial. Un guard que solo mirase en minusculas
/// habria dado verde sobre el fichero exacto que hay que vigilar — y asi estaba
/// la primera version de este guard.
#[test]
fn un_nombre_con_mayuscula_tambien_infringe() {
    let mut fuente = String::from("/// Documenta el campo.\npub struct Fichero;\n");
    let con_mayuscula = capitalizar(
        FORBIDDEN
            .iter()
            .find(|n| n.contains(' '))
            .copied()
            .unwrap_or("cargo.toml"),
    );
    fuente.push_str(&format!("// ver: {con_mayuscula}\n"));
    let hits = violations(&fuente).expect("zonas determinadas");
    assert!(
        !hits.is_empty(),
        "un nombre escrito con mayuscula inicial se le escapa al guard: {hits:?}"
    );
}

/// Primera letra en mayuscula, para probar que la caja no es una salida.
fn capitalizar(texto: &str) -> String {
    let mut chars = texto.chars();
    match chars.next() {
        Some(primero) => primero.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}
