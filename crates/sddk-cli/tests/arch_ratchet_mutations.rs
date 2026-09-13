//! Architecture ratchet: `ledger_events` writers (WU-C1.3, C1-HARDDISABLE).
//!
//! Since the C1.3 hard-disable, the legacy `ledger_events` table must not
//! gain any new write site. The allowed set is closed:
//!
//! - `crates/sddk-storage/src/lib.rs` — module that owns the hard-disable
//!   (its `append_event_on` is `#[cfg(test)]`-gated and always returns the
//!   typed `LegacyDomainWriteForbidden` error; it performs NO insert).
//! - `crates/sddk-storage/src/migrations.rs` — schema DDL (CREATE TABLE /
//!   triggers / indexes), never runtime writes.
//! - `crates/sddk-storage/tests/canonical_parity.rs` and
//!   `crates/sddk-storage/tests/cross_ledger_consistency.rs` — read-side
//!   test fixtures that SEED the pre-cutover legacy corpus by SQL (the
//!   redirect removed `append_event` legacy writes in C1.2); they never
//!   write domain events through runtime code.
//!
//! This test scans the crate sources for the legacy INSERT fragment and
//! fails naming the offending files (ratchet). A second test injects a
//! synthetic violation into a temp file OUTSIDE the scanned tree and proves
//! the classifier detects it (mutation self-check,
//! CONFORMANCE-FITNESS-RATCHETS §mutation).

use std::path::{Path, PathBuf};

/// The literal SQL fragment that constitutes a legacy write. Built via
/// `concat!` so this file's own source does not contain the contiguous
/// fragment (the ratchet scans stripped code, not string literals, but the
/// const itself would otherwise self-flag).
const FORBIDDEN_FRAGMENT: &str = concat!("INSERT INTO ", "ledger_", "events");

/// Directory name ignored by the tree scan (mutation harness scratch space).
const MUTATION_SANDBOX_DIR: &str = "arch_ratchet_mutation_sandbox";

/// Workspace root, resolved from `CARGO_MANIFEST_DIR` (…/crates/sddk-cli).
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("CARGO_MANIFEST_DIR sits at <root>/crates/sddk-cli")
        .to_path_buf()
}

/// Recursively collects `*.rs` files under `dir`, skipping `target/`,
/// dot-directories, and the mutation sandbox.
fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if path.is_dir() {
            if name == "target" || name.starts_with('.') || name == MUTATION_SANDBOX_DIR {
                continue;
            }
            rust_sources(&path, out);
        } else if name.ends_with(".rs") {
            out.push(path);
        }
    }
}

/// Files allowed to contain the forbidden fragment: the hard-disable module,
/// the migration DDL, and the legacy-corpus seed fixtures (read-side tests).
fn is_allowed(path: &Path, root: &Path) -> bool {
    let allowed = [
        "crates/sddk-storage/src/lib.rs",
        "crates/sddk-storage/src/migrations.rs",
        "crates/sddk-storage/tests/canonical_parity.rs",
        "crates/sddk-storage/tests/cross_ledger_consistency.rs",
    ];
    allowed.iter().any(|rel| path == root.join(Path::new(rel)))
}

/// Removes `//`-line and `/* */`-block comments so documentation mentions of
/// the fragment (like the ones in this very file) do not trip the ratchet.
fn strip_rust_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut rest = source;
    while let Some(idx) = rest.find("//") {
        let (before, after) = rest.split_at(idx);
        out.push_str(before);
        let line_end = after.find('\n').unwrap_or(after.len());
        rest = &after[line_end..];
    }
    out.push_str(rest);
    while let Some(start) = out.find("/*") {
        match out[start..].find("*/").map(|e| start + e + 2) {
            Some(end) => out.replace_range(start..end, ""),
            None => break,
        }
    }
    out
}

