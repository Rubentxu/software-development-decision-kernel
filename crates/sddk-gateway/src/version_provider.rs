//! Provider that observes a product version by reading declarations.
//!
//! # Where the technology lives, and why it lives HERE
//!
//! The file names in [`DEFAULT_DECLARATIONS`] are the only place in the
//! version path where a concrete tool, language or file appears. They are
//! here, in the adapter layer, and not in the crate that decides — because
//! that crate is depended on by everything, and a name it carries is a name
//! every consumer inherits.
//!
//! Nothing in this module decides anything. It observes: it reads files,
//! extracts a value, and reports one of the five answers ([`VersionProbe`]).
//! Deciding what those answers mean is
//! [`sddk_domain::version_authority::reduce`], which cannot see a file name.
//!
//! # Why there is exactly one provider per file
//!
//! An earlier version of this file also had a provider that read *every*
//! declaration at once, so that "one call resolves the repository". It was
//! removed, and the reason is worth keeping: that provider had to decide what
//! to do when two files disagreed, and every answer it could give is a policy
//! — pick one, or report a conflict. It chose to report a conflict, which is
//! the right answer, but it chose it *in the adapter layer*, where a second
//! implementation would have chosen differently and nothing would have
//! noticed. With one provider per file, disagreement is not a case any
//! provider has to handle: it is two observations, and the reducer names it
//! without being asked. A provider that decides is a second authority, and
//! this repo has a whole debt file about what those cost.
//!
//! # Why the extractor belongs to the candidate
//!
//! Each [`DeclarationSpec`] carries its own format and locator. An earlier
//! shape gave one format per ecosystem and a list of file names, which meant
//! the second and third file in a list were read with the first one's
//! parser. That is not an approximation — it is a false statement about the
//! file's format, and it failed visibly: a `setup.py` was handed to a TOML
//! parser, and because a failed parse aborted the whole resolution, a
//! `package.json` that had *already* declared its version was discarded
//! because of an unrelated file further down the list.
//!
//! A candidate with no extraction is a deliberate declaration: the file
//! exists in real projects and is **not** a declarable version source.
//! Naming that honestly is better than inventing a reader for it, and a
//! reader built by pattern-matching a build script would be a second version
//! of the same lie.

use sddk_domain::version_authority::{
    PRODUCT_VERSION_OBSERVATION, ProductVersion, ProviderError, ReleaseTarget, VersionEvidence,
    VersionProbe, VersionResolverPort,
};
use std::path::{Path, PathBuf};

/// How a file is parsed. Formats, not ecosystems: this says what the bytes
/// are, never which tool wrote them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclarationFormat {
    /// TOML.
    Toml,
    /// JSON.
    Json,
    /// One `key=value` per line.
    KeyValue,
    /// A single labelled call, e.g. `project(name VERSION 1.2.3)`.
    LabelledCall,
    /// A single tag, e.g. `<Version>1.2.3</Version>`.
    TaggedElement,
}

impl DeclarationFormat {
    /// Human-readable name, for the failure message. A failure has to say
    /// **which format it expected**, not only which file it found.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Toml => "TOML",
            Self::Json => "JSON",
            Self::KeyValue => "key=value",
            Self::LabelledCall => "labelled call",
            Self::TaggedElement => "tagged element",
        }
    }
}

