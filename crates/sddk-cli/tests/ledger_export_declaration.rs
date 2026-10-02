//! R1–R5: `sddk ledger export` must declare how many events EXISTED, not only
//! how many it wrote — and must be readable by a machine at all.
//!
//! Measured on the real ledger of `p-63676b11dc0ef88f` in session-69m, against
//! a byte-identical copy: **600** events exist, `--limit 5` writes 5 and answers
//! `exported 5 events to <path>`, and the 595 it left out are never mentioned.
//! The command exits **0**.
//!
//! This is INC-DEBT-060 / F63 in a fourth surface, and it is the worst of them
//! for one reason the other two are not: the other two truncate **on screen**,
//! where a reader can see there is a limit. This one truncates **into a file**.
//! The result is an artifact that looks complete and that another process
//! consumes with no signal that 99 % of it is missing.
//!
//! A second, separate fact came out of the same measurement and is what R2 is
//! about: `ExportOutput` **derives `Serialize` and is never serialized**. The
//! summary is a hand-written `format!` and the command has no `--format` at
//! all, so a machine cannot read `export`'s answer because there is no shape to
//! read it in. The declared form exists; it is just not the one in force.
//!
//! **These are RED today, at the CLI level, on purpose.** A storage-level test
//! calling a function that does not exist is a compile error, and a RED bought
//! by breaking the build takes the whole crate down with it.
//!
//! The reference total is never hard-coded: it is read from `ledger events`,
//! the sibling surface that already declares. So these tests compare two
//! commands against each other instead of against a literal that would rot with
//! the fixture.

use std::path::{Path, PathBuf};
use std::process::Command;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sddk"))
}

const REMOTE: &str = "https://example.test/ledger-export-declaration-fixture";

struct Sandbox {
    dir: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("sddk-exp-decl-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Sandbox { dir }
    }

    fn path(&self) -> &Path {
        &self.dir
    }

    /// A path inside the sandbox: the exported file must never land in the repo.
    fn out(&self, name: &str) -> String {
        self.dir.join(name).to_string_lossy().into_owned()
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

/// Returns the real `cycle_id` the command minted, read out of its own output.
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

fn events_json(dir: &Path) -> serde_json::Value {
    let (code, out) = run(
        dir,
        &[
            "ledger", "events", "--root", ".", "--scope", ".", "--remote", REMOTE, "--limit", "0",
            "--format", "json",
        ],
    );
    assert_eq!(code, 0, "`ledger events` must exit 0: {out}");
    serde_json::from_str(&out)
        .unwrap_or_else(|e| panic!("`ledger events` must print JSON ({e}): {out}"))
}

/// The authoritative total, read from the sibling that already declares it.
fn real_total(dir: &Path) -> u64 {
    events_json(dir)["total_events"]
        .as_u64()
        .unwrap_or_else(|| panic!("`ledger events` must declare `total_events`"))
}

fn export(dir: &Path, out_file: &str, extra: &[&str]) -> (i32, String) {
    let mut args = vec![
        "ledger", "export", "--root", ".", "--scope", ".", "--remote", REMOTE, "--output", out_file,
    ];
    args.extend_from_slice(extra);
    run(dir, &args)
}

fn written_lines(path: &str) -> Vec<String> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(str::to_string)
        .collect()
}

/// The declaration sentence, with the path removed: the digits of a temp
/// directory are not part of what the command declares, and counting them
/// measures the path instead of the sentence. Already learned once this
/// session — the guard, not the product.
fn declaration_phrase(out: &str) -> String {
    out.trim()
        .lines()
        .last()
        .unwrap_or("")
        .split(" to ")
        .next()
        .unwrap_or("")
        .to_string()
}

/// RED today: with the cap above the total nothing is left out — which is
/// exactly when a declaration that only appears on the interesting case goes
/// missing. An untruncated export is the case nobody suspects.
#[test]
fn r1_text_declares_the_total_even_when_nothing_is_left_out() {
    let s = Sandbox::new("no-left-out");
    adopt(s.path());
    cycle_start(s.path(), "one");
    cycle_start(s.path(), "two");

    let total = real_total(s.path());
    assert!(total > 0, "the fixture must produce events: {total}");

    let out = s.out("export.jsonl");
    let cap = (total + 10).to_string();
    let (code, stdout) = export(s.path(), &out, &["--limit", &cap]);

    assert_eq!(code, 0, "`ledger export` must exit 0: {stdout}");
    assert_eq!(
        written_lines(&out).len() as u64,
        total,
        "the cap is above the total, so everything must be written: {stdout}"
    );
    assert!(
        declaration_phrase(&stdout).contains(&format!("of {total}")),
        "the summary must declare how many events existed ({total}), not only how \
         many were written. A declaration that only appears when the limit bites \
         cannot be read off a log where it did not. Got: {stdout}"
    );
}

/// RED today, and this one is about the FORM, not the number: `--format` is an
/// unknown argument, so a machine cannot read `export`'s answer at all. It is
/// also the guard for O3: the JSON has to come from `ExportOutput`, the struct
/// that already derives `Serialize`, and a test that only checks the payload
/// would not notice a second, parallel serializer.
#[test]
fn r2_json_format_exists_and_carries_the_declaration() {
    let s = Sandbox::new("json-shape");
    adopt(s.path());
    cycle_start(s.path(), "alpha");

    let total = real_total(s.path());
    let out = s.out("export.jsonl");
    let (code, stdout) = export(s.path(), &out, &["--limit", "1", "--format", "json"]);

    assert_eq!(
        code, 0,
        "`ledger export --format json` must be accepted. Today the argument does \
         not exist, so there is no shape a machine could read the answer in. \
         Got: {stdout}"
    );
    let v: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("the summary must be valid JSON ({e}): {stdout}"));

