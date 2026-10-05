//! Release plan and apply commands.

use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};
use sddk_domain::GateOutcomeStatus;
use sddk_domain::release_ref::{ReleaseRef, SourceRevision, VersionNaming, binds};
use sddk_domain::release_role::{CandidateHandoff, HandoffArtifact, ReleaseRole};
use sddk_domain::version_authority::{ReleaseTarget, VersionAuthority};
use sddk_domain::version_inspection::{AssuranceLevel, VersionInspection};
use sddk_engine::version::{
    VersionLockstepError, ensure_version_lockstep, ensure_version_lockstep_detailed,
};
use sddk_gateway::{
    CapabilityGateway, CapabilityPolicy, GitExecutor, GitHubForge, LocalReleaseInput,
    LocalReleaseOutcome, LocalReleasePreconditions, PermissionPolicy, ReleasePlanInput,
    apply_local_release, apply_release, plan_release,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use time::OffsetDateTime;

use crate::architecture_cmd::error_output;
use crate::dev::github_releases_ticket::{GithubReleasesTicketError, with_github_releases_ticket};
use crate::{
    CliEnvironment, CommandOutput, OutputFormat, RuntimeArgs, RuntimeContext, render_result,
    uat::ReleaseTypeArg,
};

// Version lockstep rule lives in `sddk_engine::version` per INC-024. The CLI
// layer imports the engine helper rather than re-implementing the Cargo.toml
// read + workspace/tag matching locally. This reduces `release_cmd.rs` LOC
// and centralizes the rule so future callers (daemon, CI gate) reuse it.

#[derive(Debug, Subcommand)]
pub(crate) enum ReleaseCommand {
    /// Show the selected release sequence.
    Plan(ReleaseArgs),
    /// Apply the selected release route.
    Apply(ReleaseArgs),
    /// Package the current binary with checksums, SBOM, and attestation.
    Dist(DistArgs),
    /// Verify a dist prefix against its checksums and attestation.
    Verify(DistArgs),
    /// Manage release channels and promotion.
    Channel(ChannelArgs),
    /// Explain how a target's version was resolved, or why it was not.
    Version {
        #[command(subcommand)]
        action: VersionAction,
    },
    /// Revalidate release candidate after a correction commit (scoped recovery).
    Revalidate(RevalidateArgs),
    /// Build the envelope a producer hands to whoever certifies and promotes it.
    Handoff(HandoffArgs),
    /// Emit vault-receipt.json for a managed-closure cycle (ADR-0075).
    Vault(VaultArgs),
}

/// Los argumentos de `release handoff`.
///
/// ## Por qué están en tres grupos y no en trece banderas sueltas
///
/// El sobre tiene tres dobladores, y cada uno viene de un sitio distinto:
///
/// - **la referencia** (`--tag`, `--naming`, `--sequence`, `--channel`) la
///   entiende SDDK, porque es el mismo `ReleaseRef` que usan `plan` y `apply`;
/// - **el material** (`--artifact`) lo trae el productor, y SDDK solo le calcula
///   el digest porque content-addressable es un hecho, no una opinión;
/// - **la identidad del productor** (`--external-type`, `--external-digest`) no la
///   sabe SDDK, y por eso son **obligatorias**: un sobre que se pudiera rellenar
///   entero sin hablar con nadie diría cosas que nadie dijo.
///
/// Un grupo por doblador, porque un argument parser que mezcla las tres cosas
/// es un argument parser cuyo error no dice qué de las tres falta.
#[derive(Debug, Clone, Args)]
pub(crate) struct HandoffArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Which release target, by its path relative to the repository root.
    #[arg(long)]
    pub(crate) target: Option<String>,
    /// The role this target declares. It bounds which channels it may hand off.
    #[arg(long, default_value = "candidate_producer")]
    pub(crate) role: String,
    #[command(flatten)]
    pub(crate) reference: HandoffReferenceArgs,
    #[command(flatten)]
    pub(crate) material: HandoffMaterialArgs,
    /// Where to write the envelope. Without it the envelope goes to stdout.
    #[arg(long)]
    pub(crate) out: Option<PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
    #[command(flatten)]
    pub(crate) ask: BuildAskArgs,
}

/// La referencia que el sobre lleva.
#[derive(Debug, Clone, Args)]
pub(crate) struct HandoffReferenceArgs {
    /// The release reference, written exactly as it will be published.
    #[arg(long)]
    pub(crate) tag: String,
    /// How a release reference names a product version. See `release --naming`.
    /// Ignored when `--sequence` is given: a candidate naming is declared by its
    /// three parts, not chosen from this list.
    #[arg(long, default_value = "v_prefixed")]
    pub(crate) naming: String,
    /// Which candidate of the version this is, when the naming declares one.
    #[arg(long)]
    pub(crate) sequence: Option<u32>,
    /// The channel this material is handed off on.
    #[arg(long, default_value = "candidate")]
    pub(crate) channel: String,
    /// The prefix before the version in a candidate name. Only with `--sequence`.
    #[arg(long, default_value = "v")]
    pub(crate) candidate_prefix: String,
    /// The separator between version and marker. Only with `--sequence`.
    #[arg(long, default_value = "-")]
    pub(crate) candidate_separator: String,
    /// The marker before the candidate number. Only with `--sequence`.
    #[arg(long, default_value = "rc")]
    pub(crate) candidate_marker: String,
}

/// El material y la identidad de quien lo produjo.
#[derive(Debug, Clone, Args)]
pub(crate) struct HandoffMaterialArgs {
    /// One artifact, as `kind=path`, repeatable. The digest is computed.
    #[arg(long = "artifact", value_name = "KIND=PATH")]
    pub(crate) artifacts: Vec<String>,
    /// A reference backing the producer's own gates, repeatable.
    #[arg(long = "evidence", value_name = "REF")]
    pub(crate) evidence_refs: Vec<String>,
    /// What kind of handoff this is, in the producer's own words.
    #[arg(long)]
    pub(crate) external_type: String,
    /// The producer's own digest over its serialisation. Not SDDK's.
    #[arg(long)]
    pub(crate) external_digest: String,
}

/// Qué se puede preguntar sobre la versión de un target.
///
/// Un enum y no una bandera porque **preguntar cómo se resolvió** y **preguntar
/// si la referencia nombra la versión** son dos preguntas con dos consumidores
/// distintos: la segunda es la regla del lockstep y tiene su propia puerta, con
/// su naming declarado y su mensaje de los dos lados.
#[derive(Debug, Clone, Subcommand)]
pub(crate) enum VersionAction {
    /// Why the version resolved, or why it did not — and what was not checked.
    Inspect(VersionInspectArgs),
    /// Whether the declared release reference names the product's version.
    Matches(VersionMatchesArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct VersionMatchesArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// The release reference to check, written exactly as it will be published.
    #[arg(long)]
    pub(crate) tag: String,
    /// Which release target, by its path relative to the repository root.
    #[arg(long)]
    pub(crate) target: Option<String>,
    /// How a release reference names a product version. See `release --naming`.
    #[arg(long, default_value = "v_prefixed")]
    pub(crate) naming: String,
    #[command(flatten)]
    pub(crate) ask: BuildAskArgs,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

/// El despachador de `release version`.
fn run_release_version(action: VersionAction) -> CommandOutput {
    match action {
        VersionAction::Inspect(args) => run_release_version_inspect(args),
        VersionAction::Matches(args) => run_release_version_matches(args),
    }
}

/// Las dos banderas que deciden si se pregunta al build tool.
///
/// ## Por qué un tipo y no dos campos por `Args`
///
/// MEDIDO: `release` y `release version inspect` las declaraban, y
/// `release version matches` y `release handoff` **no**. Los dos primeros
/// podían preguntar; los dos segundos contestaban sobre la version sin poder
/// hacerlo, y por eso su respuesta no dependia de lo preguntado —con un tag
/// correcto y con uno falso, `matches` devolvia byte a byte la misma salida—.
///
/// Copiar las banderas es copiar esa deriva. Un solo tipo que los cuatro
/// `Args` aplanan hace que **no se pueda** volver a tener un comando que
/// responda sin poder preguntar, que es exactamente el defecto.
///
/// Y el par viaja junto porque el segundo sin el primero es un provider que
/// lanza un proceso con el nombre equivocado: de ahi `BuildAsk`.
///
/// ## Una medicion que se lleva por delante, y por que
///
/// El doc de `ReleaseArgs` decia que sin `--evaluate-build`
/// `ensure_release_ref_lockstep` devolvia `Ok` con un tag que el proyecto
/// contradecia, porque sin version no hay lockstep que incumplir. **Es falso, y
/// es la premisa que la adenda correctiva de ADR-0164 ya retiro**: `refusal()`
/// corre en `version.rs:182`, antes del `let Some(...)` de `:190`, y devuelve
/// `Err` para `Unresolved`, `Ambiguous` e `Invalid`. El `return Ok` de `:190`
/// solo se alcanza con `ReleaseRefIsAuthority`, que exige declaracion
/// explicita.
///
/// No se reescribe aqui porque medir de nuevo sale de otro bloque, pero no se
/// arrastra tampoco: una medicion que el codigo contradice, escrita al lado de
/// la bandera que se va a reutilizar, volveria a tener autoridad por vecindad.
#[derive(Debug, Clone, Args)]
pub(crate) struct BuildAskArgs {
    /// Ask the build tool what its model says, instead of only reading files.
    ///
    /// Opt-in porque **paga**: MEDIDO, `gradle properties --offline` tarda 3 s
    /// y levanta una JVM, por target. Sin esta bandera el informe dice
    /// explicitamente que el modelo del build no se evaluo, porque no
    /// evaluarlo por defecto es una decision y callarla seria otra.
    ///
    /// Y el motivo por el que esta en `matches` y en `handoff`, que antes no la
    /// tenian: un comando que responde sobre la version y **no puede** preguntar
    /// no mide la version, contesta que no la ha mirado. MEDIDO sobre un build
    /// Gradle con `version = '1.2.3'`: `release version matches` decia «no hay
    /// nada contra que comparar» para `v1.2.3` y para `v9.9.9` por igual.
    #[arg(long)]
    pub(crate) evaluate_build: bool,
    /// The build tool to ask, when `--evaluate-build` is given.
    ///
    /// MEDIDO: no es un detalle. Un proyecto con wrapper usa `./gradlew` y uno
    /// sin wrapper usa el `gradle` del PATH, que ademas puede estar detras de un
    /// shim de asdf que **exige** `.tool-versions` —sin el responde `No version
    /// is set for command gradle` y sale 126—. Suponer cual de los dos es
    /// suponer, y es la misma palabra que usa el resto de este bloque para lo
    /// que se declara en vez de deducirse.
    ///
    /// Sin default. MEDIDO, con un ejecutable instrumentado: `--build-tool mvn`
    /// hacia que `mvn` corriera `properties --offline` —un goal que no existe en
    /// Maven— y el informe atribuyera la respuesta a `build.gradle`, un fichero
    /// que Maven nunca abrio. El default era la causa: sin nombre, la bandera
    /// aceptaba cualquier palabra y respondia siempre en Gradle.
    ///
    /// Un nombre que SDDK no sepa preguntar es un error de la linea de comandos,
    /// no una suposicion.
    #[arg(long)]
    pub(crate) build_tool: Option<String>,
}

/// Los argumentos de `release version inspect`.
///
/// Un subcomando y no una bandera: la pregunta que responde («por qué se
/// resolvió esto») no es una variación de `release plan`, es una pregunta
/// distinta, y por eso tiene su propia forma y su propio código de salida.
#[derive(Debug, Clone, Args)]
pub(crate) struct VersionInspectArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Which release target, by its path relative to the repository root.
    ///
    /// Required as soon as the repository holds more than one, exactly as in
    /// `release plan`: dos productos y nada que diga cuál es un rechazo.
    #[arg(long)]
    pub(crate) target: Option<String>,
    #[command(flatten)]
    pub(crate) ask: BuildAskArgs,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct ChannelArgs {
    /// Promotion source channel.
    #[arg(long)]
    pub(crate) from: String,
    /// Promotion target channel.
    #[arg(long)]
    pub(crate) to: String,
    /// Assume gates passed (required for edge→candidate and candidate→stable).
    #[arg(long)]
    pub(crate) gates_ok: bool,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

/// Release authority selected for one invocation.
#[derive(Debug, Clone, Copy, ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReleaseRoute {
    /// Push the trunk branch and annotated tag with local Git only.
    Local,
    /// Use the optional external forge integration.
    Forge,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct DistArgs {
    /// Distribution prefix directory.
    #[arg(long)]
    pub(crate) prefix: PathBuf,
    /// Release channel.
    #[arg(long, default_value = "release")]
    pub(crate) channel: String,
    /// Explicit RFC 3339 timestamp.
    #[arg(long)]
    pub(crate) timestamp: Option<String>,
    /// Explicit source commit.
    #[arg(long)]
    pub(crate) commit: Option<String>,
    /// Verify a signed gate receipt: `receipt_id|gate|transition|plan_hash|signature`.
    #[arg(long)]
    pub(crate) receipt: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
    /// XDG data dir for staging area (defaults to ~/.local/share).
    #[arg(long)]
    pub(crate) sddk_data_dir: Option<PathBuf>,
    /// Skip the MANIFEST exact-set verification (logged escape hatch for dirty dev workspaces).
    #[arg(long)]
    pub(crate) skip_manifest_preflight: bool,
    /// Source checkout or bundle root containing agents/skills/prompts/workflows/assets
    /// and MANIFEST.sha256. Defaults to current working directory.
    #[arg(long)]
    pub(crate) source: Option<PathBuf>,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct ReleaseArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Release authority. `local` never uses GitHub, CI/CD, or assets.
    #[arg(long, value_enum)]
    pub(crate) route: Option<ReleaseRoute>,
    /// GitHub repository as `owner/repo`, required only for `--route forge`.
    #[arg(long)]
    pub(crate) repo: Option<String>,
    /// Branch to release. Local releases must target the trunk branch.
    #[arg(long, default_value = "main")]
    pub(crate) branch: String,
    /// Target branch for the optional forge pull request.
    #[arg(long, default_value = "main")]
    pub(crate) base: String,
    /// Annotated tag message and optional forge release title.
    #[arg(long, default_value = "SDDK release")]
    pub(crate) title: String,
    /// Release tag.
    #[arg(long)]
    pub(crate) tag: String,
    /// Which release target, by its path relative to the repository root.
    ///
    /// Omit it when the repository holds exactly one target. Required as soon
    /// as it holds more than one: two products and no way to tell them apart is
    /// a refusal, and the refusal lists the paths so naming one is mechanical.
    #[arg(long)]
    pub(crate) target: Option<String>,
    /// How a release reference names a product version.
    ///
    /// `v_prefixed` (the default) means a release is tagged `v1.2.3` for
    /// version `1.2.3`; `exact` means the reference **is** the version, with no
    /// prefix. The prefix is not optional: a convention that may or may not be
    /// applied cannot reject anything, so it cannot authorise anything either.
    ///
    /// A project whose tags carry no `v` says so here rather than being
    /// refused with no way out. The declared value is printed in the plan, so a
    /// release records the convention it was authorised under.
    #[arg(long, default_value = "v_prefixed")]
    pub(crate) naming: String,
    #[command(flatten)]
    pub(crate) ask: BuildAskArgs,
    /// What this target is responsible for in the release.
    ///
    /// `full_publisher` (the default) publishes. `candidate_producer` stops at
    /// candidate material, `certifier` says whether a release may be promoted
    /// and never does it, and `promoter` moves a release along the channels
    /// without publishing it. Declaring the role is how a repository says
    /// *I am not the one that publishes*, and the release holds it to that.
    ///
    /// Like `--naming`, this is a declaration **per release** and not a setting
    /// stored in the repository, which is a cost and is declared as one.
    #[arg(long, default_value = "full_publisher")]
    pub(crate) role: String,
    /// Release cycle providing local verification and UAT evidence.
    #[arg(long)]
    pub(crate) cycle: Option<String>,
    /// Previous tag for semver diff (alternative to `--release-type`).
    /// When omitted the release type defaults to Major (fail-closed).
    #[arg(long)]
    pub(crate) previous_tag: Option<String>,
    /// Explicit release type (overrides `--previous-tag` auto-detection).
    #[arg(long, value_enum)]
    pub(crate) release_type: Option<ReleaseTypeArg>,
    /// Release notes.
    #[arg(long, default_value = "")]
    pub(crate) notes: String,
    /// Explicit approval for capability effects.
    #[arg(long)]
    pub(crate) approve: bool,
    /// Explicit RFC 3339 timestamp for deterministic execution.
    #[arg(long)]
    pub(crate) timestamp: Option<String>,
    /// Explicit actor for deterministic execution.
    #[arg(long)]
    pub(crate) actor: Option<String>,
    /// Distribution prefix for `release dist` attestation (same semantics
    /// as `DistArgs.prefix`).
    #[arg(long)]
    pub(crate) prefix: Option<PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

/// Arguments for the `release revalidate` command.
///
/// Produces an append-only `release-revalidation.json` artifact that binds
/// the candidate SHA (current HEAD) to fresh verify/debt evidence when a
/// correction commit has moved HEAD after the original verification.
#[derive(Debug, Clone, Args)]
pub(crate) struct RevalidateArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Cycle identifier for the RELEASE_PENDING cycle.
    #[arg(long)]
    pub(crate) cycle: String,
    /// Original SHA that was previously verified (before correction commit).
    #[arg(long)]
    pub(crate) original_sha: String,
    /// Explicit RFC 3339 timestamp for deterministic execution.
    #[arg(long)]
    pub(crate) timestamp: Option<String>,
    /// Explicit actor for deterministic execution.
    #[arg(long)]
    pub(crate) actor: Option<String>,
    /// Skip the verify check (use only when verify is not applicable).
    #[arg(long)]
    pub(crate) skip_verify: bool,
    /// Skip the debt-verify check (use only for paths without debt verification).
    #[arg(long)]
    pub(crate) skip_debt: bool,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

/// Arguments for the `release vault` command (REQ-DKA-002).
#[derive(Debug, Clone, Args)]
pub(crate) struct VaultArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Cycle identifier for the BLOCKED cycle to close via vault route.
    #[arg(long)]
    pub(crate) cycle: String,
    /// Explicit RFC 3339 timestamp for deterministic execution.
    #[arg(long)]
    pub(crate) timestamp: Option<String>,
    /// Explicit actor for deterministic execution.
    #[arg(long)]
    pub(crate) actor: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

pub(crate) fn run_release(command: ReleaseCommand, environment: &CliEnvironment) -> CommandOutput {
    match command {
        ReleaseCommand::Plan(args) => run_release_plan(args, environment),
        ReleaseCommand::Apply(args) => run_release_apply(args, environment),
        ReleaseCommand::Dist(args) => run_release_dist(args, environment),
        ReleaseCommand::Verify(args) => run_release_dist_verify(args, environment),
        ReleaseCommand::Channel(args) => run_release_channel(args),
        ReleaseCommand::Version { action } => run_release_version(action),
        ReleaseCommand::Revalidate(args) => run_release_revalidate(args, environment),
        ReleaseCommand::Handoff(args) => run_release_handoff(args),
        ReleaseCommand::Vault(args) => run_release_vault(args, environment),
    }
}

/// La raíz de la inspección, sin abrir un contexto de proyecto.
///
/// `--root` gana; si no, el directorio de trabajo. A propósito **no** infiere
/// hacia arriba buscando un marcador de proyecto: una inspección que camina
/// hacia arriba puede terminar describiendo el repositorio equivocado, y un
/// diagnóstico que describe el repositorio equivocado es peor que uno que no
/// arranca.
fn resolve_inspection_root(runtime: &RuntimeArgs) -> anyhow::Result<std::path::PathBuf> {
    match &runtime.root {
        Some(root) => Ok(root.clone()),
        None => std::env::current_dir()
            .map_err(|error| anyhow::anyhow!("cannot determine the current directory: {error}")),
    }
}

/// `release version inspect` — por qué se resolvió esto, y qué no se miró.
///
/// ## Por qué un comando y no una bandera de `release plan`
///
/// Porque no es una variación de la pregunta del plan. El plan pregunta «¿puedo
/// publicar esto?» y esta pregunta «¿por qué se resolvió lo que se resolvió?»,
/// y la segunda se hace **precisamente cuando la primera falla** —es el
/// diagnóstico del rechazo—, que es un momento en que un dry-run con sus
/// prerrequisitos de preflight no ayuda.
///
/// ## Qué NO hace, y está escrito porque es lo que se le pide a un diagnóstico
///
/// No recomienda. No dice «añade `version=` a X», no dice «renombra tu
/// fichero», no dice «tu manifiesto debería ser…». Describe lo que se observó
/// y lo que no, y decide el operador. Un diagnóstico que decide el modelo del
/// proyecto es un diagnóstico que ha adoptado la forma de un repo como si fuera
/// la forma de todos, que es exactamente lo que este bloque lleva siete bloques
/// desarmando.
///
/// ## El código de salida
///
/// Sale con **0 siempre que la inspección se hizo**, incluso cuando el veredicto
/// es `CONFLICT` o `INVALID`. Salir distinto sería decir «la inspección falló»,
/// que es un hecho distinto del que hay:: una inspección que dice
/// «hay dos declaraciones que discrepan» es una inspección que **funcionó**.
fn run_release_version_inspect(args: VersionInspectArgs) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<VersionInspection> {
        // Solo la raiz. MEDIDO: la primera version abria `RuntimeContext`, y eso
        // exige una identidad de proyecto valida — luego el comando **fallaba**
        // en un repositorio sin adoptar, que es exactamente el repositorio
        // cuyo release no resuelve. Un diagnostico que no puede diagnosticar el
        // caso que lo motivo es peor que no tenerlo, y el arreglo es no pedir lo
        // que no hace falta: la inspeccion es de sistema de ficheros.
        let root = resolve_inspection_root(&args.runtime)?;
        // La MISMA seleccion que usa `release plan`, porque un diagnostico que
        // mira un target distinto del que se publico no explica el rechazo que
        // se quiere explicar.
        let ask = BuildAsk::of_parts(&args.ask)?;
        let selected = resolve_release_target(&root, args.target.as_deref(), &ask)?;
        let mut inspection = ask.registry().resolve_inspecting(
            sddk_domain::version_authority::PRODUCT_VERSION_OBSERVATION,
            &selected.target,
        );
        // Lo que NO se evaluó, y que el informe tiene que decir aunque el
        // veredicto sea rojo: no preguntar al build tool es una decisión —la
        // paga quien la pide con `--evaluate-build`— y una decisión que el
        // informe no menciona se lee como una fuente mas que se consulto.
        //
        // El proveedor se DECLARA siempre en `providers_considered` aunque no se
        // haya pedido, asi que sin esta linea el informe seria internamente
        // contradictorio: diria «estos fueron los providers mirados» nombrando
        // uno que no se ejecuto.
        if !args.ask.evaluate_build {
            inspection
                .not_checked
                .push(sddk_domain::version_inspection::NotChecked {
                    key: "build_model_not_evaluated".to_owned(),
                    statement: "the build tool was NOT asked what its model says; that costs a \
                                process per target, so it happens only with `--evaluate-build`"
                        .to_owned(),
                });
        }
        Ok(inspection)
    })();
    render_result(result, format, version_inspection_text)
}

/// El informe, en texto.
///
/// El orden importa y no es estetico: **target, capability, providers, hallazgos,
/// veredicto, y lo que no se comprobó al final**. Lo no comprobado va el ultimo a
/// proposito —es lo que se lee despues de la respuesta, no antes— y lleva su
/// propia linea de separacion para que nadie pueda confundirlo con la
/// conclusion.
fn version_inspection_text(inspection: &VersionInspection) -> String {
    let mut text = String::new();
    text.push_str(&format!("release_target: {}\n", inspection.target.id()));
    text.push_str(&format!(
        "release_target_root: {}\n",
        inspection.target.root()
    ));
    text.push_str(&format!(
        "capability_requested: {}\n",
        inspection.capability
    ));
    text.push_str(&format!(
        "providers_considered: {}\n",
        join_or_none(&inspection.providers_considered)
    ));
    text.push_str(&format!(
        "providers_answering: {}\n",
        join_or_none(&inspection.providers_answering)
    ));
    // Tres secciones y no una, y no es estetica. MEDIDO en `va7-conflict-1`:
    // un repositorio con dos declaraciones y doce ficheros ausentes salia con
    // catorce entradas en una sola lista, y el conflicto —las dos unicas lineas
    // que contestan— quedaba debajo de doce «CMakeLists.txt no esta en este
    // target». El informe es correcto y el que lo lee no llega a la respuesta.
    //
    // El corte es por `level` y no por «trae version», porque `Invalid` no trae
    // version y se queda arriba: fallo cerrado es una **respuesta**, no una
    // ausencia, y esconderlo bajo las ausencias seria esconder el unico motivo
    // por el que a este comando le interessaria un codigo de salida distinto.
    let anyone_answered = inspection
        .findings
        .iter()
        .any(|finding| finding.level != AssuranceLevel::NotApplicable);
    text.push_str("observations:\n");
    if !anyone_answered {
        text.push_str("  none\n");
    }
    for finding in &inspection.findings {
        if finding.level == AssuranceLevel::NotApplicable {
            continue;
        }
        text.push_str(&format!(
            "  - {} [{:?}] {}\n",
            finding.provider_id, finding.level, finding.detail
        ));
        if let Some(version) = &finding.product_version {
            text.push_str(&format!("      productVersion: {version}\n"));
        }
        if let Some(location) = &finding.location {
            text.push_str(&format!("      location: {location}\n"));
        }
        if let Some(digest) = &finding.digest {
            text.push_str(&format!("      digest: {digest}\n"));
        }
        if !finding.recheckable {
            text.push_str("      recheckable: no — nothing to point at\n");
        }
    }

    // Consultados y sin nada que decir. Su propia seccion porque sin ella «nadie
    // declaro nada» y «nadie que pudiera declarar fue preguntado» se leen igual.
    let absences: Vec<_> = inspection
        .findings
        .iter()
        .filter(|finding| finding.level == AssuranceLevel::NotApplicable)
        .collect();
    if !absences.is_empty() {
        text.push_str(&format!(
            "\nconsulted_and_found_nothing: {}\n",
            absences.len()
        ));
        for finding in absences {
            text.push_str(&format!("  - {} {}\n", finding.provider_id, finding.detail));
        }
    }

    // Los que se filtraron, y por que. Distinto de la seccion anterior porque es
    // otra pregunta: estos no hablan la capacidad, luego nadie llego a
    // preguntarles — mientras que los de arriba se preguntaron y contestaron
    // que aqui no habia nada.
    if !inspection.skipped.is_empty() {
        text.push_str(&format!("\nnever_asked: {}\n", inspection.skipped.len()));
        for skipped in &inspection.skipped {
            text.push_str(&format!(
                "  - {} speaks {}, not {}\n",
                skipped.provider_id,
                join_or_none(&skipped.capabilities),
                inspection.capability
            ));
        }
    }

    text.push_str(&format!("\nauthority: {:?}\n", inspection.authority));
    match &inspection.version {
        Some(version) => text.push_str(&format!("productVersion: {version}\n")),
        None => text.push_str("productVersion: none\n"),
    }
    // La linea que separa «esto es lo que hay» de «esto es lo que no». Que un
    // rechazo de verdad no lo confunda con el resultado es medio del trabajo.
    //
    // Y nada mas despues. La primera version de este informe cerraba con un
    // `note:` escrito a mano que decia «finding a product version is not
    // certifying a release» — que es, palabra por palabra, el segundo punto de
    // `NOT_CHECKED`. Es decir: una frase sobre lo que no se comprobo, escrita a
    // mano al lado de la lista de lo que no se comprobo que se deriva. Dos
    // sitios, una sola afirmacion, y el dia que el reducer cambie uno se
    // desactualiza sin que nada lo note. Se va.
    text.push_str("\nNOT_CHECKED:\n");
    for item in &inspection.not_checked {
        text.push_str(&format!("  - {}\n", item.statement));
    }
    text
}

/// Una lista de nombres, o una palabra que diga que está vacía.
///
/// «providers_considered: » con la lista vacía es indistinguible de un informe
/// que se olvidó de la línea, y esa es la clase de hueco que este bloque
/// persigue.
fn join_or_none(items: &[String]) -> String {
    if items.is_empty() {
        "none".to_owned()
    } else {
        items.join(", ")
    }
}

/// `release version matches` — ¿la referencia declarada nombra a la versión?
///
/// ## Por qué este segundo subcomando y no otro sitio
///
/// La regla del lockstep ya tiene su puerta en `release plan` y `release apply`,
/// y esas puertas necesitan un preflight, un tag y un contexto. El caso donde
/// hace falta **preguntar** es el previo: «¿nombra `v1.4.0` a `1.4.0` con lo que
/// este proyecto ha declarado, antes de que nada más falle?». Eso es una
/// pregunta de una línea que no debería arrastrar un dry-run entero.
///
/// ## Lo que declara, y es lo mismo que declara `release`
///
/// La naming y el target se resuelven con las **mismas** funciones del mismo
/// sitio. Un segundo camino que separe el nombre, el target o la convención
/// sería un segundo lugar donde equivocarse, y este bloque lleva siete
/// haciéndolo peor cada vez que aparece uno.
fn run_release_version_matches(args: VersionMatchesArgs) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<VersionMatchOutput> {
        let root = resolve_inspection_root(&args.runtime)?;
        let naming = resolve_naming(&args.naming)?;
        // MEDIDO antes de este bloque: aqui vivia `BuildAsk::never()` con el
        // comentario «no hay pregunta que hacer», y la salida de `matches` no
        // dependia del tag —`v1.2.3` y `v9.9.9` daban byte a byte lo mismo— sobre
        // un build Gradle que si declara `1.2.3`. Un comando que contesta
        // «no hay nada contra que comparar» sin haber preguntado no mide la
        // comparacion: mide su propia ausencia de pregunta.
        //
        // La pregunta se lee de la linea de comandos como en `plan` e `inspect`, y
        // por la misma funcion, porque un segundo camino para decidir si se observa
        // seria el defecto que este bloque vino a cerrar.
        let ask = BuildAsk::of_parts(&args.ask)?;
        let selected = resolve_release_target(&root, args.target.as_deref(), &ask)?;
        let version = ask.registry().resolve(
            sddk_domain::version_authority::PRODUCT_VERSION_OBSERVATION,
            &selected.target,
        );
        let Some(declarada) = version.version() else {
            // Un proyecto sin version declarada no tiene nada contra lo que
            // comparar, y eso NO es un fallo de la regla: es la respuesta
            // `release_ref_is_authority`, que ADR-0157 introdujo precisamente para
            // que estos proyectos no se parecieran a los que se olvidaron.
            //
            // Lo que NO es admisible es que las cuatro razones por las que no hay
            // version produzcan el mismo texto: eso lo cuenta `sin_version`, y la
            // razon esta en el nombre de la funcion.
            let sin_version = sin_version(&version, ask.evaluate());
            return Ok(VersionMatchOutput {
                release_target: selected.report.id.clone(),
                release_reference: args.tag.clone(),
                naming: naming.style().to_owned(),
                product_version: None,
                matches: matches!(&version, VersionAuthority::ReleaseRefIsAuthority { .. }),
                measured: sin_version.measured,
                detail: sin_version.detail,
            });
        };
        let outcome = binds(
            &ReleaseRef::new(args.tag.clone(), sddk_domain::ReleaseChannel::Stable),
            declarada,
            &naming,
        );
        Ok(VersionMatchOutput {
            release_target: selected.report.id.clone(),
            release_reference: args.tag.clone(),
            naming: naming.style().to_owned(),
            product_version: Some(declarada.to_string()),
            matches: outcome.is_bound(),
            measured: true,
            detail: outcome.message(declarada),
        })
    })();
    render_result(result, format, version_match_text)
}

