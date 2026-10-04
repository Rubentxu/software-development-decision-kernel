//! Identity types for projects, workspaces, and cycles.
//!
//! Provides stable identification independent of filesystem paths or remote URL variations.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use thiserror::Error;
use uuid::Uuid;

/// Errors that can occur during identity operations.
#[derive(Debug, Error)]
pub enum IdentityError {
    /// A project identifier does not satisfy the canonical format.
    #[error("invalid project ID format: {0}")]
    InvalidProjectId(String),
    /// A workspace identifier is empty or invalid.
    #[error("invalid workspace ID format: {0}")]
    InvalidWorkspaceId(String),
    /// A cycle identifier does not satisfy the canonical format.
    #[error("invalid cycle ID format: {0}")]
    InvalidCycleId(String),
    /// A remote URL is empty or cannot be normalized.
    #[error("empty or invalid remote URL")]
    InvalidRemoteUrl,
    /// Stable project identity was requested without a scope.
    #[error("scope is required for project identity")]
    MissingScope,
    /// A monorepo scope is unsafe or cannot be normalized.
    #[error("invalid project scope: {0}")]
    InvalidScope(String),
    /// A fallback identity seed is absent or is not a UUID.
    #[error("fallback seed must be a valid UUID")]
    InvalidFallbackSeed,
    /// An alias chain revisits a project id, so following it would not
    /// terminate.
    ///
    /// **Hard error, not a warning.** A non-terminating resolution is silent by
    /// construction: it either hangs or, if a step limit is added later, stops
    /// at an arbitrary point and reports a plausible-looking id. Both are worse
    /// than refusing. The chain is named so the operator can see the loop.
    #[error("project alias cycle: {0}")]
    AliasCycle(String),
    /// An alias chain exceeded the hop limit without cycling.
    ///
    /// **A different variant from `AliasCycle` on purpose.** The falsifier
    /// suite found the two conflated: with cycle detection removed, a two-hop
    /// cycle simply ran into the step limit, produced the same variant, and the
    /// cycle test went green for the wrong reason. A cycle and a malformed
    /// table are different defects with different fixes — re-point an alias
    /// versus repair the table — and a test that cannot tell them apart cannot
    /// certify either.
    #[error("project alias chain too long ({0} hops): the table is malformed")]
    AliasChainTooLong(usize),
}

/// One entry of the storage-level project alias table.
///
/// ADR-0152. `from_id` is **what the code derives today**; `to_id` is the
/// canonical identity, the one that holds the history.
///
/// This type **does not mint identities and does not rewrite any of them.**
/// A `project_id` is baked into the content hash of an append-only, hash-chained
/// event log (`EventEnvelopeV1::compute_content_hash` zeroes only
/// `content_hash`, `sequence` and `recorded_at`), so once a project has one
/// event its identity is immutable. That is why this exists: the only thing
/// left to do is change *what resolution points at*, never what was recorded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectAlias {
    /// The id the current derivation produces.
    pub from_id: ProjectId,
    /// The canonical id, which holds the history.
    pub to_id: ProjectId,
    /// Why the alias exists. Required by the CLI, not optional.
    pub reason: String,
    /// RFC3339 timestamp of declaration.
    pub created_at: String,
}

impl ProjectAlias {
    /// Declares an alias, rejecting the two shapes that cannot be right.
    ///
    /// - An **empty reason** is refused. An alias with no stated cause is
    ///   indistinguishable, later, from a wrong one.
    /// - A **destination that does not yet exist** cannot be checked here
    ///   (this type is pure and has no storage), so the caller verifies it.
    ///   That check is the CLI's, and it is a hard one: an alias to a
    ///   not-yet-existing project is how two real projects would merge.
    pub fn new(
        from_id: ProjectId,
        to_id: ProjectId,
        reason: impl Into<String>,
        created_at: impl Into<String>,
    ) -> Result<Self, IdentityError> {
        let reason = reason.into();
        if reason.trim().is_empty() {
            return Err(IdentityError::InvalidProjectId(
                "an alias requires a reason".into(),
            ));
        }
        Ok(Self {
            from_id,
            to_id,
            reason,
            created_at: created_at.into(),
        })
    }
}

/// The outcome of resolving a derived id through the alias table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AliasResolution {
    /// The id resolution ended at. Equal to the input when no alias applied.
    pub project_id: ProjectId,
    /// Every hop taken, in order. Empty when no alias applied.
    ///
    /// Carried so the CLI can **declare** that it resolved through an alias
    /// (ADR-0152 rule 4). A resolution that silently lands somewhere else is
    /// the false green INC-DEBT-049 was about.
    pub hops: Vec<ProjectId>,
}

impl AliasResolution {
    /// True when at least one alias was followed.
    pub fn redirected(&self) -> bool {
        !self.hops.is_empty()
    }
}

/// In-memory alias table: pure, no filesystem, no storage.
///
/// Append-only by construction: there is no `remove`. Retiring an alias means
/// re-pointing the other way, and the record of why it existed outlives the
/// project.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AliasTable {
    aliases: Vec<ProjectAlias>,
}

/// Upper bound on chain hops. A chain longer than this is a defect in the
/// table, not a legitimate configuration, and the error says so.
const MAX_ALIAS_HOPS: usize = 16;

impl AliasTable {
    /// Builds a table from its entries.
    pub fn new(aliases: Vec<ProjectAlias>) -> Self {
        Self { aliases }
    }

    /// The table's entries, in declaration order.
    pub fn entries(&self) -> &[ProjectAlias] {
        &self.aliases
    }

    fn target_of(&self, id: &ProjectId) -> Option<&ProjectAlias> {
        self.aliases.iter().find(|a| &a.from_id == id)
    }