/// Where the value sits inside a parsed file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueLocator {
    /// A key path in a structured document.
    Keys(&'static [&'static str]),
    /// Several key paths; the first that exists wins.
    AnyOfKeys(&'static [&'static [&'static str]]),
    /// A bare key or tag name in a flat format.
    Tag(&'static str),
}

impl ValueLocator {
    /// How to name the location in a message.
    pub fn describe(&self) -> String {
        match self {
            Self::Keys(keys) => keys.join("."),
            Self::AnyOfKeys(paths) => paths
                .iter()
                .map(|p| p.join("."))
                .collect::<Vec<_>>()
                .join(" / "),
            Self::Tag(tag) => tag.to_string(),
        }
    }
}

/// One file, and how to read it.
#[derive(Debug, Clone, Copy)]
pub struct DeclarationSpec {
    /// Path relative to the target root.
    pub path: &'static str,
    /// How to read it. `None` declares that the file is **not** a
    /// declarable version source, which is a fact and not a gap.
    pub extraction: Option<(DeclarationFormat, ValueLocator)>,
    /// The ecosystem this file belongs to, for diagnostics only. The
    /// resolver never reads it: a declaration that a name mattered would be
    /// a priority rule in a place that claims not to have any.
    pub ecosystem: &'static str,
    /// `true` when the ecosystem declares nothing and its tag is the
    /// authority.
    pub tag_is_authority: bool,
}

/// The declarations this provider knows how to observe.
///
/// **Data.** Adding a tool is a new row here, not a new branch anywhere.
pub const DEFAULT_DECLARATIONS: &[DeclarationSpec] = &[
    DeclarationSpec {
        path: "Cargo.toml",
        extraction: Some((
            DeclarationFormat::Toml,
            // The order is the rule and is not deducible:
            //   1. workspace declaration, 2. legacy workspace table,
            //   3. a single crate, which has no workspace table at all.
            // What this is NOT: "the first `version` under any table starting
            // with `[workspace`" — that is what read a dependencies table.
            ValueLocator::AnyOfKeys(&[
                &["workspace", "package", "version"],
                &["workspace", "version"],
                &["package", "version"],
            ]),
        )),
        ecosystem: "rust",
        tag_is_authority: false,
    },
    DeclarationSpec {
        path: "package.json",
        extraction: Some((DeclarationFormat::Json, ValueLocator::Keys(&["version"]))),
        ecosystem: "javascript",
        tag_is_authority: false,
    },
    DeclarationSpec {
        path: "pyproject.toml",
        extraction: Some((
            DeclarationFormat::Toml,
            // Two declaration dialects, in this order. A project declares in
            // one, not both.
            ValueLocator::AnyOfKeys(&[&["project", "version"], &["tool", "poetry", "version"]]),
        )),
        ecosystem: "python",
        tag_is_authority: false,
    },
    DeclarationSpec {
        // A scripting file, not a declarative manifest. Declared as NOT a
        // version source rather than handed to a parser that would misread
        // it.
        path: "setup.py",
        extraction: None,
        ecosystem: "python",
        tag_is_authority: false,
    },
    DeclarationSpec {
        path: "gradle.properties",
        extraction: Some((DeclarationFormat::KeyValue, ValueLocator::Tag("version"))),
        ecosystem: "jvm",
        tag_is_authority: false,
    },
    // Build scripts. Their version is only observable by EVALUATING the
    // build's own model — it can come from a provider reference, a version
    // catalogue, or a convention plugin — so a reader for them belongs to a
    // provider that can ask the tool, not to a pattern matcher.
    DeclarationSpec {
        path: "build.gradle.kts",
        extraction: None,
        ecosystem: "jvm",
        tag_is_authority: false,
    },
    DeclarationSpec {
        path: "build.gradle",
        extraction: None,
        ecosystem: "jvm",
        tag_is_authority: false,
    },
    DeclarationSpec {
        path: "Directory.Build.props",
        extraction: Some((
            DeclarationFormat::TaggedElement,
            ValueLocator::Tag("Version"),
        )),
        ecosystem: "dotnet",
        tag_is_authority: false,
    },
    DeclarationSpec {
        path: "Directory.Packages.props",
        extraction: Some((
            DeclarationFormat::TaggedElement,
            ValueLocator::Tag("Version"),
        )),
        ecosystem: "dotnet",
        tag_is_authority: false,
    },
    DeclarationSpec {
        path: "CMakeLists.txt",
        extraction: Some((
            DeclarationFormat::LabelledCall,
            ValueLocator::Tag("VERSION"),
        )),
        ecosystem: "cpp",
        tag_is_authority: false,
    },
    DeclarationSpec {
        // A module path and a language level. Verified: it carries no
        // product version, and writing an extractor for it would invent one.
        path: "go.mod",
        extraction: None,
        ecosystem: "go",
        tag_is_authority: true,
    },
    DeclarationSpec {
        path: "MODULE.bazel",
        extraction: None,
        ecosystem: "bazel",
        tag_is_authority: true,
    },
    DeclarationSpec {
        path: "WORKSPACE",
        extraction: None,
        ecosystem: "bazel",
        tag_is_authority: true,
    },
];

/// One file, observed as one source.
///
/// This is the granularity that makes the architecture honest. If ONE
/// provider read all the files, it would have to decide what to do when two
/// of them disagree — and every answer it could give is a policy: pick one,
/// or report a conflict. Both belong to the reducer, which is the only place
/// that is allowed to decide. With one provider per file, disagreement is
/// not a case the provider has to handle: it is two declarations, and the
/// reducer says `Ambiguous` without being asked.
#[derive(Debug)]
pub struct SingleDeclarationProvider {
    spec: &'static DeclarationSpec,
    provider_id: String,
    provider_version: String,
    capabilities: Vec<String>,
}

impl SingleDeclarationProvider {
    /// A provider over exactly one declaration.
    pub fn for_spec(spec: &'static DeclarationSpec) -> Self {
        Self {
            spec,
            provider_id: format!("sddk.gateway.declaration-file/{}", spec.path),
            provider_version: env!("CARGO_PKG_VERSION").to_owned(),
            capabilities: vec![PRODUCT_VERSION_OBSERVATION.to_owned()],
        }
    }

    /// The declaration this provider observes.
    pub fn spec(&self) -> &'static DeclarationSpec {
        self.spec
    }
}

impl VersionResolverPort for SingleDeclarationProvider {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }
    fn provider_version(&self) -> &str {
        &self.provider_version
    }
    fn capabilities(&self) -> &[String] {
        &self.capabilities
    }
    fn observe(&self, target: &ReleaseTarget) -> Result<VersionProbe, ProviderError> {
        let path = Path::new(target.root()).join(self.spec.path);
        if !path.exists() {
            return Ok(VersionProbe::NotApplicable {
                reason: format!("{} no esta en este target", self.spec.path),
            });
        }
        if self.spec.tag_is_authority {
            // A declaration of absence, not silence. The distinction is what
            // lets a target whose convention is "the version lives on the
            // release reference" be reported as resolved, while one that
            // merely said nothing still fails closed.
            return Ok(VersionProbe::ReleaseRefIsAuthority {
                declared_by: format!(
                    "{} no declara version de producto; su convencion es que la lleva la release ref",
                    self.spec.ecosystem
                ),
            });
        }
        let Some((format, locator)) = self.spec.extraction else {
            return Ok(VersionProbe::NotApplicable {
                reason: format!("{} no es fuente de version declarable", self.spec.path),
            });
        };
        let content = std::fs::read_to_string(&path).map_err(|e| ProviderError::Unavailable {
            provider_id: self.provider_id.clone(),
            reason: format!("{} no se pudo leer: {e}", self.spec.path),
        })?;
        match extract(format, locator, &content) {
            Ok(version) => Ok(VersionProbe::Declared {
                version,
                evidence: VersionEvidence {
                    source_kind: format!("declaracion-en-fichero/{}", self.spec.path),
                    digest: None,
                    location: Some(self.spec.path.to_owned()),
                },
            }),
            Err(ExtractError::Absent) => Ok(VersionProbe::Undeclared {
                reason: format!("{} existe y no declara version", self.spec.path),
            }),
            Err(ExtractError::Parse(reason)) => Err(ProviderError::Unavailable {
                provider_id: self.provider_id.clone(),
                reason: format!(
                    "{} no se pudo interpretar como {}: {reason}",
                    self.spec.path,
                    format.label()
                ),
            }),
        }
    }
}

/// Why reading a file did not yield a value. The two cases are different
/// facts and the difference is the whole point of this provider existing.
enum ExtractError {
    /// The file parsed and does not declare the value.
    Absent,
    /// The file could not be parsed as the format it is declared to be.
    Parse(String),
}

