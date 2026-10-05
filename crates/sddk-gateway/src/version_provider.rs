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
use std::path::Path;

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
pub fn default_version_registry() -> sddk_domain::version_authority::VersionResolverRegistry {
    let mut registry = sddk_domain::version_authority::VersionResolverRegistry::new();
    registry.register(Box::new(DeclaredAuthorityProvider::new()));
    for spec in DEFAULT_DECLARATIONS {
        registry.register(Box::new(SingleDeclarationProvider::for_spec(spec)));
    }
    registry
}