    /// Resolves `id` through the alias chain.
    ///
    /// Rules, all fail-closed (ADR-0152):
    ///
    /// 1. Chains resolve transitively: `A -> B -> C` yields `C`.
    /// 2. A **cycle is a hard error** naming the chain, not a truncated answer.
    /// 3. A **self-alias** (`A -> A`) resolves to `A` and terminates. It is
    ///    redundant, not broken, and treating it as an error would make a
    ///    harmless table unusable.
    /// 4. No alias means the input is returned unchanged with no hops, so the
    ///    common case cannot regress.
    pub fn resolve(&self, id: ProjectId) -> Result<AliasResolution, IdentityError> {
        let mut hops: Vec<ProjectId> = Vec::new();
        let mut seen: Vec<ProjectId> = vec![id.clone()];
        let mut current = id;

        loop {
            let Some(alias) = self.target_of(&current) else {
                return Ok(AliasResolution {
                    project_id: current,
                    hops,
                });
            };
            let next = alias.to_id.clone();
            if next == current {
                // Rule 3: self-alias terminates without a hop.
                return Ok(AliasResolution {
                    project_id: current,
                    hops,
                });
            }
            if let Some(at) = seen.iter().position(|s| s == &next) {
                let mut chain: Vec<String> = seen[at..].iter().map(|s| s.to_string()).collect();
                chain.push(next.to_string());
                return Err(IdentityError::AliasCycle(chain.join(" -> ")));
            }
            if hops.len() >= MAX_ALIAS_HOPS {
                return Err(IdentityError::AliasChainTooLong(MAX_ALIAS_HOPS));
            }
            seen.push(next.clone());
            hops.push(current.clone());
            current = next;
        }
    }
}

/// A globally unique project identifier.
///
/// Derived from normalized remote URL + scope, providing stable identity
/// regardless of local checkout location or remote URL variations.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectId(String);

impl ProjectId {
    /// Creates a new ProjectId after validating the format.
    pub fn new(id: impl Into<String>) -> Result<Self, IdentityError> {
        let id = id.into();
        if id.is_empty() {
            return Err(IdentityError::InvalidProjectId("cannot be empty".into()));
        }
        if !Regex::new(r"^[a-zA-Z][a-zA-Z0-9_-]*$")
            .unwrap()
            .is_match(&id)
        {
            return Err(IdentityError::InvalidProjectId(id));
        }
        Ok(Self(id))
    }

    /// Creates a ProjectId from a hash that may start with a digit.
    /// Use this only for computed stable IDs, not for user-provided IDs.
    pub fn from_hash_prefix(id: impl Into<String>) -> Result<Self, IdentityError> {
        Self::new(id)
    }

    /// Returns the underlying string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Source material used to derive a logical project identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentitySource {
    /// The identity was derived from a canonical Git remote and scope.
    Remote,
    /// The identity was derived from a caller-supplied stable UUID and scope.
    Fallback,
    /// The identity was pinned in the checkout (`.sddk/project-pin.json`);
    /// remote/seed derivation is bypassed (W2c).
    Pinned,
}

/// Fully resolved deterministic project identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ResolvedProjectIdentity {
    /// Stable logical project identifier.
    pub project_id: ProjectId,
    /// Canonical transport-neutral remote representation, when available.
    pub remote_url: Option<String>,
    /// Canonical monorepo scope.
    pub scope: String,
    /// Material selected for identity derivation.
    pub identity_source: IdentitySource,
    /// Canonical UUID used by fallback identity, when applicable.
    pub fallback_seed: Option<String>,
    /// Chain of identity redirects followed to reach `project_id` (ADR-0152).
    ///
    /// Empty when no alias applied. **Not** an `IdentitySource` variant on
    /// purpose: the source says how the id was *derived* (remote, seed, pin),
    /// and an alias is a redirection *after* derivation. Folding them together
    /// would make "pinned, then redirected" and "derived from remote, then
    /// redirected" indistinguishable, and would push an alias through all
    /// seventeen `match` sites on `IdentitySource` instead of four struct
    /// literals.
    ///
    /// Carried so the CLI can **declare** the redirect (ADR-0152 rule 4). A
    /// resolution that silently lands on a different project is the false
    /// green INC-DEBT-049 was about, one level up.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alias_hops: Vec<ProjectId>,
}

impl ResolvedProjectIdentity {
    /// True when this identity reached `project_id` through at least one alias.
    pub fn redirected(&self) -> bool {
        !self.alias_hops.is_empty()
    }

    /// The id this resolution started from, when it was redirected.
    ///
    /// `None` when no alias applied, so a caller cannot mistake "resolved
    /// here" for "came from here".
    pub fn alias_origin(&self) -> Option<&ProjectId> {
        self.alias_hops.first()
    }
}

/// Knowledge profile persisted at adoption time.
///
/// This is the single source of truth for the canonical knowledge vault path.
/// The vault path is selected at adoption time and stored here so it remains
/// stable even if the checkout is renamed or moved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct KnowledgeProfile {
    /// Stable project identifier derived from remote URL or fallback seed.
    pub project_id: ProjectId,
    /// Human-readable project name (basename of the adopted checkout root).
    pub project_name: String,
    /// Canonical knowledge vault path under `$HOME/.sddk-knowledge/`.
    pub vault_path: PathBuf,
    /// Whether optional Engram memory integration is enabled.
    pub engram_enabled: bool,
}

impl fmt::Display for ProjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<ProjectId> for String {
    fn from(id: ProjectId) -> Self {
        id.0
    }
}

/// A workspace-specific identifier.
///
/// Uniquely identifies a checkout or worktree within a project.
/// Changes if the project is checked out to a different path.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspaceId(String);

impl WorkspaceId {
    /// Creates a new WorkspaceId after validating the format.
    pub fn new(id: impl Into<String>) -> Result<Self, IdentityError> {
        let id = id.into();
        if id.is_empty()
            || !Regex::new(r"^[a-zA-Z][a-zA-Z0-9_-]*$")
                .unwrap()
                .is_match(&id)
        {
            return Err(IdentityError::InvalidWorkspaceId("cannot be empty".into()));
        }
        Ok(Self(id))
    }

    /// Returns the underlying string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WorkspaceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<WorkspaceId> for String {
    fn from(id: WorkspaceId) -> Self {
        id.0
    }
}

/// A cycle identifier within a project.
///
/// Format: {project_id}/{cycle_name}
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CycleId(String);