/// Que hay —o que no hay— una version de producto contra la que comparar, y si
/// eso es una **medicion** o su ausencia.
///
/// ## Por que el campo `measured` existe, y es lo que hace falsable el bloque
///
/// MEDIDO, y el dato es que `release version matches --tag v1.2.3` y
/// `release version matches --tag v9.9.9` devolvian **byte a byte la misma
/// salida** sobre un build Gradle que declara `1.2.3`. Un comando cuya
/// respuesta no depende de lo preguntado no esta midiendo la pregunta.
///
/// Arreglar solo el texto no cierra eso: si preguntar y no preguntar dieran el
/// mismo `matches: false` y solo cambiasen las frases, la salida estructurada
/// seguiria sin depender de lo que se hizo —el mismo defecto un nivel mas
/// arriba—. Por eso el `measured` va en el campo, no solo en la prosa.
#[derive(Debug, Clone)]
struct SinVersion {
    /// `false` solo cuando **nadie ha mirado lo suficiente**: no se pregunto al
    /// build tool y ningun fichero respondio.
    ///
    /// En cualquier otro caso se miro, y lo que se encontro —nada, una
    /// contradiccion, un fichero ilegible— es un hecho. Un `Conflict` con sus dos
    /// declaraciones es lo mas mirado que hay, no lo menos.
    measured: bool,
    /// Lo que se hizo, y lo que no se puede concluir.
    detail: String,
}

/// La razon por la que no hay version, en las palabras de quien la observo.
///
/// ## MEDIDO, y por que esta funcion y no cuatro textos
///
/// Antes de este bloque, los cuatro hechos que dejan a `matches` sin version
/// produzcan todos `this target declares no product version`:
///
/// | hecho | lo que decia | lo que era |
/// |---|---|---|
/// | dos ficheros se contradicen | «no declara version» | declara **dos**, y discrepan |
/// | un fichero no se pudo leer | «no declara version» | declara, y es ilegible |
/// | nadie pregunto al build tool | «no declara version» | no se pregunto |
/// | se pregunto y no hay version | «no declara version» | cierto —el unico de los cuatro— |
///
/// Los tres primeros son lo contrario de lo que dice el texto, y el que peor
/// parado sale es el primero: `Cargo.toml` en 4.2.0 y la declaracion del proyecto
/// en 9.9.9 dan `productVersion: none` y `matches: false` con **codigo de
/// salida 0**, sobre un proyecto que no se ha olvidado nada.
///
/// ## Por que nombra las dos declaraciones y no fourteen
///
/// Misma medida que tomo VA10 en el rechazo de una discrepancia: el lector viene
/// a ver **las que discrepan**, no a ver las catorce. Y no reimplementa el
/// informe de `release version inspect` —que ya lo dice bien—: nombra el
/// veredicto y remite ahi, que es una autoridad y no una segunda.
fn sin_version(authority: &VersionAuthority, asked: bool) -> SinVersion {
    match authority {
        VersionAuthority::ReleaseRefIsAuthority { .. } => SinVersion {
            measured: true,
            detail: "this target declares no product version; the release reference carries it"
                .to_owned(),
        },
        VersionAuthority::Ambiguous { candidates, .. } => {
            let declaraciones = candidates
                .iter()
                .map(|(quien, version)| format!("{quien} says {version}"))
                .collect::<Vec<_>>()
                .join(", ");
            SinVersion {
                measured: true,
                detail: format!(
                    "the sources disagree ({declaraciones}), which is not the same as this \
                     target declaring no version: it is this target declaring more than one, \
                     and this build refuses to pick. The reference cannot be compared until \
                     the project says which one it is. Run `sddk release version inspect` \
                     for the full list."
                ),
            }
        }
        VersionAuthority::Invalid { failures, .. } => {
            let ilegibles = failures
                .iter()
                .map(|(quien, por_que)| format!("{quien}: {por_que}"))
                .collect::<Vec<_>>()
                .join("; ");
            SinVersion {
                measured: true,
                detail: format!(
                    "a source exists and could not be read ({ilegibles}), which is not the \
                     same as this target declaring nothing: a source that exists and cannot \
                     be read is not silence. Run `sddk release version inspect` to see which \
                     one and why."
                ),
            }
        }
        VersionAuthority::Unresolved { .. } if asked => SinVersion {
            measured: true,
            detail: "the build tool was asked and no source produced a version. There is \
                     nothing to compare against, and that is a fact rather than an absence \
                     of looking."
                .to_owned(),
        },
        // El unico caso que NO es una medicion, y por eso es el unico que puede
        // llevar la bandera en el texto: sin `--evaluate-build` nadie leyo el
        // modelo del build, y un build script no se lee con un patron.
        _ => SinVersion {
            measured: false,
            detail: "this build did not ask the build tool, so this is NOT a measurement: \
                     only declaration files were read, and a build script is not one of \
                     them. Ask the build tool with `--evaluate-build --build-tool <tool>`, \
                     or run `sddk release version inspect` to see what was and was not \
                     looked at."
                .to_owned(),
        },
    }
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct VersionMatchOutput {
    release_target: String,
    release_reference: String,
    naming: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    product_version: Option<String>,
    matches: bool,
    /// Si `matches` es una **comparacion** o la ausencia de ella.
    ///
    /// MEDIDO, y es lo que hace falsable el bloque: antes, `matches` con un tag
    /// correcto y con uno erroneo salia byte a byte igual, porque el comando
    /// contestaba que no habia nada contra que comparar sin haber preguntado a
    /// quien lo sabia. Con esto, `false` significa dos cosas distintas y el
    /// consumidor puede distinguirlas sin leer la prosa.
    ///
    /// MEDIDO antes de tocar la forma: ningun script, workflow ni otro repositorio
    /// lee el JSON de este comando; los unicos consumidores son tests Rust y la
    /// documentacion.
    measured: bool,
    detail: String,
}

fn version_match_text(output: &VersionMatchOutput) -> String {
    let mut text = format!(
        "release_target: {}\nrelease_reference: {}\nnaming: {}\n",
        output.release_target, output.release_reference, output.naming
    );
    match &output.product_version {
        Some(version) => text.push_str(&format!("productVersion: {version}\n")),
        None => text.push_str("productVersion: none\n"),
    }
    text.push_str(&format!("matches: {}\n", output.matches));
    text.push_str(&format!("measured: {}\n", output.measured));
    text.push_str(&format!("detail: {}\n", output.detail));
    text
}

/// `sddk release handoff` — construir y entregar el sobre.
///
/// ## Qué es y qué no es
///
/// Un sobre **opaco**: SDDK lo transporta y no lo interpreta. Lo que significa
/// un candidato en el sistema que lo produjo —qué es un `external_handoff_type`,
/// quién tiene derecho a consumirlo— se queda con el productor, y esto es
/// exactamente el alcance de lo que SDDK afirma entender.
///
/// ## Por qué `--external-type` y `--external-digest` son obligatorios
///
/// Porque son hechos del **productor**, y SDDK no los tiene. Si tuvieran valor
/// por defecto, el sobre se podría rellenar entero sin hablar con nadie, y un
/// sobre que dice cosas que nadie dijo es peor que no tener sobre: el
/// certificador lee la firma del productor y no hay productor detrás.
///
/// La asimetría es el diseño completo: esto **calcula** el digest de cada
/// artefacto (content-addressable es un hecho: son los bytes) y **exige** el
/// digest del propio sobre (es la firma de otro sistema, no la suya).
///
/// ## Por qué la puerta es `may_reach` y no una nueva
///
/// Producir material hasta un canal es exactamente lo que dice el techo del
/// rol: un `CandidateProducer` termina en `Candidate`. Una función nueva de
/// «¿puede producir?» sería una segunda respuesta a una pregunta que el techo ya
/// contesta, y las dos divergirían.
///
/// ## Entregar no es publicar
///
/// El sobre dice «aquí está mi material». Que ese material se publique es la
/// puerta de `may_publish`, que este comando **no** llama: un productor que
/// publica lo suyo no está delegando nada.
fn run_release_handoff(args: HandoffArgs) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<CommandOutput2> {
        let root = resolve_inspection_root(&args.runtime)?;
        let role = resolve_role(&args.role)?;
        let naming = resolve_handoff_naming(&args.reference)?;
        let channel = sddk_domain::ReleaseChannel::parse(&args.reference.channel)
            .ok_or_else(|| anyhow::anyhow!("unknown channel '{}'", args.reference.channel))?;

        // LA PUERTA, y es la de VA6: el techo del rol, sin inventar otra.
        if !role.may_reach(channel) {
            return Err(anyhow::anyhow!(
                "{}",
                sddk_domain::release_role::RoleRefusal::BeyondCeiling {
                    role,
                    ceiling: role.ceiling(),
                    requested: channel,
                }
            ));
        }

        // MEDIDO antes de este bloque: aqui vivia `BuildAsk::never()` y, debajo,
        // `version_registry()` en vez de `ask.registry()` — **doble** partida: por
        // un lado los `Args` no declaraban las banderas, y por otro la resolucion
        // iba al registro que no pregunta. Poner las banderas sin cambiar el
        // registro habria dado un handoff que acepta `--evaluate-build` y lo
        // ignora, que es peor que no aceptarlas.
        let ask = BuildAsk::of_parts(&args.ask)?;
        let selected = resolve_release_target(&root, args.target.as_deref(), &ask)?;

        // La MISMA resolución que usa `release plan`, y por la MISMA `ask`: una
        // segunda llamada al registry sería una segunda autoridad, y la que acaba
        // dentro del sobre es la que el certificador leería como si la hubiera
        // emitido SDDK.
        let authority = ask.registry().resolve(
            sddk_domain::version_authority::PRODUCT_VERSION_OBSERVATION,
            &selected.target,
        );
        // El mismo texto que `matches`, por la misma funcion y por la misma razon:
        // las cuatro razones por las que no hay version no son la misma razon, y
        // un productor que lee «no hay nada que entregar» cuando lo que hay es una
        // contradiccion entre dos declaraciones va a arreglarlo en el sitio
        // equivocado.
        //
        // Y el texto de `sin_version` **ya** remite a `inspect` donde hace falta,
        // luego aqui no se repite: un mensaje que dice dos veces lo mismo no
        // informa mas, entrena al lector a leer el primero y saltar el segundo.
        // Ese defecto es de VA10 y lo corrigio entonces; repetirlo aqui seria
        // perderlo en la translation.
        let product_version = authority.version().cloned().ok_or_else(|| {
            anyhow::anyhow!(
                "this target has no product version to hand off: {}",
                sin_version(&authority, ask.evaluate()).detail
            )
        })?;

        let artifacts = args
            .material
            .artifacts
            .iter()
            .map(|entry| handoff_artifact(&root, entry))
            .collect::<anyhow::Result<Vec<_>>>()?;

        // La revisión es un hecho del árbol, no una afirmación sobre el release.
        let git = GitExecutor::new(root.clone());
        let source_revision = SourceRevision::new(
            git.head_sha()
                .map_err(|error| anyhow::anyhow!("cannot read the current revision: {error}"))?,
        )
        .ok_or_else(|| {
            anyhow::anyhow!(
                "the current revision is empty, so there is nothing to attribute this \
                 material to. A handoff with no source is a handoff about nothing."
            )
        })?;

        let sequence = match args.reference.sequence {
            Some(0) => {
                return Err(anyhow::anyhow!(
                    "`--sequence 0` claims to be a candidate with no predecessor. The \
                     first candidate is 1; a 0 is not a candidate that nobody asked \
                     for."
                ));
            }
            Some(raw) => Some(
                sddk_domain::release_ref::CandidateSequence::nth(raw)
                    .expect("el 0 ya se ha rechazado"),
            ),
            None => None,
        };
        let reference = match sequence {
            Some(seq) => ReleaseRef::candidate(args.reference.tag.clone(), channel, seq),
            None => ReleaseRef::new(args.reference.tag.clone(), channel),
        };

        // LA SEGUNDA PUERTA, y es la de ADR-0159: la referencia del sobre tiene
        // que nombrar a la versión del sobre, bajo la convención declarada.
        //
        // Sin esto, un sobre puede llevar `--tag v9.9.9 --sequence 1` junto a una
        // `productVersion` de `1.4.0` y llamarse coherente. El certificador
        // recibe un sobre que se contradice a sí mismo, y el que lo construyó fue
        // SDDK: eso es una autoridad usando un dato que nadie le dio.
        //
        // Se usa `binds`, no una comprobación nueva: la pregunta «¿nombra esta
        // referencia a esta versión?» ya tiene una respuesta, y una segunda sería
        // una que puede divergir.
        let binding = binds(&reference, &product_version, &naming);
        if !binding.is_bound() {
            return Err(anyhow::anyhow!(
                "{}",
                naming_rejection(&binding.message(&product_version), &naming)
            ));
        }

        let envelope = CandidateHandoff {
            target: selected.target.clone(),
            product_version,
            reference,
            sequence,
            source_revision,
            artifacts,
            evidence_refs: args.material.evidence_refs.clone(),
            external_handoff_type: args.material.external_type.clone(),
            external_handoff_digest: args.material.external_digest.clone(),
        };

        let body = serde_json::to_string_pretty(&envelope)
            .map_err(|error| anyhow::anyhow!("cannot serialise the envelope: {error}"))?;
        let written = match &args.out {
            Some(path) => {
                std::fs::write(path, format!("{body}\n")).map_err(|error| {
                    anyhow::anyhow!("cannot write the envelope to {}: {error}", path.display())
                })?;
                Some(path.display().to_string())
            }
            None => None,
        };
        let digest = format!("sha256:{:x}", Sha256::digest(body.as_bytes()));
        Ok(CommandOutput2 {
            envelope,
            written,
            digest,
            role: role.name().to_owned(),
            channel: format!("{channel:?}"),
        })
    })();

    match result {
        Ok(outcome) => match format {
            OutputFormat::Json => {
                let body = serde_json::to_string_pretty(&outcome.envelope).unwrap_or_default();
                CommandOutput {
                    status: 0,
                    stdout: format!("{body}\n"),
                    stderr: String::new(),
                }
            }
            OutputFormat::Text => CommandOutput {
                status: 0,
                stdout: handoff_text(&outcome),
                stderr: String::new(),
            },
        },
        Err(error) => error_output(format!("{error}")),
    }
}

