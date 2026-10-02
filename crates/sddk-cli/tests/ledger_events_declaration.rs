//! R1–R4: `sddk ledger events` must declare what it did NOT show.
//!
//! INC-DEBT-060, falsificador **F63**. Measured on the real ledger of
//! `p-63676b11dc0ef88f` in session-69d, against a byte-identical copy:
//! 590 events exist, the command printed **50**, named **19 of 114** cycles,
//! **declared no total**, and exited **0**. The default window covered
//! sequences 12 to 20 — the 50 newest — so the 540 older ones were invisible
//! with nothing on screen saying so.
//!
//! These are **RED today**, at the CLI level on purpose: a storage-level test
//! calling a function that does not exist yet is a compile error, and a RED
//! bought by breaking the build takes the whole crate down with it.
//!
//! What they assert is only what SCOPE-CONTRACT froze: that a total is
//! declared, that the JSON carries it, that `--limit 0` means all, and that an
//! empty ledger says zero rather than nothing.

use std::path::{Path, PathBuf};
use std::process::Command;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sddk"))
}

const REMOTE: &str = "https://example.test/ledger-declaration-fixture";

struct Sandbox {
    dir: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("sddk-ledger-decl-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}

fn run(dir: &Path, args: &[&str]) -> (i32, String) {
    // XDG isolation plus `SDDK_DATA_DIR` removal. `SDDK_DATA_DIR` wins over
    // `XDG_DATA_HOME` and is inherited from the environment, so isolating the
    // XDG vars alone would let this write into the developer's real ledger.
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

/// `cycle start` needs the project and workspace rows adoption writes, so a bare
/// sandbox fails on a foreign key before it ever reaches the ledger. Adopting
/// first makes the fixture a real project with real events.
fn adopt(dir: &Path) {
    let (code, out) = run(
        dir,
        &[
            "adopt", "apply", "--root", ".", "--scope", ".", "--remote", REMOTE,
        ],
    );
    assert_eq!(code, 0, "adopt apply must succeed in the sandbox: {out}");
}

fn cycle_start(dir: &Path, name: &str) {
    let (code, out) = run(
        dir,
        &[
            "cycle", "start", "--root", ".", "--scope", ".", "--name", name, "--remote", REMOTE,
        ],
    );
    assert_eq!(code, 0, "cycle start must succeed: {out}");
}

fn events(dir: &Path, extra: &[&str]) -> (i32, String) {
    let mut args = vec!["ledger", "events", "--root", ".", "--scope", ".", "--remote", REMOTE];
    args.extend_from_slice(extra);
    run(dir, &args)
}

/// Counts the event lines the text renderer emits. A line is `<seq> <type>
/// <evt-id> frame:<frame> <cycle>` — which is why the first version of the F63
/// measurement matched **zero** events against a command that prints fifty: it
/// assumed a `sequence: N` key/value shape.
fn text_event_lines(out: &str) -> Vec<&str> {
    out.lines()
        .filter(|l| {
            let t = l.trim();
            !t.is_empty() && t.split_whitespace().next().map_or(false, |w| {
                !w.is_empty() && w.chars().all(|c| c.is_ascii_digit())
            })
        })
        .collect()
}

/// RED today: with fewer than 50 events nothing truncates, so a product that
/// only declared the total *when truncating* would print nothing here. That is
/// the whole point of R1: a declaration that appears only on the interesting
/// case cannot be read off a log where the interesting case did not happen.
#[test]
fn r1_text_declares_the_total_even_when_it_does_not_truncate() {
    let dir = Sandbox::new("no-truncate");
    adopt(&dir);
    cycle_start(&dir, "only");

    let (code, out) = events(&dir, &[]);

    assert_eq!(code, 0, "`ledger events` must exit 0: {out}");
    let printed = text_event_lines(&out);
    assert!(
        !printed.is_empty(),
        "the fixture must produce at least one event, or this test measures nothing: {out}"
    );
    assert!(
        out.contains(&format!("of {}", printed.len())),
        "the total must be declared even when nothing is truncated. A declaration \
         that only appears when truncating cannot be read off a log where it did \
         not. Got: {out}"
    );
}

/// RED today: this is F63 literally. Fewer events than the default limit still
/// has to *say so*, and the events that do exist still have to be listed.
#[test]
fn r2_text_names_every_event_when_it_fits() {
    let dir = Sandbox::new("fits");
    adopt(&dir);
    cycle_start(&dir, "one");
    cycle_start(&dir, "two");

    let (code, out) = events(&dir, &[]);

    assert_eq!(code, 0, "`ledger events` must exit 0: {out}");
    let printed = text_event_lines(&out);
    assert!(
        printed.len() >= 2,
        "the fixture must produce several events: {out}"
    );
    // The declared total must equal the number of events actually listed, so a
    // reader can tell "this is everything" from "this is a window".
    assert!(
        out.contains(&format!("of {}", printed.len())),
        "declared total must be the number of lines printed: {out}"
    );
}

/// RED today: the JSON payload is a bare array, and an array cannot carry a
/// total. There is nowhere in the current shape to declare the omission, which
/// is why this is a shape change and not an added field.
#[test]
fn r3_json_carries_total_shown_and_truncated() {
    let dir = Sandbox::new("json-shape");
    adopt(&dir);
    cycle_start(&dir, "alpha");

    let (code, out) = events(&dir, &["--format", "json"]);

    assert_eq!(code, 0, "`ledger events --format json` must exit 0: {out}");
    let v: serde_json::Value = serde_json::from_str(&out)
        .unwrap_or_else(|e| panic!("output must be valid JSON ({e}): {out}"));

    for field in ["events", "total_events", "shown", "truncated"] {
        assert!(
            v.get(field).is_some(),
            "the JSON payload must carry `{field}`. The whole point is that a \
             machine consumer can detect that it is looking at a window; a bare \
             array has nowhere to say it. Got: {out}"
        );
    }
    assert!(
        v["events"].is_array(),
        "`events` must be the array: {out}"
    );
    assert_eq!(
        v["shown"].as_u64(),
        v["events"].as_array().map(|a| a.len() as u64),
        "`shown` must match how many events are in the payload: {out}"
    );
    assert_eq!(
        v["total_events"].as_u64(),
        v["shown"].as_u64(),
        "a listing that does not truncate must report total == shown: {out}"
    );
    assert_eq!(
        v["truncated"].as_bool(),
        Some(false),
        "a listing that fits must not claim to be truncated: {out}"
    );
}

/// RED today, and this one is a trap rather than a rounding: `ledger export`
/// documents `--limit 0` as "all" and implements it (`ledger.rs:442-445`), while
/// `ledger events` treats `0` as *zero* and exits 0 with an empty listing. An
/// operator who knows the convention from `export` types `--limit 0` to
/// unblock themselves and gets nothing.
#[test]
fn r4_limit_zero_means_all_not_none() {
    let dir = Sandbox::new("limit-zero");
    adopt(&dir);
    cycle_start(&dir, "one");
    cycle_start(&dir, "two");
    cycle_start(&dir, "three");

    let (all_code, all_out) = events(&dir, &["--limit", "0", "--format", "json"]);
    assert_eq!(all_code, 0, "`--limit 0` must exit 0: {all_out}");
    let v: serde_json::Value =
        serde_json::from_str(&all_out).unwrap_or_else(|e| panic!("{e}: {all_out}"));

    let shown = v["shown"].as_u64().unwrap_or_else(|| {
        panic!("`--limit 0` must still declare `shown`; got: {all_out}")
    });
    assert!(
        shown > 0,
        "`--limit 0` must mean ALL, matching `ledger export --limit 0` \
         (ledger.rs:442-445). Today it means ZERO and exits 0, so anyone who \
         knows the convention from `export` gets an empty listing and no clue. \
         Got: {all_out}"
    );
    assert_eq!(
        v["truncated"].as_bool(),
        Some(false),
        "an unlimited listing is not truncated: {all_out}"
    );

    // And a bounded limit still bounds, while still declaring what it dropped.
    let (b_code, b_out) = events(&dir, &["--limit", "2", "--format", "json"]);
    assert_eq!(b_code, 0, "`--limit 2` must exit 0: {b_out}");
    let b: serde_json::Value =
        serde_json::from_str(&b_out).unwrap_or_else(|e| panic!("{e}: {b_out}"));
    assert_eq!(b["shown"].as_u64(), Some(2), "the limit must bound: {b_out}");
    assert_eq!(
        b["truncated"].as_bool(),
        Some(true),
        "a listing that dropped events must say so: {b_out}"
    );
}
