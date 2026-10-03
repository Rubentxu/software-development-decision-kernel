//! What this binary is, and how it compares to the checkout it was built from.
//!
//! The motivation is INC-DEBT-064: `sddk 2.5.3` and `sddk 2.5.3` are not
//! necessarily the same binary, because the workspace does not bump between
//! releases. Everything committed after the last publication ships under the
//! same version as the installed artefact, and nothing said so.
//!
//! Two rules shape this module, both measured rather than assumed:
//!
//! 1. **The `git` provenance never decides anything.** `build.rs` can fall back
//!    to reading `.git`, and the measurement in that file's header shows why the
//!    fallback can be stale. A value obtained that way is reported with
//!    `source: Git` and is only ever shown, never concluded from.
//! 2. **No answer is "fine" by omission.** Every relation is named, and the
//!    ones that mean "I could not tell" say exactly that. A comparison that
//!    returns success when it knows nothing is worse than no comparison.

use std::path::Path;

use clap::Args;
use sddk_gateway::GitExecutor;
use serde::Serialize;

use crate::CommandOutput;
use crate::dev::OutputFormat;

use super::super::render_result;

const UNKNOWN: &str = "unknown";

/// Where the commit in [`BuildIdentity::sha`] came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ShaSource {
    /// Fixed by whoever ran the build, through `SDDK_GIT_SHA`. The only source
    /// that may be concluded from.
    Env,
    /// Read out of `.git` by `build.rs`. Diagnostic only: the measurement in
    /// `build.rs` shows this can name a stale commit.
    Git,
    /// No source at all: the build had neither the variable nor a repository.
    Absent,
}

impl ShaSource {
    fn as_str(self) -> &'static str {
        match self {
            ShaSource::Env => "env",
            ShaSource::Git => "git",
            ShaSource::Absent => "absent",
        }
    }
}

/// The identity baked into this binary at build time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BuildIdentity {
    /// The commit, or `"unknown"`.
    pub sha: String,
    /// Where `sha` came from. Always present, and never `Git` alongside a real
    /// commit that a conclusion is drawn from.
    pub source: ShaSource,
    /// Whether the tree had uncommitted changes when the build ran. `None` when
    /// it could not be established — a commit does not identify content that
    /// had changes on top of it, so this is not optional information.
    pub dirty: Option<bool>,
}

impl BuildIdentity {
    /// Reads the identity `build.rs` embedded.
    pub fn current() -> Self {
        Self::from_raw(
            option_env!("SDDK_BUILD_SHA").unwrap_or(UNKNOWN),
            option_env!("SDDK_BUILD_SHA_SOURCE").unwrap_or("absent"),
            option_env!("SDDK_BUILD_DIRTY").unwrap_or(UNKNOWN),
        )
    }

    /// The reading, separated from the compile-time constants so that the
    /// mapping can be exercised. R3's first version built `BuildIdentity` by
    /// hand and only proved the *struct* carries `Option<bool>`; it never ran
    /// the mapping that turns a raw `"unknown"` into `None`, which is exactly
    /// where a stray `Some(false)` would hide. Falsification found that: M5
    /// survived.
    fn from_raw(sha: &str, source: &str, dirty: &str) -> Self {
        let source = match source {
            "env" => ShaSource::Env,
            "git" => ShaSource::Git,
            _ => ShaSource::Absent,
        };
        let dirty = match dirty {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        };
        Self {
            sha: sha.to_owned(),
            source,
            dirty,
        }
    }

    /// Whether this identity may be concluded from. A `git` fallback may not:
    /// the build script's own header records the scenario where it names a
    /// commit that is no longer HEAD.
    pub fn is_conclusive(&self) -> bool {
        self.source == ShaSource::Env && self.sha != UNKNOWN
    }
}

/// How this binary relates to the checkout it is being compared against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckoutRelation {
    /// The checkout is on exactly the commit this binary was built from.
    Matches,
    /// The binary's commit is an ancestor of the checkout's HEAD: the checkout
    /// has work the binary does not contain. Proven with `--is-ancestor`, not
    /// inferred from the dates.
    Behind,
    /// Both exist and differ, but the binary's commit is not an ancestor of
    /// HEAD. The binary may be ahead, on another branch, or from a history that
    /// has been rewritten. **This does not say which**, because it cannot.
    Diverged,
    /// There is no checkout to compare against.
    NoCheckout,
    /// The identity is not usable, so no relation can be established. This is
    /// deliberately its own case and not a synonym for `Matches`.
    Unknown,
}

