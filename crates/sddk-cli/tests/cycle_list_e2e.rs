//! E2E: `sddk cycle list` must enumerate every cycle a project holds.
//!
//! INC-DEBT-060, D1. On `p-63676b11dc0ef88f` the ledger holds **179** rows in
//! `cycles` and **no command names 97 of them**, 91 of those claiming
//! `status: OPEN`. There is no `list_cycles` in `crates/sddk-storage` and no
//! `cycle list` in the CLI; `get_cycle(cycle_id)` exists but requires already
//! knowing the id, and nothing supplies it.
//!
//! These tests are **RED today** and that is the point: they call a
//! subcommand that does not exist, so they fail with "unrecognized subcommand"
//! rather than against an assertion about enumeration. They are written at the
//! CLI level on purpose — a storage-level test calling `list_cycles` would not
//! compile at all, and a RED bought by breaking the build takes the whole crate
//! down with it.
//!
//! What they are *not* is a licence to invent behaviour. They assert only what
//! the SCOPE-CONTRACT of `cl-cycle-enumeration` froze: every cycle is named,
//! the count is declared, and the listing is not the current-cycle resolution
//! wearing another name.

use std::path::{Path, PathBuf};
use std::process::Command;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sddk"))
}

const REMOTE: &str = "https://example.test/enumeration-fixture";

struct Sandbox {
    dir: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("sddk-cycle-list-{}-{}", name, std::process::id()));
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
    // XDG isolation plus `SDDK_DATA_DIR` removal, for the reason the other e2e
    // in this crate give: these commands resolve real storage paths and would
    // otherwise write into the developer's actual ledger. `SDDK_DATA_DIR` wins
    // over `XDG_DATA_HOME` and is inherited, so isolating the XDG vars alone is
    // not enough.
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

/// `cycle start` needs the project and workspace rows that adoption writes, so
/// a bare sandbox fails with a FOREIGN KEY error before it ever reaches the
/// cycle. Adopting first is what makes the fixture a real project instead of a
/// synthetic ledger nobody would ever have.
fn adopt(dir: &Path) -> String {
    let (code, out) = run(
        dir,
        &[
            "adopt", "apply", "--root", ".", "--scope", ".", "--remote", REMOTE,
        ],
    );
    assert_eq!(code, 0, "adopt apply must succeed in the sandbox: {out}");
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

fn list(dir: &Path) -> (i32, String) {
    run(
        dir,
        &[
            "cycle", "list", "--root", ".", "--scope", ".", "--remote", REMOTE,
        ],
    )
}

/// RED today: the subcommand does not exist. The assertion that matters is the
/// last one — every cycle must be **named**, not merely counted, because a
/// listing that reported `cycles: 3` without naming them would be an artifact
/// of the same family as the one this debt is about.
#[test]
fn cycle_list_names_every_cycle_the_project_holds() {
    let s = Sandbox::new("names-all");
    adopt(&s.dir);
    let ids: Vec<String> = ["alpha", "beta", "gamma"]
        .iter()
        .map(|n| start_cycle(&s.dir, n))
        .collect();

    let (code, out) = list(&s.dir);

    assert_eq!(
        code,
        0,
        "`sddk cycle list` must exit 0 on a project with {} cycles; it does not exist yet, \
         so this is RED. Output was: {out}",
        ids.len()
    );
    for id in &ids {
        assert!(
            out.contains(id.as_str()),
            "the listing must NAME cycle {id}, not just count it; got: {out}"
        );
    }
}

/// The distinction this debt turns on. `sddk cycle status` resolves a cycle
/// through a **live lease** and reports "no active cycle found" when there is
/// none — correct for the question it asks, and useless for "what does this
/// project hold?". On the real storage that difference hides 91 cycles that
/// claim to be OPEN. So the listing has to name cycles that are *not* the
/// current one, and it has to do it without turning into the ambiguity error
/// that lease resolution raises when it finds two.
#[test]
fn cycle_list_is_not_the_current_cycle_resolution() {
    let s = Sandbox::new("not-current");
    adopt(&s.dir);
    let first = start_cycle(&s.dir, "one");
    let second = start_cycle(&s.dir, "two");

    let (code, out) = list(&s.dir);

    assert_eq!(code, 0, "`sddk cycle list` must exit 0: {out}");
    assert!(
        out.contains(&first) && out.contains(&second),
        "two coexisting cycles must both be named. A listing that resolves to one of them \
         is the current-cycle resolution wearing another name, which is how 91 OPEN cycles \
         are invisible today. Got: {out}"
    );
    assert!(
        !out.to_lowercase().contains("ambiguous"),
        "enumeration must not raise the ambiguity error that lease-based resolution \
         raises; listing every cycle is not a choice between them. Got: {out}"
    );
}

/// The count has to be **declared**, and it has to match. A listing that prints
/// the cycles but not how many it examined answers a different question than
/// the one being asked — and silently truncating, which is what `ledger events`
/// does today at 50 of 590, is the failure mode this whole cycle exists to
/// avoid repeating.
#[test]
fn cycle_list_declares_a_count_that_matches_what_it_named() {
    let s = Sandbox::new("count");
    adopt(&s.dir);
    for name in ["one", "two"] {
        start_cycle(&s.dir, name);
    }

    let (code, out) = list(&s.dir);

    assert_eq!(code, 0, "`sddk cycle list` must exit 0: {out}");
    assert!(
        out.contains("cycles: 2"),
        "the listing must declare how many cycles it examined, and the number must \
         be right; a truncated or undeclared listing repeats the `ledger events` \
         defect (50 of 590, silently). Got: {out}"
    );
}