/// Whether one file offends: NOT in the allowlist AND its comment-stripped
/// source contains the forbidden write fragment. Exposed (privately) so the
/// mutation self-check can classify synthetic files without touching the
/// scanned tree.
fn file_offends(path: &Path, root: &Path, content: &str) -> bool {
    !is_allowed(path, root) && strip_rust_comments(content).contains(FORBIDDEN_FRAGMENT)
}

/// Returns the offending files that contain a real legacy-write
/// ledger_events` statement, sorted.
fn scan_for_legacy_writers(root: &Path) -> Vec<PathBuf> {
    let mut sources = Vec::new();
    rust_sources(&root.join("crates"), &mut sources);
    let mut offenders = Vec::new();
    for path in sources {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if file_offends(&path, root, &content) {
                offenders.push(path);
            }
        }
    }
    offenders.sort();
    offenders
}

/// THE RATCHET: the crate tree is clean. Fails naming every offender.
#[test]
fn second_event_store_authority_is_rejected_mutation() {
    let root = workspace_root();
    let offenders = scan_for_legacy_writers(&root);
    assert!(
        offenders.is_empty(),
        "arch ratchet violated: new {} write sites are forbidden since \
         WU-C1.3 (C1-HARDDISABLE). Domain events belong to the canonical \
         events_v1 stream. Offending files: {offenders:?}",
        FORBIDDEN_FRAGMENT
    );
    // Positive control: the scan actually visited the crate tree (guards
    // against a silently-empty scan making the ratchet vacuous).
    let mut sources = Vec::new();
    rust_sources(&root.join("crates"), &mut sources);
    assert!(
        sources.len() > 100,
        "scanner must cover the crate tree, found only {} files",
        sources.len()
    );
    assert!(
        sources
            .iter()
            .any(|p| p.ends_with("crates/sddk-storage/src/lib.rs")),
        "scanner must include the hard-disable module"
    );
    // Allowlist sanity: the hard-disable module would offend if it were not
    // allowlisted (it documents the fragment in comments/stub names).
    let guard_module = root.join("crates/sddk-storage/src/lib.rs");
    let content = std::fs::read_to_string(&guard_module).expect("read hard-disable module");
    assert!(
        !file_offends(&guard_module, &root, &content),
        "hard-disable module must stay allowlisted"
    );
}

/// MUTATION SELF-CHECK: classifies synthetic files (in the OS tempdir, never
/// inside the scanned tree) and proves the detector (1) flags a real legacy
/// writer, (2) does NOT flag comment-only mentions, and (3) does not flag
/// allowlisted files even when they contain real SQL.
#[test]
fn scanner_detects_injected_legacy_writer_violation() {
    let root = workspace_root();
    let sandbox = std::env::temp_dir().join(MUTATION_SANDBOX_DIR);
    std::fs::create_dir_all(&sandbox).expect("create sandbox");
    let rogue = sandbox.join("rogue_writer.rs");

    // 1. A real violation: a legacy INSERT in a non-allowlisted file. The
    // fragment is assembled with concat! so this file never contains it
    // contiguously outside the comment-stripped scan view.
    std::fs::write(
        &rogue,
        format!(
            r#"// Rogue module injected by the mutation test.
pub fn rogue_write() {{
    let sql = "{} (event_id) VALUES (?1)";
    let _ = sql;
}}
"#,
            FORBIDDEN_FRAGMENT
        ),
    )
    .expect("write violating file");
    let violating = std::fs::read_to_string(&rogue).expect("read violating file");
    assert!(
        file_offends(&rogue, &root, &violating),
        "scanner must detect the injected violation"
    );

    // 2. A comment-only mention must NOT offend.
    let comment_only = format!(
        "// This file only mentions {} in comments.\npub fn f() {}{}\n",
        FORBIDDEN_FRAGMENT, '{', '}'
    );
    assert!(
        !file_offends(&rogue, &root, &comment_only),
        "comment mentions must not trip the ratchet"
    );

    // 3. Allowlisted files never offend, even with real SQL content.
    let allowed_path = root.join("crates/sddk-storage/tests/canonical_parity.rs");
    assert!(
        !file_offends(&allowed_path, &root, &violating),
        "allowlisted fixture must not offend"
    );

    let _ = std::fs::remove_dir_all(&sandbox);
}