impl CheckoutRelation {
    /// Whether this binary can be trusted to reflect the checkout. Only
    /// `Matches` and `Behind` are answers; the rest are the absence of one.
    fn is_answer(self) -> bool {
        matches!(self, CheckoutRelation::Matches | CheckoutRelation::Behind)
    }
}

/// A `dev build-id` invocation.
#[derive(Debug, Clone, Args)]
pub(crate) struct BuildIdArgs {
    /// Compare this binary against a checkout and report how they relate.
    #[arg(long)]
    pub(crate) check: bool,
    /// Root of the checkout to compare against. Inferred from the working
    /// directory when omitted.
    #[arg(long)]
    pub(crate) root: Option<std::path::PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Serialize)]
struct BuildIdentityOutput {
    version: String,
    identity: BuildIdentity,
    /// Only present with `--check`.
    #[serde(skip_serializing_if = "Option::is_none")]
    comparison: Option<Comparison>,
}

#[derive(Debug, Serialize)]
struct Comparison {
    /// The checkout's HEAD, or `"unknown"` when there is none.
    checkout_head: String,
    relation: CheckoutRelation,
    /// Why this relation and not another. Present because `Diverged` and
    /// `Unknown` are easy to read as a verdict when they are the absence of one.
    reason: String,
}

/// Entry point for `sddk dev build-id`.
pub(super) fn run_dev_build_id(args: BuildIdArgs) -> CommandOutput {
    let format = args.format;
    let identity = BuildIdentity::current();
    // Resolving the root can fail, and that failure must be reported rather
    // than swallowed into an empty comparison. `None` here means "no `--check`
    // was asked for", which is a different thing entirely.
    let comparison: anyhow::Result<Option<Comparison>> = if args.check {
        let root = args
            .root
            .clone()
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        resolve_root(&root).map(|resolved| Some(compare(&identity, &resolved)))
    } else {
        Ok(None)
    };

    match comparison.map(|c| BuildIdentityOutput {
        version: env!("CARGO_PKG_VERSION").into(),
        identity,
        comparison: c,
    }) {
        Ok(value) => {
            // STOP 3, as code: with `--check`, a comparison that could not
            // establish the relation must NOT exit 0. A check that passes when
            // it does not know is worse than no check, because its success is
            // indistinguishable from a real pass.
            let answered = value
                .comparison
                .as_ref()
                .is_some_and(|c| c.relation.is_answer());
            let mut out = render_result(Ok(value), format, build_id_text);
            if args.check && !answered {
                out.status = 1;
            }
            out
        }
        Err(e) => render_result::<BuildIdentityOutput>(Err(e), format, build_id_text),
    }
}

fn resolve_root(root: &Path) -> anyhow::Result<std::path::PathBuf> {
    crate::canonical_root(root).map_err(|e| {
        anyhow::anyhow!(
            "no se pudo resolver el root del checkout ({}): {}",
            root.display(),
            e
        )
    })
}

/// Compares the binary's identity to a checkout.
///
/// Deliberately conservative: `Behind` is only reported when `--is-ancestor`
/// proves it, because two different commits do not tell you which one is
/// newer, and reporting a direction that was not established is the failure
/// this whole module exists to remove.
fn compare(identity: &BuildIdentity, root: &Path) -> Comparison {
    if !identity.is_conclusive() {
        return Comparison {
            checkout_head: head_or_unknown(root),
            relation: CheckoutRelation::Unknown,
            reason: format!(
                "la identidad del binario no es concluyente (sha={}, source={})",
                identity.sha,
                identity.source.as_str()
            ),
        };
    }
    let git = GitExecutor::new(root.to_path_buf());
    if !git.is_inside_work_tree().unwrap_or(false) {
        return Comparison {
            checkout_head: UNKNOWN.into(),
            relation: CheckoutRelation::NoCheckout,
            reason: format!("{} no es un checkout de git", root.display()),
        };
    }
    let head = match git.head_sha() {
        Ok(h) => h,
        Err(e) => {
            return Comparison {
                checkout_head: UNKNOWN.into(),
                relation: CheckoutRelation::NoCheckout,
                reason: format!("no se pudo leer el HEAD del checkout: {e}"),
            };
        }
    };
    if head == identity.sha {
        return Comparison {
            checkout_head: head,
            relation: CheckoutRelation::Matches,
            reason: "el checkout esta en el mismo commit que el binario".into(),
        };
    }
    // `--is-ancestor` exits 0 when the first commit is an ancestor of the
    // second. That is the only thing that proves a direction.
    let behind = git
        .run_read_only("merge-base", &["--is-ancestor", &identity.sha, "HEAD"])
        .is_ok();
    if behind {
        Comparison {
            checkout_head: head,
            relation: CheckoutRelation::Behind,
            reason: format!(
                "el commit del binario ({}) es ancestro del HEAD del checkout, luego el checkout tiene trabajo que el binario no contiene",
                short(&identity.sha)
            ),
        }
    } else {
        Comparison {
            checkout_head: head,
            relation: CheckoutRelation::Diverged,
            reason: "los commits difieren y el del binario NO es ancestro del HEAD: \
                     no se puede afirmar en que direccion van"
                .into(),
        }
    }
}