fn extract(
    format: DeclarationFormat,
    locator: ValueLocator,
    content: &str,
) -> Result<ProductVersion, ExtractError> {
    let raw = match format {
        DeclarationFormat::Toml => {
            let doc: toml::Value = content
                .parse::<toml::Value>()
                .map_err(|e: toml::de::Error| ExtractError::Parse(e.to_string()))?;
            let found = match locator {
                ValueLocator::Keys(keys) => keys.iter().find_map(|k| doc.get(*k)),
                ValueLocator::AnyOfKeys(paths) => paths.iter().find_map(|path| walk(&doc, path)),
                ValueLocator::Tag(tag) => doc.get(tag),
            };
            found
                .and_then(as_version_string)
                .ok_or(ExtractError::Absent)?
        }
        DeclarationFormat::Json => {
            let doc: serde_json::Value =
                serde_json::from_str(content).map_err(|e| ExtractError::Parse(e.to_string()))?;
            let found = match locator {
                ValueLocator::Keys(keys) => keys.iter().find_map(|k| doc.get(*k)),
                ValueLocator::AnyOfKeys(paths) => {
                    paths.iter().find_map(|path| walk_json(&doc, path))
                }
                ValueLocator::Tag(tag) => doc.get(tag),
            };
            found
                .and_then(|v| v.as_str().map(str::to_owned))
                .ok_or(ExtractError::Absent)?
        }
        DeclarationFormat::KeyValue => {
            let ValueLocator::Tag(tag) = locator else {
                return Err(ExtractError::Absent);
            };
            content
                .lines()
                .filter_map(|line| {
                    let line = line.trim();
                    let (k, v) = line.split_once('=')?;
                    (k.trim() == tag).then(|| v.trim().to_owned())
                })
                .next()
                .ok_or(ExtractError::Absent)?
        }
        DeclarationFormat::TaggedElement => {
            let ValueLocator::Tag(tag) = locator else {
                return Err(ExtractError::Absent);
            };
            let open = format!("<{tag}>");
            let close = format!("</{tag}>");
            let start = content.find(&open).ok_or(ExtractError::Absent)? + open.len();
            let end = content[start..].find(&close).ok_or(ExtractError::Absent)? + start;
            content[start..end].trim().to_owned()
        }
        DeclarationFormat::LabelledCall => {
            let ValueLocator::Tag(tag) = locator else {
                return Err(ExtractError::Absent);
            };
            // `project(VERSION 1.2.3)` — the label and the value, in that
            // order, inside one call. Deliberately narrow: a build script is
            // code, and this is the only shape read here, not a parser for
            // the language.
            let call = content
                .lines()
                .find(|line| line.contains("project("))
                .ok_or(ExtractError::Absent)?;
            let after = call.split_once(tag).ok_or(ExtractError::Absent)?.1;
            after
                .split(|c: char| c.is_whitespace() || c == ')' || c == '"')
                .find(|token| !token.is_empty())
                .ok_or(ExtractError::Absent)?
                .to_owned()
        }
    };

    ProductVersion::new(raw).map_err(|e| ExtractError::Parse(e.to_string()))
}

fn walk<'a>(doc: &'a toml::Value, path: &[&str]) -> Option<&'a toml::Value> {
    let (first, rest) = path.split_first()?;
    let next = doc.get(*first)?;
    if rest.is_empty() {
        return Some(next);
    }
    walk(next, rest)
}

fn walk_json<'a>(doc: &'a serde_json::Value, path: &[&str]) -> Option<&'a serde_json::Value> {
    let (first, rest) = path.split_first()?;
    let next = doc.get(*first)?;
    if rest.is_empty() {
        return Some(next);
    }
    walk_json(next, rest)
}

fn as_version_string(value: &toml::Value) -> Option<String> {
    value.as_str().map(str::to_owned)
}

// ---------------------------------------------------------------------------
// The project's own declaration — a provider like any other
// ---------------------------------------------------------------------------

/// Where a project declares where its own version lives, relative to the
/// target root.
///
/// This used to be a constant in the crate that made the decision, read by a
/// `match` on the failure it was rescuing. It is a **file name**, so it lives
/// here now with the other file names. What the engine kept is the law that
/// a declaration rescues an absence and never an unreadable source — and that
/// law is no longer special-cased at all: it falls out of the reducer's
/// ordinary precedence, because a declared version outranks a declared
/// absence and an unreadable source outranks both.
pub const DECLARED_AUTHORITY_PATH: &str = ".sddk/version-source.json";

/// Reads the project's own declaration, if it wrote one.
///
/// The shape is deliberately tiny and versioned: `{"schema_version": 1,
/// "authority": "tag"}`. Anything else — a missing file, a broken document, an
/// unknown schema, a value this build does not know — is answered honestly
/// and separately, because those are four different situations that an
/// operator has to tell apart.
#[derive(Debug)]
pub struct DeclaredAuthorityProvider {
    provider_id: String,
    provider_version: String,
    capabilities: Vec<String>,
}

impl Default for DeclaredAuthorityProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl DeclaredAuthorityProvider {
    /// A provider over the project's declaration.
    pub fn new() -> Self {
        Self {
            provider_id: format!("sddk.gateway/{}", DECLARED_AUTHORITY_PATH),
            provider_version: env!("CARGO_PKG_VERSION").to_owned(),
            capabilities: vec![PRODUCT_VERSION_OBSERVATION.to_owned()],
        }
    }
}

impl VersionResolverPort for DeclaredAuthorityProvider {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    fn provider_version(&self) -> &str {
        &self.provider_version
    }

    fn capabilities(&self) -> &[String] {
        &self.capabilities
    }

    fn observe(&self, target: &ReleaseTarget) -> Result<VersionProbe, ProviderError> {
        let path = Path::new(target.root()).join(DECLARED_AUTHORITY_PATH);
        let Ok(raw) = std::fs::read_to_string(&path) else {
            // Absent is the normal case for almost every project, and it is
            // not a question: this target simply has nothing to declare. Note
            // that the read error is not inspected — a file that exists but
            // cannot be read must be reported as unreadable, and treating the
            // two as one is precisely the substitution that a prior falsifier
            // exploited on the other side of this codebase.
            return if path.exists() {
                Ok(VersionProbe::Invalid {
                    reason: format!("{DECLARED_AUTHORITY_PATH} existe y no se pudo leer"),
                })
            } else {
                Ok(VersionProbe::NotApplicable {
                    reason: format!("este target no declara nada en {DECLARED_AUTHORITY_PATH}"),
                })
            };
        };

        let doc: serde_json::Value =
            raw.parse()
                .map_err(|e: serde_json::Error| ProviderError::Unavailable {
                    provider_id: self.provider_id.clone(),
                    reason: format!("{DECLARED_AUTHORITY_PATH} no se pudo interpretar: {e}"),
                })?;

        if doc.get("schema_version").and_then(|v| v.as_u64()) != Some(1) {
            return Err(ProviderError::Unavailable {
                provider_id: self.provider_id.clone(),
                reason: format!(
                    "{DECLARED_AUTHORITY_PATH} declara schema_version {:?} y este build entiende 1",
                    doc.get("schema_version")
                ),
            });
        }

        match doc.get("authority").and_then(|v| v.as_str()) {
            Some("tag") => Ok(VersionProbe::ReleaseRefIsAuthority {
                declared_by: format!(
                    "el proyecto declara en {DECLARED_AUTHORITY_PATH} que su version la lleva la release ref"
                ),
            }),
            Some(other) => Err(ProviderError::Unavailable {
                provider_id: self.provider_id.clone(),
                reason: format!(
                    "{DECLARED_AUTHORITY_PATH} declara authority {other:?}; este build entiende \"tag\""
                ),
            }),
            None => Err(ProviderError::Unavailable {
                provider_id: self.provider_id.clone(),
                reason: format!(
                    "{DECLARED_AUTHORITY_PATH} no declara \"authority\" (se esperaba \"tag\")"
                ),
            }),
        }
    }
}