/// La convención con la que se compara la referencia del sobre.
///
/// ## Por qué `--sequence` cambia la convención en vez de ser un dato suelto
///
/// `PrefixedCandidate` tiene tres partes —prefijo, separador, marcador— y
/// ninguna es un default razonable del núcleo: `rc` es una
/// convención, no una ley. Así que cuando hay secuencia se declaran las tres, y
/// `--naming` deja de aplicar.
///
/// La alternativa —una convención de candidato fija dentro del crate— daría un
/// nombre sin que nadie lo declarara, que es exactamente el defecto que
/// `VersionNaming::NAMED` excluye de `PrefixedCandidate` a propósito. Un default
/// invisible en el crate que sostiene los demás defaults no es un default, es
/// una segunda convención sin dueño.
fn resolve_handoff_naming(args: &HandoffReferenceArgs) -> anyhow::Result<VersionNaming> {
    if args.sequence.is_some() {
        return Ok(VersionNaming::prefixed_candidate(
            args.candidate_prefix.clone(),
            args.candidate_separator.clone(),
            args.candidate_marker.clone(),
        ));
    }
    resolve_naming(&args.naming)
}

/// Un artefacto del sobre: `kind=path`, con el digest de los bytes.
///
/// El digest se calcula aquí porque content-addressable es un **hecho**: son
/// estos bytes y su sha256. Lo que no se calcula es si el artefacto es el
/// correcto, si está completo, o si el material sirve para publicar — nada de
/// eso lo sabe SDDK, y opinar sería reescribir las reglas del productor.
fn handoff_artifact(root: &std::path::Path, entry: &str) -> anyhow::Result<HandoffArtifact> {
    let (kind, path) = entry
        .split_once('=')
        .ok_or_else(|| anyhow::anyhow!("`{entry}` is not an artifact: expected `kind=path`"))?;
    if kind.trim().is_empty() {
        return Err(anyhow::anyhow!(
            "`{entry}` has no kind: an artifact without a kind is a file the \
             consumer cannot interpret"
        ));
    }
    let absolute = root.join(path);
    let bytes = std::fs::read(&absolute)
        .map_err(|error| anyhow::anyhow!("cannot read artifact {}: {error}", absolute.display()))?;
    Ok(HandoffArtifact {
        kind: kind.to_owned(),
        digest: format!("sha256:{:x}", Sha256::digest(&bytes)),
        path: Some(path.to_owned()),
    })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "snake_case")]
struct CommandOutput2 {
    #[serde(flatten)]
    envelope: CandidateHandoff,
    written: Option<String>,
    digest: String,
    role: String,
    channel: String,
}

fn handoff_text(outcome: &CommandOutput2) -> String {
    let envelope = &outcome.envelope;
    let mut text = String::new();
    text.push_str(&format!("release_target: {}\n", envelope.target.id()));
    text.push_str(&format!("productVersion: {}\n", envelope.product_version));
    text.push_str(&format!(
        "release_reference: {}\nchannel: {}\n",
        envelope.reference.name(),
        outcome.channel
    ));
    match envelope.sequence {
        Some(sequence) => text.push_str(&format!("sequence: {}\n", sequence.get())),
        None => text.push_str("sequence: none\n"),
    }
    text.push_str(&format!(
        "source_revision: {}\nrole: {}\n",
        envelope.source_revision.as_str(),
        outcome.role
    ));
    text.push_str(&format!("artifacts: {}\n", envelope.artifacts.len()));
    for artifact in &envelope.artifacts {
        text.push_str(&format!("  - {} {}\n", artifact.kind, artifact.digest));
    }
    text.push_str(&format!(
        "evidence_refs: {}\n",
        join_or_none(&envelope.evidence_refs)
    ));
    text.push_str(&format!(
        "external_handoff_type: {}\nexternal_handoff_digest: {}\n",
        envelope.external_handoff_type, envelope.external_handoff_digest
    ));
    match &outcome.written {
        Some(path) => text.push_str(&format!("written: {path}\n")),
        None => text.push_str("written: none — the envelope is on stdout\n"),
    }
    text.push_str(&format!("envelope_sha256: {}\n", outcome.digest));
    // Lo que este sobre NO dice, porque no lo comprobó.
    text.push_str(
        "\nNOT_CHECKED:\n  \
         - the artifacts were hashed, not judged: whether this material is what a \
         certifier needs is the certifier's question\n  \
         - the external digest was supplied, not recomputed: SDDK does not have the \
         producer's serialisation, so it cannot check it\n  \
         - nothing was published, and nothing was certified\n",
    );
    text
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct ChannelPromoteOutput {
    from: String,
    to: String,
    allowed: bool,
    gates_ok: bool,
    reason: String,
}

fn run_release_channel(args: ChannelArgs) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<ChannelPromoteOutput> {
        let from = sddk_domain::ReleaseChannel::parse(&args.from).ok_or_else(|| {
            anyhow::anyhow!(
                "unknown channel '{}' (stable|candidate|edge|dev)",
                args.from
            )
        })?;
        let to = sddk_domain::ReleaseChannel::parse(&args.to).ok_or_else(|| {
            anyhow::anyhow!("unknown channel '{}' (stable|candidate|edge|dev)", args.to)
        })?;
        let allowed = sddk_domain::can_promote(from, to, args.gates_ok);
        let reason = if allowed {
            format!("promotion {} → {} allowed", args.from, args.to)
        } else if sddk_domain::promotion_target(from) != Some(to) {
            format!(
                "promotion {} → {} is not an adjacent channel step",
                args.from, args.to
            )
        } else {
            format!(
                "promotion {} → {} requires gates to pass (--gates-ok)",
                args.from, args.to
            )
        };
        Ok(ChannelPromoteOutput {
            from: args.from,
            to: args.to,
            allowed,
            gates_ok: args.gates_ok,
            reason,
        })
    })();
    match result {
        Ok(output) => {
            let mut command = render_result(Ok(output.clone()), format, channel_promote_text);
            if !output.allowed {
                command.status = 1;
            }
            command
        }
        Err(error) => crate::failure(error.to_string()),
    }
}

fn channel_promote_text(output: &ChannelPromoteOutput) -> String {
    format!(
        "from: {}\nto: {}\nallowed: {}\ngates_ok: {}\nreason: {}\n",
        output.from, output.to, output.allowed, output.gates_ok, output.reason
    )
}

const CHECKSUMS_FILE: &str = "checksums.txt";
const SBOM_FILE: &str = "sbom.json";
const ATTESTATION_FILE: &str = "attestation.json";

/// Generated distribution artifacts for one binary.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct DistOutput {
    version: String,
    channel: String,
    commit: String,
    binary: String,
    checksums: String,
    sbom: String,
    attestation: String,
}

fn run_release_dist(args: DistArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<DistOutput> {
        let binary = std::env::current_exe()?;
        let bytes = std::fs::read(&binary)?;
        let digest = format!("sha256:{:x}", Sha256::digest(&bytes));
        let version = env!("CARGO_PKG_VERSION").to_owned();

        // Workspace root for git operations and surface copying
        let workspace_root = args
            .source
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap());

        // Determine the commit identifier for the staging path.
        // Priority: explicit --commit flag > GITHUB_SHA env var > local HEAD > "unknown".
        // Using local HEAD SHA (not GITHUB_SHA or "unknown") ensures deterministic
        // staging paths that match the actual repository state.
        let commit = args
            .commit
            .clone()
            .or_else(|| std::env::var("GITHUB_SHA").ok())
            .or_else(|| {
                // Attempt to get local HEAD SHA from the workspace git repository.
                let git = sddk_gateway::GitExecutor::new(workspace_root.clone());
                git.head_sha().ok()
            })
            .unwrap_or_else(|| "unknown".to_owned());
        let timestamp = args
            .timestamp
            .unwrap_or_else(crate::git_cmd::default_timestamp);

        let dist_dir = args.prefix.join("dist");
        std::fs::create_dir_all(&dist_dir)?;
        let binary_path = dist_dir.join("sddk");
        std::fs::write(&binary_path, &bytes)?;

        let checksums = format!("{}  {}\n", digest, "sddk");
        std::fs::write(dist_dir.join(CHECKSUMS_FILE), &checksums)?;

        // ── Staged bundle roundtrip (REQ-RDI-002) ──────────────────────────
        // Compute the staging area.
        let data_dir = args
            .sddk_data_dir
            .clone()
            .or_else(|| environment.sddk_data_dir.clone())
            .or_else(|| environment.data_home.clone())
            .unwrap_or_else(|| {
                std::path::PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "~".into()))
                    .join(".local/share")
            });
        let staging_dir = data_dir.join("sddk/staging").join(&commit);
        // Remove any incomplete previous attempt before staging fresh.
        std::fs::remove_dir_all(&staging_dir).ok();
        std::fs::create_dir_all(&staging_dir)?;

        // Copy surfaces + MANIFEST.sha256 to staging.
        for surface in crate::dev::MANIFEST_SURFACES {
            let src = workspace_root.join(surface);
            let dst = staging_dir.join(surface);
            if src.is_dir() {
                crate::dev::copy_tree(&src, &dst, crate::dev::CopyMode::Always)?;
            }
        }
        // Copy the manifest itself.
        let manifest_file = "MANIFEST.sha256";
        let src_manifest = workspace_root.join(manifest_file);
        let dst_manifest = staging_dir.join(manifest_file);
        if src_manifest.is_file() {
            std::fs::copy(&src_manifest, &dst_manifest)?;
        }

        // Hash the staged MANIFEST.sha256 to derive manifest_sha256.
        // If no manifest is present, fields remain empty (skip verification).
        let (manifest_sha256, manifest_count) = if dst_manifest.is_file() {
            if !args.skip_manifest_preflight {
                // FAIL-CLOSED: verify staged manifest BEFORE writing attestation.
                let mismatches = crate::dev::verify_manifest(&staging_dir)?;
                if !mismatches.is_empty() {
                    anyhow::bail!(
                        "staged roundtrip FAILED ({} mismatch(es)):\n  {}",
                        mismatches.len(),
                        mismatches.join("\n  ")
                    );
                }
            } else {
                let ts = OffsetDateTime::now_utc()
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap_or_else(|_| "unknown".into());
                eprintln!("[{ts}] staged manifest verification SKIPPED: --skip-manifest-preflight");
            }
            let sha = crate::dev::sha256_hex(&dst_manifest)?;
            let content = std::fs::read_to_string(&dst_manifest)?;
            let count = content
                .lines()
                .filter(|l| !l.is_empty() && !l.starts_with("#"))
                .count();
            (sha, count)
        } else {
            (String::new(), 0)
        };
        let manifest_surfaces = if manifest_sha256.is_empty() {
            Vec::new()
        } else {
            crate::dev::MANIFEST_SURFACES
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        };
        // ── end staged roundtrip ──────────────────────────────────────────

        let sbom = serde_json::json!({
            "tool": "sddk",
            "version": version,
            "commit": commit,
            "channel": args.channel,
            "binary_sha256": digest,
            "dependencies": workspace_dependencies(),
            "manifest_sha256": manifest_sha256,
            "manifest_count": manifest_count,
            "manifest_surfaces": manifest_surfaces,
        });
        let sbom_path = dist_dir.join(SBOM_FILE);
        std::fs::write(&sbom_path, serde_json::to_string_pretty(&sbom)?)?;

        let attestation = serde_json::json!({
            "artifact": "sddk",
            "sha256": digest,
            "builder": "sddk dist",
            "channel": args.channel,
            "timestamp": timestamp,
            "commit": commit,
            "manifest_sha256": manifest_sha256,
            "manifest_count": manifest_count,
            "manifest_surfaces": manifest_surfaces,
            // Explicit flag: whether the staged bundle roundtrip was verified.
            // This is set only when manifest verification succeeded.
            "bundle_roundtrip_verified": !manifest_sha256.is_empty(),
        });
        let attestation_path = dist_dir.join(ATTESTATION_FILE);
        std::fs::write(
            &attestation_path,
            serde_json::to_string_pretty(&attestation)?,
        )?;

        Ok(DistOutput {
            version,
            channel: args.channel.clone(),
            commit,
            binary: binary_path.to_string_lossy().into_owned(),
            checksums: dist_dir.join(CHECKSUMS_FILE).to_string_lossy().into_owned(),
            sbom: sbom_path.to_string_lossy().into_owned(),
            attestation: attestation_path.to_string_lossy().into_owned(),
        })
    })();
    render_result(result, format, dist_text)
}

fn run_release_dist_verify(args: DistArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<serde_json::Value> {
        let dist_dir = args.prefix.join("dist");
        let binary_path = dist_dir.join("sddk");
        let bytes = std::fs::read(&binary_path)?;
        let digest = format!("sha256:{:x}", Sha256::digest(&bytes));

        let checksums = std::fs::read_to_string(dist_dir.join(CHECKSUMS_FILE))?;
        let expected = format!("{digest}  sddk\n");
        if checksums != expected {
            anyhow::bail!("checksums.txt does not match the binary digest");
        }

        let sbom: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dist_dir.join(SBOM_FILE))?)?;
        if sbom["binary_sha256"] != digest {
            anyhow::bail!("sbom.json binary digest does not match");
        }

        let attestation: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dist_dir.join(ATTESTATION_FILE))?)?;
        if attestation["sha256"] != digest {
            anyhow::bail!("attestation.json digest does not match");
        }

        // Verify a signed gate receipt when provided (fail-closed).
        // Accepts:
        //   - A JSON file path (e.g. release-receipt.json): loads the receipt,
        //     reconstructs the 10-field HMAC payload, and verifies the signature.
        //   - A pipe-separated legacy string: 4-part (receipt_id|gate|transition|plan_hash|signature)
        //     or 6-part widened (receipt_id|gate|transition|plan_hash|head_sha|tag|signature).
        if let Some(receipt_spec) = &args.receipt {
            // Canonical signing key location: $SDDK_DATA_DIR/keys/
            let keys_dir = crate::dev::paths::signing_keys_dir(environment)?;
            let key = sddk_engine::load_or_create_key(&keys_dir)?;

            // Detect JSON file path: contains .json or path separators
            let is_json_receipt = receipt_spec.contains(".json")
                || receipt_spec.contains('/')
                || receipt_spec.contains('\\');

            if is_json_receipt {
                // Load JSON receipt and verify widened 10-field HMAC payload
                let receipt_path = std::path::Path::new(receipt_spec);
                if !receipt_path.is_file() {
                    anyhow::bail!("receipt file not found: {}", receipt_spec);
                }
                let receipt_bytes = std::fs::read(receipt_path)?;
                let receipt: ReleaseReceipt = serde_json::from_slice(&receipt_bytes)?;

                // Reconstruct the 10-field HMAC payload
                let payload = format!(
                    "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
                    receipt.receipt_id,
                    receipt.gate,
                    receipt.transition,
                    receipt.plan_hash,
                    receipt.head_sha,
                    receipt.tag,
                    receipt.binary_sha256,
                    receipt.manifest_sha256,
                    receipt.manifest_count,
                    receipt.bundle_roundtrip_verified,
                );
                if !sddk_engine::verify_payload(&payload, &receipt.signature, &key) {
                    anyhow::bail!("release-receipt.json signature verification FAILED");
                }
                // Cross-check: binary_sha256 must match attestation.sha256.
                let binary_from_attestation = attestation["sha256"].as_str().unwrap_or("");
                if binary_from_attestation != digest {
                    anyhow::bail!(
                        "attestation.sha256 ({}) does not match receipt binary_sha256 ({})",
                        binary_from_attestation,
                        receipt.binary_sha256
                    );
                }
            } else {
                // Legacy pipe-separated format
                let parts: Vec<&str> = receipt_spec.split('|').collect();
                let (payload, sig_idx) = match parts.len() {
                    5 => (
                        format!("{}|{}|{}|{}", parts[0], parts[1], parts[2], parts[3]),
                        4,
                    ),
                    7 => (
                        format!(
                            "{}|{}|{}|{}|{}|{}",
                            parts[0], parts[1], parts[2], parts[3], parts[4], parts[5]
                        ),
                        6,
                    ),
                    _ => {
                        anyhow::bail!(
                            "receipt spec must be 4-part (legacy) or 6-part (widened): \
                             receipt_id|gate|transition|plan_hash|[head_sha|tag|]signature"
                        );
                    }
                };
                if !sddk_engine::verify_payload(&payload, parts[sig_idx], &key) {
                    anyhow::bail!("gate receipt signature verification FAILED");
                }
                // Cross-check: binary_sha256 must match attestation.sha256.
                let binary_from_attestation = attestation["sha256"].as_str().unwrap_or("");
                if binary_from_attestation != digest {
                    anyhow::bail!(
                        "attestation.sha256 ({}) does not match binary ({})",
                        binary_from_attestation,
                        digest
                    );
                }
                // When tag is present in the receipt, verify it matches attestation.tag.
                if parts.len() == 7 {
                    let expected_tag = parts[5];
                    let attestation_tag = attestation["tag"].as_str().unwrap_or("");
                    if attestation_tag != expected_tag {
                        anyhow::bail!(
                            "attestation.tag ({}) does not match receipt tag ({})",
                            attestation_tag,
                            expected_tag
                        );
                    }
                }
            }
        }
        Ok(serde_json::json!({
            "valid": true,
            "binary_sha256": digest,
            "sbom_version": sbom["version"],
            "channel": attestation["channel"],
        }))
    })();
    render_result(result, format, dist_verify_text)
}