fn head_or_unknown(root: &Path) -> String {
    GitExecutor::new(root.to_path_buf())
        .head_sha()
        .unwrap_or_else(|_| UNKNOWN.into())
}

fn short(sha: &str) -> &str {
    &sha[..sha.len().min(8)]
}

fn build_id_text(output: &BuildIdentityOutput) -> String {
    let mut text = String::new();
    text.push_str(&format!("sddk {}\n", output.version));
    text.push_str(&format!("commit: {}\n", output.identity.sha));
    text.push_str(&format!("source: {}\n", output.identity.source.as_str()));
    text.push_str(&format!(
        "dirty: {}\n",
        match output.identity.dirty {
            Some(true) => "true".to_string(),
            Some(false) => "false".to_string(),
            None => UNKNOWN.to_string(),
        }
    ));
    if let Some(c) = &output.comparison {
        text.push_str(&format!("checkout_head: {}\n", c.checkout_head));
        text.push_str(&format!("relation: {:?}\n", c.relation));
        text.push_str(&format!("reason: {}\n", c.reason));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    const THIS_FILE: &str = include_str!("build_id.rs");

    fn env_identity() -> BuildIdentity {
        BuildIdentity {
            sha: "a".repeat(40),
            source: ShaSource::Env,
            dirty: Some(false),
        }
    }

    // R6: la procedencia acompaña siempre al valor. Un `unknown` con
    // procedencia `git` sería exactamente el cuarto escenario del §3 del
    // SCOPE, con otro nombre: una identidad que parece verdad y no lo es.
    #[test]
    fn r6_unknown_is_never_reported_as_coming_from_git() {
        let forged = BuildIdentity {
            sha: UNKNOWN.into(),
            source: ShaSource::Git,
            dirty: None,
        };
        assert!(
            !forged.is_conclusive(),
            "un sha desconocido no puede ser concluyente sea cual sea su procedencia"
        );
    }

    // STOP 6, que es la misma regla aplicada al metodo: la variante `git`
    // existe como dato de diagnostico y no puede decidir nada.
    #[test]
    fn r6_the_git_fallback_never_concludes() {
        let from_git = BuildIdentity {
            sha: "b".repeat(40),
            source: ShaSource::Git,
            dirty: Some(false),
        };
        assert!(
            !from_git.is_conclusive(),
            "STOP 6: la identidad derivada de .git no decide, porque la medicion del \
             build.rs mostro que puede nombrar un commit obsoleto"
        );
        assert!(env_identity().is_conclusive());
    }

    // R4 / STOP 3: ninguna relacion es "bien" por omision. Solo `Matches` y
    // `Behind` son respuestas; las demas son la ausencia de una.
    #[test]
    fn r4_only_matches_and_behind_are_answers() {
        assert!(CheckoutRelation::Matches.is_answer());
        assert!(CheckoutRelation::Behind.is_answer());
        for relation in [
            CheckoutRelation::Diverged,
            CheckoutRelation::NoCheckout,
            CheckoutRelation::Unknown,
        ] {
            assert!(
                !relation.is_answer(),
                "{:?} no es una respuesta y no puede hacer pasar un check",
                relation
            );
        }
    }

    // El nombre de la relacion se serializa en snake_case y es lo que una
    // maquina lee. Un `Debug` en el JSON cambiaria el contrato en silencio.
    #[test]
    fn the_json_forms_are_stable_and_machine_readable() {
        let json = serde_json::to_string(&env_identity()).unwrap();
        assert!(json.contains(r#""source":"env""#), "{}", json);
        assert!(json.contains(r#""sha":"#), "{}", json);
        let relation = serde_json::to_string(&CheckoutRelation::Diverged).unwrap();
        assert_eq!(relation, "\"diverged\"", "{}", relation);
    }

    fn git_repo() -> (tempfile::TempDir, String) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let run = |args: &[&str]| {
            let out = std::process::Command::new("git")
                .current_dir(root)
                .args(args)
                .output()
                .expect("git runs");
            assert!(
                out.status.success(),
                "git {:?} failed: {}",
                args,
                String::from_utf8_lossy(&out.stderr)
            );
        };
        run(&["init", "-q", "."]);
        run(&["config", "user.email", "probe@example.invalid"]);
        run(&["config", "user.name", "probe"]);
        run(&["commit", "-q", "--allow-empty", "-m", "uno"]);
        let sha = String::from_utf8(
            std::process::Command::new("git")
                .current_dir(root)
                .args(["rev-parse", "HEAD"])
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .to_owned();
        (dir, sha)
    }

    // R5: con el checkout en el mismo commit, la relacion es `Matches`.
    #[test]
    fn r5_the_same_commit_is_matches() {
        let (dir, sha) = git_repo();
        let identity = BuildIdentity {
            sha: sha.clone(),
            source: ShaSource::Env,
            dirty: Some(false),
        };
        let comparison = compare(&identity, dir.path());
        assert_eq!(
            comparison.relation,
            CheckoutRelation::Matches,
            "{}",
            comparison.reason
        );
        assert_eq!(comparison.checkout_head, sha);
    }

    // R5: con un commit mas en el checkout, la relacion es `Behind`, y se
    // demuestra con `--is-ancestor`, no por las fechas.
    #[test]
    fn r5_a_newer_checkout_is_behind_and_the_proof_is_the_ancestry() {
        let (dir, first) = git_repo();
        std::process::Command::new("git")
            .current_dir(dir.path())
            .args(["commit", "-q", "--allow-empty", "-m", "dos"])
            .output()
            .unwrap();
        let head = String::from_utf8(
            std::process::Command::new("git")
                .current_dir(dir.path())
                .args(["rev-parse", "HEAD"])
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .to_owned();
        let identity = BuildIdentity {
            sha: first,
            source: ShaSource::Env,
            dirty: Some(false),
        };
        let comparison = compare(&identity, dir.path());
        assert_eq!(
            comparison.relation,
            CheckoutRelation::Behind,
            "{}",
            comparison.reason
        );
        assert_eq!(comparison.checkout_head, head);
    }

    // R5, el caso que faltaba y que la falsificacion vino a buscar: cuando el
    // commit del binario **no** es ancestro del HEAD. Es el unico caso donde
    // «comprobar la ancestria» y «comprobar que son distintos» dan respuestas
    // OPUESTAS, y sin el, la primera version de R5 pasaba con la segunda
    // implementada: en el test de arriba, `rev-parse HEAD` tiene exito y el
    // HEAD nuevo tiene al primero por ancestro, luego las dos implementaciones
    // coinciden y el guard no mide nada. Aqui se separan.
    #[test]
    fn r5_a_commit_outside_the_history_is_diverged_not_behind() {
        let (dir, first) = git_repo();
        // Rama huerfana: su historia no contiene el primer commit.
        for args in [
            vec!["checkout", "-q", "--orphan", "otra"],
            vec!["commit", "-q", "--allow-empty", "-m", "huerfano"],
        ] {
            let out = std::process::Command::new("git")
                .current_dir(dir.path())
                .args(&args)
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "git {:?} fallo: {}",
                args,
                String::from_utf8_lossy(&out.stderr)
            );
        }
        let identity = BuildIdentity {
            sha: first.clone(),
            source: ShaSource::Env,
            dirty: Some(false),
        };
        let comparison = compare(&identity, dir.path());
        assert_ne!(
            comparison.checkout_head, first,
            "el HEAD tiene que ser distinto del commit del binario para que el caso signifique algo"
        );
        assert_eq!(
            comparison.relation,
            CheckoutRelation::Diverged,
            "un commit que no es ancestro del HEAD NO es «retrasado»: la mutacion que \
             reemplaza `--is-ancestor` por `rev-parse` debe caer aqui. {}",
            comparison.reason
        );
    }

    // R4: sin checkout, la comparacion lo dice en vez de quedarse muda. Y la
    // identidad no concluyente gana: no se puede comparar nada sin ella.
    #[test]
    fn r4_no_checkout_and_no_identity_are_both_named() {
        let dir = tempfile::tempdir().unwrap();
        let identity = env_identity();
        let comparison = compare(&identity, dir.path());
        assert_eq!(
            comparison.relation,
            CheckoutRelation::NoCheckout,
            "{}",
            comparison.reason
        );
        assert!(!comparison.relation.is_answer());

        let inconclusive = BuildIdentity {
            sha: "c".repeat(40),
            source: ShaSource::Git,
            dirty: None,
        };
        let comparison = compare(&inconclusive, dir.path());
        assert_eq!(
            comparison.relation,
            CheckoutRelation::Unknown,
            "{}",
            comparison.reason
        );
        assert!(!comparison.relation.is_answer());
    }

    // R2 no es un test de rust: es una propiedad del `build.rs`, y se verifica
    // construyendo. Lo que este test fija es que el fallback existe y degrada
    // sin escribir la variable, que es la mitad que si se puede comprobar aqui.
    #[test]
    fn r2_the_build_script_degrades_instead_of_failing() {
        let build_rs = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/build.rs"));
        assert!(
            build_rs.contains("fn git_sha() -> Option<String>"),
            "el fallback a .git tiene que devolver Option, no anyhow::Result: \
             un Result con error haria fallar la construccion sin repositorio"
        );
        assert!(
            !build_rs.contains(".expect(") && !build_rs.contains(".unwrap()"),
            "el build script no puede hacer panic: STOP 1 dice que la construccion \
             no puede depender de nada externo"
        );
    }

    // STOP 2: la forma de `--version` no cambia. `install.sh:416` hace
    // `awk '{print $NF}'` sobre su salida, asi que anadir el commit al final
    // haria que el ultimo campo fuera un parentesis.
    //
    // La comprobacion fuerte vive en `tests/cli.rs`
    // (`r1_version_last_field_is_a_bare_semver`): ejecuta el binario y mira el
    // ultimo campo de verdad. Aqui solo se fija lo que este modulo puede
    // quitar por su cuenta, que es registrar una variante que toque la
    // version. La version anterior de este guard buscaba `version(` en este
    // fichero y se detectaba a SI MISMO: el token aparece en su propia linea.
    #[test]
    fn r1_this_module_cannot_touch_the_version_surface() {
        let main_rs = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/main.rs"));
        // El token se ENSAMBLA, no se escribe literal. Es la tercera vez en
        // este trabajo que un guard textual busca una cadena que su propio
        // codigo contiene y se detecta a si mismo — la primera fue R5 en
        // cl-release-forge-testability, la segunda fue este mismo test con
        // `version(`. Un guard que se detecta a si mismo no vigila nada: pasa
        // porque se encuentra a si mismo.
        let forbidden = ["DevCommand", "Version"].join("::");
        assert!(
            !THIS_FILE.contains(&forbidden),
            "este modulo no puede registrar una variante que exponga la version"
        );
        let forbidden_env = ["SDDK", "BUILD", "SHA"].join("_");
        assert!(
            !main_rs.contains(&forbidden_env),
            "la identidad no se imprime en --version: install.sh toma el ultimo campo"
        );
    }

    // R3: el estado sucio es un campo propio, no algo que se deduzca, y sobre
    // todo **la lectura tiene que poder devolverlo ausente**. La primera
    // version de este guard construia la identidad a mano y solo probaba que
    // la estructura admite `None`: la mutacion que convierte el `"unknown"`
    // del build en `Some(false)` —que es el modo de fallo completo, un arbol
    // con cambios sin commitear que se presenta como limpio— sobrevivio.
    #[test]
    fn r3_dirtiness_is_a_field_and_the_reading_can_be_absent() {
        // La lectura, con las tres entradas que el build.rs puede emitir.
        let from_build = BuildIdentity::from_raw(&"a".repeat(40), "env", "true");
        assert_eq!(from_build.dirty, Some(true));
        let from_build = BuildIdentity::from_raw(&"a".repeat(40), "env", "false");
        assert_eq!(from_build.dirty, Some(false));
        // Aqui esta el agujero: sin informacion, el campo se AUSENTA. Si esta
        // asercion cae, alguien esta inventando `false`.
        let unknown = BuildIdentity::from_raw(UNKNOWN, "absent", UNKNOWN);
        assert_eq!(
            unknown.dirty, None,
            "sin informacion de suciedad el campo se ausenta, no se inventa false"
        );
        let text = build_id_text(&BuildIdentityOutput {
            version: "0.0.0".into(),
            identity: unknown,
            comparison: None,
        });
        assert!(
            text.contains("dirty: unknown"),
            "el texto declara lo que no sabe: {}",
            text
        );
    }
}
