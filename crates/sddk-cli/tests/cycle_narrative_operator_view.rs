//! E2E: the operator view must DERIVE its claims, not assert them.
//!
//! INC-DEBT-067. `sddk cycle narrative` describes itself as the *operator
//! view* — the form in which a person reads a cycle's state. It used to make
//! two central claims that were not derived from anything:
//!
//! - `"Cycle completed."` was a string literal used as the default for
//!   `what_was_done`, and it never consulted the cycle's `status`. Every
//!   status rendered the same sentence, so a `CLOSED` cycle and an `OPEN` one
//!   were indistinguishable in the one surface a human reads.
//! - `"Nada por ahora."` is the footer printed when `human_action_required`
//!   is `None`, and that field had **no producer anywhere in the workspace**:
//!   declared, set to `None` by the constructor, read once by the footer, and
//!   never given a `Some(..)`. A view that structurally cannot say "an action
//!   is required" says "no action is required" to everyone.
//!
//! And worse than an absent derivation, measured on the real repo before the
//! fix: the command never opened the store at all — its `environment`
//! parameter was literally unused — so it rendered "Cycle completed." with
//! **exit 0 for a cycle id that does not exist**. That is not a weak view, it
//! is a false one, and it fails open in the direction that hides work.
//!
//! These tests assert the three properties that were violated, each against
//! a real ledger built by the same commands an operator would run.

use std::path::{Path, PathBuf};
use std::process::Command;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sddk"))
}

const REMOTE: &str = "https://example.test/narrative-fixture";

struct Sandbox {
    dir: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "sddk-cycle-narrative-{name}-{}",
            std::process::id()
        ));
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