fn workspace_dependencies() -> Vec<serde_json::Value> {
    let lock = match std::fs::read_to_string(
        std::env::current_dir()
            .unwrap_or_default()
            .join("Cargo.lock"),
    ) {
        Ok(lock) => lock,
        Err(_) => return Vec::new(),
    };
    let mut dependencies = Vec::new();
    let mut name = None;
    for line in lock.lines() {
        if let Some(rest) = line.strip_prefix("name = ") {
            name = Some(rest.trim_matches('"').to_owned());
        } else if let Some(rest) = line.strip_prefix("version = ")
            && let Some(name) = name.take()
        {
            dependencies.push(serde_json::json!({
                "name": name,
                "version": rest.trim_matches('"'),
            }));
        }
    }
    dependencies
}

fn dist_text(output: &DistOutput) -> String {
    format!(
        "version: {}\nchannel: {}\ncommit: {}\nbinary: {}\nchecksums: {}\nsbom: {}\nattestation: {}\n",
        output.version,
        output.channel,
        output.commit,
        output.binary,
        output.checksums,
        output.sbom,
        output.attestation
    )
}

fn dist_verify_text(output: &serde_json::Value) -> String {
    format!(
        "valid: {}\nbinary_sha256: {}\nsbom_version: {}\nchannel: {}\n",
        output["valid"].as_bool().unwrap_or(false),
        output["binary_sha256"].as_str().unwrap_or(""),
        output["sbom_version"].as_str().unwrap_or(""),
        output["channel"].as_str().unwrap_or("")
    )
}

/// El plan lleva **el tipo del engine**, no una estructura propia.
///
/// La primera versión de este output tenía su propia copia, con nombres de
/// campo distintos (`declared_in` frente a `candidates`, y
/// `undeclared_ecosystems` frente a `ecosystems`). Dos comandos serializaban el
/// mismo hecho con dos esquemas: quien leyera `release plan` y `release apply`
/// tenía que traducir entre ellos. Cuando `release apply` empezó a usar el
/// tipo canónico, esa copia se volvió una divergencia con fecha, así que
/// desaparece: el render de texto sabe leer los dos brazos del enum.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct ReleasePlanOutput {
    route: ReleaseRoute,
    branch: String,
    base: String,
    tag: String,
    head: Option<String>,
    steps: Vec<&'static str>,
    version_authority: VersionAuthority,
    /// Which product this plan is about, and how it was chosen.
    release_target: ResolvedTarget,
    /// The declared convention the tag was checked under.
    release_naming: String,
    /// What this target declared itself responsible for.
    release_role: String,
}

/// Which product a plan is about, and the evidence for the choice.
///
/// It is in the output and not only in the text because **a plan that does not
/// say which product it is about cannot be reviewed**. A version number without
/// a product is the ambiguity this block exists to remove, moved one level up:
/// `1.0.0` of which target?
#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct ResolvedTarget {
    /// The target's identity, which is its path relative to the repository
    /// root.
    id: String,
    /// The directory the providers read.
    root: String,
    /// Whether the repository root declared a product version, in which case
    /// nothing below it was looked for.
    root_resolved: bool,
    /// How the candidates were gathered, in one sentence a human can check.
    provenance: String,
    /// Every candidate considered, so a plan that resolved one of several says
    /// so without the operator having to go and look.
    candidates: Vec<String>,
}

fn run_release_plan(args: ReleaseArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<ReleasePlanOutput> {
        let route = selected_route(&args)?;
        if matches!(route, ReleaseRoute::Local) && (args.branch != "main" || args.base != "main") {
            anyhow::bail!("--route local requires --branch main and --base main");
        }
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let git = sddk_gateway::GitExecutor::new(context.root.clone());
        let naming = resolve_naming(&args.naming)?;
        let role = resolve_role(&args.role)?;
        // La MISMA pregunta que hace el apply, y se hace tambien aqui porque el
        // plan es el ensayo de esa decision. La regla vive en el dominio y se
        // pregunta una vez; que la pregunten los dos puntos de entrada no es
        // duplicar la regla, es no dejar un camino por el que no se pregunte.
        role_allows_publishing(role)?;

        // REQ-RDI-001: MANIFEST exact-set preflight (before any push/tag).
        // Production release route always verifies; no escape hatch.
        preflight_manifest(git.root(), false, "production release always verifies")?;

        // L1 lockstep: version tag must match workspace Cargo.toml version
        // La variante `detailed` y no la que aplana: el plan declara de dónde
        // salió la versión, y `map(|_| ())` tiraría exactamente lo que hay
        // que reportar. Sigue fallando cerrado ante un desajuste.
        let ask = BuildAsk::of_parts(&args.ask)?;
        let selected = resolve_release_target(git.root(), args.target.as_deref(), &ask)?;
        let authority =
            ensure_version_lockstep_detailed(&ask.registry(), &selected.target, &args.tag, &naming)
                .map_err(|error| lockstep_rejection(&error, &naming))?;
        let head = git.inspect()?.head;

        // REQ-RDI-002 / REQ-RDI-003: gather manifest + receipt fields.
        // Only populate when --prefix is provided (from `release dist` attestation).
        let (
            binary_sha256,
            manifest_sha256,
            manifest_count,
            manifest_surfaces,
            bundle_roundtrip_verified,
        ) = if let Some(ref prefix) = args.prefix {
            // Read from the dist attestation produced by `release dist`.
            let attestation_path = prefix.join("dist/attestation.json");
            let att: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&attestation_path)?)?;
            let ms = att["manifest_sha256"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            let mc = att["manifest_count"].as_u64().unwrap_or(0) as usize;
            let msf = att["manifest_surfaces"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            // Explicit bundle_roundtrip_verified from attestation (set by release dist
            // when staged manifest verification succeeded).
            let btv = att["bundle_roundtrip_verified"].as_bool().unwrap_or(false);
            (
                att["sha256"].as_str().unwrap_or_default().to_string(),
                ms,
                mc,
                msf,
                btv,
            )
        } else {
            // Derive from the running binary.
            let binary = std::env::current_exe()?;
            let bytes = std::fs::read(&binary)?;
            let digest = format!("sha256:{:x}", Sha256::digest(&bytes));
            (digest, String::new(), 0, Vec::new(), false)
        };

        // Write the release receipt at {cycle_artifacts_dir}/release-receipt.json
        // only when --prefix was provided (proving release dist was run first).
        if let (Some(_), Some(head_str)) = (args.prefix.as_ref(), head.as_ref()) {
            let plan_hash = format!("sha256:{:x}", Sha256::digest(head_str.as_bytes()));
            let receipt_id = format!("release-receipt-{}", &head_str[..8]);
            let channel = "release".to_string();
            let timestamp = args
                .timestamp
                .clone()
                .unwrap_or_else(crate::git_cmd::default_timestamp);
            let receipt = ReleaseReceipt {
                receipt_id,
                gate: "release-plan".to_string(),
                transition: "phase.plan.complete".to_string(),
                plan_hash,
                head_sha: head_str.clone(),
                tag: args.tag.clone(),
                binary_sha256,
                manifest_sha256,
                manifest_count,
                manifest_surfaces,
                bundle_roundtrip_verified,
                channel,
                timestamp,
                signature: String::new(),
            };
            write_release_receipt(receipt, &context.cycle_artifacts_path, environment)?;
        }

        Ok(ReleasePlanOutput {
            route,
            branch: args.branch.clone(),
            base: args.base.clone(),
            tag: args.tag.clone(),
            release_target: selected.report.clone(),
            release_naming: naming.style().to_owned(),
            release_role: role.name().to_owned(),
            head,
            steps: match route {
                ReleaseRoute::Local => vec![
                    "push_main",
                    "verify_main_sha",
                    "create_annotated_tag",
                    "verify_remote_tag",
                ],
                ReleaseRoute::Forge => vec!["create_pr", "merge_pr", "create_release"],
            },
            version_authority: authority,
        })
    })();
    render_result(result, format, release_plan_text)
}

fn run_release_apply(args: ReleaseArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let route = match selected_route(&args) {
        Ok(route) => route,
        Err(error) => return render_result(Err(error), format, release_outcome_text),
    };
    let result = (|| -> anyhow::Result<(
        String,
        std::path::PathBuf,
        CapabilityGateway,
        String,
        String,
        Option<LocalReleasePreconditions>,
    )> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let project_id = context.identity.project_id.to_string();
        let root = context.root.clone();
        let permissions = PermissionPolicy::from_file(root.join("permissions.yaml"))?;
        authorize_release(&permissions, route)?;
        let naming = resolve_naming(&args.naming)?;
        // Antes del cierre, porque el cierre devuelve una `CommandOutput` y no
        // un `Result`: dentro no hay donde propagar un fallo. Y es tambien donde
        // tiene que estar: una pregunta que se valida despues de decidir es una
        // pregunta que a veces no se valida.
        let ask = BuildAsk::of_parts(&args.ask)?;
        let local_preconditions = matches!(route, ReleaseRoute::Local)
            .then(|| {
                local_release_preconditions(
                    &context,
                    &project_id,
                    &NamedReference {
                        name: args.tag.clone(),
                        naming,
                    },
                    environment,
                    &LocalReleaseQuestion {
                        cycle_id: args.cycle.as_deref(),
                        previous_tag: args.previous_tag.as_deref(),
                        release_type: args.release_type.map(|r| r.into()),
                        ask: &ask,
                    },
                )
            })
            .transpose()?;
        let workflow = context.engine.workflow().clone();
        let policy = CapabilityPolicy::from_workflow(&workflow);
        let gateway = CapabilityGateway::new(policy, workflow, context.storage);
        let timestamp = args
            .timestamp
            .clone()
            .unwrap_or_else(crate::git_cmd::default_timestamp);
        let actor = args
            .actor
            .clone()
            .or_else(|| environment.sddk_actor.clone())
            .or_else(|| environment.user.clone())
            .unwrap_or_else(|| "sddk-cli".into());
        Ok((project_id, root, gateway, timestamp, actor, local_preconditions))
    })();
    let (project_id, root, mut gateway, timestamp, actor, local_preconditions) = match result {
        Ok(value) => value,
        Err(error) => return render_result(Err(error), format, release_outcome_text),
    };

    match route {
        ReleaseRoute::Local => {
            let result = (|| -> anyhow::Result<LocalReleaseOutcome> {
                if args.branch != "main" || args.base != "main" {
                    anyhow::bail!("--route local requires --branch main and --base main");
                }
                Ok(apply_local_release(
                    &mut gateway,
                    &LocalReleaseInput {
                        project_id,
                        cycle_id: args.cycle,
                        branch: args.branch,
                        tag: args.tag,
                        tag_message: args.title,
                        approve: args.approve,
                        timestamp,
                        actor,
                        preconditions: local_preconditions
                            .expect("local route preconditions were resolved"),
                    },
                    &GitExecutor::new(root),
                )?)
            })();
            render_result(result, format, local_release_outcome_text)
        }
        ReleaseRoute::Forge => {
            let result = (|| -> anyhow::Result<sddk_gateway::ReleaseOutcome> {
                let repo = args.repo.as_deref().ok_or_else(|| {
                    anyhow::anyhow!("--repo is required when --route forge is selected")
                })?;
                // Building the adapter is this arm's job; running the release is
                // the function's. Keeping the two apart is the whole seam: with
                // the `gh` runner chosen HERE and never below, a test can reach
                // the body with a double instead of only with a repository.
                let mut forge = GitHubForge::new(repo);
                apply_release_forge(
                    &mut gateway,
                    &mut forge,
                    &project_id,
                    &args,
                    &root,
                    &timestamp,
                    &actor,
                )
            })();
            render_result(result, format, release_outcome_text)
        }
    }
}

/// The body of `release apply --route forge`, with the forge as a parameter.
///
/// It used to be inlined in the `ReleaseRoute::Forge` arm, which built
/// `GitHubForge::new(repo)` itself and therefore pinned the `gh` runner: no test
/// could reach this code without publishing to a real repository. Neither the
/// adapter nor the engine needed anything for that — `GitHubForge::with_runner`
/// existed and `apply_release` already took `&mut dyn Forge`. The missing piece
/// was the seam here.
///
/// **Nothing about what the route does changed**, and R4 pins each of it:
/// `authorize_release` still requires the three capabilities, the chain is still
/// `CreatePr → MergePr → CreateRelease` inside one `AdmissionTicket` (ADR-0132),
/// and the version is still checked before anything runs. STOP 1 of the
/// SCOPE-CONTRACT discards this extraction if any of that relaxes — a route made
/// testable by giving up a control is not a route made testable.
fn apply_release_forge(
    gateway: &mut CapabilityGateway,
    forge: &mut dyn sddk_gateway::Forge,
    project_id: &str,
    args: &ReleaseArgs,
    root: &std::path::Path,
    timestamp: &str,
    actor: &str,
) -> anyhow::Result<sddk_gateway::ReleaseOutcome> {
    let naming = resolve_naming(&args.naming)?;
    let role = resolve_role(&args.role)?;
    // La puerta del rol, ANTES que la del lockstep y antes que cualquier efecto:
    // un target que declara no ser el publicador no tiene nada que autorizar
    // sobre un tag, y decirlo primero evita que el operador lea un desajuste de
    // version en un release que no iba a existir.
    role_allows_publishing(role)?;
    // L1 lockstep: el tag tiene que coincidir con la versión
    // declarada. Se pregunta por la variante que devuelve de
    // dónde salió, y no por un literal: escribir `true` a mano
    // informaba un lockstep comprobado en proyectos donde no se
    // comprobó nada, porque no hay manifiesto contra el que
    // comparar. Sigue fallando cerrado ante un desajuste.
    // UNA pregunta para las dos preguntas. La puerta y la identidad del producto
    // se resuelven con el mismo registro a proposito: si cada una construyera el
    // suyo, la autoridad seria de una pregunta y el target de otra, y un outcome
    // asi no se puede auditar.
    let ask = BuildAsk::of_parts(&args.ask)?;
    let version_authority = version_authority_or_fail(root, &args.tag, &naming, &ask)?;
    // La identidad del producto, de la MISMA seleccion que produjo la
    // autoridad. Volver a resolverla aqui podria dar otra respuesta si el arbol
    // cambiase entre medias, y un outcome que dice una cosa y el plan otra es
    // un registro que no se puede auditar.
    let release_target_id = resolve_release_target(root, args.target.as_deref(), &ask)?
        .report
        .id
        .clone();
    let input = ReleasePlanInput {
        project_id: project_id.to_string(),
        cycle_id: None,
        branch: args.branch.clone(),
        base_branch: args.base.clone(),
        pr_title: args.title.clone(),
        pr_body: format!("Release {} from {}", args.tag, args.branch),
        tag: args.tag.clone(),
        release_title: args.title.clone(),
        release_notes: args.notes.clone(),
        approve: args.approve,
        timestamp: timestamp.to_string(),
        actor: actor.to_string(),
    };
    let plan = plan_release(input, &*forge)?;

    // A6-2: wrap the entire CreatePr → MergePr → CreateRelease
    // chain under one AdmissionTicket (issue + consume). The body
    // runs only after the ticket is consumed; if consume fails
    // (deny / fence expired / policy changed) the chain is
    // never executed. See ADR-0132.
    let ticket_actor = sddk_engine::authority_engine::Actor {
        kind: sddk_engine::authority_engine::ActorKind::System {
            service: "sddk-cli/release-apply".to_string(),
        },
        capabilities: vec!["cli.execute".to_string()],
        lease: None,
    };
    with_github_releases_ticket::<_, sddk_gateway::ReleaseOutcome>(
        ticket_actor,
        &format!("release/{}", args.tag),
        || {
            Ok(apply_release(
                gateway,
                &plan,
                forge,
                version_authority,
                release_target_id.clone(),
                naming.style().to_owned(),
                role.name().to_owned(),
            )?)
        },
    )
    .map_err(|e| match e {
        GithubReleasesTicketError::Denied(msg) => {
            anyhow::anyhow!("github_releases ticket denied: {msg}")
        }
        GithubReleasesTicketError::Ticket(err) => {
            anyhow::anyhow!("github_releases ticket error: {err:?}")
        }
        GithubReleasesTicketError::Apply(err) => err,
    })
}

fn selected_route(args: &ReleaseArgs) -> anyhow::Result<ReleaseRoute> {
    match args.route {
        Some(route) => Ok(route),
        None if args.repo.is_some() => anyhow::bail!(
            "release route now defaults to local; legacy Forge invocations must pass --route forge"
        ),
        None => Ok(ReleaseRoute::Local),
    }
}

/// Verify the release tag matches the workspace Cargo.toml version (lockstep rule).
///
/// The lockstep rule: `version tag == workspace Cargo.toml version`.
fn authorize_release(policy: &PermissionPolicy, route: ReleaseRoute) -> anyhow::Result<()> {
    // The local route reads the local preconditions through `git.inspect` and
    // also applies `git.push` and `git.tag`. The forge route uses forge-only
    // capabilities. The permission registry MUST list every capability that
    // `apply_local_release` actually executes, otherwise the release would
    // run with an unauthorized read.
    let capabilities: &[&str] = match route {
        ReleaseRoute::Local => &["git.inspect", "git.push", "git.tag"],
        ReleaseRoute::Forge => &["pr.create", "pr.merge", "release.create"],
    };
    for capability in capabilities {
        let decision = policy.authorize("sddk-release", "release", capability);
        if !decision.allowed {
            anyhow::bail!("release permission denied: {}", decision.reason);
        }
    }
    Ok(())
}

/// La autoridad de la versión que se comprueba contra el tag, o error si la
/// regla del lockstep se incumple.
///
/// Existe como función, y no como línea dentro de un closure, porque las dos
/// cosas que se le piden son distintas y no caben en un mismo tipo: el
/// resultado de un release **registra** de dónde salió la versión, y esa
/// llamada **falla cerrado** si el tag no cuadra.
fn version_authority_or_fail(
    root: &std::path::Path,
    tag: &str,
    naming: &VersionNaming,
    ask: &BuildAsk,
) -> anyhow::Result<VersionAuthority> {
    // El mensaje del motor se pasa tal cual. Una version anterior de esta
    // capa anadia encima «si tu proyecto no declara su version, puede
    // declararlo en <fichero>» — y lo hacia SIEMPRE, incluso cuando el fallo
    // era una discrepancia o un manifiesto roto, donde ese consejo no aplica
    // y desvia la atencion de lo que hay que mirar. En el unico caso en que
    // si aplica, el mensaje ya trae el nombre: lo dice el provider de la
    // declaracion al responder que este target no declara nada.
    let selected = resolve_release_target(root, None, ask)?;
    ensure_version_lockstep_detailed(&ask.registry(), &selected.target, tag, naming)
        .map_err(|error| lockstep_rejection(&error, naming))
}

/// El rol que este target declara.
///
/// Falla cerrado con la lista, por el mismo motivo que [`resolve_naming`]: quien
/// escribe `--role` mal tiene que ver **qué** se puede escribir, y un error de
/// sintaxis de un enum que no ha visto nunca no le dice nada.
fn resolve_role(name: &str) -> anyhow::Result<ReleaseRole> {
    ReleaseRole::parse(name).map_err(|error| anyhow::anyhow!("{error}"))
}

/// La puerta del rol, para un release que va a **publicar**.
///
/// ## Por qué aqui y no en el dominio
///
/// El dominio responde «puede este rol publicar?» con la razon, y el dominio no
/// sabe que `release apply` es un paso de lattices o que las puertas ya han
/// pasado. Esta capa sabe las dos cosas, asi que la composicion —`from`,
/// `to` y `gates_ok`— es suya, y la **pregunta** es del dominio.
///
/// ## Lo que se le pregunta, y lo que sale de verdad
///
/// Un `release apply` publica, y publicar es entrar en `stable`: de ahi que
/// `from` sea `candidate` y `to` sea `stable`. Las puertas se dan por abiertas
/// porque el lockstep ya paso —si no, el release no llega aqui— y se dan por
/// **supuestas**, no por saltadas: si la suposicion fuera falsa, el rechazo lo
/// diria, porque `may_publish` vuelve a preguntar el reticulo entero.
fn role_allows_publishing(role: ReleaseRole) -> anyhow::Result<()> {
    sddk_domain::release_role::may_publish(
        role,
        sddk_domain::ReleaseChannel::Candidate,
        sddk_domain::ReleaseChannel::Stable,
        true,
    )
    .map_err(|error| anyhow::anyhow!("{error}"))
}