    for field in ["written", "total_events", "pending"] {
        assert!(
            v.get(field).is_some(),
            "the summary must carry `{field}`. A consumer of a file that looks \
             complete has no way to know it is not. Got: {stdout}"
        );
    }
    assert_eq!(
        v["total_events"].as_u64(),
        Some(total),
        "the declared total must be the real one ({total}), read from the sibling \
         surface that already declares it. Got: {stdout}"
    );
    assert_eq!(
        v["written"].as_u64(),
        Some(1),
        "`--limit 1` writes one event: {stdout}"
    );
}

/// RED today. And this is the guard for "derived, not hand-written": the three
/// numbers are produced by the same subtraction, and a `pending` typed in by
/// hand passes R1 and R2 — both of which only require the fields to exist and
/// the total to be right — and dies here.
#[test]
fn r3_the_three_numbers_close() {
    let s = Sandbox::new("closure");
    adopt(s.path());
    cycle_start(s.path(), "one");
    cycle_start(s.path(), "two");
    cycle_start(s.path(), "three");

    let out = s.out("export.jsonl");
    let (code, stdout) = export(s.path(), &out, &["--limit", "2", "--format", "json"]);
    assert_eq!(
        code, 0,
        "`ledger export --format json` must exit 0: {stdout}"
    );

    let v: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("the summary must be valid JSON ({e}): {stdout}"));
    let written = v["written"].as_u64().expect("written");
    let total = v["total_events"].as_u64().expect("total_events");
    let pending = v["pending"].as_u64().expect("pending");

    assert!(
        total >= written,
        "the total cannot be smaller than what was written: written={written} \
         total={total}. Got: {stdout}"
    );
    assert_eq!(
        written + pending,
        total,
        "`pending` is derived, not a third number written by hand: \
         written={written} + pending={pending} must equal total={total}. Got: {stdout}"
    );
    assert!(
        pending > 0,
        "this fixture caps at 2 with more available, so `pending` must be positive \
         — otherwise the assertion above is satisfied by three zeroes. Got: {stdout}"
    );
    assert_eq!(
        written_lines(&out).len() as u64,
        written,
        "the number the summary declares must be the number of lines in the \
         FILE. Those are two declarations of the same fact and they must agree. \
         Got: {stdout}"
    );
}

/// RED today, and this is the guard that makes the cycle worth its name: a
/// cheaper count that ignores the filter is not a cheaper declaration, it is a
/// false one. `--cycle X` must report the events of X, not of the ledger.
#[test]
fn r4_the_total_belongs_to_the_queried_cycle() {
    let s = Sandbox::new("cycle-filter");
    adopt(s.path());
    let a = cycle_start(s.path(), "alpha");
    let b = cycle_start(s.path(), "beta");
    let c = cycle_start(s.path(), "gamma");

    let listing = events_json(s.path());
    let events = listing["events"].as_array().expect("events array");
    let total = listing["total_events"].as_u64().expect("total_events");
    let count_of = |id: &str| -> u64 {
        events
            .iter()
            .filter(|e| e["cycle_id"].as_str() == Some(id))
            .count() as u64
    };
    // The cycle with the fewest events, so `total != that` can actually fail.
    let smallest = [
        (a.as_str(), count_of(&a)),
        (b.as_str(), count_of(&b)),
        (c.as_str(), count_of(&c)),
    ]
    .into_iter()
    .min_by_key(|(_, n)| *n)
    .expect("at least one cycle with events");
    let (small_id, small) = smallest;

    assert!(
        small > 0 && small < total,
        "the fixture needs a cycle smaller than the whole ledger, or this test \
         cannot fail: small={small} total={total}"
    );

    let out = s.out("export.jsonl");
    let (code, stdout) = export(
        s.path(),
        &out,
        &["--cycle", small_id, "--limit", "1", "--format", "json"],
    );
    assert_eq!(code, 0, "`ledger export --cycle` must exit 0: {stdout}");

    let v: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("the summary must be valid JSON ({e}): {stdout}"));
    assert_eq!(
        v["total_events"].as_u64(),
        Some(small),
        "with `--cycle {small_id}` the total must be the number of events OF THAT \
         CYCLE ({small}), not the number in the whole ledger ({total}). Got: {stdout}"
    );
}

