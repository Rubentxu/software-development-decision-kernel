//! version_source — **dónde declara su versión un proyecto** (ADR-0153).
//!
//! Cierra INC-DEBT-051: `ensure_version_lockstep` abría `Cargo.toml` sin
//! alternativa, así que `release plan` abortaba en cualquier repo que no fosse
//! Rust, contra el principio de AGENTS.md §2.3.
//!
//! # Por qué un registro y no una lista dentro de la función
//!
//! El doc comment de `ensure_version_lockstep` ya lo dice por escrito: añadir un
//! segundo formato ahí sería el mismo error que un tercer `cp -r` en el staging
//! de release (INC-DEBT-056) — una lista codificada de sitios donde mirar, cada
//! uno con su propio modo de fallo. Aquí los **ecosistemas son datos**: una
//! fila del registro, sin tocar código de resolución. El precedente es
//! `EcosystemProfileV1` de SPEC-043 («declarative, data-only ecosystem
//! description, no kernel code»); se comprueba que ese tipo no se construye
//! fuera de los tests, luego no hay registro que enchufar y este es nuevo.
//!
//! # Las dos clases de fuente, y por qué hay dos
//!
//! Se comprobó dónde declara su versión cada ecosistema del principio, sobre
//! repos reales, antes de diseñar nada. **Go y Bazel no declaran versión en
//! ningún manifiesto**: `go.mod` y `MODULE.bazel` no llevan ese campo. Un
//! registro «manifiesto → versión» no puede cubrirlos sin inventarles un
//! fichero que no existe, y son dos de los ocho que AGENTS.md §2.3 nombra.
//!
//! Así que hay dos clases, y la segunda **no** es un permiso para relajar el
//! lockstep en general: solo se concede a los ecosistemas cuya entrada la
//! declara (Rust nunca la toma, y hay un test que lo comprueba), y el
//! resultado distingue [`VersionAuthority::CrossChecked`] de
//! [`VersionAuthority::TagIsTheOnlyAuthority`] para que un release que solo
//! pasó por la segunda clase **lo diga** en vez de reportar un verde
//! indistinguible del primero.
//!
//! # Lo que este módulo NO decide
//!
//! Un repositorio polyglot con versiones **distintas** es un error duro, no una
//! adivinanza. Qué paquete se versiona al publicar es una decisión humana, y se
//! declara en `.sddk/version-source.json`. Es un bloqueo deliberado: es
//! preferible «este repositorio necesita una declaración» a «se versiona la
//! primera cosa que se encontró».

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use thiserror::Error;

// ─────────────────────────────────────────────────────────────────────────────
// El registro: DATOS. Añadir un ecosistema es añadir una fila.
// ─────────────────────────────────────────────────────────────────────────────

/// Formato de lectura. Conjunto **cerrado**: añadir un formato es añadir un
/// brazo en [`extract_from`], que es código — pero **no** una rama más en el
/// lockstep, que es lo que el ADR evita.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestFormat {
    /// TOML (`Cargo.toml`, `pyproject.toml`).
    Toml,
    /// JSON (`package.json`).
    Json,
    /// `clave=valor` por línea (`gradle.properties`).
    Properties,
    /// Una etiqueta (`Directory.Build.props`: `<Version>`).
    Xml,
    /// La llamada `project(... VERSION x.y.z)` de `CMakeLists.txt`.
    Cmake,
    /// El fichero entero, recortado.
    PlainText,
}

impl ManifestFormat {
    /// Nombre legible, para los mensajes de error. Un fallo tiene que decir
    /// **qué formato esperaba**, no solo qué fichero encontró.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Toml => "TOML",
            Self::Json => "JSON",
            Self::Properties => "properties",
            Self::Xml => "XML",
            Self::Cmake => "CMake",
            Self::PlainText => "plain text",
        }
    }
}

/// Dónde está el valor, **como datos**. Ninguna variante menciona un
/// ecosistema: un ecosistema elige un locator, no al revés.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locator {
    /// Ruta de claves en un documento estructurado. `["version"]` es
    /// `package.json`.
    Keys(&'static [&'static str]),
    /// Varias rutas candidatas; gana la primera que exista. Es lo que Rust
    /// necesita (`workspace.package`, `workspace`, `package`) y lo que Python
    /// necesita (PEP 621 y poetry declaran en sitios distintos).
    AnyOfKeys(&'static [&'static [&'static str]]),
    /// Nombre de clave o etiqueta en un formato plano (`version`, `<Version>`,
    /// `VERSION`).
    Tag(&'static str),
}

impl Locator {
    /// Cómo nombrar el valor en un mensaje de error.
    ///
    /// La forma importa por paridad: el lockstep de Rust viene emitting
    /// «could not find \`version\` in [workspace.package], [workspace] or
    /// [package]», y hay un test que afirma sobre esa cadena. Con una ruta de
    /// claves, la hoja (`version`) y los prefijos (`workspace.package`,
    /// `workspace`, `package`) se derivan de los propios datos, así que el
    /// mismo texto sale **sin una rama por ecosistema**.
    pub fn describe(&self) -> String {
        match self {
            Self::Keys(path) => Self::describe_keys(std::slice::from_ref(path)),
            Self::AnyOfKeys(paths) => Self::describe_keys(paths),
            Self::Tag(tag) => format!("`{tag}`"),
        }
    }

    fn describe_keys(paths: &[&'static [&'static str]]) -> String {
        // La hoja común es la clave pedida; los prefijos son dónde se busca.
        let leaf = paths
            .first()
            .and_then(|p| p.last())
            .copied()
            .unwrap_or("version");
        let prefixes: Vec<String> = paths
            .iter()
            .filter_map(|p| {
                let p: &'static [&'static str] = p;
                if p.last().copied() == Some(leaf) && p.len() > 1 {
                    Some(p[..p.len() - 1].join("."))
                } else {
                    None
                }
            })
            .collect();
        match prefixes.len() {
            0 => format!("`{leaf}`"),
            1 => format!("`{leaf}` in {}", prefixes[0]),
            _ => format!(
                "`{leaf}` in {} or {}",
                prefixes[..prefixes.len() - 1].join(", "),
                prefixes[prefixes.len() - 1]
            ),
        }
    }
}