/// La convencion con la que este release se autoriza.
///
/// ## Por qué vive aquí y no en el motor
///
/// El motor decide; esta capa compone. Una convención de nombres es una
/// declaración **del proyecto**, y un crate que decide ya ha decidido la
/// convención de todos los proyectos en el momento en que pone un valor por
/// defecto. Que el valor por defecto sea `v_prefixed` no lo hace menos
/// cableado: lo hace *invisible*, que es peor.
///
/// El precio de sacarlo de ahí es que el motor ya no tiene nada que decir sobre
/// la forma de un tag, y esta capa tiene que decirlo cada vez. Es un coste
/// real y es el correcto.
///
/// Y falla cerrado con la lista: quien escribe `--naming mal` tiene que ver
/// **qué** se puede escribir, no un error de sintaxis de un enum.
fn resolve_naming(style: &str) -> anyhow::Result<VersionNaming> {
    VersionNaming::parse(style).map_err(|error| anyhow::anyhow!("{error}"))
}

/// El rechazo de lockstep, con la convención que se aplicó y cómo cambiarla.
///
/// ## Por qué esto vive en la CLI y no en el motor
///
/// El mensaje del motor dice las dos cadenas —la que la naming daría y la que
/// traía la referencia—, que es lo que hace la corrección mecánica. No dice
/// **cómo** se cambia la convención, porque el motor no tiene ninguna: la
/// Flag es de esta capa y el motor no la conoce.
///
/// Y el consejo va **siempre**, no solo cuando el prefijo parece el culpable.
/// Enseñar la convención aplicada en todo rechazo es un hecho —el release se
/// autorizó bajo `v_prefixed`— y un hecho en un rechazo no puede ser consejo
/// equivocado. Un consejo condicional, en cambio, necesita acertar en su
/// diagnóstico, y el día que acierte mal le enseñará a alguien a despreciarlo.
///
/// Lo que NO se dice es «renombra tu tag» ni «añade un prefijo»: eso sería
/// decidirle al proyecto su modelo, y la convención la declara quien la
/// publica.
fn lockstep_rejection(error: &VersionLockstepError, naming: &VersionNaming) -> anyhow::Error {
    naming_rejection(&error.to_string(), naming)
}

/// El mismo rechazo, desde cualquiera de las dos preguntas que lo producen.
///
/// ## Por qué una función y no dos
///
/// El motor (puerta de lockstep) y `binds` (sobre) responden a **la misma
/// pregunta** por dos caminos distintos: uno llega por el error tipado de
/// `ensure_version_lockstep`, el otro por el `BindOutcome`. El sufijo —la
/// convención aplicada y cómo cambiarla— es un hecho, y un hecho escrito en dos
/// sitios es un hecho que se desactualiza en uno de ellos sin que nada avise.
fn naming_rejection(message: &str, naming: &VersionNaming) -> anyhow::Error {
    // El consejo tiene que ser un consejo que se pueda seguir. MEDIDO: con una
    // convención de candidato, el texto proponía `--naming <v_prefixed|exact>`,
    // y ninguna de las dos es applicable —`PrefixedCandidate` no se selecciona
    // por `--naming`, se construye con sus tres partes—. Un rechazo que sugiere
    // una salida imposible es peor que un rechazo sin consejo, porque gasta la
    // confianza del lector en un camino que no lleva a ninguna parte.
    let hint = if naming.style() == VersionNaming::NEEDS_PARAMETERS {
        format!(
            "declared naming: {} — it comes from `--sequence`; change it with \
             `--candidate-prefix`/`--candidate-separator`/`--candidate-marker`",
            naming.style()
        )
    } else {
        format!(
            "declared naming: {} — change it with `--naming <{}>`",
            naming.style(),
            VersionNaming::NAMED.join("|")
        )
    };
    anyhow::anyhow!("{message}\n{hint}")
}

/// Los providers que el release usa para preguntar.
///
/// Es una línea, y esa es la medida del éxito de este bloque: lo que antes era
/// un registro de ocho ecosistemas con sus extractores vive ahora en el
/// adapter, y añadir un TARGET más —un subdirectorio, un segundo producto— no
/// toca ni el motor ni el dominio.
fn version_registry() -> sddk_domain::version_authority::VersionResolverRegistry {
    sddk_gateway::version_provider::default_version_registry()
}

/// El dialecto que piden las banderas, o el error que dice que no hay.
///
/// ## Por que vive aqui y no en el gateway
///
/// Porque la pregunta es de la LINEA DE COMANDOS, no del dominio: `--evaluate-build`
/// sin `--build-tool` no es una pregunta a nadie, y eso lo sabe quien lee las
/// banderas. El gateway solo sabe preguntar; no sabe que le hayan pedido.
///
/// El default que este helper **no** tiene es el arreglo entero del defecto
/// medido: un nombre que no se reconoce es un error visible, no un Gradle en
/// disguise.
fn dialect_asked(
    evaluate: bool,
    tool: Option<&str>,
) -> anyhow::Result<Option<sddk_gateway::version_provider::BuildToolDialect>> {
    let Some(nombre) = tool else {
        if evaluate {
            anyhow::bail!(
                "--evaluate-build necesita --build-tool: SDDK no tiene un build tool por \
                 defecto, porque un default aqui seria responder en Gradle cuando se \
                 pidio otra cosa. Herramientas que SDDK sabe preguntar: {}",
                sddk_gateway::version_provider::UnknownBuildTool::SUPPORTED.join(", ")
            );
        }
        return Ok(None);
    };
    Ok(Some(
        sddk_gateway::version_provider::BuildToolDialect::parse(nombre)?,
    ))
}

/// Lo que el operador ha pedido **sobre el build tool**, en un valor.
///
/// Viaja como una cosa y no como dos parámetros porque es una cosa: o se evalúa
/// con una herramienta, o no se evalúa. Pasarlas sueltas invitaría a que una
/// llamada pase `evaluate_build` y olvide `build_tool`, que es un provider que
/// lanza un proceso con el nombre equivocado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BuildAsk {
    evaluate: bool,
    dialect: Option<sddk_gateway::version_provider::BuildToolDialect>,
}

impl From<BuildAsk> for BuildAskArgs {
    /// El dialecto vuelve a su NOMBRE porque `ReleaseArgs` recibe las banderas de
    /// la linea de comandos. El viaje de ida —nombre a dialecto— ya ocurrio y ya
    /// fallo cerrado; este no vuelve a interpretar nada, solo lo devuelve a la
    /// forma que la linea de comandos entiende.
    ///
    /// El viaje de vuelta si ocurre, en `ship`, y es el mismo `dialect_asked` que
    /// fallo cerrado a la ida: no hay una segunda interpretacion, hay la misma dos
    /// veces, y un nombre que se parseo una vez se parsea igual la segunda.
    fn from(ask: BuildAsk) -> Self {
        Self {
            evaluate_build: ask.evaluate,
            build_tool: ask.dialect.map(|d| d.id().to_owned()),
        }
    }
}

impl BuildAsk {
    /// La misma pregunta, desde banderas sueltas.
    ///
    /// **Una sola entrada**, y es la que usan los cuatro comandos. Existieron
    /// hasta tres —`of`, `of_inspect` y esta— porque las banderas vivian en tres
    /// `Args` distintos, y la razon que el propio codigo daba era «no un `From`
    /// porque dejaria abierta la pregunta de de donde salio esto». Al aplanar un
    /// unico tipo `BuildAskArgs` en los cuatro, **la razon desaparece**: ahora si
    /// se sabe de donde sale, y tres constructores para el mismo valor eran tres
    /// sitios donde olvidar la mitad de la pregunta.
    ///
    /// La ley es la misma en las cuatro entradas, que es lo que importa: que una
    /// ruta pueda responder en Gradle cuando se pidio otra cosa seria el defecto
    /// medido por la otra puerta.
    pub(crate) fn of_parts(args: &BuildAskArgs) -> anyhow::Result<Self> {
        Ok(Self {
            evaluate: args.evaluate_build,
            dialect: dialect_asked(args.evaluate_build, args.build_tool.as_deref())?,
        })
    }

    // `never()` **ya no existe** como constructor, y su ausencia es la prueba
    // estructural de que el hueco se cerro.
    //
    // MEDIDO antes de este bloque: lo llamaban `release version matches` y
    // `release handoff`, y ambos contestaban sobre la version sin poder
    // preguntar — con el tag correcto y con el erroneo, `matches` salia byte a
    // byte igual—.
    //
    // Los dos leen ahora `of_parts(&args.ask)`, luego este constructor se queda
    // sin un solo uso y desaparece. No es limpieza: es la prueba de que ya no
    // existe el camino de codigo que produce una `BuildAsk` sin banderas, y
    // reintroducirlo exigiria escribir el `Self` a mano.
    fn registry(&self) -> sddk_domain::version_authority::VersionResolverRegistry {
        version_registry_asking(self.evaluate, self.dialect)
    }

    /// Si se pregunto al build tool.
    ///
    /// Lo necesita `sin_version` porque **«no hay version» tiene dos causas
    /// opuestas** y el texto tiene que distinguirlas: nadie pregunto, o se
    /// pregunto y no habia. Sin este getter, el que redacta el mensaje deduce
    /// una de las dos de un valor implicito.
    pub(crate) fn evaluate(&self) -> bool {
        self.evaluate
    }
}

/// El mismo registro, y además el provider que **pregunta** a la herramienta
/// de build — si alguien lo ha pedido.
///
/// Que sea una bandera y no una preferencia del repo es la misma razón por la
/// que `--naming` y `--role` son banderas: un default en el registro se paga
/// sin que nadie lo pidiera, y lo que se paga sin pedirlo es lo que nadie
/// revisa.
fn version_registry_asking(
    evaluate_build: bool,
    dialect: Option<sddk_gateway::version_provider::BuildToolDialect>,
) -> sddk_domain::version_authority::VersionResolverRegistry {
    if !evaluate_build {
        return version_registry();
    }
    // El dialecto trae programa, args, ficheros y parser. Esta capa ya no sabe
    // como se pregunta a nadie, que es lo que hacia posible el defecto medido.
    sddk_gateway::version_provider::version_registry_with(dialect)
}

/// Elige el target, o explica por que no puede, y deja constancia de como.
///
/// Esta es la unica funcion de la CLI que decide **que producto** se
/// versiona. Que el repositorio contenga uno o varios es una pregunta que se
/// responde una vez y se registra, porque un plan que no dice que producto es
/// su no es revisable: un numero de version sin producto es la ambiguedad que
/// este bloque quita, movida un nivel mas arriba.
/// ## Por que recibe la `ask` y no construye su propio registry
///
/// MEDIDO, al cerrar este bloque: un repo Maven con solo `pom.xml` no era un
/// release target, y el provider de build tool **nunca se ejecutaba**. El motivo
/// es un deadlock: `release_targets` decide que es un target preguntando quien
/// declara version, y el unico que podria declararla —el build tool— se consultaba
/// despues de esa decision.
///
/// Un registro sin dialecto no puede digitalizar un proyecto Maven, y ese es el
/// caso que la capacidad existe para resolver. Por eso la pregunta viaja con la
/// funcion en vez de construirse dentro: una funcion que decide con un registro
/// distinto del que luego informa, es una que puede mirar dos cosas distintas.
fn resolve_release_target(
    root: &std::path::Path,
    requested: Option<&str>,
    ask: &BuildAsk,
) -> anyhow::Result<SelectedTarget> {
    use sddk_domain::version_authority::{TargetSelector, select_target};

    let set = sddk_gateway::version_provider::release_targets(root, &ask.registry());
    let selector = match requested {
        Some(id) => TargetSelector::named(id),
        None => TargetSelector::unsolicited(),
    };
    let selected = select_target(&set.targets, &selector).map_err(|error| match &error {
        sddk_domain::version_authority::TargetSelectionError::NoTarget => anyhow::anyhow!(
            "VERSION TARGET ERROR: no release target found under {}. {}.",
            root.display(),
            set.provenance()
        ),
        sddk_domain::version_authority::TargetSelectionError::AmbiguousTarget {
            candidates,
            requested,
        } => anyhow::anyhow!(
            "VERSION TARGET ERROR: {} release targets found{} and nothing names one: {}. \
             Choose with --target <path>. Which product to release is a decision, and \
             sddk will not make it.",
            candidates.len(),
            match requested {
                Some(name) => format!(" (and the name {name} matches more than one)"),
                None => String::new(),
            },
            candidates.join(", "),
        ),
        sddk_domain::version_authority::TargetSelectionError::UnknownTarget {
            requested,
            available,
        } => anyhow::anyhow!(
            "VERSION TARGET ERROR: no release target named {requested}. {}",
            if available.is_empty() {
                format!("None found under {}.", root.display())
            } else {
                format!("Available: {}.", available.join(", "))
            }
        ),
    })?;

    Ok(SelectedTarget {
        target: selected.clone(),
        report: ResolvedTarget {
            id: selected.id().to_owned(),
            root: selected.root().to_owned(),
            root_resolved: set.root_resolved,
            provenance: set.provenance(),
            candidates: set.targets.iter().map(|t| t.id().to_owned()).collect(),
        },
    })
}

/// Un target elegido, con el registro de por que se eligio.
///
/// Los dos van juntos a proposito: el `ReleaseTarget` es lo que se resuelve y el
/// `ResolvedTarget` es lo que se le dice a quien lee el plan. Separarlos
/// permitio que uno se usara sin el otro, y un plan sin registro no dice que
/// producto es su.
struct SelectedTarget {
    target: ReleaseTarget,
    report: ResolvedTarget,
}

/// Si la regla del lockstep se cumplió. `true` **no** significa que la versión
/// se comprobara contra algo: un proyecto que no declara versión en ningún
/// manifiesto —Go, Bazel— no tiene nada que comparar, y esa es justamente la
/// razón por la que no se le bloquea.
///
/// No es lo mismo que [`version_authority_or_fail`], y confundirlos es el
/// defecto que este lote cierra: uno es una **puerta** y el otro un **registro**.
/// La puerta local, con la **misma** seleccion de target que el plan.
///
/// Que use la misma funcion no es un detalle de estilo: si la puerta mirase un
/// target distinto del que el plan autorizo, la puerta estaria comprobando una
/// cosa y el release publicando otra, y un desajuste ahi es invisible porque
/// los dos exits son cero.
/// La regla de lockstep, con el registry que el operador ha pedido.
///
/// ## Por que lleva `ask` y no un default
///
/// Porque **no mirar cambia la conducta de la puerta**, y eso no puede
/// esconderse en un parametro opcional. MEDIDO: sin esta capacidad, un build
/// Gradle que declara `1.2.3` acepta un tag `v9.9.9`, porque sin version no hay
/// lockstep que incumplir. Quien no mira tiene que **decirlo** —
/// `BuildAsk::never()` —, no omitirlo: un default silencioso en una puerta es un
/// default que nadie revisa.
fn version_lockstep_satisfied_asking(
    root: &std::path::Path,
    tag: &str,
    naming: &VersionNaming,
    ask: &BuildAsk,
) -> bool {
    let Ok(selected) = resolve_release_target(root, None, ask) else {
        return false;
    };
    ensure_version_lockstep(&ask.registry(), &selected.target, tag, naming).is_ok()
}

/// El release tal como lo nombra quien publica: su nombre y la convención con
/// que ese nombre se lee.
///
/// Van juntos porque son **una** declaración partida en dos campos por el
/// parseo de la línea de comandos. Un tag sin la convención que lo autoriza no
/// es un tag: es una cadena que algún sitio tendrá que interpretar, y ese
/// «algún sitio» es donde se cuelan los recortes.
struct NamedReference {
    name: String,
    naming: VersionNaming,
}

/// Lo que el operador **declaró** para este release.
///
/// ## Por que un param object y no ocho parametros
///
/// MEDIDO, al anadir `ask`: la funcion estaba en siete —el umbral de clippy— y
/// la octava bandera la paso. La respuesta facil es un
/// `#[allow(clippy::too_many_arguments)]`, y eso es lo que hace una lista de
/// entradas que nadie vuelve a leer.
///
/// Lo agrupado aqui es una sola cosa: lo que el operador dijo. `cycle_id`,
/// `previous_tag`, `release_type` y `ask` son cuatro frases de la misma
/// declaracion, y separarlas en la firma era lo que hacia que añadir una
/// Pareciera un parametro nuevo en vez de una frase mas.
struct LocalReleaseQuestion<'a> {
    cycle_id: Option<&'a str>,
    previous_tag: Option<&'a str>,
    release_type: Option<sddk_domain::ReleaseType>,
    ask: &'a BuildAsk,
}

fn local_release_preconditions(
    context: &RuntimeContext,
    project_id: &str,
    reference: &NamedReference,
    environment: &CliEnvironment,
    question: &LocalReleaseQuestion<'_>,
) -> anyhow::Result<LocalReleasePreconditions> {
    let LocalReleaseQuestion {
        cycle_id,
        previous_tag,
        release_type: release_type_arg,
        ask,
    } = question;
    // L1 lockstep: la puerta local. `true` significa que la regla no se incumplio,
    // no que la version se comprobara: un proyecto sin version declarada no
    // tiene contra que compararse y no se bloquea por ello.
    let version_lockstep_passed =
        version_lockstep_satisfied_asking(&context.root, &reference.name, &reference.naming, ask);
    let cycle_id = cycle_id.ok_or_else(|| {
        anyhow::anyhow!("--cycle is required for --route local to verify local release evidence")
    })?;
    let cycle = context.storage.get_cycle(cycle_id)?;
    let manifest = cycle.manifest;
    if manifest.project_id != project_id
        || manifest.status != sddk_domain::CycleStatus::ReleasePending
        || manifest.phase != sddk_domain::Phase::Release
    {
        anyhow::bail!("cycle {cycle_id} is not the current release-pending cycle for this project");
    }

    // The release ties the cycle to the local trunk. The cycle MUST point at
    // a branch that is an ancestor of the current local trunk HEAD, and the
    // worktree MUST be on the trunk branch with a clean status. A cycle that
    // points at a different branch fails clearly here instead of silently
    // pushing the wrong commits.
    let trunk_branch = manifest.branch.as_str();
    if trunk_branch != "main" {
        anyhow::bail!(
            "cycle {cycle_id} points at branch {trunk_branch:?}; the local release route requires the cycle to point at the trunk branch main"
        );
    }
    let git = sddk_gateway::GitExecutor::new(context.root.clone());
    let inspect = git.inspect()?;
    if inspect.branch.as_deref() != Some("main") {
        anyhow::bail!(
            "cycle {cycle_id} is tied to trunk main but the worktree is on {}; checkout main before running the local release",
            inspect.branch.as_deref().unwrap_or("detached HEAD")
        );
    }
    if inspect.dirty {
        anyhow::bail!(
            "cycle {cycle_id} cannot release from a dirty worktree; commit or stash the changes first"
        );
    }
    if let Some(cycle_head) = manifest.head.as_deref() {
        let local_head = git.head_sha()?;
        if cycle_head != local_head && !is_ancestor(&git, cycle_head, &local_head)? {
            anyhow::bail!(
                "cycle {cycle_id} points at commit {cycle_head}, which is not an ancestor of the local trunk HEAD {local_head}; the cycle is on a different branch"
            );
        }
    }

    // UAT is not required for paths without a UAT phase (A-min, B-direct).
    let path_requires_uat = !matches!(
        manifest.path,
        sddk_domain::CyclePath::AMin | sddk_domain::CyclePath::BDirect
    );

    let uat_passed = if path_requires_uat {
        // Load UatConfig to evaluate the release gate.
        let config = crate::uat::load_uat_config(project_id, environment)?;
        let release_type = release_type_arg.unwrap_or_else(|| {
            if let Some(prev) = previous_tag {
                sddk_domain::release_type_from_diff(&reference.name, prev)
                    .unwrap_or(sddk_domain::ReleaseType::Major)
            } else {
                // No signal → fail-closed (default to Major, which requires UAT).
                sddk_domain::ReleaseType::Major
            }
        });
        let action = sddk_domain::evaluate_release_gate(&config, release_type);
        if matches!(action, sddk_domain::ReleaseGateAction::Skip) {
            // UAT gate is configured to skip for this release type.
            true
        } else {
            // Consult the gate-receipts table.
            let gates = context.storage.list_gate_receipts(cycle_id)?;
            let passed = |gate: &str| {
                gates.iter().any(|receipt| {
                    receipt.gate == gate && receipt.outcome == GateOutcomeStatus::Passed
                })
            };
            passed("release-uat-approved")
        }
    } else {
        // A-min and B-direct paths have no UAT phase — precondition satisfied.
        true
    };

    let gates = context.storage.list_gate_receipts(cycle_id)?;
    let passed = |gate: &str| {
        gates
            .iter()
            .any(|receipt| receipt.gate == gate && receipt.outcome == GateOutcomeStatus::Passed)
    };

    // REQ-RDI-004: verify release-receipt.json from the plan phase.
    let receipt_path = context.cycle_artifacts_path.join("release-receipt.json");
    let (manifest_exact_set_verified, bundle_roundtrip_verified, release_receipt_verified) =
        if receipt_path.is_file() {
            let receipt_bytes = std::fs::read(&receipt_path)?;
            let receipt: ReleaseReceipt = serde_json::from_slice(&receipt_bytes)?;

            // Canonical signing key location: $SDDK_DATA_DIR/keys/
            let keys_dir = crate::dev::paths::signing_keys_dir(environment)?;
            let key = sddk_engine::load_or_create_key(&keys_dir)?;
            // HMAC payload binds ALL receipt fields.
            let payload = format!(
                "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
                receipt.receipt_id,
                receipt.gate,
                receipt.transition,
                receipt.plan_hash,
                receipt.head_sha,
                receipt.tag,
                receipt.binary_sha256,
                receipt.manifest_sha256,
                receipt.manifest_count,
                receipt.bundle_roundtrip_verified,
            );
            let hmac_ok = sddk_engine::verify_payload(&payload, &receipt.signature, &key);

            // head_sha must match local HEAD.
            let local_head = git.head_sha()?;
            let head_match = receipt.head_sha == local_head;

            // tag must match the planned tag.
            let tag_match = receipt.tag == reference.name;

            // bundle_roundtrip_verified is explicit in the receipt.
            let bundle_ok = receipt.bundle_roundtrip_verified;

            (
                hmac_ok && head_match && tag_match && bundle_ok,
                bundle_ok,
                hmac_ok && head_match && tag_match,
            )
        } else {
            // No release-receipt.json: this is expected for A-min and B-direct
            // cycles that skip `release dist`. The RDI preconditions are not
            // applicable in that case — treat as skipped (true).
            (true, true, true)
        };

    Ok(LocalReleasePreconditions {
        verification_passed: manifest.artifacts.contains_key("verification-report")
            && passed("tests-pass")
            && passed("policy-compliant"),
        uat_passed,
        version_lockstep_passed,
        manifest_exact_set_verified,
        bundle_roundtrip_verified,
        release_receipt_verified,
    })
}