/// **Structural, not behavioural** — and it is the only guard for O3.
///
/// O3 says the form in force must be `ExportOutput`, the struct that already
/// derived `Serialize`. R2 checks the *payload* of that JSON, and it cannot
/// tell `ExportOutput` from a hand-written `json!` beside it: both produce the
/// same bytes, so both make R1–R5 pass. The defect this cycle closes started
/// exactly there — a struct that declared a shape the command never used — and
/// the way back to it is writing `json!` and leaving the struct dead again.
///
/// A test that reads the source is the only kind that can see that, and it is
/// the fourth structural guard of this series. It is deliberately narrow: it
/// looks at the body of `run_ledger_export`, because a `json!` in another
/// function of the same file is not a second serializer of *this* summary —
/// the sixth time in this session that looking at the whole file found another
/// command's code.
#[test]
fn r6_the_json_summary_comes_from_export_output_not_a_parallel_literal() {
    let src = include_str!("../src/ledger.rs");
    let body = src
        .split("fn run_ledger_export")
        .nth(1)
        .expect("run_ledger_export must exist")
        .split("\nfn ")
        .next()
        .expect("its body must be delimited");

    assert!(
        body.contains("ExportOutput"),
        "the summary of `ledger export` must be an `ExportOutput`; the function \
         does not even name it, so the JSON is coming from somewhere else: {body}"
    );
    assert!(
        body.contains("to_string(&output)"),
        "the JSON must be produced by serializing the `ExportOutput` itself, not \
         by a hand-written literal. A `json!` beside the struct would leave the \
         struct dead again, which is the defect this cycle closes. Got: {body}"
    );
    assert!(
        !body.contains("json!"),
        "there is no second, hand-written form of this summary. Two renderers \
         that spell their own numbers are two rules about the same fact and can \
         drift; that is the whole reason the struct is serialized directly. \
         Got: {body}"
    );
}

/// **Characterisation, not RED** — declared as such, because calling it RED
/// would be a lie: the payload is correct today and must stay correct.
///
/// The point is that the remedy touches the **summary**, not the file. A fix
/// that "improved" the export by writing a JSON array instead of JSONL, or by
/// reordering, would pass R1–R4 and break every consumer of the file. This is
/// the guard that says the artifact is unchanged.
#[test]
fn r5_the_payload_stays_jsonl_one_event_per_line_ascending() {
    let s = Sandbox::new("payload");
    adopt(s.path());
    cycle_start(s.path(), "one");
    cycle_start(s.path(), "two");
    cycle_start(s.path(), "three");

    let out = s.out("export.jsonl");
    let (code, stdout) = export(s.path(), &out, &["--limit", "0"]);
    assert_eq!(code, 0, "`ledger export --limit 0` must exit 0: {stdout}");

    let lines = written_lines(&out);
    assert!(
        lines.len() >= 3,
        "the fixture must produce several events: {}",
        lines.len()
    );
    for line in &lines {
        let v: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("every line must be a JSON object ({e}): {line}"));
        assert!(
            v["event_id"].is_string() && v["sequence"].is_i64(),
            "each line is one ledger event, not a wrapper: {line}"
        );
    }
    let seqs: Vec<i64> = lines
        .iter()
        .map(|l| {
            serde_json::from_str::<serde_json::Value>(l).unwrap()["sequence"]
                .as_i64()
                .unwrap()
        })
        .collect();
    let mut sorted = seqs.clone();
    sorted.sort_unstable();
    assert_eq!(
        seqs, sorted,
        "the file is written in ascending sequence order, and this fix must not \
         change that. Got: {seqs:?}"
    );
}