/// Clase de fuente. Las dos, y por qué, en el doc del módulo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    /// El manifiesto declara una versión, y hay algo con lo que comprobar.
    Declares {
        /// Cómo se lee el manifiesto.
        format: ManifestFormat,
        /// Dónde está el valor.
        locator: Locator,
    },
    /// El ecosistema **no** declara versión; el tag es la declaración.
    ///
    /// Se concede solo a los ecosistemas cuya entrada lo dice, y el resultado
    /// lo distingue de `CrossChecked` para que no se lea como una
    /// comprobación realizada.
    TagIsTheOnlyAuthority,
}

/// Una fila del registro. Datos puros: no hay función, ni closure, ni código.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersionSourceSpec {
    /// Identificador estable del ecosistema.
    pub ecosystem: &'static str,
    /// Manifiestos candidatos, en orden de preferencia.
    pub manifest_paths: &'static [&'static str],
    /// Qué clase de fuente es.
    pub kind: SourceKind,
}

/// El registro. **Datos.** Los ocho ecosistemas que nombra AGENTS.md §2.3.
pub const REGISTRY: &[VersionSourceSpec] = &[
    VersionSourceSpec {
        ecosystem: "rust",
        manifest_paths: &["Cargo.toml"],
        kind: SourceKind::Declares {
            format: ManifestFormat::Toml,
            // El ORDEN es la regla, y se aplicaba a mano en una función que
            // ya no existe. Se conserva aquí, con su motivo, porque el orden
            // es lo único que no se deduce solo:
            //
            //   1. `[workspace.package]` — la declaración del workspace.
            //   2. `[workspace]`          — workspaces antiguos, y los tests de
            //      este repo, que ponían la clave directamente en la tabla.
            //   3. `[package]`            — un crate suelto no tiene tabla de
            //      workspace; sin esta, el error sería el espejo del bug
            //      original: abortar en un proyecto cuya versión está a la vista.
            //
            // Lo que NO es esta lista: «la primera clave `version` bajo
            // cualquier tabla que empiece por `[workspace`». Eso es lo que
            // hacía el parser de línea y leía `[workspace.dependencies]`
            // (RED medido, session-65i: `left: "9.9.9" / right: "1.42.5"`),
            // porque esa tabla también empieza por `[workspace`. El fallo era
            // silencioso: un número seguro que describía una dependencia.
            // Una lista de rutas exactas no puede repetirlo.
            locator: Locator::AnyOfKeys(&[
                &["workspace", "package", "version"],
                &["workspace", "version"],
                &["package", "version"],
            ]),
        },
    },
    VersionSourceSpec {
        ecosystem: "typescript",
        manifest_paths: &["package.json"],
        kind: SourceKind::Declares {
            format: ManifestFormat::Json,
            locator: Locator::Keys(&["version"]),
        },
    },
    VersionSourceSpec {
        ecosystem: "python",
        manifest_paths: &["pyproject.toml", "setup.py"],
        kind: SourceKind::Declares {
            format: ManifestFormat::Toml,
            // PEP 621 primero; poetry después. Un proyecto no declara en los dos.
            locator: Locator::AnyOfKeys(&[&["project", "version"], &["tool", "poetry", "version"]]),
        },
    },
    VersionSourceSpec {
        ecosystem: "jvm_gradle",
        manifest_paths: &["gradle.properties", "build.gradle.kts", "build.gradle"],
        kind: SourceKind::Declares {
            format: ManifestFormat::Properties,
            locator: Locator::Tag("version"),
        },
    },
    VersionSourceSpec {
        ecosystem: "dotnet",
        manifest_paths: &["Directory.Build.props", "Directory.Packages.props"],
        kind: SourceKind::Declares {
            format: ManifestFormat::Xml,
            locator: Locator::Tag("Version"),
        },
    },
    VersionSourceSpec {
        ecosystem: "cpp_cmake",
        manifest_paths: &["CMakeLists.txt"],
        kind: SourceKind::Declares {
            format: ManifestFormat::Cmake,
            locator: Locator::Tag("VERSION"),
        },
    },
    // Las dos que no declaran versión. Verificadas: `go.mod` es
    // `module path` + `go 1.x`; `MODULE.bazel` no lleva versión. Escribir una
    // aquí sería inventarles un fichero que no existe.
    VersionSourceSpec {
        ecosystem: "go",
        manifest_paths: &["go.mod"],
        kind: SourceKind::TagIsTheOnlyAuthority,
    },
    VersionSourceSpec {
        ecosystem: "bazel",
        manifest_paths: &["MODULE.bazel", "WORKSPACE"],
        kind: SourceKind::TagIsTheOnlyAuthority,
    },
];

// ─────────────────────────────────────────────────────────────────────────────
// Resolución
// ─────────────────────────────────────────────────────────────────────────────

/// Una fuente detectada, con la versión que declaró.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionCandidate {
    /// Ecosistema al que pertenece la entrada del registro.
    pub ecosystem: &'static str,
    /// Fichero concreto que se leyó.
    pub path: PathBuf,
    /// Versión declarada. Vacía para `TagIsTheOnlyAuthority`.
    pub version: String,
}

