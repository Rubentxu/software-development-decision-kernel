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
//! extracts a value, and reports one of the four answers
//! ([`VersionProbe`]). Deciding what those answers mean is
//! [`sddk_domain::version_authority::reduce`], which cannot see a file name.
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
    ProductVersion, ProviderError, ReleaseTarget, VersionEvidence, VersionProbe, VersionResolverPort,
    PRODUCT_VERSION_OBSERVATION,
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
            ValueLocator::AnyOfKeys(&[
                &["project", "version"],
                &["tool", "poetry", "version"],
            ]),
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
        extraction: Some((
            DeclarationFormat::KeyValue,
            ValueLocator::Tag("version"),
        )),
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

/// Observes a target's version by reading declaration files.
pub struct DeclarationFileProvider {
    declarations: &'static [DeclarationSpec],
    /// Fixed at construction: a provider's identity is provenance, and a
    /// provider that could rename itself between calls would make two
    /// observations of the same thing look like two sources.
    provider_id: String,
    provider_version: String,
    capabilities: Vec<String>,
}

impl Default for DeclarationFileProvider {
    fn default() -> Self {
        Self::new(DEFAULT_DECLARATIONS)
    }
}

impl DeclarationFileProvider {
    /// A provider over an explicit set of declarations, each observed as its
    /// own source.
    pub fn new(declarations: &'static [DeclarationSpec]) -> Self {
        Self {
            declarations,
            provider_id: "sddk.gateway.declaration-files".to_owned(),
            provider_version: env!("CARGO_PKG_VERSION").to_owned(),
            capabilities: vec![PRODUCT_VERSION_OBSERVATION.to_owned()],
        }
    }

    /// A provider for exactly one declaration.
    pub fn for_spec(spec: &'static DeclarationSpec) -> SingleDeclarationProvider {
        SingleDeclarationProvider {
            spec,
            provider_id: format!("sddk.gateway.declaration-file/{}", spec.path),
            provider_version: env!("CARGO_PKG_VERSION").to_owned(),
            capabilities: vec![PRODUCT_VERSION_OBSERVATION.to_owned()],
        }
    }