/// The registry a caller gets when it has not composed one itself.
///
/// **One provider per declaration file**, deliberately. Two files declaring
/// different versions is not a case this function has to handle: it is two
/// observations, and the reducer names the conflict. A single provider
/// reading every file would have to choose, and choosing is a policy that
/// does not belong here.
///
/// The project's own declaration is registered alongside them, and it is
/// registered rather than special-cased for the same reason: it is one more
/// way of saying something about the version, so it answers the same question
/// the same way.
/// Every provider SDDK ships, and **nothing that runs a process**.
///
/// Esta es la función que todo el resto del repo llama, y su propiedad es lo que
/// hace que el precio de preguntar al build tool sea una decisión y no una
/// sorpresa: quien quiera pagarlo pide [`version_registry_with`].
pub fn default_version_registry() -> sddk_domain::version_authority::VersionResolverRegistry {
    version_registry_with(None)
}

// ---------------------------------------------------------------------------
// Asking the build tool, instead of reading its script
// ---------------------------------------------------------------------------

/// The files whose presence means "this directory is a Gradle build".
///
/// **Presence only, and never content.** El criterio es que el directorio tenga
/// un fichero de build *propio*, no lo que ese fichero diga: decidir con el
/// contenido sería volver a leer el lenguaje, que es justo lo que este provider
/// existe para no hacer.
///
/// MEDIDO: separa los casos que importan. `docs/guia` dentro de un build no
/// tiene ninguno y sin este criterio lanzaría un subproceso de 3 s para que
/// Gradle contestara `Project directory '…' is not part of the build`. Y
/// `lib-b`, un submódulo que **sí** declara version propia, tiene el suyo y sin
/// este criterio se perdería.
pub const GRADLE_BUILD_FILES: &[&str] = &[
    "build.gradle.kts",
    "build.gradle",
    "settings.gradle.kts",
    "settings.gradle",
];

/// One build tool SDDK knows how to **ask**, with every face it implies.
///
/// ## Por qué un tipo y no un nombre
///
/// MEDIDO, y esta es la razón de existir. Con `--build-tool mvn` sobre un build
/// Gradle y un ejecutable instrumentado, `mvn` recibió `properties --offline` —un
/// goal que no existe en Maven— y el informe atribuyó la respuesta a
/// `././build.gradle`, un fichero que Maven nunca abrió.
///
/// Eso no fue un error de un argumento mal puesto: fueron **tres** caras
/// desalineadas a la vez. El programa venía de la bandera, los args de una
/// constante de Gradle, y la atribución de `location` de `GRADLE_BUILD_FILES`.
/// Cada una era correcta por separado y las tres juntas fabricaron una evidencia.
///
/// El tipo las ata: un dialecto **es** el programa, sus args, sus ficheros de
/// build, su parser y su centinela. Pedir una herramienta distinta de la que
/// aporta el nombre ya no se puede expresar, porque no hay forma de(pair) un
/// nombre suelto con las otras cuatro.
///
/// ## Por qué no hay `Default`
///
/// Porque un default aquí es un Gradle silencioso con el nombre de otra cosa, y
/// eso fue exactamente lo medido. Un nombre no reconocido es un error de la línea
/// de comandos ([`UnknownBuildTool`]), no una suposición.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BuildToolDialect {
    /// `gradle properties --offline`, parsed from `key: value` output.
    GradleProperties,
    /// `mvn help:evaluate`, parsed from the single-expression report.
    ///
    /// MEDIDO en su forma de comando; su salida se declara aquí como el contrato
    /// que el parser acepta, que es lo que hace falsificable la ley sin Maven
    /// instalado.
    MavenHelpEvaluate,
}

/// The `--build-tool` name was not one SDDK knows how to ask.
///
/// Deliberately separate from [`ProviderError`]: this one happens before any
/// provider exists, and its remedy is "spell a supported tool", which is a
/// different fix than anything an operator can do about a provider that ran and
/// failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown build tool `{0}`; SDDK knows how to ask: {supported}", supported = Self::SUPPORTED.join(", "))]
pub struct UnknownBuildTool(pub String);

impl UnknownBuildTool {
    /// The names `parse` accepts, in the order they are offered.
    pub const SUPPORTED: [&'static str; 2] = ["gradle", "maven"];
}

impl BuildToolDialect {
    /// The dialects SDDK can ask.
    pub const ALL: &'static [Self] = &[Self::GradleProperties, Self::MavenHelpEvaluate];

    /// The dialect for a name, or the error that names the ones that exist.
    ///
    /// Sin default y sin normalización silenciosa: `--build-tool GRADLE` falla
    /// igual que `--build-tool mvn`. Aceptar mayúsculas would be a guess, y this
    /// function exists to replace guesses.
    pub fn parse(name: &str) -> Result<Self, UnknownBuildTool> {
        match name {
            "gradle" => Ok(Self::GradleProperties),
            "maven" => Ok(Self::MavenHelpEvaluate),
            otro => Err(UnknownBuildTool(otro.to_owned())),
        }
    }

    /// The name this dialect is invoked by, and by which it is reported.
    pub fn id(self) -> &'static str {
        match self {
            Self::GradleProperties => "gradle",
            Self::MavenHelpEvaluate => "maven",
        }
    }

    /// How this dialect is run.
    ///
    /// `--offline` on Gradle because a version resolution cannot need the
    /// network: a provider that has to download dependencies turns a local
    /// question into one that fails when the mirror is down.
    pub fn invocation(self) -> BuildModelInvocation {
        match self {
            Self::GradleProperties => BuildModelInvocation {
                program: "gradle".to_owned(),
                args: vec!["properties".to_owned(), "--offline".to_owned()],
                dialect: self,
            },
            Self::MavenHelpEvaluate => BuildModelInvocation {
                program: "mvn".to_owned(),
                args: vec![
                    "--offline".to_owned(),
                    "help:evaluate".to_owned(),
                    "-Dexpression=project.version".to_owned(),
                    "-DforceStdout".to_owned(),
                    "-q".to_owned(),
                ],
                dialect: self,
            },
        }
    }

    /// The build files whose **presence** makes a target this dialect's subject.
    ///
    /// Presence, never content: deciding by content would be reading the build
    /// language again, which is what this provider exists to avoid.
    pub fn build_files(self) -> &'static [&'static str] {
        match self {
            Self::GradleProperties => GRADLE_BUILD_FILES,
            Self::MavenHelpEvaluate => &["pom.xml"],
        }
    }

    /// Reads what one run of this dialect's tool said.
    pub fn parse_answer(self, stdout: &str) -> BuildModelAnswer {
        match self {
            Self::GradleProperties => parse_build_model(stdout),
            Self::MavenHelpEvaluate => parse_maven_model(stdout),
        }
    }
}