impl CycleId {
    /// Creates a new CycleId after validating the format.
    pub fn new(id: impl Into<String>) -> Result<Self, IdentityError> {
        let id = id.into();
        if id.is_empty() {
            return Err(IdentityError::InvalidCycleId("cannot be empty".into()));
        }
        if !Regex::new(r"^[a-zA-Z][a-zA-Z0-9_-]+/[a-z][a-z0-9_-]*$")
            .unwrap()
            .is_match(&id)
        {
            return Err(IdentityError::InvalidCycleId(id));
        }
        Ok(Self(id))
    }

    /// Creates a CycleId from project and cycle name.
    pub fn from_parts(project: &ProjectId, cycle_name: &str) -> Result<Self, IdentityError> {
        if cycle_name.is_empty() {
            return Err(IdentityError::InvalidCycleId(
                "cycle name cannot be empty".into(),
            ));
        }
        Self::new(format!("{}/{}", project, cycle_name))
    }

    /// Returns the underlying string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the project portion of the cycle ID.
    pub fn project(&self) -> &str {
        self.0.split('/').next().unwrap_or(&self.0)
    }

    /// Returns the cycle name portion (after the slash).
    pub fn cycle_name(&self) -> &str {
        self.0.split('/').nth(1).unwrap_or(&self.0)
    }
}

impl fmt::Display for CycleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<CycleId> for String {
    fn from(id: CycleId) -> Self {
        id.0
    }
}

/// Normalizes a remote URL to a canonical form.
///
/// Strips transport credentials, query/fragment suffixes, trailing `.git`, and
/// normalizes HTTPS, `ssh://`, and SCP-style remotes to one HTTPS-shaped form.
pub fn normalize_remote_url(url: &str) -> Result<String, IdentityError> {
    let url = url.trim();
    if url.is_empty() || url.chars().any(char::is_whitespace) {
        return Err(IdentityError::InvalidRemoteUrl);
    }

    let without_suffix = url
        .split(['?', '#'])
        .next()
        .ok_or(IdentityError::InvalidRemoteUrl)?
        .trim_end_matches('/');
    let (scheme, authority, path) = if let Some((scheme, rest)) = without_suffix.split_once("://") {
        if !scheme.eq_ignore_ascii_case("https") && !scheme.eq_ignore_ascii_case("ssh") {
            return Err(IdentityError::InvalidRemoteUrl);
        }
        let (authority, path) = rest
            .split_once('/')
            .ok_or(IdentityError::InvalidRemoteUrl)?;
        (scheme, authority, path)
    } else {
        let (authority, path) = without_suffix
            .split_once(':')
            .ok_or(IdentityError::InvalidRemoteUrl)?;
        if authority.contains('/') || authority.is_empty() {
            return Err(IdentityError::InvalidRemoteUrl);
        }
        ("scp", authority, path)
    };

    let authority = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    let authority = normalize_authority(authority, scheme)?;
    let path = normalize_remote_path(path)?;
    Ok(format!("https://{authority}/{path}"))
}

/// Compara dos remotos opcionales COMO IDENTIDAD. Esta es la UNICA regla del
/// sistema para esa pregunta, y vive aqui a proposito.
///
/// Session-76 (OBSERVADO en este repo, no supuesto). La pregunta "estas dos
/// filas son el mismo remoto" tenia TRES respuestas distintas, y las tres se
/// ejecutaban en el MISMO comando:
///
///   (a) `sddk-engine` `same_identity`, sobre el recibo: case-insensitive,
///       corregido en session-65i cuando `adopt status` reportaba `conflict`
///       por un recibo acuñado con `Rubentxu` antes de que existiera la
///       normalizacion.
///   (b) `sddk-engine` `inspect_ledger`, sobre la fila: case-insensitive, la
///       misma funcion que (a).
///   (c) `sddk-storage` `register_project_workspace`: byte a byte. Esa tercera
///       nunca recibio el arreglo.
///
/// Consecuencia medida: `adopt status` lee con (a) y (b) y declara `complete`;
/// `adopt apply`, `adopt repair` y `sddk context bootstrap` escriben con (c) y
/// responden `RegistrationConflict` sobre el MISMO estado. Es decir: **el
/// comando de estado no puede detectar la condicion que el de aplicar va a
/// rechazar** — un fallo abierto en la lectura contra uno cerrado en la
/// escritura, con dos veredictos incompatibles para el mismo hecho. Quien lee
/// `status: complete` se lleva la conclusion contraria de la que la
/// herramienta va a sostener un segundo despues.
///
/// Por eso la comparacion no vive en la capa que la necesita sino en el
/// dominio: mientras cada crate tenga su copia, las dos vuelven a divergir en
/// cuanto una se toque y la otra no. Y el por que de que el case no sea parte
/// de la identidad esta en `normalize_remote_path`: el `project_id` lo acuña ya
/// normalizado, con cada segmento en minusculas (test golden
/// `case_change_in_owner_or_repo_resolves_to_same_project_id`).
///
/// La caida a igualdad cruda es deliberada: si alguna de las dos no normaliza,
/// no se declara coincidencia **solo porque la otra si normalizo**. Una URL
/// invalida no puede ganar por el case de su vecina.
pub fn remote_urls_equivalent(left: Option<&str>, right: Option<&str>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => {
            match (normalize_remote_url(left), normalize_remote_url(right)) {
                (Ok(left), Ok(right)) => left == right,
                _ => left == right,
            }
        }
        _ => false,
    }
}

fn normalize_authority(authority: &str, scheme: &str) -> Result<String, IdentityError> {
    if authority.is_empty() {
        return Err(IdentityError::InvalidRemoteUrl);
    }
    let (host, port) = if authority.starts_with('[') {
        let closing = authority.find(']').ok_or(IdentityError::InvalidRemoteUrl)?;
        let host = &authority[..=closing];
        let remainder = &authority[closing + 1..];
        let port = if remainder.is_empty() {
            None
        } else {
            Some(
                remainder
                    .strip_prefix(':')
                    .ok_or(IdentityError::InvalidRemoteUrl)?,
            )
        };
        (host, port)
    } else if let Some((host, port)) = authority.rsplit_once(':') {
        if port.chars().all(|character| character.is_ascii_digit()) {
            (host, Some(port))
        } else {
            (authority, None)
        }
    } else {
        (authority, None)
    };
    if host.is_empty() || port.is_some_and(|port| port.is_empty()) {
        return Err(IdentityError::InvalidRemoteUrl);
    }
    let default_port = match scheme.to_ascii_lowercase().as_str() {
        "https" => Some("443"),
        "ssh" => Some("22"),
        _ => None,
    };
    let host = host.to_ascii_lowercase();
    match port.filter(|port| Some(*port) != default_port) {
        Some(port) => Ok(format!("{host}:{port}")),
        None => Ok(host),
    }
}

