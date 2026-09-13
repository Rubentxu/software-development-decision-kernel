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

#![allow(deprecated)] // ratchet mutation tests exercise the deprecated forwarder by design (C1.3)

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
        if let Ok(content) = std::fs::read_to_string(&path)
            && file_offends(&path, root, &content)
        {
            offenders.push(path);
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
    let items: Vec<&str> = raw.split("symbol/path:").skip(1).collect();
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
        if let Ok(content) = std::fs::read_to_string(&path)
            && strip_rust_comments(&content).contains(LEGACY_READ_FRAGMENT)
        {
            offenders.push(path);
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
    let allowlisted: Vec<String> = doc.entries.iter().map(|e| e.symbol_path.clone()).collect();

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
        if let Ok(content) = std::fs::read_to_string(&path)
            && file_offends(&path, &root, &content)
        {
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

// ─────────────────────────────────────────────────────────────────────────────
// WU-C2 universal evidence cutover ratchets (DELTA-CONF-003 / design §2).
//
// Since the cutover, planning evidence is authored on the universal
// substrate (EvidenceRef + CoreRelationKind via
// `resolve_planning_evidence_relation` /
// `EvidenceAttachmentRecord::from_universal_relation`). The legacy
// `PlanningEvidenceKind` enum is a read-only compat type: its references
// are confined to the closed compat set (definition + re-export + mapping
// + decoders + corpus), and its VARIANT CONSTRUCTION is confined to the
// even narrower constructor set (the compat module and the canonical
// mapping module only).
//
// Two ratchets, both mutation self-checked (CONFORMANCE-FITNESS-RATCHETS
// §mutation):
//   1. conf09_universal_evidence_only — no reference to the legacy type
//      outside the closed compat set; E1 (plan.rs run_evidence) stays
//      redirected to the universal resolver.
//   2. conf09_no_planning_evidence_new_writes — no variant construction
//      outside the constructor compat set, so no new legacy-typed writes
//      can appear anywhere in the crate tree.
// ─────────────────────────────────────────────────────────────────────────────

/// The legacy type identifier, assembled so this file never contains it
/// contiguously (self-flag avoidance, same trick as `FORBIDDEN_FRAGMENT`).
const CONF09_TYPE_FRAGMENT: &str = concat!("Planning", "EvidenceKind");

/// The variant-construction prefix (`Type::`), narrower than the type
/// reference itself.
const CONF09_CONSTRUCTION_FRAGMENT: &str = concat!("Planning", "EvidenceKind", "::");

/// Closed compat set for TYPE REFERENCES (mirrors the `evidence_kind_v1`
/// lint exclude_paths in docs/architecture/lints/deprecated_patterns.toml;
/// keep the two in sync).
// Note: semantic_kind.rs cites the legacy name only inside a doc comment,
// which the comment-stripping scan does not see, so it needs no entry here
// (the evidence_kind_v1 lint exclude_paths keeps it for its raw-text scan).
// Note: spine_import.rs (the E4 import decoder) was fully migrated to the
// universal constructor in WU-C2 and no longer names the legacy type.
// semantic_kind.rs cites the legacy name only inside a doc comment, which
// the comment-stripping scan does not see. Neither needs an entry here
// (the evidence_kind_v1 lint exclude_paths keeps raw-text entries).
const CONF09_TYPE_ALLOWLIST: [&str; 4] = [
    "crates/sddk-domain/src/planning/mod.rs",
    "crates/sddk-domain/src/lib.rs",
    "crates/sddk-engine/src/evidence_relation_mapping.rs",
    "crates/sddk-engine/src/spike_sp06.rs",
];

/// Closed constructor set for VARIANT CONSTRUCTION (narrower than
/// `CONF09_TYPE_ALLOWLIST`: decoders and the corpus may NAME the legacy
/// type, but only the compat module and the canonical mapping module may
/// CONSTRUCT its variants).
const CONF09_CONSTRUCTION_ALLOWLIST: [&str; 2] = [
    "crates/sddk-domain/src/planning/mod.rs",
    "crates/sddk-engine/src/evidence_relation_mapping.rs",
];

/// Whether one file offends a conf09 scan: NOT in the given allowlist AND
/// its comment-stripped source contains the given fragment.
fn conf09_file_offends(
    path: &Path,
    root: &Path,
    content: &str,
    allowlist: &[&str],
    fragment: &str,
) -> bool {
    let declared = allowlist
        .iter()
        .any(|rel| path == root.join(Path::new(rel)));
    !declared && strip_rust_comments(content).contains(fragment)
}

/// Scans the crate tree for conf09 offenders under the given allowlist,
/// sorted.
fn scan_for_conf09_offenders(root: &Path, allowlist: &[&str], fragment: &str) -> Vec<PathBuf> {
    let mut sources = Vec::new();
    rust_sources(&root.join("crates"), &mut sources);
    let mut offenders = Vec::new();
    for path in sources {
        if let Ok(content) = std::fs::read_to_string(&path)
            && conf09_file_offends(&path, root, &content, allowlist, fragment)
        {
            offenders.push(path);
        }
    }
    offenders.sort();
    offenders
}

/// RATCHET 1 (WU-C2, DELTA-CONF-003): the legacy planning evidence type is
/// referenced ONLY from the closed compat set, and the E1 write path stays
/// redirected to the universal resolver.
#[test]
fn conf09_universal_evidence_only() {
    let root = workspace_root();

    let offenders = scan_for_conf09_offenders(&root, &CONF09_TYPE_ALLOWLIST, CONF09_TYPE_FRAGMENT);
    assert!(
        offenders.is_empty(),
        "WU-C2 ratchet violated: legacy planning evidence type referenced \
         outside the closed compat set. Author evidence via \
         resolve_planning_evidence_relation + \
         EvidenceAttachmentRecord::from_universal_relation (universal \
         substrate, DELTA-CONF-003). To extend the compat set, update BOTH \
         this allowlist and the evidence_kind_v1 lint exclude_paths, with an \
         8-field entry in the legacy-compat allowlist. Offending files: \
         {offenders:?}"
    );

    // Positive control 1: the scan actually visited the crate tree.
    let mut sources = Vec::new();
    rust_sources(&root.join("crates"), &mut sources);
    assert!(
        sources.len() > 100,
        "scanner must cover the crate tree, found only {} files",
        sources.len()
    );

    // Positive control 2: the E1 production write path is really redirected
    // (plan.rs must resolve through the universal relation resolver).
    let plan_rs = root.join("crates/sddk-cli/src/plan.rs");
    let plan_src = std::fs::read_to_string(&plan_rs).expect("read plan.rs");
    assert!(
        strip_rust_comments(&plan_src).contains("resolve_planning_evidence_relation"),
        "E1 (plan.rs run_evidence) must author evidence via the universal \
         resolver; the redirect regressed"
    );

    // Allowlist sanity: each compat file would offend if unlisted (they
    // really contain the fragment, so the allowlist is load-bearing).
    for rel in CONF09_TYPE_ALLOWLIST {
        let path = root.join(rel);
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read compat file {rel}: {e}"));
        assert!(
            strip_rust_comments(&content).contains(CONF09_TYPE_FRAGMENT),
            "compat file '{rel}' no longer contains the legacy type; \
             shrink the allowlist instead of keeping dead entries"
        );
    }
}

/// RATCHET 2 (WU-C2, DELTA-CONF-003): the legacy enum is never CONSTRUCTED
/// outside the compat constructor set, so no new legacy-typed writes can
/// appear (read-only compat, decode-only).
#[test]
fn conf09_no_planning_evidence_new_writes() {
    let root = workspace_root();

    let offenders = scan_for_conf09_offenders(
        &root,
        &CONF09_CONSTRUCTION_ALLOWLIST,
        CONF09_CONSTRUCTION_FRAGMENT,
    );
    assert!(
        offenders.is_empty(),
        "WU-C2 ratchet violated: legacy planning evidence variants \
         constructed outside the compat constructor set. New evidence \
         writes must go through EvidenceAttachmentRecord::\
         from_universal_relation (relation-tagged, universal substrate). \
         Offending files: {offenders:?}"
    );

    // Positive control: the compat constructor file really contains
    // variant constructions, so the narrow allowlist is load-bearing.
    let compat = root.join("crates/sddk-domain/src/planning/mod.rs");
    let content = std::fs::read_to_string(&compat).expect("read compat module");
    assert!(
        strip_rust_comments(&content).contains(CONF09_CONSTRUCTION_FRAGMENT),
        "compat constructor module must contain the construction fragment \
         for this allowlist entry to be meaningful"
    );
}

/// MUTATION SELF-CHECK for the conf09 classifiers: synthetic files (in the
/// OS tempdir, never inside the scanned tree) prove the detectors (1) flag
/// real violations, (2) do not flag comment-only mentions, and (3) do not
/// flag allowlisted paths even with real content.
#[test]
fn conf09_mutations_detect_injected_violations() {
    let root = workspace_root();
    let sandbox = std::env::temp_dir().join(MUTATION_SANDBOX_DIR);
    std::fs::create_dir_all(&sandbox).expect("create sandbox");
    let rogue = sandbox.join("rogue_evidence.rs");

    // 1. A rogue type reference in a non-allowlisted file offends the type
    //    ratchet... and also the (narrower-allowlist) construction ratchet
    //    when it constructs a variant.
    std::fs::write(
        &rogue,
        format!(
            r#"// Rogue module injected by the conf09 mutation test.
pub fn rogue_evidence() -> &'static str {{
    let kind: {} = Default::default();
    let _ = kind;
    "{}"
}}
"#,
            CONF09_TYPE_FRAGMENT, "legacy"
        ),
    )
    .expect("write violating file");
    let violating = std::fs::read_to_string(&rogue).expect("read violating file");
    assert!(
        conf09_file_offends(
            &rogue,
            &root,
            &violating,
            &CONF09_TYPE_ALLOWLIST,
            CONF09_TYPE_FRAGMENT
        ),
        "type ratchet must detect the injected reference"
    );

    // 2. A rogue VARIANT CONSTRUCTION offends the write ratchet even when the
    //    file would be innocent for a mere type mention.
    std::fs::write(
        &rogue,
        format!(
            r#"// Rogue module injected by the conf09 mutation test.
pub fn rogue_construction() -> u8 {{
    let v = {}::Log;
    v as u8
}}
"#,
            CONF09_TYPE_FRAGMENT
        ),
    )
    .expect("write construction violation");
    let construction = std::fs::read_to_string(&rogue).expect("read construction violation");
    assert!(
        conf09_file_offends(
            &rogue,
            &root,
            &construction,
            &CONF09_CONSTRUCTION_ALLOWLIST,
            CONF09_CONSTRUCTION_FRAGMENT
        ),
        "write ratchet must detect the injected construction"
    );

    // 3. Comment-only mentions never offend either classifier.
    let comment_only = format!(
        "// mentions {} and {}::Log in comments only\npub fn f() {{}}\n",
        CONF09_TYPE_FRAGMENT, CONF09_TYPE_FRAGMENT
    );
    assert!(
        !conf09_file_offends(
            &rogue,
            &root,
            &comment_only,
            &CONF09_TYPE_ALLOWLIST,
            CONF09_TYPE_FRAGMENT
        ),
        "comment mentions must not trip the type ratchet"
    );
    assert!(
        !conf09_file_offends(
            &rogue,
            &root,
            &comment_only,
            &CONF09_CONSTRUCTION_ALLOWLIST,
            CONF09_CONSTRUCTION_FRAGMENT
        ),
        "comment mentions must not trip the write ratchet"
    );

    // 4. Allowlisted files never offend, even with real fragment content.
    for rel in CONF09_TYPE_ALLOWLIST {
        assert!(
            !conf09_file_offends(
                &root.join(rel),
                &root,
                &construction,
                &CONF09_TYPE_ALLOWLIST,
                CONF09_TYPE_FRAGMENT
            ),
            "allowlisted compat file '{rel}' must not offend"
        );
    }

    let _ = std::fs::remove_dir_all(&sandbox);
}

// ─────────────────────────────────────────────────────────────────────────────
// conf09b — WU-C3 cycle/run lifecycle cutover (DELTA-CONF-004)
//
// Runtime-derived cycle statuses (Remediating, Recovering, UatWaiting,
// ApprovalPending) are DECODE-ONLY since the cutover: production code must
// not persist them as canonical Cycle truth. Truth for wait/remediation/
// recovery lives in Run/Authority facts (approval events, gate receipts,
// transition ledger); the runtime label is derived (cycle_summary).
//
// Ratchets:
//   1. conf09b_no_new_runtime_status_references — production code may not
//      reference the runtime-derived variants outside the decode-only compat
//      set (variant definition, pause compatibility, wire helpers).
// ─────────────────────────────────────────────────────────────────────────────

/// Fragment assembled so this file never contains it contiguously
/// (self-flag avoidance).
const CONF09B_FRAGMENT: &str = concat!("CycleStatus", "::Rem", "ediating");

/// Closed decode-only compat set for runtime-derived status references.
/// cycle.rs defines the variants (wire decode); cycle_pause.rs accepts them
/// as legacy pause sources; storage_error.rs/lib.rs (engine/storage) name
/// the guard errors; the cutover test pins the partition.
const CONF09B_ALLOWLIST: [&str; 10] = [
    "crates/sddk-domain/src/cycle.rs",
    "crates/sddk-domain/src/workflow.rs",
    "crates/sddk-engine/src/cycle_pause.rs",
    "crates/sddk-domain/src/models/storage_error.rs",
    "crates/sddk-engine/src/lib.rs",
    "crates/sddk-storage/src/lib.rs",
    "crates/sddk-domain/tests/sp04_cycle_status_slimming.rs",
    "crates/sddk-domain/tests/workflow_yaml.rs",
    "crates/sddk-engine/tests/runtime_cycle_status_cutover.rs",
    "crates/sddk-cli/tests/arch_ratchet_mutations.rs",
];

/// RATCHET (WU-C3, DELTA-CONF-004): runtime-derived cycle statuses are only
/// referenced from the closed decode-only compat set; production write paths
/// (workflow manifest validation, storage guards, derived summary) carry the
/// cutover, so no new `CycleStatus::<runtime>` references may appear.
#[test]
fn conf09b_no_new_runtime_status_references() {
    let root = workspace_root();

    let offenders = scan_for_conf09_offenders(&root, &CONF09B_ALLOWLIST, CONF09B_FRAGMENT);
    assert!(
        offenders.is_empty(),
        "WU-C3 ratchet violated: runtime-derived cycle status referenced \
         outside the decode-only compat set. Wait/remediation/recovery truth \
         belongs to Run/Authority facts; derive the runtime label via \
         sddk_engine::cycle_summary::derive_cycle_summary instead of naming \
         the variant. To extend the compat set, update CONF09B_ALLOWLIST with \
         justification. Offending files: {offenders:?}"
    );

    // Positive control: the scan visited the crate tree.
    let mut sources = Vec::new();
    rust_sources(&root.join("crates"), &mut sources);
    assert!(
        sources.len() > 100,
        "scanner must cover the crate tree, found only {} files",
        sources.len()
    );

    // Positive control: the compat definition file really contains the
    // fragment, so the allowlist entry is load-bearing.
    let domain = root.join("crates/sddk-domain/src/cycle.rs");
    let content = std::fs::read_to_string(&domain).expect("read cycle.rs");
    assert!(
        strip_rust_comments(&content).contains(CONF09B_FRAGMENT),
        "cycle.rs must contain the runtime variant reference for this \
         allowlist entry to be meaningful"
    );

    // Mutation injection: a rogue reference in a synthetic file (outside the
    // scanned tree) must offend the classifier.
    let sandbox = std::env::temp_dir().join(MUTATION_SANDBOX_DIR);
    std::fs::create_dir_all(&sandbox).expect("create sandbox");
    let rogue = sandbox.join("rogue_runtime_status.rs");
    std::fs::write(
        &rogue,
        format!("pub fn rogue() -> {} {{ todo!() }}\n", CONF09B_FRAGMENT),
    )
    .expect("write violating file");
    let violating = std::fs::read_to_string(&rogue).expect("read violating file");
    assert!(
        conf09_file_offends(
            &rogue,
            &root,
            &violating,
            &CONF09B_ALLOWLIST,
            CONF09B_FRAGMENT
        ),
        "ratchet must detect the injected runtime status reference"
    );

    // Comment-only mentions never offend.
    let comment_only = format!(
        "// mentions {} in a comment only\npub fn f() {{}}\n",
        CONF09B_FRAGMENT
    );
    assert!(
        !conf09_file_offends(
            &rogue,
            &root,
            &comment_only,
            &CONF09B_ALLOWLIST,
            CONF09B_FRAGMENT
        ),
        "comment mentions must not trip the ratchet"
    );

    let _ = std::fs::remove_dir_all(&sandbox);
}