/// How to ask the build tool what its model says.
///
/// The `dialect` is what makes this type worth existing: `program` and `args`
/// alone can be desynchronised from the parser and the build files, and that is
/// how a Maven run came to be reported against a Gradle file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildModelInvocation {
    /// The program to run.
    program: String,
    /// Its arguments.
    args: Vec<String>,
    /// The dialect these two belong to.
    dialect: BuildToolDialect,
}

impl BuildModelInvocation {
    /// `gradle properties --offline`, measured.
    pub fn gradle() -> Self {
        BuildToolDialect::GradleProperties.invocation()
    }

    /// The program to run.
    pub fn program(&self) -> &str {
        &self.program
    }

    /// Its arguments.
    pub fn args(&self) -> &[String] {
        &self.args
    }

    /// The dialect this invocation belongs to.
    pub fn dialect(&self) -> BuildToolDialect {
        self.dialect
    }
}

/// What one run of the tool said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildModelAnswer {
    /// It named a version.
    Declared {
        /// The version, verbatim from the tool.
        version: ProductVersion,
        /// Which project it answered for, when it said.
        project: Option<String>,
    },
    /// It answered, and the answer was "no version".
    ///
    /// MEDIDO: Gradle responde `version: unspecified`, y esa palabra **no** es
    /// una versión. Es la respuesta de una herramienta diciendo que no tiene
    /// nada, y tomarla por un valor sería inventar `unspecified` como si fuera
    /// `1.2.3`.
    Unspecified {
        /// Which project it answered for, when it said.
        project: Option<String>,
    },
    /// It exited successfully and named no version at all.
    Silent,
}

/// Reads one `key: value` report, and nothing else.
///
/// ## Por qué la ley vive en una función pura
///
/// Porque es falsificable sin lanzar un subproceso. Un mutante que rompe esto
/// necesita un `gradle` de verdad para ejecutarse, y una ley que necesita la
/// herramienta para falsificarse es una ley que nadie falsifica.
pub fn parse_build_model(stdout: &str) -> BuildModelAnswer {
    let mut project = None;
    let mut version = None;
    for line in stdout.lines() {
        let line = line.trim();
        // `Root project 'multi'` / `Project ':lib-b'`. Solo el encabezado: el
        // resto de `properties` son valores de otros objetos con `@` en medio,
        // y un `:` dentro de ellos no convierte la linea en una clave.
        if let Some(rest) = line.strip_prefix("Root project ") {
            project = Some(rest.trim_matches('\'').to_owned());
            continue;
        }
        if let Some(rest) = line.strip_prefix("Project ") {
            project = Some(rest.trim_matches('\'').to_owned());
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        if key.trim() == "version" {
            version = Some(value.trim().to_owned());
        }
    }
    match version {
        None => BuildModelAnswer::Silent,
        Some(value) if value.is_empty() || value == "unspecified" => {
            BuildModelAnswer::Unspecified { project }
        }
        Some(value) => match ProductVersion::new(value) {
            Ok(version) => BuildModelAnswer::Declared { version, project },
            // La herramienta dijo algo que no es una versión. No se inventa un
            // valor y no se pierde SU palabra: se devuelve la ausencia, que es
            // lo que measurablemente significa.
            Err(_) => BuildModelAnswer::Unspecified { project },
        },
    }
}

/// Reads one `help:evaluate` report, and nothing else.
///
/// ## Por qué otra función y no un parámetro
///
/// Porque los dos dialectos no comparten ni el formato de salida ni la palabra
/// que significa «no lo sé». Gradle contesta `version: unspecified` en un
/// informe `clave: valor`; Maven contesta la expresión sin resolver,
/// `${project.version}`, en una sola línea. Un parser con un parámetro de
/// dialecto acabaría siendo un parser con dos senos, y la mitad que no se
/// ejercita es la que miente.
///
/// Y por la misma razón que [`parse_build_model`]: la ley vive en una función
/// pura para que sus mutantes corran sin lanzar Maven.
pub fn parse_maven_model(stdout: &str) -> BuildModelAnswer {
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // `[WARNING]`, `[ERROR]`, `[INFO]`: Maven hablando ruido, no la respuesta.
        if line.starts_with('[') {
            continue;
        }
        // `${…}` es Maven diciendo que no sabe resolver la expresión. No es un
        // valor, y no hay nada que reportarlo como si lo fuera.
        if line.starts_with("${") && line.ends_with('}') {
            return BuildModelAnswer::Unspecified { project: None };
        }
        return match ProductVersion::new(line) {
            Ok(version) => BuildModelAnswer::Declared {
                version,
                project: None,
            },
            Err(_) => BuildModelAnswer::Unspecified { project: None },
        };
    }
    BuildModelAnswer::Silent
}

/// A provider that asks the build tool, and reports what it says.
///
/// ## Why this exists and why it is not a reader
///
/// Because the version of a Gradle project is only knowable by **evaluating**
/// Gradle: it can come from a literal, from a version catalogue, from a
/// convention plugin, or from a value computed at configuration time. A reader
/// would have to guess which, and its guess would be a silent one.
///
/// MEDIDO, en `v2/build.gradle.kts`: el fichero menciona `version` **tres veces
/// en comentarios** —una de ellas con un `0.44.0` que ya no es cierto— y una
/// vez de verdad. Un patrón que cogiera la primera se llevaría un número que el
/// proyecto ya había corregido. Ese es el fallo entero, y no es hipotético: está
/// en el fichero que motivó este provider.
///
/// ## Y no hay fallback
///
/// Si la herramienta no responde, la respuesta es que no respondió. Nunca un
/// patrón. Un provider que degrada a «me lo leo yo» reintroduce el defecto en
/// silencio, que es la forma en que vuelve.
#[derive(Debug)]
pub struct BuildModelProvider {
    provider_id: String,
    provider_version: String,
    dialect: BuildToolDialect,
    /// El binario a ejecutar. Lo decide el dialecto, salvo que un test lo
    /// sustituya; los args nunca se sustituyen.
    invocation: BuildModelInvocation,
}

impl BuildModelProvider {
    /// A provider that asks this tool, with the faces its name implies.
    pub fn new(dialect: BuildToolDialect) -> Self {
        let invocation = dialect.invocation();
        Self {
            provider_id: format!("sddk.gateway.build-model/{}", dialect.id()),
            provider_version: env!("CARGO_PKG_VERSION").to_owned(),
            dialect,
            invocation,
        }
    }