fn normalize_remote_path(path: &str) -> Result<String, IdentityError> {
    let path = path.trim_matches('/');
    let path = path
        .strip_suffix(".git")
        .unwrap_or(path)
        .trim_end_matches('/');
    if path.is_empty()
        || path
            .split('/')
            .any(|segment| segment.is_empty() || matches!(segment, "." | ".."))
    {
        return Err(IdentityError::InvalidRemoteUrl);
    }
    // GitHub treats owner/repo case-insensitively (the same repo is served
    // under any case), but a case change in the remote URL used to mint a
    // DIFFERENT project_id and silently fork the ledger (agent-secretless
    // report D2). Normalize the case away before hashing. Segments are
    // lowercased individually so the structure stays inspectable.
    let lowered: Vec<String> = path.split('/').map(|s| s.to_lowercase()).collect();
    Ok(lowered.join("/"))
}

/// Normalizes and validates a required monorepo scope.
pub fn normalize_scope(scope: &str) -> Result<String, IdentityError> {
    let scope = scope.trim().replace('\\', "/");
    if scope.is_empty() {
        return Err(IdentityError::MissingScope);
    }
    if scope == "." {
        return Ok(scope);
    }
    if scope.starts_with('/') {
        return Err(IdentityError::InvalidScope(scope));
    }
    let segments = scope
        .trim_matches('/')
        .split('/')
        .filter(|segment| *segment != ".")
        .collect::<Vec<_>>();
    if segments.is_empty()
        || segments
            .iter()
            .any(|segment| segment.is_empty() || *segment == "..")
    {
        return Err(IdentityError::InvalidScope(scope));
    }
    Ok(segments.join("/"))
}

/// Resolves project identity from either a remote or a stable fallback UUID.
pub fn resolve_project_identity(
    remote_url: Option<&str>,
    scope: &str,
    fallback_seed: Option<&str>,
) -> Result<ResolvedProjectIdentity, IdentityError> {
    let scope = normalize_scope(scope)?;
    match (remote_url, fallback_seed) {
        (Some(remote), None) => {
            let remote_url = normalize_remote_url(remote)?;
            let project_id = ProjectId::new(stable_project_id(&remote_url, &scope))?;
            Ok(ResolvedProjectIdentity {
                project_id,
                remote_url: Some(remote_url),
                scope,
                identity_source: IdentitySource::Remote,
                fallback_seed: None,
                // Derivation is alias-blind on purpose: this function answers
                // "what would the remote derive?", and the alias answers "which
                // project is that, really?". Folding them here would make the
                // pure function depend on state it cannot see.
                alias_hops: Vec::new(),
            })
        }
        (None, Some(seed)) => {
            let seed = Uuid::parse_str(seed).map_err(|_| IdentityError::InvalidFallbackSeed)?;
            let fallback_seed = seed.hyphenated().to_string();
            let project_id = ProjectId::new(stable_fallback_project_id(&fallback_seed, &scope))?;
            Ok(ResolvedProjectIdentity {
                project_id,
                remote_url: None,
                scope,
                identity_source: IdentitySource::Fallback,
                fallback_seed: Some(fallback_seed),
                alias_hops: Vec::new(),
            })
        }
        _ => Err(IdentityError::InvalidFallbackSeed),
    }
}

/// Computes a stable project identifier from a normalized remote URL and scope.
///
/// The scope is typically the owner/organization or a unique context identifier.
/// This ensures that forks or multiple remotes don't collide.
/// Returns a ProjectId-compatible string prefixed with "p-" so it always starts
/// with a letter and is valid for use as a ProjectId.
pub fn stable_project_id(normalized_remote: &str, scope: &str) -> String {
    let hex = framed_hash("sddk.project.remote.v1", &[normalized_remote, scope]);
    format!("p-{}", &hex[..16])
}

/// Computes a stable project identifier from a fallback UUID and scope.
pub fn stable_fallback_project_id(fallback_seed: &str, scope: &str) -> String {
    let hex = framed_hash("sddk.project.fallback.v1", &[fallback_seed, scope]);
    format!("p-{}", &hex[..16])
}

/// Computes a stable workspace identifier from project ID and canonical filesystem path.
pub fn stable_workspace_id(project: &ProjectId, canonical_path: &str) -> String {
    let hex = framed_hash("sddk.workspace.v1", &[project.as_str(), canonical_path]);
    format!("w-{}", &hex[..24])
}

/// Derives a **deterministic** fallback seed from a canonical workspace path.
///
/// A project with no git remote still needs a stable identity, otherwise
/// every invocation mints a fresh `project_id` and the ledger, receipts and
/// `project resolve` output all drift. Deriving the seed from the canonical
/// path makes the identity reproducible from the filesystem alone, without
/// requiring a persisted adoption receipt to exist first.
///
/// The returned string is a well-formed hyphenated UUID (the shape
/// `resolve_project_identity` requires), formed by stamping a constant
/// version nibble onto the path hash. Only the *grammar* is RFC 4122: the
/// variant bits are whatever the hash produced, so this is **not** an RFC
/// 4122 namespace UUID and must not be fed to `Uuid::new_v5`. It exists only
/// to satisfy the seed grammar at the domain boundary.
///
/// Domain string `sddk.project.fallback.seed.v1` is separate from
/// `sddk.project.fallback.v1` (which hashes the seed) so that the two
/// derivations can never collide.
pub fn stable_fallback_seed(canonical_workspace_path: &str) -> String {
    // framed_hash is SHA-256, so it yields 64 hex chars. A UUID needs 32
    // (16 bytes), so take the first 32 and shape them 8-4-4-4-12.
    let hex = framed_hash("sddk.project.fallback.seed.v1", &[canonical_workspace_path]);
    let hex = &hex[..32];
    // The constant version nibble REPLACES hex[16] (the first char of the
    // 4th group), so the groups stay 8-4-4-4-12 and the tail is hex[20..32].
    // Appending instead of replacing yields 37 chars and an unparseable UUID.
    format!(
        "{}-{}-{}-5{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[17..20],
        &hex[20..32]
    )
}