    /// The declarations this provider knows.
    pub fn declarations(&self) -> &'static [DeclarationSpec] {
        self.declarations
    }
}

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
    fn observe(
        &self,
        target: &ReleaseTarget,
    ) -> Result<VersionProbe, ProviderError> {
        let path = Path::new(target.root()).join(self.spec.path);
        if !path.exists() {
            return Ok(VersionProbe::NotApplicable {
                reason: format!("{} no esta en este target", self.spec.path),
            });
        }
        if self.spec.tag_is_authority {
            return Ok(VersionProbe::Undeclared {
                reason: format!("el tag es la autoridad de {}", self.spec.ecosystem),
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

impl DeclarationFileProvider {
    /// Observes one directory.
    ///
    /// The four answers, and what each one means here:
    ///
    /// - nothing present at all -> `NotApplicable`: this provider has no
    ///   subject in this target.
    /// - files present, none declaring, or declaring nothing -> `Undeclared`:
    ///   it looked, and the target is silent. That is a fact about the
    ///   target, and it is different from the case above.
    /// - a file that cannot be read or parsed -> `Invalid`: the target HAD
    ///   something to say and it could not be understood. Fails closed.
    /// - a value -> `Declared` with the file as its evidence location.
    ///
    /// Divergence between two declaring files is **not** decided here. This
    /// function reports one answer per provider, and if two files disagree
    /// that is `Invalid` with both names — because a provider that reported
    /// one of them and stayed silent about the other would be deciding by
    /// order, and order is not a policy.
    pub fn observe_at(&self, root: &Path) -> Result<VersionProbe, ProviderError> {
        let mut declared: Vec<(ProductVersion, &'static str)> = Vec::new();
        let mut present: Vec<&'static str> = Vec::new();
        let mut tag_only: Vec<&'static str> = Vec::new();

        for spec in self.declarations {
            let path = root.join(spec.path);
            if !path.exists() {
                continue;
            }
            present.push(spec.path);

            if spec.tag_is_authority {
                tag_only.push(spec.ecosystem);
                continue;
            }

            let Some((format, locator)) = spec.extraction else {
                // Present, and declared as not a version source. Not a
                // finding, not a failure.
                continue;
            };

            let content = std::fs::read_to_string(&path).map_err(|e| ProviderError::Unavailable {
                provider_id: self.provider_id.clone(),
                reason: format!("{} no se pudo leer: {e}", spec.path),
            })?;

            match extract(format, locator, &content) {
                Ok(version) => declared.push((version, spec.path)),
                Err(ExtractError::Absent) => {}
                Err(ExtractError::Parse(reason)) => {
                    return Err(ProviderError::Unavailable {
                        provider_id: self.provider_id.clone(),
                        reason: format!(
                            "{} no se pudo interpretar como {}: {reason}",
                            spec.path,
                            format.label()
                        ),
                    });
                }
            }
        }

        match declared.len() {
            0 => {
                if !tag_only.is_empty() {
                    // The ecosystem's tag is the authority and it declares
                    // nothing. That is `Undeclared`, and it is informative:
                    // it is exactly the case that must NOT block another
                    // provider that did read a value.
                    return Ok(VersionProbe::Undeclared {
                        reason: format!("el tag es la autoridad de {tag_only:?}"),
                    });
                }
                if present.is_empty() {
                    return Ok(VersionProbe::NotApplicable {
                        reason: "ningun fichero de declaracion en este target".to_owned(),
                    });
                }
                Ok(VersionProbe::Undeclared {
                    reason: format!("encontrados sin declarar: {present:?}"),
                })
            }
            1 => {
                let (version, path) = declared.remove(0);
                Ok(VersionProbe::Declared {
                    version,
                    evidence: VersionEvidence {
                        source_kind: format!("declaracion-en-fichero/{path}"),
                        digest: None,
                        location: Some(path.to_owned()),
                    },
                })
            }
            _ => Err(ProviderError::Unavailable {
                provider_id: self.provider_id.clone(),
                reason: format!(
                    "varios ficheros declaran versiones distintas: {}",
                    declared
                        .iter()
                        .map(|(v, p)| format!("{p}={v}"))
                        .collect::<Vec<_>>()
                        .join("; ")
                ),
            }),
        }
    }
}

impl VersionResolverPort for DeclarationFileProvider {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    fn provider_version(&self) -> &str {
        &self.provider_version
    }

    fn capabilities(&self) -> &[String] {
        &self.capabilities
    }

    fn observe(
        &self,
        target: &ReleaseTarget,
    ) -> Result<VersionProbe, ProviderError> {
        self.observe_at(Path::new(target.root()))
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
                ValueLocator::AnyOfKeys(paths) => {
                    paths.iter().find_map(|path| walk(&doc, path))
                }
                ValueLocator::Tag(tag) => doc.get(tag),
            };
            found.and_then(as_version_string).ok_or(ExtractError::Absent)?
        }
        DeclarationFormat::Json => {
            let doc: serde_json::Value = serde_json::from_str(content)
                .map_err(|e| ExtractError::Parse(e.to_string()))?;
            let found = match locator {
                ValueLocator::Keys(keys) => keys.iter().find_map(|k| doc.get(*k)),
                ValueLocator::AnyOfKeys(paths) => {
                    paths.iter().find_map(|path| walk_json(&doc, path))
                }
                ValueLocator::Tag(tag) => doc.get(tag),
            };
            found.and_then(|v| v.as_str().map(str::to_owned)).ok_or(ExtractError::Absent)?
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

/// The registry a caller gets when it has not composed one itself.
///
/// **One provider per declaration file**, deliberately. Two files declaring
/// different versions is not a case this function has to handle: it is two
/// observations, and the reducer names the conflict. A single provider
/// reading every file would have to choose, and choosing is a policy that
/// does not belong here.
pub fn default_version_registry() -> sddk_domain::version_authority::VersionResolverRegistry {
    let mut registry = sddk_domain::version_authority::VersionResolverRegistry::new();
    for spec in DEFAULT_DECLARATIONS {
        registry.register(Box::new(DeclarationFileProvider::for_spec(spec)));
    }
    registry
}
