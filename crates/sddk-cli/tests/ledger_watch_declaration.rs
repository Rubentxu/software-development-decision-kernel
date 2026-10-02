//! R1–R5: `sddk ledger watch` must declare how many events EXISTED, not only
//! how many it emitted.
//!
//! Measured on the real ledger of `p-63676b11dc0ef88f` in session-69l, against
//! a byte-identical copy: **591** events exist, `--max-events 5` prints
//! `[watch] emitted 5 events, exiting` and `{"__watch_complete":true,
//! "emitted":5}`, and the 586 unseen events are never mentioned. The command
//! does declare — that is true and it is why session-69f called it "the model
//! of correct behaviour" — but **declaring that it emitted N is not declaring
//! that there were M**.
//!
//! This is INC-DEBT-060 / F63 in a third surface, and it is F63 *by
//! construction*: `Storage::list_events_after` (sddk-storage/src/lib.rs:1022)
//! walks every stream and then `.take(limit)`s, throwing the length away on
//! every poll, from a `canonical_events()` that has already loaded the whole
//! ledger (`lib.rs:982`).
//!
//! **These are RED today, at the CLI level, on purpose.** A storage-level test
//! calling a function that does not exist is a compile error, and a RED bought
//! by breaking the build takes the whole crate down.
//!
//! What they assert is only what SCOPE-CONTRACT froze: that a total is
//! declared, that the JSON carries it, that the arithmetic closes, and — the
//! two that carry the cycle — that the total is the one for **this query**:
//! the cycle filter (R4) and the start cursor (R5). R1 and R2 can both be
//! satisfied by printing some number; R4 and R5 are what stop that number from
//! being the wrong one.
//!
//! The reference total is never hard-coded: it is read from `ledger events`,
//! the sibling surface that already declares (`total_events`, F63). So these
//! tests compare two commands against each other instead of against a literal
//! that would rot with the fixture.

use std::path::{Path, PathBuf};
use std::process::Command;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sddk"))
}

const REMOTE: &str = "https://example.test/ledger-watch-declaration-fixture";