/// Length-prefix framed hash. Same algorithm as `stable_fallback_project_id`
/// / `stable_workspace_id`; exposed at crate visibility so test modules
/// can use it instead of duplicating the body.
///
/// Algorithm: `SHA256(domain_len || domain || part_0_len || part_0 ||
/// ... || part_n_len || part_n)`, where every length is encoded as a
/// fixed-width u64 (platform-independent — see addendum 16 of
/// HANDOFF-2026-09-21-session-10.md for the drift-history rationale).
pub fn framed_hash(domain: &str, parts: &[&str]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    // Use u64 (not usize) for the length-prefix encoding so that the
    // hash output is platform-independent. On 64-bit (the project's
    // deployment matrix: x86_64/aarch64-linux-musl, x86_64/aarch64-darwin
    // per .github/workflows/release.yml), usize==u64 and both forms
    // produce the same bytes. On a hypothetical 32-bit target, usize
    // would be 4 bytes but u64 is always 8 — agreeing with the inline
    // test copies in cli_approval_loop_e2e.rs / cli_pack_e2e.rs /
    // cli_approval_e2e.rs / ledger_watch.rs. See addendum 16 of
    // HANDOFF-2026-09-21-session-10.md for full rationale.
    hasher.update((domain.len() as u64).to_be_bytes());
    hasher.update(domain.as_bytes());
    for part in parts {
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part.as_bytes());
    }
    let hash = hasher.finalize();
    format!("{hash:x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_id_valid() {
        let id = ProjectId::new("my-project").unwrap();
        assert_eq!(id.as_str(), "my-project");
        assert_eq!(id.to_string(), "my-project");
    }

    #[test]
    fn test_project_id_invalid_starts_with_number() {
        let result = ProjectId::new("123-project");
        assert!(result.is_err());
    }

    #[test]
    fn test_project_id_from_hash() {
        // stable_project_id returns p-* prefix so it always starts with letter
        let stable = stable_project_id("https://github.com/owner/repo", "owner");
        assert!(stable.starts_with("p-"));
        // Should be usable as ProjectId
        let id = ProjectId::from_hash_prefix(&stable).unwrap();
        assert!(id.as_str().starts_with("p-"));
    }

    #[test]
    fn test_cycle_id_parts() {
        let id = CycleId::new("my-project/add-oauth").unwrap();
        assert_eq!(id.project(), "my-project");
        assert_eq!(id.cycle_name(), "add-oauth");
    }

    #[test]
    fn test_cycle_id_from_parts() {
        let project = ProjectId::new("my-project").unwrap();
        let id = CycleId::from_parts(&project, "add-oauth").unwrap();
        assert_eq!(id.as_str(), "my-project/add-oauth");
    }

    #[test]
    fn test_normalize_remote_url_https() {
        let url = "https://github.com/owner/repo.git";
        let normalized = normalize_remote_url(url).unwrap();
        assert_eq!(normalized, "https://github.com/owner/repo");
    }

    #[test]
    fn test_normalize_remote_url_ssh() {
        let url = "git@github.com:owner/repo.git";
        let normalized = normalize_remote_url(url).unwrap();
        assert_eq!(normalized, "https://github.com/owner/repo");
    }

    #[test]
    fn common_remote_forms_are_equivalent() {
        let forms = [
            "https://GitHub.COM/owner/repo.git/",
            "https://github.com:443/owner/repo",
            "ssh://git@github.com/owner/repo.git",
            "ssh://git@github.com:22/owner/repo",
            "git@github.com:owner/repo.git",
        ];
        let normalized = forms.map(normalize_remote_url).map(Result::unwrap);
        assert!(normalized.iter().all(|remote| remote == &normalized[0]));
        assert_eq!(normalized[0], "https://github.com/owner/repo");
    }

    /// D2 fix (agent-secretless): un cambio de case en owner/repo del remote
    /// minteaba OTRO project_id y materializaba un ledger nuevo en silencio.
    /// El host ya se normalizaba; el path del repo no.
    #[test]
    fn case_change_in_owner_or_repo_resolves_to_same_project_id() {
        let lower = "https://github.com/rubentxu/agent-secretless.git";
        let upper = "https://github.com/Rubentxu/agent-secretless.git";
        let mixed = "git@github.com:RubentXu/Agent-Secretless.git";
        let a = stable_project_id(&normalize_remote_url(lower).unwrap(), ".");
        let b = stable_project_id(&normalize_remote_url(upper).unwrap(), ".");
        let c = stable_project_id(&normalize_remote_url(mixed).unwrap(), ".");
        assert_eq!(a, b, "owner case change must not fork the project id");
        assert_eq!(a, c, "mixed case via scp form must match");
    }

    #[test]
    fn rejects_unsupported_or_unsafe_remote_forms() {
        for remote in [
            "http://github.com/owner/repo",
            "file:///tmp/repo",
            "git@github.com:owner/../repo",
            "https://github.com/owner repo",
        ] {
            assert!(normalize_remote_url(remote).is_err(), "accepted {remote}");
        }
    }

    #[test]
    fn test_normalize_remote_url_with_fragment() {
        let url = "https://github.com/owner/repo#main";
        let normalized = normalize_remote_url(url).unwrap();
        assert_eq!(normalized, "https://github.com/owner/repo");
    }

    #[test]
    fn test_normalize_remote_url_no_git_suffix() {
        let url = "https://github.com/owner/repo";
        let normalized = normalize_remote_url(url).unwrap();
        assert_eq!(normalized, "https://github.com/owner/repo");
    }

    #[test]
    fn test_stable_project_id_https_vs_ssh_equivalent() {
        // HTTPS and SSH forms of the same repo should produce the same stable ID
        let https = normalize_remote_url("https://github.com/owner/repo.git").unwrap();
        let ssh = normalize_remote_url("git@github.com:owner/repo.git").unwrap();
        assert_eq!(https, ssh);

        let id_https = stable_project_id(&https, "owner");
        let id_ssh = stable_project_id(&ssh, "owner");
        assert_eq!(id_https, id_ssh);
    }

    #[test]
    fn test_stable_project_id_different_scopes() {
        let remote = "https://github.com/owner/repo";
        let id_owner = stable_project_id(remote, "owner");
        let id_other = stable_project_id(remote, "other");
        assert_ne!(id_owner, id_other);
    }

    // --- INC-DEBT-028: deterministic fallback seed ---------------------
    //
    // The regression these pin: the fallback seed used to be a fresh
    // `Uuid::new_v4()` per invocation, so a remote-less workspace got a
    // different `project_id` on every command and `adopt status` reported
    // "not adopted" immediately after `adopt apply`.

    #[test]
    fn fallback_seed_is_deterministic_for_the_same_path() {
        let path = "/home/dev/projects/my-app";
        let a = stable_fallback_seed(path);
        let b = stable_fallback_seed(path);
        assert_eq!(a, b, "same canonical path must yield the same seed");
    }

    #[test]
    fn fallback_seed_is_a_parseable_hyphenated_uuid() {
        // resolve_project_identity rejects anything Uuid::parse_str cannot
        // read, so the derived seed must satisfy that grammar.
        let seed = stable_fallback_seed("/home/dev/projects/my-app");
        assert!(
            Uuid::parse_str(&seed).is_ok(),
            "derived seed must be a valid UUID: {seed}"
        );
        assert_eq!(seed.len(), 36, "expected hyphenated form: {seed}");
    }

    #[test]
    fn fallback_seed_round_trips_through_resolve_project_identity() {
        let path = "/home/dev/projects/my-app";
        let seed = stable_fallback_seed(path);
        let first = resolve_project_identity(None, ".", Some(&seed)).unwrap();
        let second = resolve_project_identity(None, ".", Some(&seed)).unwrap();
        assert_eq!(first.project_id, second.project_id);
        assert_eq!(first.identity_source, IdentitySource::Fallback);
    }

    #[test]
    fn fallback_seed_differs_per_path() {
        // Two different remote-less workspaces must not collide.
        let a = stable_fallback_seed("/home/dev/projects/app-one");
        let b = stable_fallback_seed("/home/dev/projects/app-two");
        assert_ne!(a, b);
        let id_a = resolve_project_identity(None, ".", Some(&a))
            .unwrap()
            .project_id;
        let id_b = resolve_project_identity(None, ".", Some(&b))
            .unwrap()
            .project_id;
        assert_ne!(id_a, id_b);
    }

    #[test]
    fn fallback_seed_distinguishes_paths_that_share_a_prefix() {
        // Framing matters: "/a/app" and "/a/app/x" must not hash alike.
        let a = stable_fallback_seed("/a/app");
        let b = stable_fallback_seed("/a/app/x");
        assert_ne!(a, b);
    }

    #[test]
    fn fallback_seed_is_pinned_to_known_value() {
        // Golden pin. The domain string is part of the identity contract:
        // changing it silently reassigns every remote-less project's
        // project_id, orphaning its ledger and receipts. A structural test
        // cannot catch that (nothing else in the derivation depends on the
        // domain string), so the expected value is pinned explicitly.
        //
        // Recompute deliberately: see
        // docs/debt/INC-DEBT-028-NONDETERMINISTIC-FALLBACK-IDENTITY.md.
        let seed = stable_fallback_seed("/home/dev/projects/my-app");
        assert_eq!(seed, "8ff7193a-954e-c498-5edd-2d9cb7416f7a");
    }

    #[test]
    fn test_stable_project_id_different_remotes_same_scope() {
        // Same scope but different repos should produce different IDs
        let id_repo1 = stable_project_id("https://github.com/owner/repo1", "owner");
        let id_repo2 = stable_project_id("https://github.com/owner/repo2", "owner");
        assert_ne!(id_repo1, id_repo2);
    }

    #[test]
    fn test_stable_project_id_deterministic() {
        let remote = "https://github.com/owner/repo";
        let id1 = stable_project_id(remote, "owner");
        let id2 = stable_project_id(remote, "owner");
        assert_eq!(id1, id2);
    }

    #[test]
    fn project_hash_frames_remote_and_scope() {
        assert_ne!(stable_project_id("ab", "c"), stable_project_id("a", "bc"));
    }

    // ── Golden pins of the REMOTE identity path (INC-DEBT-050) ──────────────
    //
    // `stable_project_id` had NO golden pin while `stable_fallback_seed` had
    // one since INC-DEBT-028. That asymmetry is the whole defect: the seed
    // path was protected, the remote path — the one every real project takes
    // — was not. When commit 52182522 lowercased `normalize_remote_path`, it
    // silently reassigned the `project_id` of every already-adopted project
    // with **no migration**: 25 of 104 adoption receipts on one machine were
    // orphaned (16 ids across 13 remotes), their ledgers intact but out of
    // the CLI's reach.
    //
    // The existing property test (`stable_project_id_is_deterministic`) cannot
    // catch that class of change: asserting `f(x) == f(x)` still passes when
    // `f` is replaced wholesale. Only pinning the *absolute* output makes a
    // normalisation change fail loudly, which is what a migration decision
    // needs to be forced rather than discovered later.

    #[test]
    fn project_id_is_pinned_to_known_values() {
        // Golden pins. The domain string `sddk.project.remote.v1` and the
        // framing of the derivation are part of the identity contract:
        // changing either reassigns the `project_id` of every project in the
        // world, orphaning its ledger and receipts. Nothing structural
        // depends on them, so the expected values are pinned explicitly.
        //
        // If one of these fails, DO NOT just copy the new value over. That
        // reassigns the identity of every project derived from the same
        // remote. Decide first whether a migration is warranted, and pin the
        // new value only as part of that migration.
        //
        // See docs/debt/INC-DEBT-050-REMOTE-CASE-NORMALIZATION-REASSIGNS-PROJECT-IDS-WITHOUT-MIGRATION.md
        // and docs/debt/INC-DEBT-028-NONDETERMINISTIC-FALLBACK-IDENTITY.md.
        assert_eq!(
            stable_project_id("https://github.com/rubentxu/example", "."),
            "p-fd57187922005b40"
        );
        assert_eq!(
            stable_project_id("https://github.com/acme/widgets", "acme"),
            "p-09add0c4901adb20"
        );
        assert_eq!(
            stable_project_id("https://gitlab.com/g/sub/p", "sub"),
            "p-8a1e919e1cbfb921"
        );
    }

    #[test]
    fn remote_normalization_is_pinned_to_known_values() {
        // The normaliser is the layer that actually caused INC-DEBT-050, so
        // it is pinned separately from the hash. A change here is the
        // dangerous one even when the hash stays untouched: it changes the
        // *input* to the derivation, which reassigns ids just as silently.
        //
        // Cases cover, in order: mixed-case host and path, `.git` suffix,
        // default ports that must be dropped, ssh/scp form, and transport
        // credentials that must be stripped.
        assert_eq!(
            normalize_remote_url("https://GitHub.com/Acme/Widgets.git").unwrap(),
            "https://github.com/acme/widgets"
        );
        assert_eq!(
            normalize_remote_url("https://github.com:443/acme/widgets").unwrap(),
            "https://github.com/acme/widgets"
        );
        assert_eq!(
            normalize_remote_url("https://github.com:8443/acme/widgets").unwrap(),
            "https://github.com:8443/acme/widgets"
        );
        assert_eq!(
            normalize_remote_url("git@github.com:Acme/Widgets.git").unwrap(),
            "https://github.com/acme/widgets"
        );
        assert_eq!(
            normalize_remote_url("https://user:tok@github.com/Acme/Widgets.git?x=1#frag").unwrap(),
            "https://github.com/acme/widgets"
        );
    }

    #[test]
    fn case_normalization_reassigned_real_project_ids_without_migration() {
        // Historical regression pin, using the two ids that genuinely coexisted
        // on one machine. This asserts the DAMAGE, not the intent: it is the
        // reason the two golden pins above exist.
        //
        // `Rubentxu` (pre-normalisation, minted 2026-09-30T07:47Z, carrying 65
        // cycles and a 3.9 MB ledger) vs `rubentxu` (post-normalisation, minted
        // 12 hours later the same day, empty). Same repository, same owner,
        // only the case differs.
        let repo = "software-development-decision-kernel";
        let pre = stable_project_id(&format!("https://github.com/Rubentxu/{repo}"), ".");
        let post = stable_project_id(&format!("https://github.com/rubentxu/{repo}"), ".");

        assert_ne!(
            pre, post,
            "changing the case of the owner changes the project_id — that IS the defect"
        );
        assert_eq!(
            pre, "p-63676b11dc0ef88f",
            "historical id, holds the real ledger"
        );
        assert_eq!(
            post, "p-995939af668a53d8",
            "id minted after the normalisation"
        );

        // The property D2 actually wanted, and which now holds: once
        // normalised, both spellings collapse onto the same project.
        let normalised = normalize_remote_url(&format!("https://github.com/Rubentxu/{repo}"))
            .unwrap()
            .to_lowercase();
        assert_eq!(normalised, format!("https://github.com/rubentxu/{repo}"));
        assert_eq!(stable_project_id(&normalised, "."), post);

        // The two properties together are why a migration was required and
        // why a golden pin is the only thing that would have announced it:
        // case-insensitive going forward, and silently reassigning everything
        // already minted.
    }

    #[test]
    fn fallback_identity_requires_and_canonicalizes_uuid_seed() {
        let identity = resolve_project_identity(
            None,
            "crates/./engine/",
            Some("A0B1C2D3-E4F5-4678-9ABC-DEF012345678"),
        )
        .unwrap();
        assert_eq!(identity.identity_source, IdentitySource::Fallback);
        assert_eq!(identity.scope, "crates/engine");
        assert_eq!(
            identity.fallback_seed.as_deref(),
            Some("a0b1c2d3-e4f5-4678-9abc-def012345678")
        );
        assert!(resolve_project_identity(None, ".", Some("not-a-uuid")).is_err());
    }

    #[test]
    fn test_stable_workspace_id() {
        let project = ProjectId::new("test-project").unwrap();
        let ws1 = stable_workspace_id(&project, "/home/user/project");
        let ws2 = stable_workspace_id(&project, "/home/user/project");
        assert_eq!(ws1, ws2);
    }

    #[test]
    fn test_stable_workspace_id_different_paths() {
        let project = ProjectId::new("test-project").unwrap();
        let ws1 = stable_workspace_id(&project, "/home/user/project");
        let ws2 = stable_workspace_id(&project, "/home/user/other-project");
        assert_ne!(ws1, ws2);
    }

    #[test]
    fn workspace_hash_frames_project_and_path() {
        let first = ProjectId::new("ab").unwrap();
        let second = ProjectId::new("a").unwrap();
        assert_ne!(
            stable_workspace_id(&first, "c"),
            stable_workspace_id(&second, "bc")
        );
    }

    // ── ADR-0152: resolución de identidad por alias ──────────────────────
    //
    // Cada criterio de ADR-0152 §Verification con su falsificador. Un criterio
    // sin falsificador que se pueda ejecutar no es un criterio, es una opinión.

    fn pid(s: &str) -> ProjectId {
        ProjectId::new(s).unwrap()
    }

    fn alias(from: &str, to: &str) -> ProjectAlias {
        ProjectAlias::new(pid(from), pid(to), "caso medido", "2026-10-02T00:00:00Z").unwrap()
    }

    #[test]
    fn no_alias_returns_the_input_untouched() {
        // Criterio 1, caso normal: el caso común no puede cambiar.
        let table = AliasTable::new(vec![]);
        let got = table.resolve(pid("p-aaa")).unwrap();
        assert_eq!(got.project_id, pid("p-aaa"));
        assert!(got.hops.is_empty());
        assert!(!got.redirected());
    }

    #[test]
    fn an_alias_not_applicable_leaves_resolution_unchanged() {
        // Falsificador del criterio 1: un alias que no aplica no puede mover nada.
        let table = AliasTable::new(vec![alias("p-other", "p-elsewhere")]);
        let got = table.resolve(pid("p-aaa")).unwrap();
        assert_eq!(got.project_id, pid("p-aaa"));
        assert!(!got.redirected());
    }

    #[test]
    fn an_applicable_alias_redirects_and_reports_the_hop() {
        // Criterio 1, caso con alias. Y criterio 3: el salto es visible.
        let table = AliasTable::new(vec![alias("p-derived", "p-canonical")]);
        let got = table.resolve(pid("p-derived")).unwrap();
        assert_eq!(got.project_id, pid("p-canonical"));
        assert_eq!(got.hops, vec![pid("p-derived")]);
        assert!(got.redirected());
    }

    #[test]
    fn chains_resolve_transitively() {
        // Regla 1: A -> B -> C yields C, con los dos saltos declarados.
        let table = AliasTable::new(vec![
            alias("p-a", "p-b"),
            alias("p-b", "p-c"),
            alias("p-c", "p-d"),
        ]);
        let got = table.resolve(pid("p-a")).unwrap();
        assert_eq!(got.project_id, pid("p-d"));
        assert_eq!(got.hops, vec![pid("p-a"), pid("p-b"), pid("p-c")]);
    }

    #[test]
    fn a_self_alias_terminates_instead_of_looping() {
        // Falsificador explícito del criterio 1. Un `A -> A` que se colgara
        // sería el modo de fallo más caro posible: no avisa.
        let table = AliasTable::new(vec![alias("p-a", "p-a")]);
        let got = table.resolve(pid("p-a")).unwrap();
        assert_eq!(got.project_id, pid("p-a"));
        assert!(got.hops.is_empty(), "un auto-alias no es un salto");
    }

    #[test]
    fn a_cycle_is_a_hard_error_that_names_the_chain() {
        // Criterio 2: `A -> B -> A` falla Y nombra el ciclo. Un guard que sólo
        // avisa es un FAIL, y un truncamiento silencioso también.
        //
        // Se afirma sobre la VARIANTE, no sobre el texto. Con la detección de
        // ciclo retirada, un ciclo de dos saltos corría hasta el tope de pasos
        // y devolvía el mismo tipo de error, y este test pasaba por el motivo
        // equivocado. Afirmar sobre el texto es no poder distinguir "hay un
        // ciclo" de "la tabla está mal", que son arreglos distintos.
        let table = AliasTable::new(vec![alias("p-a", "p-b"), alias("p-b", "p-a")]);
        let err = table.resolve(pid("p-a")).expect_err("un ciclo debe fallar");
        assert!(
            matches!(&err, IdentityError::AliasCycle(chain) if chain == "p-a -> p-b -> p-a"),
            "esperaba AliasCycle con la cadena nombrada, obtuve: {err:?}"
        );
    }

    #[test]
    fn a_three_hop_cycle_also_fails_as_a_cycle() {
        let table = AliasTable::new(vec![
            alias("p-a", "p-b"),
            alias("p-b", "p-c"),
            alias("p-c", "p-a"),
        ]);
        let err = table
            .resolve(pid("p-a"))
            .expect_err("un ciclo largo también falla");
        assert!(
            matches!(&err, IdentityError::AliasCycle(chain) if chain.contains("p-c -> p-a")),
            "esperaba AliasCycle, obtuve: {err:?}"
        );
    }

    #[test]
    fn a_chain_longer_than_the_limit_is_refused_not_truncated() {
        // Un tope de pasos que en silencio devuelve un id plausible sería peor
        // que no tener tope. Se falla, y con una variante DISTINTA del ciclo:
        // aquí el arreglo es reparar la tabla, no re-apuntar un alias.
        let mut entries = Vec::new();
        for i in 0..MAX_ALIAS_HOPS + 4 {
            entries.push(alias(
                &format!("p-step{i:02}"),
                &format!("p-step{:02}", i + 1),
            ));
        }
        let table = AliasTable::new(entries);
        let err = table
            .resolve(pid("p-step00"))
            .expect_err("una cadena absurda debe fallar");
        assert!(
            matches!(err, IdentityError::AliasChainTooLong(MAX_ALIAS_HOPS)),
            "esperaba AliasChainTooLong, obtuve: {err:?}"
        );
    }

    #[test]
    fn an_alias_without_a_reason_is_refused() {
        // Un alias sin causa declarada es indistinguible, más adelante, de uno
        // equivocado. Y uno equivocado manda una identidad entera a otro sitio.
        let err = ProjectAlias::new(pid("p-a"), pid("p-b"), "   ", "2026-10-02T00:00:00Z")
            .expect_err("un alias sin motivo no debe existir");
        assert!(err.to_string().contains("reason"), "{err}");
    }

    #[test]
    fn a_duplicate_from_id_is_visible_rather_than_merged() {
        // Criterio 4 en la parte que el tipo puede sostener: no hay `remove`,
        // así que la API no ofrece la operación. Lo que sí se comprueba es que
        // declarar dos veces el mismo `from_id` no se fusiona en silencio:
        // gana el primero y el segundo sigue siendo una entrada visible.
        let table = AliasTable::new(vec![alias("p-a", "p-first"), alias("p-a", "p-second")]);
        assert_eq!(table.entries().len(), 2);
        assert_eq!(
            table.resolve(pid("p-a")).unwrap().project_id,
            pid("p-first")
        );
    }

    #[test]
    fn the_real_case_of_this_machine_resolves_to_the_identity_with_history() {
        // El caso medido, no un ejemplo inventado: p-9959 deriva hoy y tiene 0
        // ciclos; p-6367 tiene 179. El alias es lo que evita que el CLI resuelva
        // al vacío sin avisar.
        let table = AliasTable::new(vec![alias("p-995939af668a53d8", "p-63676b11dc0ef88f")]);
        let got = table.resolve(pid("p-995939af668a53d8")).unwrap();
        assert_eq!(got.project_id, pid("p-63676b11dc0ef88f"));
        assert!(got.redirected());
    }
}