fn is_ancestor(
    git: &sddk_gateway::GitExecutor,
    ancestor: &str,
    descendant: &str,
) -> anyhow::Result<bool> {
    let status = std::process::Command::new("git")
        .arg("-C")
        .arg(git.root())
        .arg("merge-base")
        .arg("--is-ancestor")
        .arg(ancestor)
        .arg(descendant)
        .status()?;
    Ok(status.success())
}

fn release_plan_text(output: &ReleasePlanOutput) -> String {
    let route = match output.route {
        ReleaseRoute::Local => "local",
        ReleaseRoute::Forge => "forge",
    };
    let mut text = format!(
        "route: {route}\nbranch: {}\nbase: {}\ntag: {}\nhead: {}\n",
        output.branch,
        output.base,
        output.tag,
        output.head.as_deref().unwrap_or("null")
    );
    // Qué producto es este plan, y cómo se decidió. Va antes que la autoridad
    // de versión porque es lo primero que hay que saber para leerla: un número
    // sin producto no significa nada.
    text.push_str(&release_target_text(&output.release_target));
    // Y la convencion con la que el tag se leyo. Va pegada al target porque es
    // la otra mitad de la pregunta «que se esta autorizando»: sin ella, un
    // `v0.47.0` y un `0.47.0` son el mismo numero en un plan que no dice cual
    // de los dos es el nombre.
    text.push_str(&format!("release_naming: {}\n", output.release_naming));
    // Y el rol, pegado a la convencion porque los dos son «como se autorizo
    // esto»: uno dice con que regla se leyo el nombre, el otro dice quien se
    // responsabiliza de la decision.
    text.push_str(&format!("release_role: {}\n", output.release_role));
    // La autoridad se imprime antes de los pasos: es la línea que decide si lo
    // que viene después fue comprobado o no tenía nada que comprobarlo.
    text.push_str(&version_authority_text(&output.version_authority));
    text.push_str("steps:\n");
    for step in &output.steps {
        text.push_str(&format!("- {step}\n"));
    }
    text
}

/// Un solo render para los dos comandos: `release plan` y `release apply`
/// declaran la autoridad y sus líneas no pueden divergir, porque salen de la
/// misma función sobre el mismo tipo.
/// Qué producto es este plan, y cómo se decidió.
///
/// Va **antes** de la autoridad de versión porque es lo primero que hay que
/// saber para leerla: un número sin producto no significa nada, y por eso el
/// target se declara aunque solo haya uno — un plan que nombra su producto
/// puede compararse con otro; uno que no lo nombra, no.
fn release_target_text(target: &ResolvedTarget) -> String {
    let mut text = format!(
        "release_target: {}\nrelease_target_root: {}\nrelease_target_candidates: {}\n",
        target.id,
        target.root,
        target.candidates.join(", ")
    );
    text.push_str(&format!(
        "release_target_provenance: {}\n",
        target.provenance
    ));
    if !target.root_resolved && !target.candidates.is_empty() {
        text.push_str(
            "note: este repositorio contiene mas de un producto; el plan nombra uno. \
             Los demas no se han resuelto.\n",
        );
    }
    text
}

fn version_authority_text(authority: &VersionAuthority) -> String {
    use sddk_domain::version_authority::VersionProbe;

    let mut text = match authority {
        VersionAuthority::Resolved {
            version,
            observations,
            ..
        } => {
            let mut text = format!("version_authority: resolved\nversion: {version}\n");
            for observation in observations
                .iter()
                .filter(|o| matches!(o.probe, VersionProbe::Declared { .. }))
            {
                let VersionProbe::Declared { version, evidence } = &observation.probe else {
                    unreachable!("filtrado por Declared")
                };
                text.push_str(&format!(
                    "version_declared_in: {} ({}) = {version}\n",
                    evidence.location.as_deref().unwrap_or("sin localizacion"),
                    observation.provider_id
                ));
            }
            // Una lectura no es una comprobación, y el nombre `resolved` solo
            // no lo dice. Esta línea es la que evita que el plan de un
            // proyecto con un unico manifiesto se lea como el de uno con
            // dos fuentes que coinciden.
            text.push_str("note: one declaration; nothing was cross-checked\n");
            text
        }
        VersionAuthority::CrossValidated { version, .. } => {
            let mut text = format!("version_authority: cross_validated\nversion: {version}\n");
            for observation in observations_of(authority) {
                let VersionProbe::Declared { version, evidence } = &observation.probe else {
                    continue;
                };
                text.push_str(&format!(
                    "version_declared_in: {} ({}) = {version}\n",
                    evidence.location.as_deref().unwrap_or("sin localizacion"),
                    observation.provider_id
                ));
            }
            text.push_str("note: independent sources agree\n");
            text
        }
        VersionAuthority::ReleaseRefIsAuthority { declarations, .. } => {
            let mut text = String::from(
                "version_authority: release_ref_is_authority\nversion: null\n\
                 note: this target declares no product version; the release reference carries \
                 it, and nothing was cross-checked\n",
            );
            for declaration in declarations {
                text.push_str(&format!("version_carried_by_release_ref: {declaration}\n"));
            }
            text
        }
        // Los tres veredictos de fallo no llegan a un render: son `Err` antes
        // de que exista un plan. Se dibujan igual, y no con `unreachable!`,
        // porque un `unreachable!` en la salida de un release es un panic en
        // produccion por una diferencia de versiones entre dos copias del
        // binario —que es exactamente el caso que este bloque no puede
        // descartar.
        VersionAuthority::Ambiguous { candidates, .. } => {
            let mut text = String::from("version_authority: ambiguous\nversion: null\n");
            for (who, version) in candidates {
                text.push_str(&format!("version_conflict: {who} = {version}\n"));
            }
            text
        }
        VersionAuthority::Invalid { failures, .. } => {
            let mut text = String::from("version_authority: invalid\nversion: null\n");
            for (who, why) in failures {
                text.push_str(&format!("version_unreadable: {who} = {why}\n"));
            }
            text
        }
        VersionAuthority::Unresolved { .. } => {
            String::from("version_authority: unresolved\nversion: null\n")
        }
    };
    text.push('\n');
    text
}

fn observations_of(
    authority: &VersionAuthority,
) -> &[sddk_domain::version_authority::VersionObservation] {
    match authority {
        VersionAuthority::Resolved { observations, .. }
        | VersionAuthority::CrossValidated { observations, .. }
        | VersionAuthority::Ambiguous { observations, .. }
        | VersionAuthority::Invalid { observations, .. }
        | VersionAuthority::Unresolved { observations }
        | VersionAuthority::ReleaseRefIsAuthority { observations, .. } => observations,
    }
}

fn release_outcome_text(output: &sddk_gateway::ReleaseOutcome) -> String {
    let mut text = format!(
        "converged: {}\napplied: {}\n",
        output.converged,
        output.applied.len()
    );
    // El resultado de un release dice de dónde salió la versión, igual que el
    // plan. Informar solo `converged` deja al lector sin forma de saber si la
    // versión se comprobó contra algo o si no había nada que comprobar.
    text.push_str(&format!("release_target: {}\n", output.release_target));
    text.push_str(&format!("release_naming: {}\n", output.release_naming));
    text.push_str(&format!("release_role: {}\n", output.release_role));
    text.push_str(&version_authority_text(&output.version_authority));
    for step in &output.applied {
        text.push_str(&format!("- {} {}\n", step.step, step.receipt_id));
    }
    for skip in &output.skipped {
        text.push_str(&format!("- skipped: {skip}\n"));
    }
    text
}

fn local_release_outcome_text(output: &LocalReleaseOutcome) -> String {
    let mut text = format!(
        "converged: {}\nsha: {}\ntag: {}\napplied: {}\n",
        output.converged,
        output.sha,
        output.tag,
        output.applied.len()
    );
    for step in &output.applied {
        text.push_str(&format!("- {} {}\n", step.step, step.receipt_id));
    }
    for skip in &output.skipped {
        text.push_str(&format!("- skipped: {skip}\n"));
    }
    text
}

/// Run the MANIFEST exact-set preflight check, mirroring `dev/install.rs:17-27`.
///
/// When `skip` is true, writes an audit entry and returns `Ok(())`.
/// When `skip` is false, calls `crate::dev::manifest::verify_manifest(root)` and
/// bails with the same error envelope as `dev install` on mismatch.
/// If the manifest file is absent, the check is skipped (no manifest to verify).
fn preflight_manifest(root: &std::path::Path, skip: bool, audit_msg: &str) -> anyhow::Result<()> {
    if skip {
        let ts = OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "unknown".into());
        eprintln!("[{ts}] MANIFEST preflight SKIPPED: {audit_msg}");
        return Ok(());
    }
    // Skip if no manifest is present (e.g., test environments or bare repos).
    let manifest_path = root.join(crate::dev::manifest::MANIFEST_FILE);
    if !manifest_path.is_file() {
        return Ok(());
    }
    let mismatches = crate::dev::verify_manifest(root)?;
    if !mismatches.is_empty() {
        anyhow::bail!(
            "manifest verification FAILED ({} mismatch(es)):\n  {}",
            mismatches.len(),
            mismatches.join("\n  ")
        );
    }
    Ok(())
}

/// Release receipt written by `release plan` after preflight passes.
/// Stored at `{cycle_artifacts_dir}/release-receipt.json`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct ReleaseReceipt {
    pub(crate) receipt_id: String,
    pub(crate) gate: String,
    pub(crate) transition: String,
    pub(crate) plan_hash: String,
    pub(crate) head_sha: String,
    pub(crate) tag: String,
    pub(crate) binary_sha256: String,
    pub(crate) manifest_sha256: String,
    pub(crate) manifest_count: usize,
    pub(crate) manifest_surfaces: Vec<String>,
    /// Explicit flag: whether the staged bundle roundtrip was verified.
    /// No longer inferred from non-empty manifest_sha256.
    pub(crate) bundle_roundtrip_verified: bool,
    pub(crate) channel: String,
    pub(crate) timestamp: String,
    #[serde(default)]
    pub(crate) signature: String,
}

/// Write `release-receipt.json` signed with the local gate-signing key.
///
/// HMAC payload binds ALL receipt fields so any tampering is detected:
/// `receipt_id|gate|transition|plan_hash|head_sha|tag|binary_sha256|manifest_sha256|manifest_count|bundle_roundtrip_verified`
fn write_release_receipt(
    receipt: ReleaseReceipt,
    cycle_artifacts_dir: &std::path::Path,
    environment: &CliEnvironment,
) -> anyhow::Result<()> {
    let receipt_path = cycle_artifacts_dir.join("release-receipt.json");
    // Canonical signing key location: $SDDK_DATA_DIR/keys/
    let keys_dir = crate::dev::paths::signing_keys_dir(environment)?;
    let key = sddk_engine::load_or_create_key(&keys_dir)?;
    let payload = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        receipt.receipt_id,
        receipt.gate,
        receipt.transition,
        receipt.plan_hash,
        receipt.head_sha,
        receipt.tag,
        receipt.binary_sha256,
        receipt.manifest_sha256,
        receipt.manifest_count,
        receipt.bundle_roundtrip_verified,
    );
    let signature = sddk_engine::sign_payload(&payload, &key)?;

    let signed_receipt = ReleaseReceipt {
        signature,
        ..receipt
    };
    crate::dev::atomic_write(
        &receipt_path,
        serde_json::to_string_pretty(&signed_receipt)?.as_bytes(),
        None,
    )?;
    Ok(())
}

/// Vault receipt for managed-closure cycles (REQ-DKA-002).
/// Stored at `{cycle_artifacts_dir}/vault-receipt.json`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct VaultReceipt {
    pub(crate) receipt_id: String,
    pub(crate) gate: String,
    pub(crate) transition: String,
    pub(crate) cycle_id: String,
    pub(crate) delivery_kind: String,
    pub(crate) content_hash: String,
    pub(crate) timestamp: String,
    #[serde(default)]
    pub(crate) signature: String,
}

/// Write `vault-receipt.json` signed with the local gate-signing key.
fn write_vault_receipt(
    receipt: VaultReceipt,
    cycle_artifacts_dir: &std::path::Path,
    environment: &CliEnvironment,
) -> anyhow::Result<()> {
    let receipt_path = cycle_artifacts_dir.join("vault-receipt.json");
    // Canonical signing key location: $SDDK_DATA_DIR/keys/
    let keys_dir = crate::dev::paths::signing_keys_dir(environment)?;
    let key = sddk_engine::load_or_create_key(&keys_dir)?;
    let payload = format!(
        "{}|{}|{}|{}|{}|{}|{}",
        receipt.receipt_id,
        receipt.gate,
        receipt.transition,
        receipt.cycle_id,
        receipt.delivery_kind,
        receipt.content_hash,
        receipt.timestamp,
    );
    let signature = sddk_engine::sign_payload(&payload, &key)?;

    let signed_receipt = VaultReceipt {
        signature,
        ..receipt
    };
    crate::dev::atomic_write(
        &receipt_path,
        serde_json::to_string_pretty(&signed_receipt)?.as_bytes(),
        None,
    )?;
    Ok(())
}

// ── Release Revalidation ──────────────────────────────────────────────────────

use sddk_domain::{CycleStatus, FreshEvidence, Phase, ReleaseRevalidation, RevalidationCheck};

/// Output of the `release revalidate` command.
#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct RevalidateOutput {
    /// Whether revalidation succeeded.
    success: bool,
    /// Original SHA that was previously verified.
    original_sha: String,
    /// Candidate SHA (current HEAD).
    candidate_sha: String,
    /// Number of checks performed.
    checks_performed: usize,
    /// Number of checks that passed.
    checks_passed: usize,
    /// Revalidation artifact path.
    artifact_path: String,
    /// SHA-256 of the artifact.
    artifact_sha256: String,
}

/// Run the `release revalidate` command.
///
/// Produces an append-only `release-revalidation.json` artifact that binds the
/// candidate SHA (current HEAD) to fresh verify/debt evidence.
///
/// Safety invariants enforced:
/// - Only RELEASE_PENDING/release cycles can enter recovery
/// - Candidate SHA must equal current HEAD
/// - Fresh verify/debt evidence recorded with argv/exit/output digest
/// - Revalidation is idempotent
/// - Original reports/receipts remain immutable
/// - Failed revalidation blocks publication
pub(crate) fn run_release_revalidate(
    args: RevalidateArgs,
    environment: &CliEnvironment,
) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<RevalidateOutput> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let cycle = context.storage.get_cycle(&args.cycle)?;

        // SAFETY INVARIANT: Only RELEASE_PENDING/release cycles can enter recovery
        if cycle.manifest.status != CycleStatus::ReleasePending {
            anyhow::bail!(
                "cycle {} is not RELEASE_PENDING (status={:?}); \
                 release revalidation is only available for RELEASE_PENDING cycles",
                args.cycle,
                cycle.manifest.status
            );
        }
        if cycle.manifest.phase != Phase::Release {
            anyhow::bail!(
                "cycle {} is not in release phase (phase={:?}); \
                 release revalidation is only available in release phase",
                args.cycle,
                cycle.manifest.phase
            );
        }

        let git = GitExecutor::new(context.root.clone());
        let current_head = git.head_sha()?;

        // Candidate SHA must equal current HEAD (candidate IS the current HEAD)
        let candidate_sha = current_head;
        let original_sha = args.original_sha.as_str();

        // SAFETY INVARIANT: original_sha must be a CORRECTION (different from candidate)
        // A correction revalidation occurs when HEAD moved after original verification.
        if original_sha == candidate_sha {
            anyhow::bail!(
                "original_sha ({}) equals current HEAD ({}) — no correction detected; \
                 original_sha must be the SHA verified BEFORE the correction commit",
                original_sha,
                candidate_sha
            );
        }

        // SAFETY INVARIANT: original_sha must be a strict ancestor of candidate_sha
        // (the correction commit is a descendant of the originally-verified commit)
        if !is_ancestor(&git, original_sha, &candidate_sha)? {
            anyhow::bail!(
                "original_sha ({}) is not an ancestor of current HEAD ({}); \
                 original_sha must be a prior verified commit and candidate must be its descendant",
                original_sha,
                candidate_sha
            );
        }

        // PATH POLICY: A-full/A-min/A-lite require BOTH verify AND debt-verify
        // (debt verification is mandatory for these paths; no skips allowed)
        // B-direct may skip debt-verify per workflow policy
        let path_requires_debt = !matches!(cycle.manifest.path, sddk_domain::CyclePath::BDirect);

        if path_requires_debt && args.skip_debt {
            anyhow::bail!(
                "--skip-debt is not allowed for path {:?}; \
                 debt-verify is mandatory for A-min/A-lite/A-full paths",
                cycle.manifest.path
            );
        }
        if args.skip_verify {
            anyhow::bail!(
                "--skip-verify is not allowed; \
                 verify is mandatory for all release paths"
            );
        }

        let timestamp = args
            .timestamp
            .clone()
            .unwrap_or_else(crate::git_cmd::default_timestamp);
        let actor = args
            .actor
            .clone()
            .or_else(|| environment.sddk_actor.clone())
            .or_else(|| environment.user.clone())
            .unwrap_or_else(|| "sddk-cli".into());

        // Build the revalidation record with candidate=current HEAD and original=previously-verified SHA
        let mut revalidation = ReleaseRevalidation::new(
            args.cycle.clone(),
            context.identity.project_id.to_string(),
            original_sha.to_string(),
            candidate_sha.to_string(),
            "release.complete".to_string(),
            actor,
            timestamp,
        );

        // Run verify check (mandatory for all paths)
        let verify_check = run_verify_check(&git)?;
        revalidation.add_check(verify_check);

        // Run debt-verify check (mandatory for non-B-direct paths)
        if !args.skip_debt {
            let debt_check = run_debt_check(&git)?;
            revalidation.add_check(debt_check);
        }

        // Use candidate-specific filename to ensure append-only behavior.
        // Each candidate SHA gets its own artifact; no artifact is ever overwritten.
        let candidate_short: String = candidate_sha.chars().take(8).collect();
        let artifact_path = context
            .cycle_artifacts_path
            .join(format!("release-revalidation-{}.json", candidate_short));

        // Idempotency: if an artifact already exists for this exact candidate with same checks,
        // return it without modification (preserves immutability of original reports).
        if artifact_path.is_file() {
            let existing: ReleaseRevalidation =
                serde_json::from_str(&std::fs::read_to_string(&artifact_path)?)?;
            if existing.candidate_sha() == revalidation.candidate_sha()
                && existing.checks.len() == revalidation.checks.len()
            {
                // Verify check names and outcomes match exactly for semantic idempotency
                let names_and_outcomes_match = revalidation
                    .checks
                    .iter()
                    .zip(existing.checks.iter())
                    .all(|(a, b)| a.check_name == b.check_name && a.passed == b.passed);
                if names_and_outcomes_match {
                    let artifact_sha256 = crate::dev::sha256_hex(&artifact_path)?;
                    return Ok(RevalidateOutput {
                        success: existing.all_passed(),
                        original_sha: existing.original_sha().to_string(),
                        candidate_sha: existing.candidate_sha().to_string(),
                        checks_performed: existing.checks.len(),
                        checks_passed: existing.checks.iter().filter(|c| c.passed).count(),
                        artifact_path: artifact_path.to_string_lossy().into_owned(),
                        artifact_sha256,
                    });
                }
            }
            // Artifact exists for same candidate but with different checks — conflict
            anyhow::bail!(
                "artifact conflict: {} exists with different checks for candidate {}; \
                 refusing to overwrite (append-only, candidate-specific artifact)",
                artifact_path.display(),
                candidate_sha
            );
        }

        // Write the new artifact atomically (append-only, candidate-specific)
        let json = serde_json::to_string_pretty(&revalidation)?;
        crate::dev::atomic_write(&artifact_path, json.as_bytes(), None)?;

        // Compute SHA-256 of the artifact for the report
        let artifact_sha256 = crate::dev::sha256_hex(&artifact_path)?;

        Ok(RevalidateOutput {
            success: revalidation.all_passed(),
            original_sha: revalidation.original_sha().to_string(),
            candidate_sha: revalidation.candidate_sha().to_string(),
            checks_performed: revalidation.checks.len(),
            checks_passed: revalidation.checks.iter().filter(|c| c.passed).count(),
            artifact_path: artifact_path.to_string_lossy().into_owned(),
            artifact_sha256,
        })
    })();

    match result {
        Ok(output) => {
            let mut cmd = render_result(Ok(output.clone()), format, revalidate_text);
            if !output.success {
                cmd.status = 1; // Failed revalidation blocks publication
            }
            cmd
        }
        Err(error) => crate::failure(error.to_string()),
    }
}