/// De dónde sale la versión, y con qué fuerza.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionAuthority {
    /// Hay una versión declarada contra la que comprobar el tag.
    CrossChecked {
        /// La versión, con la que todas las candidatas coinciden.
        version: String,
        /// Todas las fuentes que la declararon. Más de una significa que el
        /// repositorio se reconoció como polyglot **y** que sus manifiestos
        /// concuerdan; se expone en vez de ocultarse.
        candidates: Vec<VersionCandidate>,
    },
    /// Ningún manifiesto declara versión: el tag es la única autoridad.
    ///
    /// Distinto de `CrossChecked` a propósito. Un release que solo pasó por
    /// aquí no ha tenido una comprobación de lockstep, y confundirse con un
    /// verde de verdade es el falso verde que ADR-0152 acaba de cerrar para la
    /// identidad. El coste se admite en el ADR: para Go y Bazel no hay nada
    /// contra qué comprobar.
    TagIsTheOnlyAuthority {
        /// Ecosistemas presentes que no declaran versión.
        ecosystems: Vec<&'static str>,
    },
}

impl VersionAuthority {
    /// La versión, si la hay. `None` cuando el tag es la única autoridad.
    pub fn version(&self) -> Option<&str> {
        match self {
            Self::CrossChecked { version, .. } => Some(version),
            Self::TagIsTheOnlyAuthority { .. } => None,
        }
    }

    /// `true` cuando hubo una comprobación real contra el tag.
    pub fn was_cross_checked(&self) -> bool {
        matches!(self, Self::CrossChecked { .. })
    }
}

/// Por qué no se pudo resolver. Todos son **duros**: ninguno degrada a un
/// valor por defecto.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum VersionSourceError {
    /// Ningún manifiesto del registro existe en el repositorio.
    #[error("VERSION LOCKSTEP ERROR: no known manifest found under {root}; looked for {searched}")]
    NoSource {
        /// Raíz inspeccionada.
        root: PathBuf,
        /// Rutas que se buscaron, ya unidas, en el orden del registro.
        searched: String,
    },

    /// El manifiesto existe pero no se pudo leer.
    #[error("VERSION LOCKSTEP ERROR: could not read {path}: {reason}")]
    Unreadable {
        /// Ecosistema al que pertenece.
        ecosystem: &'static str,
        /// Fichero que falló.
        path: PathBuf,
        /// Error de bajo nivel.
        reason: String,
    },

    /// El manifiesto se leyó pero no se pudo parsear con su formato.
    #[error("VERSION LOCKSTEP ERROR: could not parse {path} as {format}: {reason}")]
    Unparsable {
        /// Ecosistema al que pertenece.
        ecosystem: &'static str,
        /// Fichero que falló.
        path: PathBuf,
        /// Formato que se esperaba.
        format: &'static str,
        /// Error de parseo.
        reason: String,
    },

    /// El manifiesto se leyó y se parseó, pero no declara versión.
    ///
    /// Distinto de [`Self::Unparsable`] a propósito, y con mensaje distinto: un
    /// fichero válido que no declara versión es otro hecho que uno roto, y
    /// juntar los dos es el defecto que un test previo ya exploité.
    #[error("VERSION LOCKSTEP ERROR: could not find {locator} of {path}")]
    NotDeclared {
        /// Ecosistema al que pertenece.
        ecosystem: &'static str,
        /// Fichero inspeccionado.
        path: PathBuf,
        /// Cómo se describe lo que se buscó.
        locator: String,
    },

    /// Varias fuentes declaran versiones **distintas**.
    ///
    /// No se elige una. Qué paquete se versiona al publicar es una decisión
    /// humana, y una lista de candidatos discrepantes es la información que
    /// hace falta para tomarla.
    #[error(
        "VERSION LOCKSTEP ERROR: {ecosystems} declare different versions: {detail}. Declaring which one is authoritative is a human decision (.sddk/version-source.json); sddk will not pick one."
    )]
    Divergent {
        /// Ecosistemas implicados, ordenados.
        ecosystems: String,
        /// Detalle `ecosistema=version` por candidato.
        detail: String,
    },
}

/// Ruta de la declaración explícita de autoridad, relativa a la raíz del
/// repositorio.
pub const AUTHORITY_DECLARATION: &str = ".sddk/version-source.json";

/// Lo que un proyecto **declara** sobre su propia versión.
///
/// No se infiere. La inferencia es lo que produjo esta clase de fallo: si
/// «no encuentro versión» se degrada solo a «el tag manda», un `Cargo.toml`
/// de Rust al que se le quite la `version` pasa el release en verde. Un
/// proyecto que usa el tag por convención **tiene que decirlo**, y decirlo es
/// una línea.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeclaredAuthority {
    /// El tag es la versión del proyecto. No hay nada contra qué comparar, y
    /// el resultado lo distingue de `CrossChecked` para que no se lea como una
    /// comprobación realizada.
    Tag,
}

/// Lee la declaración explícita, si existe. Ausente = `None` (no «el tag
/// manda»).
pub fn read_declared_authority(
    root: &Path,
) -> Result<Option<DeclaredAuthority>, VersionSourceError> {
    let path = root.join(AUTHORITY_DECLARATION);
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&path).map_err(|e| VersionSourceError::Unreadable {
        ecosystem: "declared-authority",
        path: path.clone(),
        reason: e.to_string(),
    })?;
    let doc: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| VersionSourceError::Unparsable {
            ecosystem: "declared-authority",
            path: path.clone(),
            format: "JSON",
            reason: e.to_string(),
        })?;
    if doc.get("schema_version").and_then(|v| v.as_u64()) != Some(1) {
        return Err(VersionSourceError::Unparsable {
            ecosystem: "declared-authority",
            path: path.clone(),
            format: "JSON",
            reason: format!(
                "schema_version {:?}, este build acepta 1",
                doc.get("schema_version")
            ),
        });
    }
    match doc.get("authority").and_then(|v| v.as_str()) {
        Some("tag") => Ok(Some(DeclaredAuthority::Tag)),
        Some(other) => Err(VersionSourceError::Unparsable {
            ecosystem: "declared-authority",
            path,
            format: "JSON",
            reason: format!("authority {other:?} desconocida; se acepta \"tag\""),
        }),
        None => Err(VersionSourceError::NotDeclared {
            ecosystem: "declared-authority",
            path,
            locator: "`authority` (se esperaba \"tag\")".to_string(),
        }),
    }
}