    /// The dialect this provider asks with.
    pub fn dialect(&self) -> BuildToolDialect {
        self.dialect
    }

    /// A provider that runs `program` **in place of** the dialect's binary, and
    /// asks it the dialect's question.
    ///
    /// ## Por que esto no reabre el defecto que se midio
    ///
    /// Lo que se fabrico era un par `(programa, args)` distinto del dialecto que
    /// decia, desde la capa de arriba. Aqui el dialecto sigue mandando en todo lo
    /// observable —args, ficheros de build, parser, centinela y `provider_id`— y
    /// lo unico sustituible es **el binario que se ejecuta**, que es lo que un
    /// test necesita cambiar y lo que un operador no cambia por accidente.
    ///
    /// Los args no se pueden sustituir porque no se aceptan: no hay por donde
    /// pasarlos. Esa es la diferencia entre esto y lo medido.
    #[doc(hidden)]
    pub fn with_program(dialect: BuildToolDialect, program: impl Into<String>) -> Self {
        Self::with_program_and_prefix(dialect, program, &[])
    }

    /// Igual que [`BuildModelProvider::with_program`], y ademas antepone unos
    /// argumentos al binario.
    ///
    /// ## Que es `leading` y por que existe
    ///
    /// Es **como arrancar el binario**, no **que preguntarle**: el prefijo va
    /// delante de los args del dialecto, y quien pregunta sigue siendo el
    /// dialecto. Un test lo usa para decir «arranca este script con `/bin/sh`»
    /// en vez de «ejecuta este script con su shebang», que es la diferencia
    /// entre un ETXTBSY posible y uno imposible.
    #[doc(hidden)]
    pub fn with_program_and_prefix(
        dialect: BuildToolDialect,
        program: impl Into<String>,
        leading: &[&str],
    ) -> Self {
        let mut invocation = dialect.invocation();
        invocation.program = program.into();
        let mut args: Vec<String> = leading.iter().map(|a| (*a).to_owned()).collect();
        args.append(&mut invocation.args);
        invocation.args = args;
        Self {
            provider_id: format!("sddk.gateway.build-model/{}", dialect.id()),
            provider_version: env!("CARGO_PKG_VERSION").to_owned(),
            dialect,
            invocation,
        }
    }
}

/// El motivo por el que el proceso no llego a arrancar, y **de quien es**.
///
/// ## Por que esta distincion existe
///
/// `b4` existe porque «no tengo la herramienta» y «la herramienta fallo» son dos
/// problemas con dos reparaciones opuestas, y confundirlos manda al operador a
/// instalar algo que ya tiene o a depurar un binario que no existe.
///
/// MEDIDO, durante la construccion de este fichero: un `ETXTBSY` —el binario
/// existe y esta en uso por otro proceso— se reportaba con el mismo texto que
/// un `ENOENT`. Medido de verdad: 4 de 15 ejecuciones de una suiteFallaron asi,
/// con 1219 procesos en el host. Es un fallo del ENTORNO y su texto decia que
/// faltaba una herramienta, que es justo la mentira que este bloque vino a
/// arreglar.
///
/// La herramienta no puede decir por que no arranco —nunca arranco— asi que el
/// motivo es NUESTRO, y por eso tiene que ser exacto: «no lo tengo» solo para
/// `ENOENT`, y «otro proceso lo tiene ocupado» para el resto.
pub fn motivo_de_ejecucion(dialecto: &str, root: &str, error: &std::io::Error) -> String {
    match error.kind() {
        std::io::ErrorKind::NotFound => format!(
            "`{dialecto}` no esta instalado, asi que no se pudo ejecutar en {root}: \
             se necesita en el PATH"
        ),
        _ if error.raw_os_error() == Some(ETXTBSY) => format!(
            "`{dialecto}` esta en {root} pero otro proceso lo tiene ocupado \
             (ETXTBSY): el binario existe y es ejecutable, asi que esto es una \
             carrera del entorno y no una herramienta que falte. Reintentar lo \
             resuelve; instalarla no. Error del sistema: {error}"
        ),
        _ => format!("`{dialecto}` no se pudo ejecutar en {root}: {error}"),
    }
}

/// `ETXTBSY` en Linux. Constante en vez de numero suelto porque un numero
/// magico en un mensaje es un mensaje que no se puede revisar.
#[cfg(unix)]
const ETXTBSY: i32 = 26;
#[cfg(not(unix))]
const ETXTBSY: i32 = -1;

impl VersionResolverPort for BuildModelProvider {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    fn provider_version(&self) -> &str {
        &self.provider_version
    }

    fn capabilities(&self) -> &[String] {
        use std::sync::OnceLock;
        static ONE: OnceLock<Vec<String>> = OnceLock::new();
        ONE.get_or_init(|| vec![PRODUCT_VERSION_OBSERVATION.to_owned()])
    }

