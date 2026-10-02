//! The `adopt` ↔ alias-store seam, which no test crossed. INC-DEBT-059.
//!
//! **Why this file exists.** `adoption_contract.rs` covers the adoption
//! *contract* (which tokens the agent file carries) and
//! `project_pin_e2e.rs` covers the *pin*. Between them there were **zero**
//! occurrences of "alias" — measured, not assumed:
//!
//! ```text
//! $ grep -c alias crates/sddk-cli/tests/adoption_contract.rs   # 0
//! $ grep -c alias crates/sddk-cli/tests/project_pin_e2e.rs     # 0
//! ```
//!
//! The tests lived on both sides of the seam and none crossed it, which is the
//! same shape as the `resolve_bypasses_the_wiring` mutation of alias lot 3 —
//! it escaped for exactly that reason, and finding it cost a refactor of
//! `run_project_resolve` so a test *could* cross it. Here the seam exists and
//! nobody crossed it, so the two surfaces drifted apart for as long as the
//! alias store was declared complete.
//!
//! **The measured defect these tests are written against** (see
//! `docs/debt/INC-DEBT-059-…md`): `prepare_adoption_plan` and
//! `converge_adoption` never load the alias table, so `adopt` and
//! `context bootstrap` re-derive the identity and land on the *retired* id. On
//! a checkout with a declared alias, `adopt status` read a ledger that does not
//! exist and reported `absent`, and `adopt apply` wrote a **second** adoption
//! receipt under the retired id — the orphan ADR-0152 exists to make
//! un-re-creatable.
//!
//! Every test here must fail before the fix and pass after it. A test that
//! passes on both sides measures nothing.

use std::path::Path;
use std::process::Command;

use sddk_testkit::{CliSandbox, TestRepository};

/// A checkout with a declared alias `from -> to`, where the adoption receipt
/// lives under `to` and the pin has been removed.
struct Aliased {
    sandbox: CliSandbox,
    /// The derived id, now retired.
    from: String,
    /// The id the alias points at, and the one holding the receipt.
    to: String,
}

fn cmd(sandbox: &CliSandbox) -> Command {
    // `CliSandbox` sets HOME and the three XDG vars but *inherits* the rest of
    // the environment, so an `SDDK_DATA_DIR` exported in the developer's shell
    // would send every write in this test to their real data dir. Hermeticity
    // is the whole point of a sandbox: assert it rather than assume it.
    let mut command = sandbox.sddk_command();
    command.env_remove("SDDK_DATA_DIR");
    command
}