/// Resuelve la versión declarada de un repositorio.
///
/// Fail-closed en los cinco casos del ADR. El que más importa es el tercero:
/// **un manifiesto que existe pero no se puede leer o parsear NO hace que la
/// resolución pase al siguiente candidato.** Ceder ahí cambiaría la autoridad
/// de un proyecto en silencio, que es la clase de fallo que
/// `test_lockstep_reports_a_parse_failure_as_such` ya cubre para un solo
/// ecosistema y que aquí tiene que valer para todos.
///
/// ## La declaración explícita es un rescate acotado
///
/// Si la resolución normal falla por `NotDeclared` o `NoSource` —es decir,
/// **nadie declaró una versión**— y el proyecto escribió
/// `.sddk/version-source.json` diciendo que su autoridad es el tag, entonces el
/// tag es su versión.
///
/// Lo que NO hace, y es deliberado:
///
/// - No rescata `Unreadable` ni `Unparsable`. Un manifiesto que no se puede
///   leer puede estar escondiendo algo; declarar el tag no lo repara.
/// - No rescata `Divergent`. Si dos manifiestos discrepan, esa es una
///   pregunta humana y una declaración no la responde.
/// - No se consulta si hay versión. Si existe, se comprueba contra el tag
///   aunque el proyecto haya declarado el tag como autoridad: una declaración
///   es un rescate, no una prioridad.
///
/// Es lo que separa este contrato de una degradación silenciosa: sin la
/// declaración, un `Cargo.toml` al que se le quite la `version` falla. Con ella,
/// falla hasta que alguien la escriba.
pub fn resolve_project_version(root: &Path) -> Result<VersionAuthority, VersionSourceError> {
    match resolve_from_manifests(root) {
        Ok(authority) => Ok(authority),
        Err(err)
            if matches!(
                err,
                VersionSourceError::NotDeclared { .. } | VersionSourceError::NoSource { .. }
            ) =>
        {
            match read_declared_authority(root)? {
                Some(DeclaredAuthority::Tag) => Ok(VersionAuthority::TagIsTheOnlyAuthority {
                    ecosystems: vec!["declared-by-project"],
                }),
                None => Err(err),
            }
        }
        Err(err) => Err(err),
    }
}