/// Output of the `release vault` command.
#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct VaultOutput {
    /// Whether the vault receipt was emitted successfully.
    success: bool,
    /// Cycle identifier.
    cycle_id: String,
    /// Delivery kind declared in the cycle.
    delivery_kind: String,
    /// Vault receipt path.
    artifact_path: String,
    /// SHA-256 of the artifact.
    artifact_sha256: String,
}

/// Run the `release vault` command for managed-closure cycles (REQ-DKA-002).
///
/// Emits `vault-receipt.json` at `{cycle_artifacts_dir}/vault-receipt.json`.
/// Refuses if:
/// - delivery_kind != ManagedClosureDelivery
/// - release-receipt.json already exists (REQ-DKA-005-S3)
/// - cycle status != BLOCKED (REQ-DKA-005-S2)
pub(crate) fn run_release_vault(args: VaultArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<VaultOutput> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let cycle = context.storage.get_cycle(&args.cycle)?;

        // SAFETY INVARIANT: Only BLOCKED cycles can enter the vault route
        if cycle.manifest.status != CycleStatus::Blocked {
            anyhow::bail!(
                "cycle {} is not BLOCKED (status={:?}); \
                 archive.vault.complete is only available for BLOCKED cycles",
                args.cycle,
                cycle.manifest.status
            );
        }

        // SAFETY INVARIANT: release-receipt.json must NOT exist
        let receipt_path = context.cycle_artifacts_path.join("release-receipt.json");
        if receipt_path.is_file() {
            anyhow::bail!(
                "release-receipt.json already exists at {}; \
                 managed-closure cycles cannot have a release receipt",
                receipt_path.display()
            );
        }

        // SAFETY INVARIANT: delivery_kind must be ManagedClosureDelivery
        use sddk_domain::delivery_kind::DeliveryKind;
        let delivery_kind_str = match cycle.manifest.delivery_kind {
            Some(DeliveryKind::ManagedClosureDelivery) => "managed-closure-delivery",
            Some(dk) => {
                anyhow::bail!(
                    "cycle {} has delivery_kind={:?}; vault route requires ManagedClosureDelivery",
                    args.cycle,
                    dk
                );
            }
            None => {
                anyhow::bail!(
                    "cycle {} has no delivery_kind declared; vault route requires ManagedClosureDelivery",
                    args.cycle
                );
            }
        };

        let timestamp = args.timestamp.unwrap_or_else(|| {
            time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap()
        });
        let actor = args.actor.unwrap_or_else(|| "sddk.vault".to_string());

        // Generate deterministic receipt_id from string content
        use sha2::{Digest, Sha256};
        let receipt_id = format!(
            "{:x}",
            Sha256::digest(
                format!(
                    "vault|{}|{}|{}|{}",
                    args.cycle, delivery_kind_str, timestamp, actor
                )
                .as_bytes()
            )
        );

        let git = GitExecutor::new(context.root.clone());
        let head_sha = git.head_sha()?;

        let receipt = VaultReceipt {
            receipt_id: receipt_id.clone(),
            gate: "archive.vault.complete".to_string(),
            transition: "archive.vault.complete".to_string(),
            cycle_id: args.cycle.clone(),
            delivery_kind: delivery_kind_str.to_string(),
            content_hash: head_sha,
            timestamp: timestamp.clone(),
            signature: String::new(), // Will be filled by write_vault_receipt
        };

        write_vault_receipt(receipt, &context.cycle_artifacts_path, environment)?;

        // Compute SHA-256 of the artifact
        let vault_receipt_path = context.cycle_artifacts_path.join("vault-receipt.json");
        let artifact_sha256 = crate::dev::sha256_hex(&vault_receipt_path)?;

        Ok(VaultOutput {
            success: true,
            cycle_id: args.cycle.clone(),
            delivery_kind: delivery_kind_str.to_string(),
            artifact_path: vault_receipt_path.to_string_lossy().into_owned(),
            artifact_sha256,
        })
    })();

    match result {
        Ok(output) => render_result(Ok(output.clone()), format, vault_text),
        Err(error) => crate::failure(error.to_string()),
    }
}

/// Run the verify check and capture fresh evidence.
///
/// Uses the project's canonical passing local test command:
/// `cargo test --workspace --all-targets --locked`
/// NOTE: `--release` is NOT used because `target/debug/sddk` (needed by dev::rdi_tests)
/// does not exist in release mode, causing the test to fail.
pub(crate) fn run_verify_check(git: &GitExecutor) -> anyhow::Result<RevalidationCheck> {
    let verify_argv = vec![
        "cargo".into(),
        "test".into(),
        "--workspace".into(),
        "--all-targets".into(),
        "--locked".into(),
    ];

    let output = std::process::Command::new(&verify_argv[0])
        .args(&verify_argv[1..])
        .current_dir(git.root())
        .output()?;

    let exit_code = output.status.code().unwrap_or(-1);
    let output_digest = format!("sha256:{:x}", Sha256::digest(&output.stdout));

    Ok(RevalidationCheck {
        check_name: "verify".to_string(),
        passed: exit_code == 0,
        evidence: Some(FreshEvidence {
            argv: verify_argv,
            exit_code,
            output_digest,
        }),
    })
}

/// Run the debt-verify check and capture fresh evidence.
pub(crate) fn run_debt_check(git: &GitExecutor) -> anyhow::Result<RevalidationCheck> {
    let debt_argv = vec![
        "cargo".into(),
        "clippy".into(),
        "--workspace".into(),
        "--all-targets".into(),
        "--locked".into(),
        "--".into(),
        "-D".into(),
        "warnings".into(),
    ];

    let output = std::process::Command::new(&debt_argv[0])
        .args(&debt_argv[1..])
        .current_dir(git.root())
        .output()?;

    let exit_code = output.status.code().unwrap_or(-1);
    let output_digest = format!("sha256:{:x}", Sha256::digest(&output.stdout));

    Ok(RevalidationCheck {
        check_name: "debt-verify".to_string(),
        passed: exit_code == 0,
        evidence: Some(FreshEvidence {
            argv: debt_argv,
            exit_code,
            output_digest,
        }),
    })
}

fn revalidate_text(output: &RevalidateOutput) -> String {
    format!(
        "success: {}\noriginal_sha: {}\ncandidate_sha: {}\nchecks_performed: {}\nchecks_passed: {}\nartifact_path: {}\nartifact_sha256: {}\n",
        output.success,
        output.original_sha,
        output.candidate_sha,
        output.checks_performed,
        output.checks_passed,
        output.artifact_path,
        output.artifact_sha256
    )
}

fn vault_text(output: &VaultOutput) -> String {
    format!(
        "success: {}\ncycle_id: {}\ndelivery_kind: {}\nartifact_path: {}\nartifact_sha256: {}\n",
        output.success,
        output.cycle_id,
        output.delivery_kind,
        output.artifact_path,
        output.artifact_sha256
    )
}

#[cfg(test)]
mod tests {
    use sddk_domain::{GateOutcomeStatus, GateReceipt};

    /// Verifies the exact logic used by release_cmd's private `passed` closures
    /// (lines ~536-539 and ~549-552): a receipt satisfies a release gate only
    /// when outcome == Passed.  Waived does NOT satisfy release gates.
    #[test]
    fn release_gate_requires_passed_not_waived() {
        // Replicate the passed() closure from release_preconditions:
        //   let passed = |gate: &str| {
        //       gates.iter().any(|receipt| {
        //           receipt.gate == gate && receipt.outcome == GateOutcomeStatus::Passed
        //       })
        //   };
        let passed = |gates: &[GateReceipt], gate_name: &str| {
            gates
                .iter()
                .any(|r| r.gate == gate_name && r.outcome == GateOutcomeStatus::Passed)
        };

        let receipt_waived = GateReceipt {
            receipt_id: "rcpt-waived".into(),
            project_id: "p".into(),
            cycle_id: Some("c".into()),
            gate: "tests-pass".into(),
            evaluator: "eval".into(),
            transition_id: "t".into(),
            plan_hash: "h".into(),
            outcome: GateOutcomeStatus::Waived,
            evidence: serde_json::json!({}),
            actor: "test".into(),
            actor_ref: None,
            command_id: "cmd".into(),
            frame_id: "frame".into(),
            evaluated_at: "2026-08-03T12:00:00Z".into(),
            seq: 1,
            causation_id: None,
            correlation_id: None,
        };
        let receipt_passed = GateReceipt {
            receipt_id: "rcpt-passed".into(),
            project_id: "p".into(),
            cycle_id: Some("c".into()),
            gate: "tests-pass".into(),
            evaluator: "eval".into(),
            transition_id: "t".into(),
            plan_hash: "h".into(),
            outcome: GateOutcomeStatus::Passed,
            evidence: serde_json::json!({}),
            actor: "test".into(),
            actor_ref: None,
            command_id: "cmd".into(),
            frame_id: "frame".into(),
            evaluated_at: "2026-08-03T12:00:00Z".into(),
            seq: 2,
            causation_id: None,
            correlation_id: None,
        };

        // Waived receipt does NOT satisfy the release gate
        assert!(
            !passed(std::slice::from_ref(&receipt_waived), "tests-pass"),
            "Waived receipt must NOT satisfy release gate (fails-closed)"
        );
        // Passed receipt DOES satisfy the release gate
        assert!(
            passed(std::slice::from_ref(&receipt_passed), "tests-pass"),
            "Passed receipt must satisfy release gate"
        );
        // Only Passed matters; Waived alongside Passed still passes
        assert!(
            passed(&[receipt_waived, receipt_passed], "tests-pass"),
            "Passed receipt among Waived must still satisfy release gate"
        );
    }

    use super::{
        ReleasePlanOutput, ReleaseRoute, ResolvedTarget, VersionAuthority, release_plan_text,
        release_target_text,
    };
    use sddk_domain::release_ref::VersionNaming;
    use sddk_domain::release_role::ReleaseRole;
    use sddk_domain::version_authority::{
        ProductVersion, VersionEvidence, VersionObservation, VersionProbe,
    };

    fn declaring(provider: &str, path: &str, version: &str) -> VersionObservation {
        VersionObservation {
            provider_id: provider.to_owned(),
            provider_version: "test".to_owned(),
            capability: "product-version.observation/v1".to_owned(),
            probe: VersionProbe::Declared {
                version: ProductVersion::new(version).expect("version valida"),
                evidence: VersionEvidence {
                    source_kind: "test".to_owned(),
                    digest: None,
                    location: Some(path.to_owned()),
                },
            },
        }
    }

    /// Una sola declaracion. Antes este caso se llamaba `CrossChecked` y esa
    /// era la mentira que este bloque quita: no hubo nada que cruzar.
    fn resolved() -> VersionAuthority {
        VersionAuthority::Resolved {
            version: ProductVersion::new("1.0.0").expect("version valida"),
            observations: vec![
                declaring(
                    "sddk.gateway.declaration-file/Cargo.toml",
                    "Cargo.toml",
                    "1.0.0",
                ),
                VersionObservation {
                    provider_id: "sddk.gateway.declaration-file/package.json".to_owned(),
                    provider_version: "test".to_owned(),
                    capability: "product-version.observation/v1".to_owned(),
                    probe: VersionProbe::NotApplicable {
                        reason: "package.json no esta en este target".to_owned(),
                    },
                },
            ],
        }
    }

    /// Dos fuentes independientes que dicen lo mismo.
    fn cross_validated() -> VersionAuthority {
        VersionAuthority::CrossValidated {
            version: ProductVersion::new("1.0.0").expect("version valida"),
            observations: vec![
                declaring(
                    "sddk.gateway.declaration-file/Cargo.toml",
                    "Cargo.toml",
                    "1.0.0",
                ),
                declaring(
                    "sddk.gateway.declaration-file/package.json",
                    "package.json",
                    "1.0.0",
                ),
            ],
        }
    }

    /// Un target que declara que su version la lleva la release ref.
    fn release_ref_only() -> VersionAuthority {
        VersionAuthority::ReleaseRefIsAuthority {
            declarations: vec!["go no declara version de producto".to_owned()],
            observations: vec![VersionObservation {
                provider_id: "sddk.gateway.declaration-file/go.mod".to_owned(),
                provider_version: "test".to_owned(),
                capability: "product-version.observation/v1".to_owned(),
                probe: VersionProbe::ReleaseRefIsAuthority {
                    declared_by: "go no declara version de producto".to_owned(),
                },
            }],
        }
    }

    /// Un target, que es el caso normal: el repositorio es el producto.
    fn single_target() -> ResolvedTarget {
        ResolvedTarget {
            id: ".".to_owned(),
            root: "/repo".to_owned(),
            root_resolved: true,
            provenance: "la raiz del repositorio declara su propia version".to_owned(),
            candidates: vec![".".to_owned()],
        }
    }

    /// Varios productos, con el plan nombrando uno: el caso de un monorepo.
    fn one_of_many() -> ResolvedTarget {
        ResolvedTarget {
            id: "packages/runtime".to_owned(),
            root: "/repo/packages/runtime".to_owned(),
            root_resolved: false,
            provenance: "la raiz no declara version; se buscaron 2 raiz/raices hasta 2 \
                         nivel(es), omitiendo nada"
                .to_owned(),
            candidates: vec!["packages/api".to_owned(), "packages/runtime".to_owned()],
        }
    }

    fn plan(authority: VersionAuthority) -> ReleasePlanOutput {
        ReleasePlanOutput {
            route: ReleaseRoute::Local,
            branch: "main".to_string(),
            base: "main".to_string(),
            tag: "v1.0.0".to_string(),
            head: Some("abc1234".to_string()),
            steps: vec!["push_main"],
            version_authority: authority,
            release_target: single_target(),
            release_naming: VersionNaming::v_prefixed().style().to_owned(),
            release_role: ReleaseRole::FullPublisher.name().to_owned(),
        }
    }

    /// Un proyecto que declara version dice contra que se leyo, y ADMITE que
    /// no se cruzo con nadie. Antes esta linea decia `cross_checked` sobre un
    /// unico manifiesto, y el nombre era el problema entero: una lectura se
    /// presentava como una comprobacion.
    #[test]
    fn release_plan_text_declares_where_the_version_came_from() {
        let text = release_plan_text(&plan(resolved()));
        assert!(text.contains("version_authority: resolved"), "{text}");
        assert!(text.contains("version: 1.0.0"), "{text}");
        assert!(
            text.contains(
                "version_declared_in: Cargo.toml (sddk.gateway.declaration-file/Cargo.toml) = 1.0.0"
            ),
            "el plan tiene que nombrar el fichero que se leyo Y quien lo leyó: {text}"
        );
        assert!(
            text.contains("nothing was cross-checked"),
            "una lectura tiene que decir que no se cruzo con nadie: {text}"
        );
    }

    /// Dos fuentes que coinciden SI se cruzan, y el texto lo distingue de la
    /// lectura unica. Si estas dos lineas se parecieran, el nombre
    /// `cross_validated` seria una palabra.
    #[test]
    fn release_plan_text_distinguishes_a_check_from_a_read() {
        let text = release_plan_text(&plan(cross_validated()));
        assert!(
            text.contains("version_authority: cross_validated"),
            "{text}"
        );
        assert!(text.contains("independent sources agree"), "{text}");
        assert!(
            !text.contains("nothing was cross-checked"),
            "una comprobacion cruzada no puede decir que no se cruzo: {text}"
        );
    }

    /// Un target que declara que su version la lleva la release ref no esta
    /// roto y no esta comprobado, y el texto dice las dos cosas.
    #[test]
    fn release_plan_text_admits_when_nothing_declares_a_version() {
        let text = release_plan_text(&plan(release_ref_only()));
        assert!(
            text.contains("version_authority: release_ref_is_authority"),
            "{text}"
        );
        assert!(
            text.contains("nothing was cross-checked"),
            "el texto tiene que decir que no hubo comprobacion: {text}"
        );
        assert!(text.contains("version: null"), "{text}");
        assert!(
            text.contains("version_carried_by_release_ref: go no declara version de producto"),
            "el texto tiene que decir POR QUE no hay version: {text}"
        );
        assert!(
            !text.contains("cross_validated"),
            "un target sin version de producto no puede aparecer como comprobado: {text}"
        );
    }

    /// El JSON y el texto no pueden divergir porque salen del mismo tipo.
    /// El riesgo declarado en el PRE-FLIGHT, verificado sobre la forma
    /// serializada.
    #[test]
    fn the_serialized_authority_matches_the_rendered_one() {
        let json = serde_json::to_value(plan(resolved())).unwrap();
        assert_eq!(json["version_authority"]["verdict"], "resolved");
        assert_eq!(json["version_authority"]["version"], "1.0.0");
        let observation = &json["version_authority"]["observations"][0];
        assert_eq!(observation["probe"]["result"], "declared");
        assert_eq!(observation["probe"]["evidence"]["location"], "Cargo.toml");
        assert_eq!(observation["probe"]["version"], "1.0.0");

        let checked = serde_json::to_value(plan(cross_validated())).unwrap();
        assert_eq!(checked["version_authority"]["verdict"], "cross_validated");

        let ref_json = serde_json::to_value(plan(release_ref_only())).unwrap();
        assert_eq!(
            ref_json["version_authority"]["verdict"],
            "release_ref_is_authority"
        );
        assert!(
            ref_json["version_authority"]["version"].is_null(),
            "sin version de producto no hay version que reportar: {ref_json}"
        );
        assert_eq!(
            ref_json["version_authority"]["declarations"][0],
            "go no declara version de producto"
        );
    }

    /// Los campos existentes no se mueven: el campo nuevo es aditivo, y un
    /// consumidor que lee `tag` o `steps` tiene que seguir viendo lo mismo.
    #[test]
    fn the_plan_keeps_its_previous_fields() {
        let json = serde_json::to_value(plan(resolved())).unwrap();
        assert_eq!(json["route"], "local");
        assert_eq!(json["branch"], "main");
        assert_eq!(json["base"], "main");
        assert_eq!(json["tag"], "v1.0.0");
        assert_eq!(json["head"], "abc1234");
        assert_eq!(json["steps"][0], "push_main");
    }