// ─────────────────────────────────────────────────────────────────────────────
// WU-C1.4 read-only window ratchet (C1-READWINDOW, R-002.5).
//
// Since the redirect (C1.2) and hard-disable (C1.3), the legacy
// `ledger_events` table must be READ only, and only from the paths declared
// in docs/architecture/lints/legacy-compat-allowlist.yaml (8-field entries
// per CONFORMANCE-FITNESS-RATCHETS §Allowlist policy, read_or_write = read).
//
// This test:
//   1. Loads the allowlist YAML and enforces its structure (8 fields per
//      entry, read_or_write = "read", write entries forbidden).
//   2. Scans the workspace for READ references to the legacy table
//      (`load_all_ledger_events`) and requires every offender file to be
//      declared in the allowlist (allowlist creep guard).
//   3. Scans for the legacy WRITE fragment and requires every offender file
//      to be either the WU-C1.3 closed write set or an allowlisted test
//      fixture (no new write-capable entries).
// ─────────────────────────────────────────────────────────────────────────────

/// Path of the allowlist, relative to the workspace root.
const ALLOWLIST_REL: &str = "docs/architecture/lints/legacy-compat-allowlist.yaml";

/// The literal symbol that constitutes a legacy READ. Built via `concat!` so
/// this file's own source does not contain the contiguous fragment (same
/// self-flag avoidance as `FORBIDDEN_FRAGMENT`).
const LEGACY_READ_FRAGMENT: &str = concat!("load_all_", "ledger_events");

/// The 8 mandatory fields of every allowlist entry.
const ALLOWLIST_REQUIRED_FIELDS: [&str; 8] = [
    "symbol/path",
    "reason",
    "canonical_replacement",
    "read_or_write",
    "owner",
    "removal_trigger",
    "expiry/version",
    "parity_test",
];

/// One allowlist entry, deserialized leniently from the YAML.
#[derive(Debug, serde::Deserialize)]
struct AllowlistEntry {
    #[serde(rename = "symbol/path")]
    symbol_path: String,
    #[serde(rename = "read_or_write")]
    read_or_write: String,
}

#[derive(Debug, serde::Deserialize)]
struct AllowlistDoc {
    entries: Vec<AllowlistEntry>,
}

/// Loads and structurally validates the allowlist: every entry carries the
/// 8 ratchet fields, at least the decoder layer is declared, and no entry
/// claims write capability (forbidden at C7).
fn load_validated_allowlist(root: &Path) -> AllowlistDoc {
    let path = root.join(ALLOWLIST_REL);
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read allowlist {}: {e}", path.display()));
    // Structural check independent of serde field filtering: the doc splits
    // into `entries:` list items on the `symbol/path:` key (which each item
    // starts with, so it is verified by construction), and every item must
    // mention the remaining 7 mandatory field keys.
    let item_count = raw.matches("symbol/path:").count();
    let items: Vec<&str> = raw
        .split("symbol/path:")
        .skip(1)
        .collect();
    assert!(
        !items.is_empty() && items.len() == item_count,
        "allowlist must declare at least one entry"
    );
    for item in &items {
        for field in ALLOWLIST_REQUIRED_FIELDS.iter().skip(1) {
            assert!(
                item.contains(field),
                "allowlist entry is missing required field '{field}' \
                 (CONFORMANCE-FITNESS-RATCHETS §Allowlist policy)"
            );
        }
    }
    let doc: AllowlistDoc = serde_saphyr::from_str(&raw).expect("parse allowlist yaml");
    for entry in &doc.entries {
        assert!(
            entry.read_or_write == "read",
            "allowlist entry '{}' declares read_or_write = '{}'; \
             write-capable legacy entries are forbidden (C7)",
            entry.symbol_path,
            entry.read_or_write
        );
    }
    assert!(
        doc.entries
            .iter()
            .any(|e| e.symbol_path == "crates/sddk-storage/src/lib.rs"),
        "the decoder layer (sddk-storage lib.rs) must remain allowlisted"
    );
    doc
}