fn run(dir: &Path, args: &[&str]) -> (i32, String) {
    let xdg = dir.join(".xdg");
    let out = Command::new(bin())
        .args(args)
        .env_remove("SDDK_DATA_DIR")
        .env_remove("SDDK_STATE_HOME")
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

fn adopt(dir: &Path) -> String {
    let (code, out) = run(
        dir,
        &[
            "adopt", "apply", "--root", ".", "--scope", ".", "--remote", REMOTE,
        ],
    );
    assert_eq!(code, 0, "adopt apply must succeed: {out}");
    out.lines()
        .find_map(|l| l.strip_prefix("project_id: "))
        .expect("adopt apply prints project_id")
        .trim()
        .to_string()
}

fn start_cycle(dir: &Path, name: &str) -> String {
    let (code, out) = run(
        dir,
        &[
            "cycle", "start", "--root", ".", "--scope", ".", "--name", name, "--remote", REMOTE,
        ],
    );
    assert_eq!(code, 0, "cycle start must succeed: {out}");
    out.lines()
        .find_map(|l| l.strip_prefix("cycle_id: "))
        .expect("cycle start prints cycle_id")
        .trim()
        .to_string()
}

fn narrative(dir: &Path, cycle: &str) -> (i32, String) {
    run(
        dir,
        &[
            "cycle",
            "narrative",
            "--root",
            ".",
            "--scope",
            ".",
            "--remote",
            REMOTE,
            "--cycle",
            cycle,
        ],
    )
}

/// A cycle that does not exist must not get a narrative. Before, it got
/// "Cycle completed." with exit 0 — a confident completion claim about an
/// object that was never there, which is the worst failure direction a view
/// can have: it tells the operator their work is done.
#[test]
fn a_cycle_that_does_not_exist_fails_closed_without_a_narrative() {
    let s = Sandbox::new("not-found");
    adopt(&s.dir);

    for bogus in [
        "no-existe-este-ciclo",
        "p-0000000000000000/ciclo-que-no-existe",
    ] {
        let (code, out) = narrative(&s.dir, bogus);
        assert_ne!(code, 0, "{bogus:?} must fail closed, not narrate: {out}");
        assert!(
            !out.contains("Cycle completed."),
            "{bogus:?} was narrated as completed: {out}"
        );
        assert!(
            !out.contains("Necesito de ti"),
            "{bogus:?} emitted an operator footer for a cycle that does not exist: {out}"
        );
    }
}

/// The property INC-DEBT-067 names: a live cycle is never described as
/// completed. The constant this replaces is the literal the debt record
/// quotes, so the assertion is written against that literal — restoring the
/// old default makes this test fall.
#[test]
fn a_started_cycle_is_never_narrated_as_completed() {
    let s = Sandbox::new("open");
    adopt(&s.dir);
    let cycle = start_cycle(&s.dir, "narrative-open");

    let (code, out) = narrative(&s.dir, &cycle);
    assert_eq!(code, 0, "the narrative of a real cycle must render: {out}");
    assert!(
        !out.contains("Cycle completed."),
        "a freshly started cycle was narrated as completed: {out}"
    );
    assert!(
        out.contains("is open"),
        "the narrative must state what the cycle IS, derived from its status: {out}"
    );
}

/// The view must be about the cycle it was asked about. A template that
/// ignores its input would satisfy "not completed" while describing nothing,
/// so the cycle id and the phase have to be in the sentence.
#[test]
fn the_narrative_is_about_the_cycle_it_was_asked_about() {
    let s = Sandbox::new("subject");
    adopt(&s.dir);
    let a = start_cycle(&s.dir, "alpha");
    let b = start_cycle(&s.dir, "beta");

    let (code_a, out_a) = narrative(&s.dir, &a);
    let (code_b, out_b) = narrative(&s.dir, &b);
    assert_eq!((code_a, code_b), (0, 0));
    assert!(
        out_a.contains(&a),
        "{a} missing from its own narrative:\n{out_a}"
    );
    assert!(
        out_b.contains(&b),
        "{b} missing from its own narrative:\n{out_b}"
    );
}

/// The footer had no producer, so it could only ever say "Nada por ahora.".
///
/// The property under test is that the footer FOLLOWS THE FACT, and the
/// fact is walked through its whole lifecycle here: no lease → live lease →
/// no lease. Before the fix all three rendered the same sentence, so the
/// middle state and the two ends were indistinguishable.
///
/// Measured, not assumed: `cycle start` does NOT leave a lease behind
/// across processes (`cycle status` reports `lease: none` afterwards), so
/// the first assertion is the one that discriminates — a freshly started
/// OPEN cycle genuinely needs a lease, and the old footer claimed it did
/// not. `cycle lock release` requires `--owner` AND `--fencing-token`; a
/// release that silently fails would leave the middle state standing and
/// make the last assertion vacuous, so its output is checked.
#[test]
fn the_operator_footer_follows_the_lease() {
    let s = Sandbox::new("lease");
    adopt(&s.dir);
    let cycle = start_cycle(&s.dir, "lease-cycle");

    // 1. No lease. The old footer said "Nada por ahora." here.
    let (code_no, none) = narrative(&s.dir, &cycle);
    assert_eq!(code_no, 0, "narrative without a lease: {none}");
    assert!(
        !none.contains("Nada por ahora."),
        "an OPEN cycle with no lease must not claim nothing is needed: {none}"
    );
    assert!(
        none.contains("lease"),
        "the footer must name what is missing: {none}"
    );

    // 2. Live lease. Now, and only now, nothing is needed.
    let (code_acq, acq) = run(
        &s.dir,
        &[
            "cycle",
            "lock",
            "acquire",
            "--owner",
            "narrative-test",
            "--root",
            ".",
            "--scope",
            ".",
            "--remote",
            REMOTE,
            "--cycle",
            &cycle,
        ],
    );
    assert_eq!(code_acq, 0, "acquiring the lease must succeed: {acq}");
    let token = acq
        .split_whitespace()
        .find_map(|tok| tok.strip_prefix("fencing_token="))
        .expect("lock acquire prints the fencing token it minted")
        .to_string();

    let (code_live, live) = narrative(&s.dir, &cycle);
    assert_eq!(code_live, 0, "narrative with a live lease: {live}");
    assert!(
        live.contains("Nada por ahora."),
        "a cycle holding a live lease needs nothing: {live}"
    );

    // 3. Back to no lease, with the fencing token the acquire reported.
    let (code_rel, rel) = run(
        &s.dir,
        &[
            "cycle",
            "lock",
            "release",
            "--owner",
            "narrative-test",
            "--fencing-token",
            &token,
            "--root",
            ".",
            "--scope",
            ".",
            "--remote",
            REMOTE,
            "--cycle",
            &cycle,
        ],
    );
    assert_eq!(code_rel, 0, "releasing the lease must succeed: {rel}");

    let (code_after, after) = narrative(&s.dir, &cycle);
    assert_eq!(
        code_after, 0,
        "narrative after releasing the lease: {after}"
    );
    assert!(
        after.contains("lease") && !after.contains("Nada por ahora."),
        "the footer must follow the lease back to needing one: {after}"
    );
}

/// The caller override still wins, but over a DERIVED default. Before, the
/// constant was the answer for every case and the override was the only
/// escape, which is what the debt record rejected as a fix.
#[test]
fn an_explicit_override_still_wins_over_the_derived_sentence() {
    let s = Sandbox::new("override");
    adopt(&s.dir);
    let cycle = start_cycle(&s.dir, "override-cycle");

    let (code, out) = run(
        &s.dir,
        &[
            "cycle",
            "narrative",
            "--root",
            ".",
            "--scope",
            ".",
            "--remote",
            REMOTE,
            "--cycle",
            &cycle,
            "--what-was-done",
            "Cerrado a mano por el operador.",
        ],
    );
    assert_eq!(code, 0, "an explicit override must still render: {out}");
    assert!(
        out.contains("Cerrado a mano por el operador."),
        "the caller override must be honoured: {out}"
    );
    assert!(
        !out.contains("is open"),
        "the override replaces the derived sentence: {out}"
    );
}
