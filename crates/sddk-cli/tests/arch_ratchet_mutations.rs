//! Architecture ratchets: legacy `ledger_events` exclusion (WU-C1.3/C1.4
//! history) and DDL-absence (C1.5/WU-C15-7).
//!
//! Since C1.5 (cycle p-63676b11dc0ef88f/c15-ledger-removal-2026-09-13), the
//! legacy `ledger_events` table is physically GONE: schema v20 drops it and
//! the only sanctioned mention of its name in the tree is the DROP block of
//! MIGRATION_20 itself. Domain events live exclusively in the canonical
//! `events_v1` stream (single authority, AGENTS.md §2.7).
//!
//! The DDL-absence ratchet (R-15-006) scans the storage schema source
//! comment-stripped and fails on ANY occurrence of the table name outside
//! the DROP block, with no allowlist and no exemptions: re-adding the DDL in
//! a future `MIGRATION_*` re-creates the dead surface and is caught here.
//! Mutation self-checks (CONFORMANCE-FITNESS-RATCHETS §mutation) prove the
//! classifier flags a `CREATE TABLE`/`INSERT INTO` injection while ignoring
//! comment-only mentions.
//!
//! WU-C2 (conf09 universal evidence) and WU-C3 (conf09b runtime status)
//! ratchets below are UNTOUCHED by C1.5 — they guard unrelated legacies.

use std::path::{Path, PathBuf};

/// The legacy table name, assembled via `concat!` so this file's own source
/// does not contain the contiguous fragment (self-flag avoidance).
const LEGACY_DDL_FRAGMENT: &str = concat!("ledger_", "events");

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

// ─────────────────────────────────────────────────────────────────────────────
// D4 ratchet — legacy `ledger_events` DDL-absence (C1.5/WU-C15-7, R-15-006).
//
// The table died in schema v20 (WU-C15-4). Its name may appear in
// `crates/sddk-storage/src/migrations.rs` ONLY inside the DROP block of
// MIGRATION_20 (every surviving occurrence is a DROP statement). Any other
// occurrence — a re-added CREATE TABLE in a future migration, an INSERT, a
// leftover reference — fails this ratchet. No allowlist, no exemptions:
// the exclusion is absolute because the surface no longer exists.
// ─────────────────────────────────────────────────────────────────────────────

/// THE RATCHET: the legacy table name survives in the schema source only as
/// DROP statements. Fails listing every offending line.
#[test]
fn ledger_events_ddl_is_absent_from_schema() {
    let root = workspace_root();
    let schema = root.join("crates/sddk-storage/src/migrations.rs");
    let content = std::fs::read_to_string(&schema).expect("read migrations.rs");

    let stripped = strip_rust_comments(&content);
    let mut offenders: Vec<(usize, &str)> = Vec::new();
    for (idx, line) in stripped.lines().enumerate() {
        let trimmed = line.trim_start();
        if line.contains(LEGACY_DDL_FRAGMENT)
            && !line.contains("DROP")
            && !trimmed.starts_with("--")
        {
            offenders.push((idx + 1, line.trim()));
        }
    }
    assert!(
        offenders.is_empty(),
        "arch ratchet violated (C1.5, R-15-006): the legacy table name may \
         appear in migrations.rs ONLY inside DROP statements of MIGRATION_20. \
         Domain events belong to the canonical events_v1 stream; re-adding \
         the legacy surface is forbidden. Offending lines: {offenders:?}"
    );

    // Positive control 1: the scan really sees the DROP block (guards
    // against a path/string drift making the ratchet vacuous).
    assert!(
        stripped.contains(LEGACY_DDL_FRAGMENT),
        "scanner must still observe the MIGRATION_20 DROP block"
    );

    // Positive control 2: the tree scan still covers the crate tree.
    let mut sources = Vec::new();
    rust_sources(&root.join("crates"), &mut sources);
    assert!(
        sources.len() > 100,
        "scanner must cover the crate tree, found only {} files",
        sources.len()
    );
}