/// La resolución por sí sola, sin mirar la declaración del proyecto.
fn resolve_from_manifests(root: &Path) -> Result<VersionAuthority, VersionSourceError> {
    let mut declared: Vec<VersionCandidate> = Vec::new();
    let mut tag_only: Vec<&'static str> = Vec::new();
    let mut searched: Vec<String> = Vec::new();

    for spec in REGISTRY {
        for rel in spec.manifest_paths {
            let path = root.join(rel);
            searched.push(path.display().to_string());
            if !path.exists() {
                continue;
            }
            match spec.kind {
                SourceKind::TagIsTheOnlyAuthority => {
                    tag_only.push(spec.ecosystem);
                }
                SourceKind::Declares { format, locator } => {
                    let content = std::fs::read_to_string(&path).map_err(|e| {
                        VersionSourceError::Unreadable {
                            ecosystem: spec.ecosystem,
                            path: path.clone(),
                            reason: e.to_string(),
                        }
                    })?;
                    let version =
                        extract_from(format, locator, &content).map_err(|why| match why {
                            ExtractError::Parse(reason) => VersionSourceError::Unparsable {
                                ecosystem: spec.ecosystem,
                                path: path.clone(),
                                format: format.label(),
                                reason,
                            },
                            ExtractError::Absent => VersionSourceError::NotDeclared {
                                ecosystem: spec.ecosystem,
                                path: path.clone(),
                                locator: locator.describe(),
                            },
                        })?;
                    declared.push(VersionCandidate {
                        ecosystem: spec.ecosystem,
                        path,
                        version,
                    });
                }
            }
        }
    }

    // Una fuente que declara versión manda sobre una que solo aporta un tag:
    // el tag no es un valor con el que comparar, es la ausencia de uno.
    if !declared.is_empty() {
        let first = declared[0].version.as_str();
        if declared.iter().any(|c| c.version != first) {
            let mut ecosystems: Vec<&str> = declared.iter().map(|c| c.ecosystem).collect();
            ecosystems.sort_unstable();
            let detail = declared
                .iter()
                .map(|c| format!("{}={} ({})", c.ecosystem, c.version, c.path.display()))
                .collect::<Vec<_>>()
                .join("; ");
            return Err(VersionSourceError::Divergent {
                ecosystems: ecosystems.join(", "),
                detail,
            });
        }
        return Ok(VersionAuthority::CrossChecked {
            version: first.to_string(),
            candidates: declared,
        });
    }

    if !tag_only.is_empty() {
        tag_only.sort_unstable();
        return Ok(VersionAuthority::TagIsTheOnlyAuthority {
            ecosystems: tag_only,
        });
    }

    Err(VersionSourceError::NoSource {
        root: root.to_path_buf(),
        searched: searched.join(", "),
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// El lector genérico: un `match` por FORMATO, ninguno por ecosistema
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug)]
enum ExtractError {
    Parse(String),
    Absent,
}

fn extract_from(
    format: ManifestFormat,
    locator: Locator,
    content: &str,
) -> Result<String, ExtractError> {
    match format {
        ManifestFormat::Toml => {
            let doc: toml::Value = content
                .parse::<toml::Value>()
                .map_err(|e: toml::de::Error| ExtractError::Parse(e.to_string()))?;
            let found = match locator {
                Locator::Keys(path) => lookup_key(&doc, path),
                Locator::AnyOfKeys(paths) => paths.iter().find_map(|p| lookup_key(&doc, p)),
                Locator::Tag(tag) => lookup_key(&doc, &tag.split('.').collect::<Vec<_>>()),
            };
            scalar_to_string(found).ok_or(ExtractError::Absent)
        }
        ManifestFormat::Json => {
            let doc: serde_json::Value =
                serde_json::from_str(content).map_err(|e| ExtractError::Parse(e.to_string()))?;
            let found = match locator {
                Locator::Keys(path) => lookup_json(&doc, path),
                Locator::AnyOfKeys(paths) => paths.iter().find_map(|p| lookup_json(&doc, p)),
                Locator::Tag(tag) => lookup_json(&doc, &tag.split('.').collect::<Vec<_>>()),
            };
            match found {
                Some(serde_json::Value::String(s)) => Ok(s.clone()),
                Some(serde_json::Value::Number(n)) => Ok(n.to_string()),
                _ => Err(ExtractError::Absent),
            }
        }
        ManifestFormat::Properties => {
            let tag = match locator {
                Locator::Tag(t) => t,
                Locator::Keys(p) => p.last().copied().unwrap_or("version"),
                Locator::AnyOfKeys(ps) => ps
                    .first()
                    .and_then(|p| p.last().copied())
                    .unwrap_or("version"),
            };
            for line in content.lines() {
                let line = line.trim();
                if line.starts_with('#') || line.starts_with('!') || line.is_empty() {
                    continue;
                }
                let Some((k, v)) = line.split_once('=') else {
                    continue;
                };
                if k.trim() == tag {
                    let v = v.trim();
                    if v.is_empty() {
                        continue;
                    }
                    return Ok(v.to_string());
                }
            }
            Err(ExtractError::Absent)
        }
        ManifestFormat::Xml => {
            let tag = match locator {
                Locator::Tag(t) => t,
                Locator::Keys(p) => p.last().copied().unwrap_or("Version"),
                Locator::AnyOfKeys(ps) => ps
                    .first()
                    .and_then(|p| p.last().copied())
                    .unwrap_or("Version"),
            };
            // Extracción dirigida, no un parser XML: solo se necesita el
            // contenido de una etiqueta, y añadir una dependencia de XML por
            // eso sería el intercambio equivocado. Documentado a propósito.
            let open = format!("<{tag}>");
            let close = format!("</{tag}>");
            let start = content.find(&open).ok_or(ExtractError::Absent)? + open.len();
            let rest = &content[start..];
            let end = rest.find(&close).ok_or(ExtractError::Absent)?;
            let value = rest[..end].trim();
            if value.is_empty() {
                return Err(ExtractError::Absent);
            }
            Ok(value.to_string())
        }
        ManifestFormat::Cmake => {
            // La llamada `project(<name> VERSION x.y.z ...)`. El token que
            // precede a VERSION es el nombre del proyecto; se lee el que le
            // sigue, que es la versión.
            let tag = match locator {
                Locator::Tag(t) => t,
                Locator::Keys(p) => p.last().copied().unwrap_or("VERSION"),
                Locator::AnyOfKeys(ps) => ps
                    .first()
                    .and_then(|p| p.last().copied())
                    .unwrap_or("VERSION"),
            };
            for line in content.lines() {
                let Some(idx) = line.find("project(") else {
                    continue;
                };
                let Some(close) = line[idx..].find(')') else {
                    continue;
                };
                let args = &line[idx + "project(".len()..idx + close];
                let tokens: Vec<&str> = args.split_whitespace().collect();
                let Some(pos) = tokens.iter().position(|t| t.eq_ignore_ascii_case(tag)) else {
                    continue;
                };
                if let Some(v) = tokens.get(pos + 1) {
                    return Ok((*v).to_string());
                }
            }
            Err(ExtractError::Absent)
        }
        ManifestFormat::PlainText => {
            let v = content.trim();
            if v.is_empty() {
                Err(ExtractError::Absent)
            } else {
                Ok(v.to_string())
            }
        }
    }
}

fn lookup_key<'a>(doc: &'a toml::Value, path: &[&str]) -> Option<&'a toml::Value> {
    let mut current = doc;
    for key in path {
        current = current.get(key)?;
    }
    Some(current)
}

fn lookup_json<'a>(doc: &'a serde_json::Value, path: &[&str]) -> Option<&'a serde_json::Value> {
    let mut current = doc;
    for key in path {
        current = current.get(key)?;
    }
    Some(current)
}

fn scalar_to_string(value: Option<&toml::Value>) -> Option<String> {
    match value? {
        toml::Value::String(s) => Some(s.clone()),
        toml::Value::Integer(i) => Some(i.to_string()),
        toml::Value::Float(f) => Some(f.to_string()),
        _ => None,
    }
}