/// Scans the workspace for files containing the legacy read symbol
/// `load_all_ledger_events` (comment-stripped, like the writer scan) and
/// returns those NOT declared in the allowlist, sorted.
fn scan_for_undeclared_readers(root: &Path, allowlisted: &[String]) -> Vec<PathBuf> {
    let mut sources = Vec::new();
    rust_sources(&root.join("crates"), &mut sources);
    let mut offenders = Vec::new();
    for path in sources {
        let declared = allowlisted
            .iter()
            .any(|rel| path == root.join(Path::new(rel)));
        if declared {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(&path) {
            if strip_rust_comments(&content).contains(LEGACY_READ_FRAGMENT) {
                offenders.push(path);
            }
        }
    }
    offenders.sort();
    offenders
}

/// THE RATCHET (WU-C1.4, R-002.5): every static reference to the legacy
/// ledger read path lives inside the allowlist, no allowlist entry is
/// write-capable, and the legacy write surface stays closed.
#[test]
fn legacy_reads_only_via_readonly_decoder_allowlist() {
    let root = workspace_root();
    let doc = load_validated_allowlist(&root);
    let allowlisted: Vec<String> = doc
        .entries
        .iter()
        .map(|e| e.symbol_path.clone())
        .collect();

    // 1. Every reader of the legacy table is declared in the allowlist.
    let undeclared = scan_for_undeclared_readers(&root, &allowlisted);
    assert!(
        undeclared.is_empty(),
        "WU-C1.4 ratchet violated: files read the legacy ledger \
         (the read fragment) without an allowlist entry in \
         {ALLOWLIST_REL}. Add an 8-field entry with read_or_write = read, \
         or migrate to the canonical events_v1 stream. Offending files: \
         {undeclared:?}"
    );

    // 2. Positive control: the known decoder layer is really covered by the
    // scan (guards against allowlist paths drifting out of the scanned tree).
    for required in [
        "crates/sddk-storage/src/lib.rs",
        "crates/sddk-storage/src/graph_store.rs",
        "crates/sddk-cli/src/fork_cmd.rs",
        "crates/sddk-cli/src/telemetry.rs",
    ] {
        assert!(
            allowlisted.iter().any(|rel| rel == required),
            "allowlist must cover {required} (WU-C1.4 inventory, design §1.1)"
        );
    }

    // 3. The legacy WRITE fragment stays confined: any file containing it is
    // either the WU-C1.3 closed write set or an explicitly allowlisted
    // (read-only) fixture. Nothing new may enter either set silently.
    let write_set = [
        "crates/sddk-storage/src/lib.rs",
        "crates/sddk-storage/src/migrations.rs",
        "crates/sddk-storage/tests/canonical_parity.rs",
        "crates/sddk-storage/tests/cross_ledger_consistency.rs",
    ];
    let mut sources = Vec::new();
    rust_sources(&root.join("crates"), &mut sources);
    for path in sources {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if file_offends(&path, &root, &content) {
                let rel = path
                    .strip_prefix(&root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .to_string();
                assert!(
                    allowlisted.contains(&rel),
                    "legacy write fragment in '{rel}' is neither in the WU-C1.3 \
                     closed write set nor in the WU-C1.4 allowlist; the \
                     ledger_events write surface is CLOSED since C1.3"
                );
                assert!(
                    write_set.contains(&rel.as_str()),
                    "'{rel}' contains a legacy write but is not in the closed \
                     write set; write-capable allowlist entries are forbidden"
                );
            }
        }
    }
}