    fn observe(&self, target: &ReleaseTarget) -> Result<VersionProbe, ProviderError> {
        let root = std::path::Path::new(target.root());
        let build_file = self
            .dialect
            .build_files()
            .iter()
            .map(|name| root.join(name))
            .find(|path| path.is_file());
        let Some(build_file) = build_file else {
            return Ok(VersionProbe::NotApplicable {
                reason: format!(
                    "{} no tiene fichero de build de {}, asi que no hay modelo que \
                     preguntar",
                    target.root(),
                    self.dialect.id()
                ),
            });
        };

        let invocation = &self.invocation;
        let output = std::process::Command::new(invocation.program())
            .args(invocation.args())
            .current_dir(root)
            .output()
            .map_err(|error| ProviderError::Unavailable {
                provider_id: self.provider_id.clone(),
                reason: motivo_de_ejecucion(self.dialect.id(), target.root(), &error),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let motivo = meaningful_line(&stderr)
                .unwrap_or_else(|| format!("salio con {} sin decir por que", output.status));
            // El motivo es DE LA HERRAMIENTA, y esa es toda la ley: un fallo
            // cerrado cuyo motivo es indistinguible de cien causas deja al
            // operador sin nada que hacer. MEDIDO: Gradle contesta
            // `Directory '…' does not contain a Gradle build.`, que es mas
            // util que cualquier resumen que escribiramos aqui.
            return Err(ProviderError::Unavailable {
                provider_id: self.provider_id.clone(),
                reason: format!("`{}` no pudo responder por {}", self.dialect.id(), motivo),
            });
        }

        match self
            .dialect
            .parse_answer(&String::from_utf8_lossy(&output.stdout))
        {
            BuildModelAnswer::Declared { version, project } => Ok(VersionProbe::Declared {
                version,
                evidence: VersionEvidence {
                    // Con el proyecto que contesto, porque es de quien es la
                    // respuesta: MEDIDO, desde un subdirectorio Gradle contesta
                    // `Project ':lib-b'`, y un informe que no dice de que
                    // proyecto es el valor tiene el mismo defecto que un provider
                    // sin `provider_id` — describes algo sin decir de quien es.
                    source_kind: match &project {
                        Some(project) => {
                            format!("build-model/{}/{project}", self.dialect.id())
                        }
                        None => format!("build-model/{}", self.dialect.id()),
                    },
                    // Sin digest, y a proposito: el valor no sale de los bytes de
                    // un fichero, sale de EVALUAR el build. Poner el digest del
                    // fichero afirmaria que esos bytes determinan el valor, que
                    // es exactamente lo falso que este provider vino a evitar.
                    digest: None,
                    location: Some(build_file.display().to_string()),
                },
            }),
            BuildModelAnswer::Unspecified { project } => Ok(VersionProbe::Undeclared {
                reason: match project {
                    Some(project) => format!(
                        "{} contesto que {project} no tiene version ({}), y eso es una \
                         ausencia declarada, no un valor",
                        self.dialect.id(),
                        UNSPECIFIED_WORD
                    ),
                    None => format!(
                        "{} contesto que no hay version ({}), y eso es una ausencia \
                         declarada, no un valor",
                        self.dialect.id(),
                        UNSPECIFIED_WORD
                    ),
                },
            }),
            BuildModelAnswer::Silent => Ok(VersionProbe::Undeclared {
                reason: format!(
                    "{} salio con {} y no nombr ninguna version: se le pregunto y no \
                     contesto",
                    self.dialect.id(),
                    output.status
                ),
            }),
        }
    }
}

/// The word the tool uses for «no version», measured.
///
/// Va en una constante porque es una **palabra de la herramienta**, y porque el
/// falsador tiene que poder nombrarla sin repetirla en tres sitios del código.
pub const UNSPECIFIED_WORD: &str = "unspecified";

/// The tool's own words, out of its own noise.
///
/// ## Por qué busca despues de «What went wrong:» y no la primera linea
///
/// MEDIDO: Gradle imprime su banner antes de la causa, y el banner dice
/// `FAILURE: Build failed with an exception.` — **que es cierto para todos los
/// fallos de Gradle que han ocurrido jamás**. Una v1 de esta función tomaba la
/// primera linea no vacia y devolvia justo eso, luego un provider que fallaba
/// cerrado con un motivo que no distinguia nada: exactamente el defecto que
/// `ProviderError::Unavailable` dice evitar.
///
/// La causa vive entre `What went wrong:` y la siguiente linea en blanco. Si no
/// aparece, se cae al primer motivo util, porque hay herramientas que no
/// usan esa frase y no se puede exigir la de Gradle a todas.
fn meaningful_line(stderr: &str) -> Option<String> {
    let mut tras_el_titulo = false;
    for line in stderr.lines() {
        let line = line.trim();
        if tras_el_titulo {
            if !line.is_empty() {
                return Some(line.to_owned());
            }
            continue;
        }
        if line.ends_with("What went wrong:") {
            tras_el_titulo = true;
        }
    }
    stderr
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_owned)
}

/// The default registry, plus — when someone asks for it — a provider that
/// **runs** the build tool.
///
/// ## Why evaluating is opt-in and not a default
///
/// MEDIDO: `gradle properties --offline` against un build trivial tarda **3
/// segundos** y levanta una JVM. Ponerlo en el registro por defecto significa
/// que cada `release version inspect` de un repositorio JVM paga tres segundos
/// por target, y que uno de esos targets se cuelgue retrasa el diagnóstico que
/// se pidió precisamente porque algo va mal.
///
/// Invocar una herramienta externa es un **coste y un acoplamiento al entorno**
/// —MEDIDO: el shim de asdf sin `.tool-versions` responde `No version is set
/// for command gradle` y sale 126—, y ninguno de los dos se paga sin que alguien
/// lo pida. Es la misma razón por la que `--naming` y `--role` son banderas y no
/// configuración del repo.
pub fn version_registry_with(
    build_tool: Option<BuildToolDialect>,
) -> sddk_domain::version_authority::VersionResolverRegistry {
    let mut registry = sddk_domain::version_authority::VersionResolverRegistry::new();
    registry.register(Box::new(DeclaredAuthorityProvider::new()));
    for spec in DEFAULT_DECLARATIONS {
        registry.register(Box::new(SingleDeclarationProvider::for_spec(spec)));
    }
    if let Some(dialect) = build_tool {
        registry.register(Box::new(BuildModelProvider::new(dialect)));
    }
    registry
}

// ---------------------------------------------------------------------------
// Which targets exist, in a repository that may hold several
// ---------------------------------------------------------------------------

/// Directories that are never products, because they hold build output or
/// vendored copies rather than source.
///
/// A list, in the adapter, about the filesystem's shape — which is exactly the
/// kind of knowledge that does not belong in the kernel and is not a
/// technology preference either. It is here for one reason: without it, a
/// vendored dependency or a build directory is discovered as a target and turns
/// every monorepo into `AmbiguousTarget`.
///
/// The failure direction matters and is the safe one. A name on this list that
/// is actually a product makes that product **invisible**, and an invisible
/// product shows up as «no version declared» — a refusal that tells the
/// operator to look — never as a wrong answer.
pub const NON_PRODUCT_DIRECTORIES: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "vendor",
    "dist",
    "build",
    ".venv",
    "__pycache__",
];

/// How deep below the repository root a target is looked for.
///
/// **Two**, and the number is the shape of a monorepo rather than a taste:
/// `packages/alpha` and `crates/alpha` are both *two* levels below the root,
/// because `packages` is a grouping directory and not a product. Counting only
/// the grouping directory as one level finds `packages` —which is not a
/// product— and reports a monorepo with four products as having none. That was
/// MEDIDO, no supuesto: la primera versión de este escaneo usaba `1` y
/// devolvía cero targets para un árbol con un producto dentro de `packages/`.
///
/// The bound is a **parameter** rather than a hard-coded depth so that a caller
/// who needs more can ask for more, and the answer always says how far it
/// looked. An unbounded walk of an unknown tree is the kind of thing that is
/// fine in a fixture and fatal in a monorepo with a `node_modules` of three
/// gigabytes.
pub const DEFAULT_TARGET_DEPTH: usize = 2;