struct Sandbox {
    dir: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir()
            .join(format!("sddk-watch-decl-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Sandbox { dir }
    }

    fn path(&self) -> &Path {
        &self.dir
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn run(dir: &Path, args: &[&str]) -> (i32, String) {
    // `SDDK_DATA_DIR` wins over `XDG_DATA_HOME` and is inherited, so isolating
    // the XDG vars alone would let this write into the real ledger.
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
/// sandbox fails on a foreign key before it ever reaches the ledger.
fn adopt(dir: &Path) {
    let (code, out) = run(
        dir,
        &[
            "adopt", "apply", "--root", ".", "--scope", ".", "--remote", REMOTE,
        ],
    );
    assert_eq!(code, 0, "adopt apply must succeed in the sandbox: {out}");
}

/// Returns the real `cycle_id` the command minted, read out of its own output
/// rather than reconstructed from the name: the id is derived from the remote,
/// so guessing it here would make R4 test a cycle that does not exist.
fn cycle_start(dir: &Path, name: &str) -> String {
    let (code, out) = run(
        dir,
        &[
            "cycle", "start", "--root", ".", "--scope", ".", "--name", name, "--remote", REMOTE,
        ],
    );
    assert_eq!(code, 0, "cycle start must succeed: {out}");
    out.lines()
        .find_map(|l| l.trim().strip_prefix("cycle_id: "))
        .unwrap_or_else(|| panic!("cycle start must print its cycle_id: {out}"))
        .trim()
        .to_string()
}

fn events_json(dir: &Path) -> (i32, serde_json::Value) {
    let (code, out) = run(
        dir,
        &[
            "ledger", "events", "--root", ".", "--scope", ".", "--remote", REMOTE, "--limit", "0",
            "--format", "json",
        ],
    );
    assert_eq!(code, 0, "`ledger events` must exit 0: {out}");
    let v = serde_json::from_str(&out)
        .unwrap_or_else(|e| panic!("`ledger events` must print JSON ({e}): {out}"));
    (code, v)
}

/// The authoritative total, read from the sibling that already declares it.
fn real_total(dir: &Path) -> u64 {
    let (_, v) = events_json(dir);
    v["total_events"]
        .as_u64()
        .unwrap_or_else(|| panic!("`ledger events` must declare `total_events`"))
}

/// `watch` has no other exit: `--max-events` is the only condition that ends a
/// run against a quiet ledger, and the 5-minute safety bound is not a test
/// strategy. So **every** call carries a cap and an idle timeout. Omitting them
/// hangs the suite for five minutes instead of failing.
fn watch(dir: &Path, extra: &[&str]) -> (i32, String) {
    let mut args = vec![
        "ledger", "watch", "--root", ".", "--scope", ".", "--remote", REMOTE, "--interval-ms",
        "20", "--idle-timeout-ms", "400",
    ];
    args.extend_from_slice(extra);
    run(dir, &args)
}

/// The control lines are the ones the command writes about itself. Event lines
/// start with the sequence number and carry prose, so counting "all numbers in
/// the output" would measure the payload — which is what the first version of
/// the F63 measurement did when it matched zero events against a command that
/// printed fifty.
fn control_lines(out: &str) -> Vec<&str> {
    out.lines()
        .filter(|l| l.trim().starts_with("[watch]"))
        .collect()
}

fn json_summary(out: &str) -> serde_json::Value {
    out.lines()
        .rev()
        .find_map(|l| {
            let t = l.trim();
            if t.is_empty() {
                return None;
            }
            serde_json::from_str::<serde_json::Value>(t).ok()
        })
        .unwrap_or_else(|| panic!("`ledger watch --format json` must print JSON lines: {out}"))
}

/// RED today: with the cap above every event, nothing is left out — and that is
/// precisely when a declaration that only appears on the interesting case goes
/// missing. F63's R1 said the same thing for `ledger events`, and it is the
/// reason this test exists rather than a `truncated` assertion.
#[test]
fn r1_text_declares_the_total_even_when_nothing_is_left_out() {
    let s = Sandbox::new("no-left-out");
    adopt(s.path());
    cycle_start(s.path(), "one");
    cycle_start(s.path(), "two");

    let total = real_total(s.path());
    assert!(total > 0, "the fixture must produce events: {total}");

    // Cap above the total, so the run emits everything and leaves nothing.
    let cap = (total + 10).to_string();
    let (code, out) = watch(s.path(), &["--max-events", &cap]);

    assert_eq!(code, 0, "`ledger watch` must exit 0: {out}");
    let ctrl = control_lines(&out);
    assert!(
        !ctrl.is_empty(),
        "`ledger watch` must write a control line about itself: {out}"
    );
    assert!(
        ctrl.iter().any(|l| l.contains(&format!("of {total}"))),
        "the footer must declare how many events existed ({total}), not only how \
         many were emitted. A declaration that only appears when events are left \
         out cannot be read off a log where they were not. Got: {out}"
    );
}

/// RED today, and this one is a shape change: the summary is a hand-written JSON
/// object literal with one key, so there is nowhere to put a total.
#[test]
fn r2_json_carries_total_and_pending() {
    let s = Sandbox::new("json-shape");
    adopt(s.path());
    cycle_start(s.path(), "alpha");

    let total = real_total(s.path());
    let (code, out) = watch(s.path(), &["--max-events", "1", "--format", "json"]);

    assert_eq!(code, 0, "`ledger watch --format json` must exit 0: {out}");
    let summary = json_summary(&out);
    assert_eq!(
        summary["__watch_complete"].as_bool(),
        Some(true),
        "the summary marker must be kept: it is what tells a NDJSON consumer \
         that the last line is the summary and not an event. Got: {out}"
    );
    for field in ["emitted", "total_events", "pending"] {
        assert!(
            summary.get(field).is_some(),
            "the summary must carry `{field}`. A machine consumer cannot detect \
             that it is looking at a window if the window does not say so. Got: {out}"
        );
    }
    assert_eq!(
        summary["total_events"].as_u64(),
        Some(total),
        "the declared total must be the real one ({total}), read from the \
         sibling surface that already declares it. Got: {out}"
    );
}

/// RED today. And this is the guard for O2: `pending` is **derived**, so the
/// three numbers cannot drift apart. A `pending` written by hand passes R1 and
/// R2 — both of which only require the fields to exist and the total to be
/// right — and dies here.
#[test]
fn r3_the_three_numbers_close() {
    let s = Sandbox::new("closure");
    adopt(s.path());
    cycle_start(s.path(), "one");
    cycle_start(s.path(), "two");
    cycle_start(s.path(), "three");

    let (code, out) = watch(s.path(), &["--max-events", "2", "--format", "json"]);
    assert_eq!(code, 0, "`ledger watch --format json` must exit 0: {out}");
    let summary = json_summary(&out);

    let emitted = summary["emitted"].as_u64().expect("emitted");
    let total = summary["total_events"].as_u64().expect("total_events");
    let pending = summary["pending"].as_u64().expect("pending");

    assert!(
        total >= emitted,
        "the total cannot be smaller than what was emitted: emitted={emitted} \
         total={total}. Got: {out}"
    );
    assert_eq!(
        emitted + pending,
        total,
        "`pending` is derived, not a third number written by hand: \
         emitted={emitted} + pending={pending} must equal total={total}. Got: {out}"
    );
    assert!(
        pending > 0,
        "this fixture caps at 2 with more events available, so `pending` must be \
         positive — otherwise the assertion above is satisfied by three zeroes. \
         Got: {out}"
    );
}

/// RED today, and this is the guard that makes the cycle worth its name.
///
/// `COUNT(*)` is 51,8x cheaper than materialising the stream and was measured
/// as such. It is also wrong here: with `--cycle` it would declare the whole
/// ledger. A fix that prints *a* number passes R1 and R2; this is what stops it.
#[test]
fn r4_the_total_belongs_to_the_queried_cycle() {
    let s = Sandbox::new("cycle-filter");
    adopt(s.path());
    cycle_start(s.path(), "alpha");
    cycle_start(s.path(), "beta");
    cycle_start(s.path(), "gamma");

    let (_, listing) = events_json(s.path());
    let events = listing["events"].as_array().expect("events array");
    let total = listing["total_events"].as_u64().expect("total_events");

    // The cycle with the fewest events, in one pass. Its total must be that
    // number, and the number of the whole ledger would be a declaration that is
    // plainly false.
    let mut per_cycle: std::collections::BTreeMap<&str, u64> = std::collections::BTreeMap::new();
    for e in events {
        let id = e["cycle_id"].as_str().expect("cycle_id");
        *per_cycle.entry(id).or_insert(0) += 1;
    }
    let (smallest_id, &small) = per_cycle
        .iter()
        .min_by_key(|(_, n)| *n)
        .expect("at least one cycle with events");
    let smallest_id = smallest_id.to_string();

    assert!(
        small > 0 && small < total,
        "the fixture needs a cycle smaller than the whole ledger, or this test \
         cannot fail: small={small} total={total} over {per_cycle:?}"
    );

    let (code, out) = watch(
        s.path(),
        &["--cycle", &smallest_id, "--max-events", "1", "--format", "json"],
    );
    assert_eq!(code, 0, "`ledger watch --cycle` must exit 0: {out}");
    let summary = json_summary(&out);
    let declared = summary["total_events"].as_u64().expect("total_events");

    assert_eq!(
        declared, small,
        "with `--cycle {smallest_id}` the total must be the number of events OF \
         THAT CYCLE ({small}), not the number in the whole ledger ({total}). A \
         count that ignores the filter is not a cheaper declaration, it is a \
         false one. Got: {out}"
    );
}

/// **Characterisation, not RED** — and it is declared as such rather than called
/// RED, which would be a lie: the emitter's filter works today.
///
/// R4 checks the *number* the filter produces. This checks the *events*: a
/// mutation that drops `apply_watch_filters` from the poll loop while leaving
/// it on the count emits every cycle's events while declaring the total for
/// one, and R1–R5 all still pass — the arithmetic closes, the total is right,
/// the number is right. Only the stream itself is wrong. It is the mirror of
/// R4, and without it the guard has a direction it cannot see.
#[test]
fn r6_the_filter_bounds_what_is_actually_emitted() {
    let s = Sandbox::new("emitted-bounded");
    adopt(s.path());
    cycle_start(s.path(), "alpha");
    cycle_start(s.path(), "beta");

    let (_, listing) = events_json(s.path());
    let events = listing["events"].as_array().expect("events array");
    let mut per_cycle: std::collections::BTreeMap<&str, u64> = std::collections::BTreeMap::new();
    for e in events {
        *per_cycle
            .entry(e["cycle_id"].as_str().expect("cycle_id"))
            .or_insert(0) += 1;
    }
    let (target, _) = per_cycle
        .iter()
        .min_by_key(|(_, n)| **n)
        .expect("at least one cycle");
    let target = target.to_string();

    let (code, out) = watch(
        s.path(),
        &["--cycle", &target, "--max-events", "1", "--format", "json"],
    );
    assert_eq!(code, 0, "`ledger watch --cycle` must exit 0: {out}");

    // Every event line — not the summary, which carries no `cycle_id`.
    let emitted: Vec<&str> = out
        .lines()
        .filter(|l| l.contains(r#""event_id""#))
        .collect();
    assert_eq!(
        emitted.len(),
        1,
        "the cap is 1, so exactly one event may be emitted: {out}"
    );
    assert!(
        emitted[0].contains(&format!(r#""cycle_id":"{target}""#)),
        "every emitted event must belong to the queried cycle `{target}`. A run \
         that emits other cycles' events while declaring that cycle's total is \
         making a false declaration about the stream it just produced. Got: {out}"
    );
}

///
/// RED today, and this is the other half of R4: the **cursor**.
///
/// Sequences are **per stream** (C1.5), and every `cycle start` opens its own
/// frame, so a fixture of N cycles has N events that all carry `sequence: 1`.
/// That is not a nuisance to work around, it is the fact the assertion rests
/// on: a cursor above the highest sequence must yield an empty query, and the
/// only way to get that wrong is to ignore the cursor.
///
/// The first version of this test asked for a cursor that split the ledger in
/// two and could not: with every sequence at 1 there is no value that leaves
/// "some but not all" behind, and the test died on its own fixture rather than
/// on the defect. It failed loudly, which is the point of failing loudly — but
/// a RED that measures its own fixture measures nothing.
#[test]
fn r5_the_total_respects_the_start_cursor() {
    let s = Sandbox::new("cursor");
    adopt(s.path());
    cycle_start(s.path(), "one");
    cycle_start(s.path(), "two");
    cycle_start(s.path(), "three");

    let (_, listing) = events_json(s.path());
    let events = listing["events"].as_array().expect("events array");
    let total = listing["total_events"].as_u64().expect("total_events");
    assert!(total > 1, "the fixture must hold several events: {total}");

    // A cursor above every sequence: the query this run describes is empty.
    let highest = events
        .iter()
        .map(|e| e["sequence"].as_i64().expect("sequence"))
        .max()
        .expect("max sequence");
    let expected = events
        .iter()
        .filter(|e| e["sequence"].as_i64().expect("sequence") > highest)
        .count() as u64;
    assert_eq!(
        expected, 0,
        "a cursor above the highest sequence must select nothing, or this test \
         is not testing what it claims: highest={highest} expected={expected}"
    );

    let cursor = highest.to_string();
    let (code, out) = watch(
        s.path(),
        &["--from-sequence", &cursor, "--max-events", "1", "--format", "json"],
    );
    assert_eq!(code, 0, "`ledger watch --from-sequence` must exit 0: {out}");
    let summary = json_summary(&out);

    assert_eq!(
        summary["emitted"].as_u64(),
        Some(0),
        "nothing is after the cursor, so nothing may be emitted: {out}"
    );
    assert_eq!(
        summary["total_events"].as_u64(),
        Some(0),
        "with `--from-sequence {cursor}` (above the highest sequence) the total \
         must count only events after the cursor, which is none — not the {total} \
         the ledger holds from the beginning. A total that ignores the cursor is \
         off by exactly the events the operator asked to skip. Got: {out}"
    );
}