/// Runs a **setup** step, which must succeed. A failure here is a fixture
/// problem, not a verdict on the property, so it is reported as such.
fn run(sandbox: &CliSandbox, args: &[&str]) -> String {
    let output = cmd(sandbox).args(args).output().expect("sddk should run");
    assert!(
        output.status.success(),
        "el paso de preparacion `sddk {args:?}` fallo: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Runs the command **under test** and returns its stdout regardless of the
/// exit code.
///
/// The exit code is deliberately not asserted here. `adopt status` exits 1 when
/// the project is not adopted and `context bootstrap` exits 4 on
/// `no_capsule_source`, so requiring success would make these tests measure the
/// exit code instead of the property — which is what happened twice on the
/// first pass and is the reason this helper exists. The property is *what the
/// command reports*; the exit code follows from it, and asserting it here would
/// bury the real failure message under a status line.
fn run_reporting(sandbox: &CliSandbox, args: &[&str]) -> String {
    let output = cmd(sandbox).args(args).output().expect("sddk should run");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Reads a `key: value` line out of the text rendering. Anchored at the line
/// start so `identity_alias: from -> to` never satisfies a `project_id` query.
fn field(stdout: &str, key: &str) -> Option<String> {
    stdout.lines().find_map(|line| {
        let rest = line.strip_prefix(&format!("{key}: "))?;
        Some(rest.trim().to_string())
    })
}

/// Every `adoption.json` under the sandbox data dir, as `project_id` strings.
fn adoption_receipt_ids(sandbox: &CliSandbox) -> Vec<String> {
    let projects = sandbox.xdg().xdg_data.join("sddk/projects");
    let mut ids = Vec::new();
    let Ok(entries) = std::fs::read_dir(&projects) else {
        return ids;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("p-") {
            continue;
        }
        if walk_has_receipt(&entry.path()) {
            ids.push(name);
        }
    }
    ids.sort();
    ids
}

fn walk_has_receipt(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() && walk_has_receipt(&path) {
            return true;
        }
        if path.is_file() && path.file_name().is_some_and(|n| n == "adoption.json") {
            return true;
        }
    }
    false
}

/// Builds the aliased checkout: adopt under a pin, drop the pin, declare the
/// alias from the id the checkout derives back to the id holding the receipt.
fn aliased_checkout() -> Aliased {
    let repo = TestRepository::new().unwrap();
    repo.init().unwrap();
    let sandbox = CliSandbox::new(repo, env!("CARGO_BIN_EXE_sddk")).unwrap();
    let root = sandbox.path().to_string_lossy().to_string();

    let derived = run(
        &sandbox,
        &["project", "resolve", "--root", &root, "--scope", "."],
    );
    let from = field(&derived, "project_id").expect("project resolve must report a project_id");

    // The id the receipt will live under. Distinct from `from` by construction:
    // a pin is the only way to make a checkout adopt under an id its own
    // derivation would not produce, which is the real-world shape of a
    // re-adoption.
    let to = "p-0000000000000aaa".to_string();
    assert_ne!(from, to, "the pin must not collide with the derived id");

    run(
        &sandbox,
        &["project", "pin", "--root", &root, "--project-id", &to],
    );
    run(
        &sandbox,
        &["adopt", "apply", "--root", &root, "--scope", "."],
    );

    // Drop the pin so identity derives again — otherwise the pin alone would
    // explain the result and the test would measure the wrong thing.
    std::fs::remove_file(sandbox.path().join(".sddk/project-pin.json")).unwrap();

    run(
        &sandbox,
        &[
            "project",
            "alias",
            "--from",
            &from,
            "--to",
            &to,
            "--reason",
            "INC-DEBT-059 probe: the derived id was retired",
        ],
    );

    Aliased { sandbox, from, to }
}

fn root_of(a: &Aliased) -> String {
    a.sandbox.path().to_string_lossy().to_string()
}

/// **Criterion 3 of ADR-0152, on the `adopt` side.** `project resolve` already
/// declared the redirect (that half passes). `adopt status` did not, and not
/// only because the line was missing: the identity it received never went
/// through the table, so there was nothing to declare.
#[test]
fn adopt_status_follows_the_alias_and_declares_the_hop() {
    let a = aliased_checkout();
    let stdout = run_reporting(
        &a.sandbox,
        &["adopt", "status", "--root", &root_of(&a), "--scope", "."],
    );

    assert_eq!(
        field(&stdout, "project_id").as_deref(),
        Some(a.to.as_str()),
        "`adopt status` reporto el id RETIRADO, no el `to` del alias.\n\
         El store de alias no se consulta en esta ruta (INC-DEBT-059).\n\
         salida:\n{stdout}"
    );
    assert_eq!(
        field(&stdout, "identity_alias").as_deref(),
        Some(format!("{} -> {}", a.from, a.to).as_str()),
        "`adopt status` no nombro el `from` ni el `to` de la redireccion.\n\
         salida:\n{stdout}"
    );
}

/// The orphan ADR-0152 exists to stop re-creating. `adopt apply` on an aliased
/// checkout must converge on the surviving project, not adopt a second time
/// under the retired id.
#[test]
fn adopt_apply_does_not_write_a_second_receipt_under_the_retired_id() {
    let a = aliased_checkout();
    assert_eq!(
        adoption_receipt_ids(&a.sandbox),
        vec![a.to.clone()],
        "el estado de partida debe tener exactamente un recibo, bajo el `to`"
    );

    run(
        &a.sandbox,
        &["adopt", "apply", "--root", &root_of(&a), "--scope", "."],
    );

    let ids = adoption_receipt_ids(&a.sandbox);
    assert!(
        !ids.contains(&a.from),
        "`adopt apply` volvio a escribir un recibo de adopcion bajo el project_id \
         RETIRADO {}. Esa es exactamente la enfermedad que ADR-0152 viene a \
         cerrar, y el mecanismo que la evita la ha recreado.\nrecibos: {ids:?}",
        a.from
    );
    assert_eq!(
        ids,
        vec![a.to.clone()],
        "los recibos de adopcion deberian seguir siendo exactamente uno, bajo el `to`"
    );
}

/// The second surface, and the one that asserts a falsehood: it reported
/// `adoption: complete` **under the retired id** and wrote a session binding
/// into that id's data dir, which after the fix nothing would ever read.
#[test]
fn context_bootstrap_follows_the_alias_and_binds_the_surviving_project() {
    let a = aliased_checkout();
    let root = root_of(&a);
    let stdout = run_reporting(
        &a.sandbox,
        &[
            "context",
            "bootstrap",
            "--root",
            &root,
            "--scope",
            ".",
            "--session",
            "probe",
        ],
    );

    assert_eq!(
        field(&stdout, "project").as_deref(),
        Some(a.to.as_str()),
        "`context bootstrap` resolvio el id RETIRADO. Antes del arreglo de \
         INC-DEBT-059 decia `adoption: complete` sobre el, que es una \
         afirmacion falsa en positivo y no un error ruidoso.\nsalida:\n{stdout}"
    );
    assert!(
        !adoption_receipt_ids(&a.sandbox).contains(&a.from),
        "`context bootstrap` es el segundo escritor del mismo huerfano"
    );
}

/// Structural, CLI side, and it is the one that would have caught the fourth
/// resolver.
///
/// The engine-side test above cannot see this: `generation_destination` lives in
/// the CLI, calls `sddk_domain::resolve_project_identity` directly, and nobody
/// had looked. The falsifier of this lot found it while hunting for a *second*
/// resolver, after the adoption fix was already green — which is the point.
/// Reading the SCOPE did not turn it up; counting call sites did.
///
/// The property is an exact count, not a substring. `lib.rs` legitimately calls
/// `resolve_project_identity` inside `resolve_identity_honoring_pin_with`, so a
/// blanket "must not appear" would forbid the one call that is the authority.
/// "Exactly once" says what is meant: one decision point, and any other call
/// site is a second resolver wearing the same function name.
#[test]
fn the_cli_has_no_second_identity_resolver() {
    let canonical = include_str!("../src/lib.rs");
    let context = include_str!("../src/context_cmd.rs");

    let cli = production_code(canonical);
    let ctx = production_code(context);

    assert_eq!(
        ctx.matches("resolve_project_identity(").count(),
        0,
        "context_cmd.rs vuelve a resolver la identidad por su cuenta. Su doc de \
         antes —«with the SAME resolver as `adopt`»— era literalmente cierto y \
         exactamente lo contrario de lo que importa: ambos se saltaban la \
         autoridad igual, y la concordancia entre dos bypass no es convergencia."
    );
    // Counted WITH the parenthesis, so the `use` line does not inflate it: a
    // first pass counted the bare name, got 2, and reported a violation that
    // was the import. The count is of *calls*, and this is the second time in
    // this lot that a guard measured the wrong thing and said so.
    assert_eq!(
        cli.matches("resolve_project_identity(").count(),
        1,
        "lib.rs deberia tener UNA llamada a `resolve_project_identity`: la de \
         `resolve_identity_honoring_pin_with`. Hay mas de una, luego hay un \
         segundo resolutor. La cuenta va sobre el codigo sin comentarios: \
         documentacion no es codigo, y un doc que nombra la derivacion no la \
         hace."
    );
}

/// The production part of a source file, with comments removed.
///
/// Comments are documentation, not code: the same convention as
/// `version_source.rs::production_only`, and for the same reason. A comment
/// that *names* a function is not a call to it, so counting one as a violation
/// would make the guard unsatisfiable the moment someone explains themselves.
fn production_code(source: &str) -> String {
    let production = match source.find("#[cfg(test)]") {
        Some(i) => &source[..i],
        None => source,
    };
    production
        .lines()
        .filter(|line| {
            let t = line.trim_start();
            !t.starts_with("//!") && !t.starts_with("///") && !t.starts_with("//")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The fourth surface, found by the falsifier rather than by reading: it wrote
/// generated documentation into the retired project's data dir while
/// `project resolve` named the surviving one. `repro-c3d.sh` is the manual
/// version of this test.
#[test]
fn generate_docs_writes_under_the_surviving_project() {
    let a = aliased_checkout();
    let root = root_of(&a);
    run_reporting(&a.sandbox, &["generate", "docs", "--root", &root]);

    let retired = a.sandbox.xdg().xdg_data.join("sddk/projects").join(&a.from);
    assert!(
        !retired.join("generated").exists(),
        "`sddk generate docs` volvio a escribir bajo el project_id RETIRADO {}. \
         Documentacion generada para un proyecto al que el alias ya no lleva a \
         nadie.\n{}",
        a.from,
        retired.join("generated").display()
    );
    let surviving = a.sandbox.xdg().xdg_data.join("sddk/projects").join(&a.to);
    assert!(
        surviving.join("generated").exists(),
        "`sddk generate docs` deberia escribir bajo el `to` del alias {}.\n\
         existe: {:?}",
        a.to,
        surviving.join("generated").display()
    );
}

/// Structural, and it has to be: none of the tests above can see a *second*
/// resolver being reintroduced, because it behaves identically until the alias
/// happens to be declared. A reader that hardcodes a derivation is invisible to
/// every behavioural test here.
///
/// The property is crisp because after the fix the fields that fed the
/// derivation are **gone from the input struct**, so the call has nothing left
/// to call itself with. That is the point of moving the resolution rather than
/// adding a second path: with an optional field, someone re-adds the call and
/// nothing notices.
#[test]
fn the_engine_no_longer_derives_project_identity() {
    let source = include_str!("../../sddk-engine/src/adoption.rs");
    let production = match source.find("#[cfg(test)]") {
        Some(i) => &source[..i],
        None => source,
    };
    let code: String = production
        .lines()
        .filter(|line| {
            let t = line.trim_start();
            !t.starts_with("//!") && !t.starts_with("///") && !t.starts_with("//")
        })
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        !code.contains("resolve_project_identity"),
        "adoption.rs vuelve a derivar la identidad por su cuenta. El engine es \
         filesystem-free y no puede cargar la tabla de aliases, luego cualquier \
         identidad que produzca es sistemáticamente la PRE-alias: es el defecto \
         de INC-DEBT-059. La resolucion va antes, en la CLI, y entra ya resuelta."
    );
    assert!(
        !code.contains("pinned_project_id"),
        "adoption.rs vuelve a llevar un `pinned_project_id`. Reenviar el id \
         resuelto por ese campo no funciona: el engine lo trataria como pin, \
         `identity_source` no viajaria y `alias_origin` se perderia igual, un \
         nivel mas adentro y con una forma que PARECE correcta. \
         `context_cmd.rs` hacia exactamente eso y por eso un checkout con alias \
         y sin pin era el caso que se rompia."
    );
}
