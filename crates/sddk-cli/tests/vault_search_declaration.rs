//! R1–R5: `sddk vault search` must declare what it did NOT show.
//!
//! Same defect class as INC-DEBT-060 F63, measured in session-69f on the real
//! vault index of this machine: the command prints **20 of 75 documents**,
//! declares no total, exits 0, and `--limit 0` returns `no hits` because
//! `LIMIT 0` is not special in SQL — it returns zero rows, not all of them.
//!
//! These are **RED today**, at the CLI level. The fixture is a real vault with
//! real markdown and a real FTS index built by `vault index`, because a table
//! written by hand would not exercise the query path this is about.
//!
//! What they assert is only what SCOPE-CONTRACT froze: that a total is declared,
//! that the JSON carries it, that `--limit 0` means all, and that a search with
//! no matches says zero rather than nothing.

use std::path::{Path, PathBuf};
use std::process::Command;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sddk"))
}

struct Sandbox {
    root: PathBuf,
    vault: PathBuf,
    db: PathBuf,
}

impl Sandbox {
    /// `docs` documents whose body all mentions `TERM-Auth`, so one query can be
    /// made to match any number of them without changing the query text. The
    /// default `--limit` is 20, so `docs > 20` is what makes the truncation case
    /// real rather than hypothetical.
    fn new(name: &str, docs: usize) -> Self {
        let root = std::env::temp_dir().join(format!(
            "sddk-vault-decl-{}-{}",
            name,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let vault = root.join("vault");
        std::fs::create_dir_all(vault.join("terms")).unwrap();
        for i in 0..docs {
            std::fs::write(
                vault.join(format!("terms/TERM-{i:03}.md")),
                format!(
                    "---\nid: TERM-{i:03}\ntype: term\nstatus: active\n---\n# Term {i}\n\nOAuth token exchange for service {i}.\n"
                ),
            )
            .unwrap();
        }
        // One document that shares nothing with the others, so "no matches" is
        // reachable with a real index rather than an empty vault.
        std::fs::write(
            vault.join("terms/TERM-SOLO.md"),
            "---\nid: TERM-SOLO\ntype: term\nstatus: active\n---\n# Solo\n\nUnrelated content about cryptography.\n",
        )
        .unwrap();
        let db = root.join("index.sqlite");
        let s = Sandbox { root, vault, db };
        s.index();
        s
    }

    fn run(&self, args: &[&str]) -> (i32, String) {
        let out = Command::new(bin())
            .args(args)
            .args([
                "--root",
                self.root.to_str().unwrap(),
                "--scope",
                ".",
                "--fallback-seed",
                "00000000-0000-0000-0000-000000000001",
            ])
            .env_remove("SDDK_DATA_DIR")
            .env_remove("SDDK_STATE_HOME")
            .env("XDG_DATA_HOME", self.root.join(".xdg/data"))
            .env("XDG_STATE_HOME", self.root.join(".xdg/state"))
            .env("XDG_CACHE_HOME", self.root.join(".xdg/cache"))
            .current_dir(&self.root)
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

    fn index(&self) {
        let (code, out) = self.run(&[
            "vault",
            "index",
            "--vault",
            self.vault.to_str().unwrap(),
            "--db",
            self.db.to_str().unwrap(),
        ]);
        assert_eq!(code, 0, "vault index must build a real FTS index: {out}");
    }

    fn search(&self, query: &str, extra: &[&str]) -> (i32, String) {
        let mut args = vec![
            "vault",
            "search",
            "--db",
            self.db.to_str().unwrap(),
            "--query",
            query,
        ];
        args.extend_from_slice(extra);
        self.run(&args)
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// Counts the hit lines the text renderer emits. One line is `<id> <kind> <path>`.
/// The declaration line is excluded by name rather than by shape, because
/// guessing the shape is exactly what made the F63 measurement read zero events
/// against a command that prints fifty.
fn hit_lines(out: &str) -> Vec<&str> {
    out.lines()
        .filter(|l| {
            let t = l.trim();
            !t.is_empty() && !t.starts_with("hits:")
        })
        .collect()
}

fn declared_total(out: &str) -> Option<u64> {
    for line in out.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("hits:") {
            // `hits: <shown> of <total> (truncated|complete)`
            let mut parts = rest.split_whitespace();
            let _shown = parts.next()?;
            parts.next()?; // `of`
            return parts.next()?.parse().ok();
        }
    }
    None
}

/// RED today: with fewer documents than the default limit nothing truncates, so
/// a product that declared the total *only when truncating* would print nothing
/// here — and "nothing was truncated" is exactly the claim a reader needs to be
/// able to check off a log.
#[test]
fn r1_declares_the_total_even_when_it_does_not_truncate() {
    let s = Sandbox::new("no-truncate", 5);

    let (code, out) = s.search("crypto", &[]);

    assert_eq!(code, 0, "`vault search` must exit 0: {out}");
    assert_eq!(
        hit_lines(&out).len(),
        1,
        "the fixture must produce exactly one hit for 'crypto': {out}"
    );
    assert_eq!(
        declared_total(&out),
        Some(1),
        "the total must be declared even when nothing is truncated: {out}"
    );
}

/// RED today: this is F63 in another surface. More documents than the default
/// limit, and the output must both list them and say that it did not list all.
#[test]
fn r2_declares_and_flags_when_it_truncates() {
    let s = Sandbox::new("truncates", 30);

    let (code, out) = s.search("token", &[]);

    assert_eq!(code, 0, "`vault search` must exit 0: {out}");
    assert_eq!(
        hit_lines(&out).len(),
        20,
        "the default limit is 20 and must bound: {out}"
    );
    assert_eq!(
        declared_total(&out),
        Some(30),
        "the declared total must be every matching document, not the page: {out}"
    );
    assert!(
        out.contains("truncated"),
        "a listing that dropped 10 of 30 must say so: {out}"
    );
}

/// RED today: the payload is a bare array, and an array has nowhere to put a
/// total. That is why the fix is an envelope rather than an added key.
#[test]
fn r3_json_carries_total_shown_and_truncated() {
    let s = Sandbox::new("json", 30);

    let (code, out) = s.search("token", &["--format", "json"]);

    assert_eq!(code, 0, "`vault search --format json` must exit 0: {out}");
    let v: serde_json::Value =
        serde_json::from_str(&out).unwrap_or_else(|e| panic!("valid JSON ({e}): {out}"));

    for field in ["hits", "total_hits", "shown", "truncated"] {
        assert!(
            v.get(field).is_some(),
            "the JSON payload must carry `{field}`: {out}"
        );
    }
    assert!(v["hits"].is_array(), "`hits` must be the array: {out}");
    assert_eq!(v["shown"].as_u64(), Some(20), "{out}");
    assert_eq!(v["total_hits"].as_u64(), Some(30), "{out}");
    assert_eq!(v["truncated"].as_bool(), Some(true), "{out}");
}

/// RED today, and this is the trap: `LIMIT 0` is not special in SQL, so `--limit
/// 0` returns zero rows. Three commands in this same binary document `0` as
/// "all" — `ledger export --limit 0`, `ledger events --limit 0`, `ledger watch
/// --max-events 0` — and this is the fourth.
#[test]
fn r4_limit_zero_means_all_not_none() {
    let s = Sandbox::new("limit-zero", 30);

    let (code, out) = s.search("token", &["--limit", "0", "--format", "json"]);

    assert_eq!(code, 0, "`--limit 0` must exit 0: {out}");
    let v: serde_json::Value =
        serde_json::from_str(&out).unwrap_or_else(|e| panic!("valid JSON ({e}): {out}"));
    assert_eq!(
        v["shown"].as_u64(),
        Some(30),
        "`--limit 0` must mean ALL, matching the other three commands in this \
         binary. Today it means ZERO, because `LIMIT 0` is not special in SQL. \
         Got: {out}"
    );
    assert_eq!(v["truncated"].as_bool(), Some(false), "{out}");

    // And a bounded limit still bounds while still declaring what it dropped.
    let (b_code, b_out) = s.search("token", &["--limit", "5", "--format", "json"]);
    assert_eq!(b_code, 0, "`--limit 5` must exit 0: {b_out}");
    let b: serde_json::Value =
        serde_json::from_str(&b_out).unwrap_or_else(|e| panic!("valid JSON ({e}): {b_out}"));
    assert_eq!(b["shown"].as_u64(), Some(5), "{b_out}");
    assert_eq!(b["truncated"].as_bool(), Some(true), "{b_out}");
}

/// RED today, and it is a different failure from R1: R1 is the path with
/// results, this is the path without them. A bare `no hits` says "there was
/// nothing to find"; what it should say is "there were zero matches and nothing
/// was left out", because those are different claims and only one of them is
/// usually true.
#[test]
fn r5_no_matches_declares_zero() {
    let s = Sandbox::new("no-matches", 3);

    let (code, out) = s.search("zzzznothingmatchesthis", &[]);

    assert_eq!(code, 0, "a search with no matches must exit 0: {out}");
    assert!(
        hit_lines(&out).is_empty(),
        "no match must mean no hit lines: {out}"
    );
    assert_eq!(
        declared_total(&out),
        Some(0),
        "zero matches must be declared as zero, not replaced by a bare \
         `no hits` that reads as 'there was nothing to look at': {out}"
    );
}

/// Silence is the one thing this must never produce: an exit-0 empty output
/// that says nothing is indistinguishable from a broken install.
#[test]
fn r6_never_returns_empty_output_for_a_search_that_ran() {
    let s = Sandbox::new("never-silent", 3);

    for (query, label) in [("crypto", "with matches"), ("zzzznone", "without matches")] {
        let (code, out) = s.search(query, &[]);
        assert_eq!(code, 0, "`vault search` must exit 0 ({label}): {out}");
        assert!(
            !out.trim().is_empty(),
            "a search that ran must always say something ({label}); \
             an empty exit-0 output is indistinguishable from a broken install: {out:?}"
        );
    }
    let _ = Path::new(".");
}