/// What a scan found, and how it looked.
///
/// The `depth` travels with the answer because a set of targets with no
/// statement of what was searched is indistinguishable from a set of all the
/// targets that exist, and those two claims are very different.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetScan {
    /// The roots that hold at least one declaration, sorted so the answer does
    /// not depend on the order the filesystem handed them over.
    pub roots: Vec<String>,
    /// How many levels below the repository root were visited.
    pub depth: usize,
    /// Directory names that were skipped, and why they might matter.
    pub skipped: Vec<String>,
}

impl TargetScan {
    /// The scan as a set of [`ReleaseTarget`]s, identified by their path.
    ///
    /// The identity is the path **relative to the repository root**, not the
    /// directory name: two products called `index` in different folders are two
    /// products, and a name that collides is a different problem from a product
    /// that is in two places.
    pub fn targets(&self, repository_root: &Path) -> Vec<ReleaseTarget> {
        self.roots
            .iter()
            .map(|relative| {
                ReleaseTarget::at(
                    relative,
                    Path::new(repository_root)
                        .join(relative)
                        .display()
                        .to_string(),
                )
            })
            .collect()
    }
}

/// Finds the roots that hold a declaration, down to `depth` levels.
///
/// Read-only, and it says so: this only reads directory entries and asks each
/// [`DeclarationSpec`] whether its file is there. It never parses a file,
/// because finding a *candidate* and knowing what it declares are two
/// questions, and mixing them would mean this function could report a product
/// whose declaration turns out to be unreadable — which is the reducer's
/// decision to make, with the evidence in hand.
pub fn scan_release_targets(repository_root: &Path, depth: usize) -> TargetScan {
    let mut roots: Vec<String> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();

    if declaration_present(repository_root) {
        roots.push(".".to_owned());
    }

    let mut frontier = vec![PathBuf::from(repository_root)];
    for _ in 0..depth {
        let mut next = Vec::new();
        for directory in &frontier {
            let Ok(entries) = std::fs::read_dir(directory) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let name = entry.file_name().to_string_lossy().into_owned();
                if NON_PRODUCT_DIRECTORIES.contains(&name.as_str()) || name.starts_with('.') {
                    skipped.push(name);
                    continue;
                }
                if declaration_present(&path) {
                    let relative = path
                        .strip_prefix(repository_root)
                        .unwrap_or(&path)
                        .display()
                        .to_string();
                    roots.push(relative);
                }
                next.push(path);
            }
        }
        frontier = next;
    }

    // Orden canónico por la misma razón que en el dominio: una lista de
    // candidatos que cambia al reordenar el disco es una lista que no se puede
    // comparar con nada, y `AmbiguousTarget` es un error que se compara.
    roots.sort();
    roots.dedup();
    skipped.sort();
    skipped.dedup();
    TargetScan {
        roots,
        depth,
        skipped,
    }
}

/// Whether any known declaration lives directly in `directory`.
///
/// The question is «is there something here that might declare a version»,
/// which is weaker than «does this declare a version» and is asked on purpose:
/// this function must not decide, and a directory holding an unreadable
/// declaration is still a target whose unreadability the reducer reports.
fn declaration_present(directory: &Path) -> bool {
    DEFAULT_DECLARATIONS
        .iter()
        .any(|spec| directory.join(spec.path).exists())
}

/// The targets a release may resolve against, and how the choice was made.
///
/// ## The law, and why it is a law about *location* and not about priority
///
/// **A repository root that resolves is the target.** If the root declares a
/// product version, the repository is about itself, and looking further down
/// for other products would mean answering a question nobody asked.
///
/// This is worth defending against the obvious objection, which is that it is
/// a preference between candidates — the thing the reducer is forbidden from
/// doing. It is not, and the difference is the whole point:
///
/// - the reducer's forbidden preference is between **answers**: given two
///   declarations that disagree, no technology outranks another;
/// - this is between **entities**: asked about a repository and the two
///   products inside it, those are different questions, and the default for
///   «which of these did you mean» when the answer is available at the top is
///   «the one at the top».
///
/// Making it structural rather than a fallback also removes a failure mode:
/// with a fallback, a repository that declares a version at the root and has a
/// product below it would resolve the root and never mention the other, which
/// looks identical to a repository with one product. The scan's result travels
/// with the decision so a caller can always say what else was there.
///
/// ## What happens when the root does not resolve
///
/// Then the products are the ones the scan found, and more than one is
/// `AmbiguousTarget` — closed, with the paths listed. Not a preference between
/// them: they are equally supported and choosing would be inventing an answer.
pub fn release_targets(
    repository_root: &Path,
    registry: &sddk_domain::version_authority::VersionResolverRegistry,
) -> TargetSet {
    let capability = PRODUCT_VERSION_OBSERVATION;
    let root_target = ReleaseTarget::at(".", repository_root.display().to_string());

    // Una sola resolución de la raiz, y dos respuestas distintas de ella. Un
    // target de la raiz esta resuelto si declara una version de producto **o**
    // si declara que su version la lleva la release ref: en el segundo caso
    // tambien ha dicho que es un producto, y buscar mas abajo volveria a
    // preguntar lo que ya contesto.
    let root_authority = registry.resolve(capability, &root_target);
    let root_declares_version = root_authority.version().is_some();
    let root_declares_release_ref = !root_authority.release_ref_declarations().is_empty();

    if root_declares_version || root_declares_release_ref {
        return TargetSet {
            targets: vec![root_target],
            scan: None,
            root_resolved: true,
        };
    }

    let scan = scan_release_targets(repository_root, DEFAULT_TARGET_DEPTH);
    let targets = scan.targets(repository_root);
    TargetSet {
        targets,
        scan: Some(scan),
        root_resolved: false,
    }
}

/// The candidates a release may choose among, and the evidence for how they
/// were gathered.
#[derive(Debug, Clone)]
pub struct TargetSet {
    /// The candidates, in canonical order.
    pub targets: Vec<ReleaseTarget>,
    /// What the scan found, when it ran. `None` means the root answered and
    /// nothing was looked for.
    pub scan: Option<TargetScan>,
    /// Whether the repository root declared a product version.
    pub root_resolved: bool,
}

impl TargetSet {
    /// One-line description of how the candidates were gathered, for a plan
    /// that has to be readable by someone who did not run the scan.
    pub fn provenance(&self) -> String {
        match &self.scan {
            None => "la raiz del repositorio declara su propia version".to_owned(),
            Some(scan) => format!(
                "la raiz no declara version; se buscaron {} raiz/raices hasta {} nivel(es), \
                 omitiendo {}",
                scan.roots.len(),
                scan.depth,
                if scan.skipped.is_empty() {
                    "nada".to_owned()
                } else {
                    scan.skipped.join(", ")
                }
            ),
        }
    }
}