    /// Fixtures reales, no una estructura imitada. Los dos caminos que se
    /// comprueban aqui (la autoridad que se registra y la puerta que decide)
    /// ya habian pasado con proyectos de prueba que no son de Rust, y un doble
    /// habria pasado igual: el punto es que el contrato se cumpla en un
    /// directorio de verdad, no en una estructura que lo imita.
    fn go_project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module example.com/f\n\ngo 1.22\n",
        )
        .unwrap();
        dir
    }

    fn rust_project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        dir
    }

    /// Un proyecto Rust resuelve su version por el manifest que se leyo, y
    /// dice que NO se cruzo con nadie. Un unico manifiesto es una lectura.
    #[test]
    fn a_rust_project_resolves_its_version_without_claiming_a_check() {
        let dir = rust_project();
        let authority = super::version_authority_or_fail(
            dir.path(),
            "v1.0.0",
            &VersionNaming::v_prefixed(),
            &super::BuildAsk::of_parts(&super::BuildAskArgs {
                evaluate_build: false,
                build_tool: None,
            })
            .expect("sin preguntar no falla"),
        )
        .unwrap();
        assert!(
            matches!(authority, VersionAuthority::Resolved { .. }),
            "{authority:?}"
        );
        assert_eq!(
            authority.version().map(|v| v.to_string()),
            Some("1.0.0".into())
        );
        assert!(
            !authority.was_cross_validated(),
            "un solo manifiesto no se cruza consigo mismo: {authority:?}"
        );
    }

    /// Y dos target que si declaren se cruzan de verdad. La mitad que de esto
    /// no se podia probar antes es que la CRUZ requires dos fuentes
    /// INDEPENDIENTES, no dos ficheros en el mismo repositorio.
    #[test]
    fn dos_fuentes_que_coinciden_se_cruzan_de_verdad() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"name":"x","version":"1.0.0"}"#,
        )
        .unwrap();
        let authority = super::version_authority_or_fail(
            dir.path(),
            "v1.0.0",
            &VersionNaming::v_prefixed(),
            &super::BuildAsk::of_parts(&super::BuildAskArgs {
                evaluate_build: false,
                build_tool: None,
            })
            .expect("sin preguntar no falla"),
        )
        .unwrap();
        assert!(
            authority.was_cross_validated(),
            "dos fuentes independientes que coinciden se han cruzado: {authority:?}"
        );
    }

    /// Un proyecto Go **no** puede presentarse como comprobado, y este test
    /// existe porque el falsificador del lote encontro que nadie lo cubria: el
    /// call site de la ruta forge podia informar un lockstep que no se habia
    /// comprobado sin que la suite se enterara. Esa ruta ya no esta fuera del
    /// alcance de la suite -- `apply_release_forge` recibe `&mut dyn Forge` y R2
    /// la ejecuta con un doble -- pero este test sigue aqui porque el defecto
    /// que lo motivo era del comando de al lado, no de la ruta forge.
    #[test]
    fn a_go_project_declares_that_the_release_ref_carries_its_version() {
        let dir = go_project();
        let authority = super::version_authority_or_fail(
            dir.path(),
            "v1.0.0",
            &VersionNaming::v_prefixed(),
            &super::BuildAsk::of_parts(&super::BuildAskArgs {
                evaluate_build: false,
                build_tool: None,
            })
            .expect("sin preguntar no falla"),
        )
        .unwrap();
        assert!(
            matches!(authority, VersionAuthority::ReleaseRefIsAuthority { .. }),
            "un go.mod presente DECLARA que su version no esta en un manifiesto, \
             que no es lo mismo que no declarar nada: {authority:?}"
        );
        assert!(
            !authority.was_cross_validated(),
            "sin manifest no hubo comparacion, y no puede decir lo contrario"
        );
        assert!(
            !authority.is_failure(),
            "y no esta roto: declaro su convencion y eso es una respuesta: {authority:?}"
        );
    }

    /// El caso del que salio todo, medido sobre ficheros reales: una
    /// configuracion auxiliar que no declara version, al lado de otra que si.
    /// Antes abortaba la resolucion entera.
    #[test]
    fn una_configuracion_que_no_declara_no_impide_publicar() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("gradle.properties"),
            "org.gradle.caching=true\nkotlin.code.style=official\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"name":"x","version":"1.0.0"}"#,
        )
        .unwrap();
        let authority = super::version_authority_or_fail(
            dir.path(),
            "v1.0.0",
            &VersionNaming::v_prefixed(),
            &super::BuildAsk::of_parts(&super::BuildAskArgs {
                evaluate_build: false,
                build_tool: None,
            })
            .expect("sin preguntar no falla"),
        )
        .unwrap();
        assert_eq!(
            authority.version().map(|v| v.to_string()),
            Some("1.0.0".into()),
            "un fichero legitimo que no declara version no puede impedir publicar: {authority:?}"
        );
    }

    /// Y un repositorio en el que no declara NADIE sigue fallando cerrado. La
    /// puerta no se abre por ser permisiva con los ficheros: se abre solo
    /// cuando alguien ha declarado.
    #[test]
    fn un_repositorio_sin_declarar_ninguna_version_sigue_cerrado() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("README.md"), "nada que declarar\n").unwrap();
        assert!(
            super::version_authority_or_fail(
                dir.path(),
                "v1.0.0",
                &VersionNaming::v_prefixed(),
                &super::BuildAsk::of_parts(&super::BuildAskArgs {
                    evaluate_build: false,
                    build_tool: None
                })
                .expect("sin preguntar no falla"),
            )
            .is_err(),
            "silencio no es una declaracion de convencion"
        );
    }

    /// La puerta local **deja pasar** a un proyecto sin versión declarada. Este
    /// es el criterio que el falsificador vio sobrevivir a una mutacion que
    /// la convertia en `was_cross_checked()`: habria dejado a Go y a Bazel sin
    /// poder publicar jamas, y ningun test de la suite lo notaba porque todos
    /// sus fixtures de la ruta local son de Rust.
    #[test]
    fn the_local_gate_lets_a_go_project_through() {
        let dir = go_project();
        assert!(
            super::version_lockstep_satisfied_asking(
                dir.path(),
                "v1.0.0",
                &VersionNaming::v_prefixed(),
                &super::BuildAsk::of_parts(&super::BuildAskArgs {
                    evaluate_build: false,
                    build_tool: None
                })
                .expect("sin preguntar no falla"),
            ),
            "un proyecto sin version declarada no tiene contra que comparar, y eso no es una infraccion"
        );
    }

    /// Y sigue cerrandose cuando la hay y no cuadra. La puerta tiene que
    /// seguir siendo una puerta: `false` aqui aborta el release.
    #[test]
    fn the_local_gate_still_refuses_a_mismatch() {
        let dir = rust_project();
        assert!(super::version_lockstep_satisfied_asking(
            dir.path(),
            "v1.0.0",
            &VersionNaming::v_prefixed(),
            &super::BuildAsk::of_parts(&super::BuildAskArgs {
                evaluate_build: false,
                build_tool: None
            })
            .expect("sin preguntar no falla"),
        ));
        assert!(
            !super::version_lockstep_satisfied_asking(
                dir.path(),
                "v9.9.9",
                &VersionNaming::v_prefixed(),
                &super::BuildAsk::of_parts(&super::BuildAskArgs {
                    evaluate_build: false,
                    build_tool: None
                })
                .expect("sin preguntar no falla"),
            ),
            "un tag que no coincide con la version declarada tiene que cerrar la puerta"
        );
        assert!(
            super::version_authority_or_fail(
                dir.path(),
                "v9.9.9",
                &VersionNaming::v_prefixed(),
                &super::BuildAsk::of_parts(&super::BuildAskArgs {
                    evaluate_build: false,
                    build_tool: None
                })
                .expect("sin preguntar no falla"),
            )
            .is_err()
        );
    }

    /// Un plan de un monorepo nombra su producto Y dice que habia mas.
    ///
    /// La segunda mitad es la que importa: un plan que nombra su producto sin
    /// decir que hay otros parece el plan de un repositorio con un producto, y
    /// esa es la confusion que el bloque entero elimina. Un plan que solo
    /// dice `release_target: packages/runtime` no permite a quien lo lee
    /// saber si omite algo.
    #[test]
    fn un_plan_de_monorepo_dice_que_producto_es_y_que_hay_mas() {
        let text = release_target_text(&one_of_many());
        assert!(text.contains("release_target: packages/runtime"), "{text}");
        assert!(
            text.contains("release_target_candidates: packages/api, packages/runtime"),
            "el plan lista los candidatos, para que la eleccion sea revisable: {text}"
        );
        assert!(
            text.contains("no declara version") && text.contains("hasta 2 nivel"),
            "y dice como se llego a la lista: {text}"
        );
        assert!(
            text.contains("contiene mas de un producto"),
            "un plan que nombra su producto sin decir que hay mas es \
             indistinguible de un repositorio con un producto: {text}"
        );
    }

    /// Y un repositorio con un solo producto NO dice que hay mas, porque no la
    /// hay. La nota tiene que ser una afirmacion, no un aviso generico.
    #[test]
    fn un_plan_de_un_producto_no_inventa_una_advertencia() {
        let text = release_target_text(&single_target());
        assert!(text.contains("release_target: ."), "{text}");
        assert!(
            !text.contains("contiene mas de un producto"),
            "no hay mas productos: {text}"
        );
        assert!(
            text.contains("la raiz del repositorio declara su propia version"),
            "y dice por que no se busco nada mas: {text}"
        );
    }

    /// El resultado de un apply declara la autoridad, igual que el plan. Este
    /// render no lo cubria nadie tampoco: `release apply --route forge` no es
    /// alcanzable sin red, y quitar el bloque entero no rompia nada.
    #[test]
    fn the_apply_outcome_text_declares_the_authority() {
        let tag_only = super::release_outcome_text(&sddk_gateway::ReleaseOutcome {
            applied: Vec::new(),
            skipped: Vec::new(),
            converged: true,
            version_authority: release_ref_only(),
            release_target: "packages/runtime".into(),
            release_naming: VersionNaming::v_prefixed().style().to_owned(),
            release_role: ReleaseRole::FullPublisher.name().to_owned(),
        });
        assert!(
            tag_only.contains("version_authority: release_ref_is_authority"),
            "{tag_only}"
        );
        assert!(tag_only.contains("nothing was cross-checked"), "{tag_only}");

        let checked = super::release_outcome_text(&sddk_gateway::ReleaseOutcome {
            applied: Vec::new(),
            skipped: Vec::new(),
            converged: true,
            version_authority: cross_validated(),
            release_target: "packages/runtime".into(),
            release_naming: VersionNaming::v_prefixed().style().to_owned(),
            release_role: ReleaseRole::FullPublisher.name().to_owned(),
        });
        assert!(
            checked.contains("version_authority: cross_validated"),
            "{checked}"
        );
        assert!(checked.contains("version: 1.0.0"), "{checked}");
    }

    // ===================================================================
    // cl-release-forge-testability — R1..R5 del PRE-FLIGHT.
    //
    // R2 (el conductual) NO esta aqui y su ausencia es deliberada: llama a
    // `apply_release_forge`, y un test que llama a una funcion que todavia no
    // existe no es un test ROJO, es un error de compilacion que se lleva el
    // crate entero. Es lo que advierte la cabecera de
    // `ledger_export_declaration.rs`: una RED comprada rompiendo el build no es
    // evidencia, es un destrozo. R2 llega con el codigo al que pertenece, en el
    // mismo commit que lo implementa, y entonces es GREEN de verdad.
    //
    // Los cuatro de aqui son ESTRUCTURALES: leen el fuente y no dependen de que
    // exista ninguna funcion, asi que compilan hoy y caen por la razon correcta.
    // ===================================================================

    /// El fuente del propio modulo, que es lo que estos guards miran.
    const THIS_FILE: &str = include_str!("release_cmd.rs");

    /// El cuerpo del brazo `ReleaseRoute::Forge`, acotado. Un `GitHubForge` en
    /// otra funcion del mismo fichero no es el call site de esta rama, y buscar
    /// en el fichero entero es la sexta vez en esta sesion que un detector mide
    /// lo que tiene al lado.
    fn forge_arm() -> &'static str {
        THIS_FILE
            .split("ReleaseRoute::Forge => {")
            .nth(1)
            .expect("the ReleaseRoute::Forge arm must exist")
            .split("\n        }")
            .next()
            .expect("its body must be delimited")
    }

    /// **R1** — la rama delega en una funcion que recibe el forge por parametro.
    /// RED hoy: la rama no delega en ninguna funcion; construye el adaptador
    /// entero dentro de la llamada, y por eso ningun test la alcanza.
    #[test]
    fn r1_the_forge_arm_delegates_to_a_function_that_takes_the_forge() {
        assert!(
            THIS_FILE.contains("fn apply_release_forge"),
            "the body of the forge route must live in a function of its own, so \
             that it can be called with a double instead of only with `gh`. This \
             is the whole defect of INC of the forge route: the branch exists and \
             no test can reach it."
        );
        assert!(
            THIS_FILE.contains("forge: &mut dyn Forge"),
            "that function must receive the forge as `&mut dyn Forge`, which is \
             what `apply_release` already takes (`release.rs:422`). Taking it \
             concretely (`&mut GitHubForge`) would not be testable, because a \
             double cannot be one."
        );
        assert!(
            forge_arm().contains("apply_release_forge("),
            "the ReleaseRoute::Forge arm must call it. Today the body is inlined: {}",
            forge_arm()
        );
    }

    /// **R3** — estructural de la costura: el brazo **delega**, y no es la
    /// implementación.
    ///
    /// La primera versión de este guard afirmaba que el brazo no puede construir
    /// `GitHubForge::new`, y eso es **falso**: el DISEÑO dice exactamente que el
    /// brazo resuelve `--repo`, construye el adaptador y delega. Un guard que
    /// contradice el diseño obliga a elegir entre romper el diseño o romper el
    /// guard, y en ese conflicto el que se equivoca es el guard — porque la
    /// propiedad que de verdad importa no es *quién elige el runner*, sino que
    /// **el cuerpo no vuelva a la rama**.
    ///
    /// El peligro real es otro: que alguien pegue el cuerpo de vuelta en el brazo
    /// y deje `apply_release_forge` como una envoltura fina. R1 seguiría
    ///—la función existe— y R2 ejecutaría la envoltura, no la ruta. Por eso
    /// estos son los anclas: lo que identifica al cuerpo, no lo que identifica
    /// al brazo.
    #[test]
    fn r3_the_forge_arm_delegates_instead_of_being_the_implementation() {
        let arm = forge_arm();
        assert!(
            arm.contains("apply_release_forge("),
            "the ReleaseRoute::Forge arm must call `apply_release_forge`: {}",
            arm
        );
        for inlined in [
            "with_github_releases_ticket::<",
            "plan_release(",
            "apply_release(",
        ] {
            assert!(
                !arm.contains(inlined),
                "`{inlined}` is back inside the ReleaseRoute::Forge arm. The arm \
                 must CALL the release, not BE it: if the body is inlined again \
                 and the function left as a thin wrapper, R1 still passes and R2 \
                 would exercise the wrapper instead of the route. Arm: {arm}"
            );
        }
    }

    /// **R4** — estructural de no-regresion. La extraccion es un cambio de FORMA
    /// y no puede cambiar lo que la rama HACE: mismas capacidades, mismo orden
    /// `CreatePr -> MergePr -> CreateRelease`, y el `AdmissionTicket` envolviendo
    /// la cadena completa. STOP 1 dice que si esto se relaja, el arreglo se
    /// descarta aunque los tests passen.
    #[test]
    fn r4_the_extraction_preserves_capabilities_order_and_the_ticket() {
        // Las tres capacidades, y EN `authorize_release`.
        //
        // La primera version de este guard busco `ReleaseRoute::Forge => vec![`
        // y se llevo la lista de PASOS del plan (`"create_pr", "merge_pr",
        // "create_release"`), que esta un poco mas arriba en el mismo fichero y
        // tiene la misma forma. Caia por el motivo equivocado —de hecho caia
        // siempre, extractsse o no el cuerpo— y ademas no vigilaba lo que decia
        // vigilar: la lista que de verdad autoriza las capacidades nunca se
        // miraba. Treceva vez en esta sesion que un detector mide lo que tiene
        // al lado. El ancla es ahora la FUNCION, no una linea que se repite.
        let authorize = THIS_FILE
            .split("fn authorize_release")
            .nth(1)
            .expect("authorize_release must exist")
            .split("\nfn ")
            .next()
            .expect("its body must be delimited");
        for capability in ["pr.create", "pr.merge", "release.create"] {
            assert!(
                authorize.contains(capability),
                "`authorize_release` must keep requiring `{capability}` for the \
                 forge route. Extracting the body for testability must not become \
                 a way to run three privileged effects without their \
                 authorization. Note that the STEP names are `create_pr` / \
                 `merge_pr` / `create_release` and are NOT capabilities: do not \
                 anchor on those. Body: {authorize}"
            );
        }

        // El ticket sigue envolviendo la cadena: si desaparece, la rama publica
        // sin pasar por el gate que ADR-0132 creo para eso.
        //
        // El ancla es la LLAMADA, con su turbofish (`::<_, …>`), y no el nombre
        // a secas: `with_github_releases_ticket` aparece tambien en el `use` de
        // la cabecera, y anclar por el nombre se llevaba el import en vez del
        // cuerpo. Mismo defecto que el de las capacidades, otro sitio.
        let ticket = THIS_FILE
            .split("with_github_releases_ticket::<")
            .nth(1)
            .expect("the forge chain must run under an AdmissionTicket")
            .split(");")
            .next()
            .unwrap();
        assert!(
            ticket.contains("apply_release("),
            "`apply_release` must stay INSIDE the ticket, not beside it. The \
             whole point of ADR-0132 is that the body runs only after the ticket \
             is consumed: {ticket}"
        );

        // Y la version se sigue comprobando antes de tocar nada.
        assert!(
            ticket.contains("version_authority"),
            "the chain must still carry the `version_authority` it was checked \
             with. An extraction that drops it would report a release whose \
             version was never cross-checked: {ticket}"
        );
    }

    /// **R5** — el guard del comentario. El codigo afirmaba, en dos sitios,
    /// que esta rama carece de prueba y que no se alcanza sin red. En cuanto un
    /// test la alcanza, esas frases son falsas, y una afirmacion falsa en el
    /// codigo es exactamente lo que este trabajo viene a cerrar.
    ///
    /// **Se busca solo ANTES de esta misma definicion.** La primera version
    /// buscaba en el fichero entero y se detectaba a si misma: este doc y el
    /// array de abajo contienen las frases proibidas, luego el guard caia con el
    /// defecto ya corregido. Un guard que se veta con su propio texto no vigila
    /// el codigo, vigila su redaccion.
    #[test]
    fn r5_the_source_no_longer_claims_the_forge_route_is_untested() {
        let before_self = THIS_FILE
            .split("fn r5_the_source_no_longer_claims_the_forge_route_is_untested")
            .next()
            .expect("this test must exist in its own source");
        for claim in ["no tiene test", "no es alcanzable sin red"] {
            assert!(
                !before_self.contains(claim),
                "the source still says `{claim}` about `release apply --route \
                 forge`. If this test passes, that statement is false and has to \
                 go: a comment that contradicts the suite is a comment that will \
                 be believed."
            );
        }
    }

    /// **R2** — el guard conductual: el cuerpo de la ruta forge se EJECUTA, con
    /// un doble, y devuelve pasos aplicados. Este test no podía existir antes de
    /// la extracción: llamaba a una función que no estaba, y eso no es un test
    /// rojo sino un error de compilación que se lleva el crate entero. Por eso
    /// llega con el código al que pertenece.
    ///
    /// Lo que afirma es concreto y no decorativo: con `MockForge`, la cadena
    /// `CreatePr → MergePr → CreateRelease` corre hasta el final y el outcome
    /// trae los pasos **aplicados**. Un `Err`, o un outcome vacío, son las dos
    /// formas de pasar sin haber hecho nada.
    #[test]
    fn r2_the_forge_body_runs_to_completion_with_a_double() {
        const WORKFLOW_YAML: &str = include_str!("../../../workflow/workflow.yaml");
        let directory = tempfile::tempdir().unwrap();
        // The body opens with the L1 lockstep check, and that check fails CLOSED
        // on a version it cannot read — which is correct behaviour, and the first
        // thing this test had to accommodate. An empty directory is not a project
        // that declares no version; it is a project whose version cannot be read.
        std::fs::write(
            directory.path().join("Cargo.toml"),
            "[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        let path = directory.path().join("ledger.sqlite");
        let storage = sddk_storage::Storage::open(&path).unwrap();
        storage
            .insert_project(&sddk_storage::ProjectRecord {
                project_id: "project-1".into(),
                display_name: "project".into(),
                remote_url: Some("https://example.test/owner/project".into()),
                scope: "owner".into(),
                created_at: "2026-08-04T10:00:00Z".into(),
            })
            .unwrap();
        let workflow = sddk_engine::load_workflow_str(WORKFLOW_YAML).unwrap();
        let policy = sddk_gateway::CapabilityPolicy::from_workflow(&workflow);
        let mut gateway = sddk_gateway::CapabilityGateway::new(
            policy,
            workflow,
            sddk_storage::Storage::open(&path).unwrap(),
        );

        let mut forge = sddk_gateway::MockForge::new();
        let args = super::ReleaseArgs {
            runtime: super::RuntimeArgs::default(),
            route: Some(super::ReleaseRoute::Forge),
            repo: Some("owner/project".into()),
            branch: "main".into(),
            base: "main".into(),
            title: "SDDK release".into(),
            tag: "v1.0.0".into(),
            target: None,
            naming: "v_prefixed".into(),
            ask: super::BuildAskArgs {
                evaluate_build: false,
                build_tool: None,
            },
            role: "full_publisher".into(),
            notes: String::new(),
            approve: true,
            cycle: None,
            previous_tag: None,
            release_type: None,
            timestamp: Some("2026-08-04T10:00:00Z".into()),
            actor: Some("r2".into()),
            prefix: None,
            format: crate::OutputFormat::Text,
        };

        let outcome = super::apply_release_forge(
            &mut gateway,
            &mut forge,
            "project-1",
            &args,
            directory.path(),
            "2026-08-04T10:00:00Z",
            "r2",
        )
        .expect("the forge body must reach `apply_release` with a double");

        assert!(
            forge.is_published("v1.0.0"),
            "the body must publish the release through the forge it was given, \
             not return without doing anything. Forge state: {}",
            forge.state_text()
        );
        assert!(
            !outcome.applied.is_empty(),
            "an outcome with no applied step reports a release that did not \
             happen: {outcome:?}"
        );
    }
}