/// Índice ecosistema → rutas candidatas, para diagnóstico. No participa en la
/// resolución: leer el registro no debería cambiar el resultado.
pub fn searched_manifests() -> BTreeMap<&'static str, Vec<&'static str>> {
    REGISTRY
        .iter()
        .map(|s| (s.ecosystem, s.manifest_paths.to_vec()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write(dir: &Path, name: &str, body: &str) {
        let mut f = std::fs::File::create(dir.join(name)).unwrap();
        writeln!(f, "{body}").unwrap();
    }

    fn version_of(dir: &Path) -> String {
        match resolve_project_version(dir).expect("debe resolver") {
            VersionAuthority::CrossChecked { version, .. } => version,
            other => panic!("se esperaba CrossChecked, vino {other:?}"),
        }
    }

    // ── Criterio 2: los ocho ecosistemas del principio, uno a uno ──────────

    #[test]
    fn every_ecosystem_in_the_principle_has_a_row() {
        // Los ocho que nombra AGENTS.md §2.3. Si el registro deja de cubrir uno,
        // este test cae: es la versión ejecutable de "el principio se cumple".
        let expected = [
            "rust",
            "typescript",
            "python",
            "jvm_gradle",
            "dotnet",
            "cpp_cmake",
            "go",
            "bazel",
        ];
        for eco in expected {
            assert!(
                REGISTRY.iter().any(|s| s.ecosystem == eco),
                "{eco} aparece en AGENTS.md §2.3 y no tiene fila en el registro"
            );
        }
        assert_eq!(
            REGISTRY.len(),
            expected.len(),
            "hay filas de más: un ecosistema que el principio no nombra, o un duplicado"
        );
    }

    #[test]
    fn rust_resolves_workspace_package() {
        let d = tempfile::tempdir().unwrap();
        write(
            d.path(),
            "Cargo.toml",
            "[workspace.package]\nversion = \"1.42.5\"\nmembers = []",
        );
        assert_eq!(version_of(d.path()), "1.42.5");
    }

    #[test]
    fn rust_resolves_the_single_crate_case() {
        let d = tempfile::tempdir().unwrap();
        write(
            d.path(),
            "Cargo.toml",
            "[package]\nname = \"demo\"\nversion = \"0.9.1\"",
        );
        assert_eq!(version_of(d.path()), "0.9.1");
    }

    #[test]
    fn typescript_resolves_package_json() {
        let d = tempfile::tempdir().unwrap();
        write(
            d.path(),
            "package.json",
            r#"{"name":"demo","version":"3.1.4"}"#,
        );
        assert_eq!(version_of(d.path()), "3.1.4");
    }

    #[test]
    fn python_resolves_pep621_and_poetry_separately() {
        let pep = tempfile::tempdir().unwrap();
        write(
            pep.path(),
            "pyproject.toml",
            "[project]\nname = \"demo\"\nversion = \"2.0.1\"",
        );
        assert_eq!(version_of(pep.path()), "2.0.1");

        let poetry = tempfile::tempdir().unwrap();
        write(
            poetry.path(),
            "pyproject.toml",
            "[tool.poetry]\nname = \"demo\"\nversion = \"2.0.2\"",
        );
        assert_eq!(version_of(poetry.path()), "2.0.2");
    }

    #[test]
    fn jvm_gradle_resolves_gradle_properties() {
        let d = tempfile::tempdir().unwrap();
        write(
            d.path(),
            "gradle.properties",
            "org.gradle.caching=true\nversion=0.46.0",
        );
        assert_eq!(version_of(d.path()), "0.46.0");
    }

    #[test]
    fn dotnet_resolves_the_directory_build_props_tag() {
        let d = tempfile::tempdir().unwrap();
        write(
            d.path(),
            "Directory.Build.props",
            "<Project>\n  <PropertyGroup>\n    <Version>5.4.0</Version>\n  </PropertyGroup>\n</Project>",
        );
        assert_eq!(version_of(d.path()), "5.4.0");
    }

    #[test]
    fn cpp_cmake_resolves_the_project_call() {
        let d = tempfile::tempdir().unwrap();
        write(
            d.path(),
            "CMakeLists.txt",
            "cmake_minimum_required(VERSION 3.20)\nproject(demo VERSION 7.1.3 LANGUAGES CXX)",
        );
        assert_eq!(version_of(d.path()), "7.1.3");
    }

    // ── Criterio 7: los dos que NO declaran versión ───────────────────────

    #[test]
    fn go_has_no_version_to_check_and_says_so() {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), "go.mod", "module example.com/demo\n\ngo 1.22");
        let authority = resolve_project_version(d.path()).expect("debe resolver");
        assert_eq!(
            authority,
            VersionAuthority::TagIsTheOnlyAuthority {
                ecosystems: vec!["go"]
            }
        );
        assert_eq!(authority.version(), None);
        assert!(
            !authority.was_cross_checked(),
            "sin manifiesto no hubo comprobacion, y no puede reportarse como si la hubiera"
        );
    }

    #[test]
    fn bazel_has_no_version_to_check_and_says_so() {
        let d = tempfile::tempdir().unwrap();
        write(
            d.path(),
            "MODULE.bazel",
            "module(name = \"demo\", version = \"1.0\")",
        );
        let authority = resolve_project_version(d.path()).expect("debe resolver");
        assert!(matches!(
            authority,
            VersionAuthority::TagIsTheOnlyAuthority { .. }
        ));
        assert!(!authority.was_cross_checked());
    }

    /// Rust **nunca** puede tomar la segunda clase. Es lo que impide que
    /// `TagIsTheOnlyAuthority` se convierta en un atajo para relajar el
    /// predicado en todas partes.
    #[test]
    fn rust_never_takes_the_tag_only_path() {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), "Cargo.toml", "[package]\nversion = \"1.0.0\"");
        let authority = resolve_project_version(d.path()).unwrap();
        assert!(
            authority.was_cross_checked(),
            "un repo Rust SI tiene version con la que comprobar"
        );
    }

    /// Un repo Go **con** un `package.json` se comprueba contra el JSON: el tag
    /// no es un valor, es la ausencia de uno, y no debe tapar una declaración
    /// que sí existe.
    #[test]
    fn a_declaring_source_outranks_a_tag_only_one() {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), "go.mod", "module example.com/demo");
        write(d.path(), "package.json", r#"{"version":"4.0.0"}"#);
        let authority = resolve_project_version(d.path()).unwrap();
        assert_eq!(authority.version(), Some("4.0.0"));
        assert!(authority.was_cross_checked());
    }

    // ── Criterios 4, 5, 6, 8: la política fail-closed ────────────────────

    #[test]
    fn two_manifests_that_disagree_are_a_hard_error_naming_both() {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), "Cargo.toml", "[package]\nversion = \"1.0.0\"");
        write(d.path(), "package.json", r#"{"version":"2.0.0"}"#);
        let err = resolve_project_version(d.path()).unwrap_err();
        let msg = err.to_string();
        assert!(
            matches!(err, VersionSourceError::Divergent { .. }),
            "no se elige uno: {msg}"
        );
        assert!(msg.contains("rust=1.0.0"), "{msg}");
        assert!(msg.contains("typescript=2.0.0"), "{msg}");
        assert!(
            msg.contains(".sddk/version-source.json"),
            "el error debe decir como se resuelve: {msg}"
        );
    }

    #[test]
    fn two_manifests_that_agree_are_a_cross_check_that_says_it_was_several() {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), "Cargo.toml", "[package]\nversion = \"1.0.0\"");
        write(d.path(), "package.json", r#"{"version":"1.0.0"}"#);
        let authority = resolve_project_version(d.path()).unwrap();
        match authority {
            VersionAuthority::CrossChecked {
                version,
                candidates,
            } => {
                assert_eq!(version, "1.0.0");
                assert_eq!(candidates.len(), 2, "las dos candidatas se exponen");
            }
            other => panic!("se esperaba CrossChecked, vino {other:?}"),
        }
    }

    /// **Este es el criterio que más importa.** Un manifiesto roto no puede
    /// ceder la autoridad al siguiente candidato: en un repo con `Cargo.toml`
    /// corrupto y un `package.json` sano, responder con la versión del JSON
    /// sería cambiar de autoridad en silencio y con exit 0.
    #[test]
    fn a_broken_manifest_does_not_hand_authority_to_the_next_candidate() {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), "Cargo.toml", "esto no es TOML [[[");
        write(d.path(), "package.json", r#"{"version":"9.9.9"}"#);
        let err = resolve_project_version(d.path()).unwrap_err();
        let msg = err.to_string();
        assert!(
            matches!(err, VersionSourceError::Unparsable { .. }),
            "debe fallar en el manifiesto roto, no seguir buscando: {msg}"
        );
        assert!(msg.contains("Cargo.toml"), "{msg}");
        assert!(
            !msg.contains("9.9.9"),
            "la version del candidato sano no debe aparecer: la autoridad no se cede. {msg}"
        );
    }

    #[test]
    fn a_valid_manifest_without_a_version_is_a_different_fact_than_a_broken_one() {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), "Cargo.toml", "[workspace]\nmembers = []");
        let err = resolve_project_version(d.path()).unwrap_err();
        let msg = err.to_string();
        assert!(
            matches!(err, VersionSourceError::NotDeclared { .. }),
            "{msg}"
        );
        assert!(
            msg.contains("could not find `version`"),
            "el mensaje de paridad con el lockstep de Rust se conserva: {msg}"
        );
    }

    #[test]
    fn no_candidate_lists_every_manifest_that_was_looked_for() {
        let d = tempfile::tempdir().unwrap();
        let err = resolve_project_version(d.path()).unwrap_err();
        let msg = err.to_string();
        assert!(matches!(err, VersionSourceError::NoSource { .. }), "{msg}");
        for expected in ["Cargo.toml", "package.json", "gradle.properties", "go.mod"] {
            assert!(msg.contains(expected), "falta {expected} en: {msg}");
        }
    }

    // ── Criterio 3: añadir un ecosistema es SOLO datos ────────────────────

    /// El criterio que distingue este contrato de la lista codificada que el
    /// ADR evita. Se construye un `VersionSourceSpec` en runtime —con su
    /// formato, su locator y sus rutas— y se resuelve **sin tocar código**.
    /// Si mañana el lector genérico estuviera escrito con un `if` por
    /// ecosistema, este test no compilaría ni significaría nada.
    #[test]
    fn a_new_ecosystem_is_data_and_needs_no_code() {
        let spec = VersionSourceSpec {
            ecosystem: "cargo_of_the_test",
            manifest_paths: &["Cargo.toml"],
            kind: SourceKind::Declares {
                format: ManifestFormat::Toml,
                locator: Locator::Keys(&["package", "version"]),
            },
        };
        let d = tempfile::tempdir().unwrap();
        write(d.path(), "Cargo.toml", "[package]\nversion = \"8.8.8\"");

        // El mismo lector, el mismo format, otro locator: sin una rama nueva.
        let content = std::fs::read_to_string(d.path().join("Cargo.toml")).unwrap();
        let got = extract_from(
            match spec.kind {
                SourceKind::Declares { format, .. } => format,
                SourceKind::TagIsTheOnlyAuthority => unreachable!(),
            },
            match spec.kind {
                SourceKind::Declares { locator, .. } => locator,
                SourceKind::TagIsTheOnlyAuthority => unreachable!(),
            },
            &content,
        );
        assert_eq!(got.unwrap(), "8.8.8");
    }

    #[test]
    fn the_registry_has_no_duplicate_ecosystems() {
        let mut seen: Vec<&str> = REGISTRY.iter().map(|s| s.ecosystem).collect();
        let before = seen.len();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(
            before,
            seen.len(),
            "hay ecosistemas repetidos en el registro"
        );
    }

    #[test]
    fn every_declaring_row_declares_a_format_and_a_locator() {
        for spec in REGISTRY {
            match spec.kind {
                SourceKind::Declares { format, locator } => {
                    let _ = format.label();
                    assert!(
                        !locator.describe().is_empty(),
                        "{} produce un mensaje vacio",
                        spec.ecosystem
                    );
                }
                SourceKind::TagIsTheOnlyAuthority => {}
            }
            assert!(
                !spec.manifest_paths.is_empty(),
                "{} no tiene rutas",
                spec.ecosystem
            );
        }
    }

    // ── Criterio 3, la parte que ningún test funcional puede ver ───────────

    /// **Este test existe porque una mutación escapó.**
    ///
    /// Reintroducir `if spec.ecosystem == "rust" { … }` en el bucle de
    /// resolución —un `if` por ecosistema, exactamente la lista codificada que
    /// ADR-0153 dice evitar— dejó la suite **verde**. La mutación es
    /// comportamentalmente idéntica: da la misma respuesta para todos los
    /// casos que los tests ejercitan. Ningún test de comportamiento puede
    /// detectarla, porque no cambia comportamiento.
    ///
    /// La propiedad es **estructural**: los identificadores de ecosistema viven
    /// en el registro y en ningún otro sitio. Eso solo se puede afirmar
    /// leyendo la fuente, y por eso este test la lee.
    ///
    /// Se aplica al código de producción de los dos ficheros: la sección de
    /// tests puede nombrar ecosistemas —debe hacerlo, para comprobar que el
    /// registro los cubre— y no es lo que se está vigilando.
    #[test]
    fn ecosystem_ids_exist_only_inside_the_registry() {
        let here = include_str!("version_source.rs");
        let there = include_str!("version.rs");

        let registry_start = here
            .find("pub const REGISTRY")
            .expect("el registro tiene que existir");
        let registry_tail = &here[registry_start..];
        let registry_end = registry_start
            + registry_tail
                .find("\n];")
                .map(|i| i + 3)
                .expect("el registro tiene que cerrar con ];");
        let registry_block = &here[registry_start..registry_end];
        // El corte por `#[cfg(test)]` importa en los DOS lados: el módulo de
        // tests nombra ecosistemas a proposito (para comprobar que el registro
        // los cubre) y este mismo test escribe "rust" en su documentacion.
        let outside_here = production_only(&format!(
            "{}{}",
            &here[..registry_start],
            &here[registry_end..]
        ));
        let outside_there = production_only(there);

        for spec in REGISTRY {
            let quoted = format!("\"{}\"", spec.ecosystem);
            assert!(
                registry_block.contains(&quoted),
                "'{}' deberia estar en su propia fila del registro",
                spec.ecosystem
            );
            for (where_, chunk) in [
                ("version_source.rs", &outside_here),
                ("version.rs", &outside_there),
            ] {
                assert!(
                    !chunk.contains(&quoted),
                    "'{}' aparece FUERA del registro, en {}. El codigo de resolucion \
                     ha vuelto a saber de un ecosistema concreto, que es la lista \
                     codificada que ADR-0153 existe para evitar. Anade una fila al \
                     REGISTRY, no una rama.",
                    spec.ecosystem,
                    where_
                );
            }
        }
    }

    /// Corta un fichero por su bloque de tests: a partir de ahí, nombrar
    /// ecosistemas es lo esperado.
    fn production_only(source: &str) -> String {
        match source.find("#[cfg(test)]") {
            Some(i) => source[..i].to_string(),
            None => source.to_string(),
        }
    }

    // ── La declaración explícita, y lo que NO rescata ─────────────────────
    //
    // Este bloque existe porque un repo REAL, no un fixture, enseñó que el
    // contrato de dos estados no basta: hay un tercer caso entre «declara
    // versión» y «el ecosistema no puede declararla».

    fn declare_tag_authority(dir: &Path) {
        std::fs::create_dir_all(dir.join(".sddk")).unwrap();
        write(
            dir,
            ".sddk/version-source.json",
            r#"{"schema_version":1,"authority":"tag"}"#,
        );
    }

    /// El caso medido: un repo Gradle cuyo `gradle.properties` existe pero no
    /// declara versión, y cuya convención real es el tag git.
    #[test]
    fn a_manifest_without_a_version_is_rescued_only_by_an_explicit_declaration() {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), "gradle.properties", "org.gradle.caching=true");

        // Sin declaración: falla. Un `version=` que no está no es un
        // `version=` vacío, y degradar a "el tag manda" sin preguntar
        // convertiría un descuido en un release verde.
        let err = resolve_project_version(d.path()).unwrap_err();
        assert!(
            matches!(err, VersionSourceError::NotDeclared { .. }),
            "{err}"
        );

        // Con declaración: el tag es su autoridad, y se dice que no hubo
        // comprobación.
        declare_tag_authority(d.path());
        let authority = resolve_project_version(d.path()).expect("debe resolver");
        assert_eq!(
            authority,
            VersionAuthority::TagIsTheOnlyAuthority {
                ecosystems: vec!["declared-by-project"]
            }
        );
        assert!(!authority.was_cross_checked());
    }

    /// La declaración es un **rescate**, no una prioridad: si el manifiesto sí
    /// declara versión, se comprueba contra el tag aunque exista la
    /// declaración.
    #[test]
    fn a_declaration_does_not_override_a_version_that_does_exist() {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), "gradle.properties", "version=1.2.3");
        declare_tag_authority(d.path());
        let authority = resolve_project_version(d.path()).expect("debe resolver");
        assert_eq!(authority.version(), Some("1.2.3"));
        assert!(authority.was_cross_checked());
    }

    /// La declaración **no** repara un manifiesto roto. Declarar el tag no
    /// convierte un `Cargo.toml` corrupto en un release publicable.
    #[test]
    fn a_declaration_does_not_rescue_a_broken_manifest() {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), "Cargo.toml", "esto no es TOML [[[");
        declare_tag_authority(d.path());
        let err = resolve_project_version(d.path()).unwrap_err();
        assert!(
            matches!(err, VersionSourceError::Unparsable { .. }),
            "un manifiesto roto no lo repara una declaracion: {err}"
        );
    }

    /// Ni dos versiones que discrepan, que es una pregunta humana.
    #[test]
    fn a_declaration_does_not_rescue_a_divergence() {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), "Cargo.toml", "[package]\nversion = \"1.0.0\"");
        write(d.path(), "package.json", r#"{"version":"2.0.0"}"#);
        declare_tag_authority(d.path());
        let err = resolve_project_version(d.path()).unwrap_err();
        assert!(matches!(err, VersionSourceError::Divergent { .. }), "{err}");
    }

    #[test]
    fn a_malformed_declaration_fails_loud_instead_of_being_ignored() {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), "gradle.properties", "org.gradle.caching=true");
        std::fs::create_dir_all(d.path().join(".sddk")).unwrap();
        write(
            d.path(),
            ".sddk/version-source.json",
            r#"{"schema_version":99,"authority":"tag"}"#,
        );
        let err = resolve_project_version(d.path()).unwrap_err();
        assert!(
            matches!(err, VersionSourceError::Unparsable { .. }),
            "una declaracion de un build futuro no se ignora: {err}"
        );
    }
}
