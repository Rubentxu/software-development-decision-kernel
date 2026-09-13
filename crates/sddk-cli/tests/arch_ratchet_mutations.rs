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
