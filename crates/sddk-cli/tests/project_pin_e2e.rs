//! E2E: `sddk project pin` (W2c, agente-secretless D2).
//!
//! Contrato:
//! - `project pin` escribe `.sddk/project-pin.json` y `resolve` pasa a reportar
//!   `identity_source: pinned` con el project_id fijado.
//! - Con pin presente, un remote con otro nombre (fork de identidad) NO cambia
//!   el project_id resuelto.
//! - Repinar a otro project_id falla sin `unpin` previo; `unpin` restaura el
//!   derive por remote.
//! - INC-DEBT-049: el pin también es autoridad para `adopt status` y para la
//!   inferencia de `cycle status`. Antes esas dos vías
//!   rederivaban desde el remote y reportaban un project_id DISTINTO al que
//!   `project resolve` mostraba sobre el mismo checkout con pin. Los dos tests
//!   anteriores pasaban mientras esas tres seguían mintiendo, porque sólo
//!   ejercitaban el resolver que ya honraba el pin.

use std::path::PathBuf;
use std::process::Command;

fn bin() -> PathBuf {
    // Sigue el target-dir global como el resto de e2e del workspace.
    PathBuf::from(env!("CARGO_BIN_EXE_sddk"))
}

struct Sandbox {
    dir: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("sddk-pin-e2e-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Sandbox { dir }
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn run(dir: &std::path::Path, args: &[&str]) -> (i32, String) {
    // XDG isolation: these commands resolve real storage paths and some of
    // them (adopt) open a ledger. Without isolation, two cases sharing a
    // remote would contend on the developer's actual ledger — observed as
    // "database is locked" — and would write into it besides.
    let xdg = dir.join(".xdg");
    std::fs::create_dir_all(&xdg).ok();
    let out = Command::new(bin())
        .args(args)
        .env_remove("SDDK_PROJECT_ID")
        // Same reason as the XDG vars above, one level up: `SDDK_DATA_DIR` wins
        // over `XDG_DATA_HOME` and is inherited, so one exported in the
        // developer's shell would send these cases into their real storage
        // despite the isolation three lines up.
        .env_remove("SDDK_DATA_DIR")
        .env("XDG_DATA_HOME", xdg.join("data"))
        .env("XDG_STATE_HOME", xdg.join("state"))
        .env("XDG_CACHE_HOME", xdg.join("cache"))
        .current_dir(dir)
        .output()
        .expect("sddk binary runs");
    (
        out.status.code().unwrap_or(-1),
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    )
}

/// `adopt status` exits non-zero when the project is not adopted (`status:
/// absent`), which is the correct answer for a fresh checkout. The project_id
/// it reports is what matters, so read it regardless of the exit code.
fn project_id_from(out: &str) -> String {
    out.lines()
        .find_map(|l| l.strip_prefix("project_id: "))
        .expect("project_id line in output")
        .trim()
        .to_string()
}

#[test]
fn pin_overrides_remote_drift() {
    let s = Sandbox::new("override");

    // Resolve inicial por remote (puede no haber git: usamos --remote explícito).
    let (code, out) = run(
        &s.dir,
        &[
            "project",
            "resolve",
            "--root",
            ".",
            "--remote",
            "git@EXAMPLE.com:Org/Repo.git",
            "--scope",
            ".",
        ],
    );
    assert_eq!(code, 0, "resolve inicial: {out}");
    let pinned_id = out
        .lines()
        .find_map(|l| l.strip_prefix("project_id: "))
        .expect("project_id line")
        .trim()
        .to_string();
    assert!(pinned_id.starts_with("p-"), "{out}");

    // Pin al id resuelto.
    let (code, out) = run(
        &s.dir,
        &[
            "project",
            "pin",
            "--root",
            ".",
            "--project-id",
            &pinned_id,
            "--reason",
            "test",
        ],
    );
    assert_eq!(code, 0, "pin: {out}");
    assert!(s.dir.join(".sddk/project-pin.json").exists());

    // Resolve ahora reporta pinned y el MISMO id incluso con remote distinto.
    let (code, out) = run(
        &s.dir,
        &[
            "project",
            "resolve",
            "--root",
            ".",
            "--remote",
            "git@other.example:Totally/Different.git",
            "--scope",
            ".",
        ],
    );
    assert_eq!(code, 0, "resolve tras pin: {out}");
    assert!(
        out.contains("identity_source: pinned"),
        "debe reportar pinned: {out}"
    );
    assert!(
        out.contains(&format!("project_id: {pinned_id}")),
        "debe mantener el id pineado: {out}"
    );

    // Y con un remote que derivaría a otro id: sigue el pin.
    let (code, out) = run(
        &s.dir,
        &[
            "project",
            "pin",
            "--root",
            ".",
            "--project-id",
            "p-other0000000000000000000000000000000000000000000000000000000000.00",
        ],
    );
    assert_ne!(code, 0, "repin a otro id debe fallar: {out}");
}

#[test]
fn unpin_restores_remote_derivation() {
    let s = Sandbox::new("unpin");

    // Sin pin: derive por remote A.
    let (code, out_a) = run(
        &s.dir,
        &[
            "project",
            "resolve",
            "--root",
            ".",
            "--remote",
            "git@example.com:org/a.git",
            "--scope",
            ".",
        ],
    );
    assert_eq!(code, 0, "{out_a}");
    let id_a = out_a
        .lines()
        .find_map(|l| l.strip_prefix("project_id: "))
        .unwrap()
        .trim()
        .to_string();

    // Sin pin, remote B da OTRO id (precondición del fork).
    let (code, out_b) = run(
        &s.dir,
        &[
            "project",
            "resolve",
            "--root",
            ".",
            "--remote",
            "git@example.com:org/b.git",
            "--scope",
            ".",
        ],
    );
    assert_eq!(code, 0, "{out_b}");
    let id_b = out_b
        .lines()
        .find_map(|l| l.strip_prefix("project_id: "))
        .unwrap()
        .trim()
        .to_string();
    assert_ne!(id_a, id_b, "remotes distintos deben derivar ids distintos");

    // Pin a id_a; remote B ya no mueve la identidad.
    let (code, out) = run(
        &s.dir,
        &["project", "pin", "--root", ".", "--project-id", &id_a],
    );
    assert_eq!(code, 0, "{out}");
    let (code, out) = run(
        &s.dir,
        &[
            "project",
            "resolve",
            "--root",
            ".",
            "--remote",
            "git@example.com:org/b.git",
            "--scope",
            ".",
        ],
    );
    assert_eq!(code, 0, "{out}");
    assert!(out.contains(&format!("project_id: {id_a}")), "{out}");

    // Unpin: vuelve el derive (remote B da id_b otra vez).
    let (code, out) = run(&s.dir, &["project", "unpin", "--root", "."]);
    assert_eq!(code, 0, "{out}");
    assert!(!s.dir.join(".sddk/project-pin.json").exists());
    let (code, out) = run(
        &s.dir,
        &[
            "project",
            "resolve",
            "--root",
            ".",
            "--remote",
            "git@example.com:org/b.git",
            "--scope",
            ".",
        ],
    );
    assert_eq!(code, 0, "{out}");
    assert!(out.contains(&format!("project_id: {id_b}")), "{out}");
}

/// INC-DEBT-049: un pin declarado por el operador es la autoridad de identidad.
/// Este test recorre las superficies visibles que ANTES rederivaban desde el
/// remote. La tercera vía rota (`resolve_project_ids`, usada por `config set`)
/// no expone el id en su salida, así que se cubre con un test unitario en
/// `lib.rs::tests::resolve_project_ids_honors_the_pin`.
///
/// El fallo original, medido en vivo: sobre un checkout con pin, `project
/// resolve` reportaba `p-63676b11dc0ef88f` mientras `adopt status` reportaba
/// `p-995939af668a53d8` y `cycle status` buscaba ciclos del id equivocado. El
/// pin se escribía y no surtía efecto en dos de las tres vías.
#[test]
fn pinned_identity_is_authoritative_for_adopt_config_and_cycle() {
    let s = Sandbox::new("authoritative");

    let remote = "git@example.com:Org/Repo.git";
    let id_pinned = "p-pinnedauthoritative01";

    // Baseline sin pin: `adopt status` deriva del remote.
    let (_, out) = run(
        &s.dir,
        &[
            "adopt", "status", "--root", ".", "--scope", ".", "--remote", remote,
        ],
    );
    let derived = project_id_from(&out);
    assert!(derived.starts_with("p-"), "{out}");
    assert_ne!(
        derived, id_pinned,
        "el id derivado debe diferir del pin; si no, el test no prueba nada"
    );

    let (code, out) = run(
        &s.dir,
        &[
            "project",
            "pin",
            "--root",
            ".",
            "--project-id",
            id_pinned,
            "--reason",
            "inc-debt-049",
        ],
    );
    assert_eq!(code, 0, "pin: {out}");

    // 1. adopt status: debe reportar el id pinneado, no el derivado.
    let (_, out) = run(
        &s.dir,
        &[
            "adopt", "status", "--root", ".", "--scope", ".", "--remote", remote,
        ],
    );
    assert!(
        out.contains(&format!("project_id: {id_pinned}")),
        "adopt status debe honourar el pin (esperaba {id_pinned}, derivado {derived}):\n{out}"
    );

    // 2. cycle status: sin ciclos activos nombra el proyecto en el error. Ese
    //    nombre es el project_id que buscó, así que delata el resolver usado.
    let (_, out) = run(&s.dir, &["cycle", "status", "--root", ".", "--scope", "."]);
    assert!(
        out.contains(id_pinned),
        "cycle status debe buscar ciclos del proyecto pinneado:\n{out}"
    );
}

/// Contrapartida del anterior: sin pin, nada cambia. Un fix que alterase el
/// caso normal (el 99% de los checkouts) pasaría el test de arriba y rompería
/// este. INC-DEBT-049.
#[test]
fn unpinned_checkout_still_derives_from_the_remote() {
    let s = Sandbox::new("unpinned-unchanged");

    let remote = "git@example.com:Org/Repo.git";
    let (_, out) = run(
        &s.dir,
        &[
            "adopt", "status", "--root", ".", "--scope", ".", "--remote", remote,
        ],
    );
    let from_adopt = project_id_from(&out);

    let (code, out) = run(
        &s.dir,
        &[
            "project", "resolve", "--root", ".", "--remote", remote, "--scope", ".",
        ],
    );
    assert_eq!(code, 0, "{out}");
    let from_resolve = project_id_from(&out);

    assert_eq!(
        from_adopt, from_resolve,
        "sin pin, adopt status y project resolve deben coincidir (derive por remote)"
    );
}

/// A malformed pin must fail **closed**, not fall through to derivation.
///
/// This used to be measured in the engine
/// (`adoption_identity.rs::malformed_pin_fails_closed_instead_of_falling_back_to_the_remote`)
/// by `validate_plan_input`. That check moved with the identity: a
/// checkout-local concern is not a filesystem-free engine's business, and the
/// engine cannot read the pin file anyway. INC-DEBT-059.
///
/// It is re-measured here because the property is unchanged and the place it
/// lives changed: an unreadable pin silently ignored is exactly how a checkout
/// ended up split across two `project_id`s, and `adopt status` reporting
/// `status: complete` over an empty storage while the real ledger sat under the
/// id the pin named.
#[test]
fn a_malformed_pin_fails_closed_instead_of_deriving() {
    let s = Sandbox::new("malformed");

    // Valid JSON, valid `schema_version`, but the `project_id` is not a
    // project_id — no `p-` prefix. Every other field is well-formed, so this
    // is a pin that a lenient reader would happily accept.
    std::fs::create_dir_all(s.dir.join(".sddk")).unwrap();
    std::fs::write(
        s.dir.join(".sddk/project-pin.json"),
        r#"{"schema_version":1,"project_id":"63676b11dc0ef88f","pinned_at":"2026-10-01T12:00:00Z"}"#,
    )
    .unwrap();

    let (code, out) = run(
        &s.dir,
        &[
            "adopt",
            "status",
            "--root",
            ".",
            "--scope",
            ".",
            "--remote",
            "git@EXAMPLE.com:Org/Repo.git",
        ],
    );

    assert_ne!(
        code, 0,
        "un pin malformado debe salir con codigo no cero, no derivar en silencio.\nsalida: {out}"
    );
    assert!(
        out.contains("project pin") || out.contains("not a project_id"),
        "el error debe NOMBRAR al pin, no hablar del remote: {out}"
    );
    assert!(
        !out.contains("status: complete"),
        "un pin que no se pudo leer no puede acabar en `complete`: es el \
         `status: complete` sobre storage vacio que INC-DEBT-049 describio.\n\
         salida: {out}"
    );
}