/// Line-level classifier for the DDL-absence ratchet: any comment-stripped
/// line naming the legacy table outside a DROP statement offends. SQL
/// comments (`--`, which live inside Rust string literals and thus survive
/// the Rust comment stripper) are ignored like Rust comments. Exposed
/// (privately) so the mutation self-check can classify synthetic files
/// without touching the scanned tree.
fn line_offends(source: &str) -> bool {
    strip_rust_comments(source).lines().any(|line| {
        let trimmed = line.trim_start();
        line.contains(LEGACY_DDL_FRAGMENT) && !line.contains("DROP") && !trimmed.starts_with("--")
    })
}

/// MUTATION SELF-CHECK: the DDL-absence classifier flags a synthetic
/// re-creation (`CREATE TABLE` and `INSERT INTO`) and ignores comment-only
/// mentions. Synthetic files live in the OS tempdir, never in the tree.
#[test]
fn scanner_detects_injected_legacy_ddl() {
    let fragment = LEGACY_DDL_FRAGMENT;

    // 1. A re-added CREATE TABLE in any MIGRATION_* is an offender: the
    //    ratchet scans the whole migrations.rs, not just MIGRATION_20.
    let rogue_create = format!(
        "// Rogue migration injected by the mutation test.\npub const MIGRATION_XX: &str = \"CREATE TABLE {fragment} (id INTEGER);\";\n"
    );
    assert!(
        line_offends(&rogue_create),
        "scanner must detect a re-added CREATE TABLE in a new migration"
    );

    // 2. An INSERT INTO is equally forbidden (no dormant writer may return).
    let rogue_insert = format!(
        "// Rogue module injected by the mutation test.\npub fn rogue_write() {{\n    let sql = \"INSERT INTO {fragment} VALUES (?1)\";\n}}\n"
    );
    assert!(
        line_offends(&rogue_insert),
        "scanner must detect an injected INSERT INTO the retired table"
    );

    // 3. Comment-only mentions never offend (comment stripping is real).
    let comment_only = format!("// mentions {fragment} in a comment only\npub fn f() {{}}\n");
    assert!(
        !line_offends(&comment_only),
        "comment mentions must not trip the ratchet"
    );
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
const CONF09_TYPE_ALLOWLIST: [&str; 5] = [
    "crates/sddk-domain/src/planning/mod.rs",
    "crates/sddk-domain/src/lib.rs",
    "crates/sddk-engine/src/evidence_relation_mapping.rs",
    "crates/sddk-engine/src/spike_sp06.rs",
    // Post-A5-EVIDENCE-ATTACHMENT-MIGRATION-V1: the storage CRUD test
    // exercises the universal substrate end-to-end (CAS reopen, fail-closed
    // writes, legacy NULL-relation read path) and necessarily names the
    // legacy `PlanningEvidenceKind` and constructs variants to build
    // canonical-equivalent test fixtures. It is NOT a producer.
    "crates/sddk-storage/tests/planning_cas_crud.rs",
    // Note: concurrency_planning_substrate.rs (A5-SQLITE-CONCURRENCY-R) uses
    // only the universal `EvidenceAttachmentRecord`; it does NOT reference
    // the legacy `PlanningEvidenceKind` enum and therefore needs no entry
    // here. Constructors of the legacy enum are still confined to
    // CONF09_CONSTRUCTION_ALLOWLIST.
];

/// Closed constructor set for VARIANT CONSTRUCTION (narrower than
/// `CONF09_TYPE_ALLOWLIST`: decoders and the corpus may NAME the legacy
/// type, but only the compat module and the canonical mapping module may
/// CONSTRUCT its variants — plus the M3 reopen test, which constructs
/// variants to build a canonical-equivalent fixture).
const CONF09_CONSTRUCTION_ALLOWLIST: [&str; 3] = [
    "crates/sddk-domain/src/planning/mod.rs",
    "crates/sddk-engine/src/evidence_relation_mapping.rs",
    "crates/sddk-storage/tests/planning_cas_crud.rs",
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
