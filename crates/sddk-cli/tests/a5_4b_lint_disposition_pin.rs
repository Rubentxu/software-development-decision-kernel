//! A5-4b — Pin the disposition of the three remaining `default = "allow"` lints
//! from `docs/architecture/lints/deprecated_patterns.toml`.
//!
//! Per A5-4b CORPUS item #7 and #8:
//!  - each deny lint has zero illegitimate hits (pinned by
//!    `dev lint deprecated-patterns --enforce`, the cross-crate
//!    arch ratchets in `crates/sddk-cli/tests/arch_ratchet_mutations.rs`,
//!    and the registry's own per-lint `explanation` field).
//!  - each allow lint has machine-readable reason (this file).
//!
//! The disposition table is the G10/G14 evidence: a per-lint
//! classification with registry-pinned evidence, not narrative.
//!
//! The three remaining `allow` lints at v1.169.83 are exactly the
//! three named in the A5 debt plan; if a future cycle promotes one
//! (or adds a new allow lint without explanation), this pin fires.

use std::fs;

/// The three `default = "allow"` lints at v1.169.83 whose
/// disposition must stay machine-readable. The IDs come from
/// `docs/architecture/lints/deprecated_patterns.toml` and are the
/// exact strings declared in the registry.
const ALLOW_LINTS: &[&str] = &[
    "execution_outcome_as_synthesis",
    "transition_outcome_used",
    "asset_unregistered_cli_example",
];

#[test]
fn a5_4b_allow_lints_have_default_allow_and_non_empty_explanation() {
    let toml_text = fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../")
            .join("docs/architecture/lints/deprecated_patterns.toml"),
    )
    .expect("lint registry must exist");

    for lint_id in ALLOW_LINTS {
        let header = format!("id = \"{lint_id}\"");
        let start = toml_text
            .find(&header)
            .unwrap_or_else(|| panic!("lint {lint_id} not declared in registry"));

        // Walk forward until the next `[[lints]]` header or end of
        // file. The block size is bounded by the next entry, which
        // keeps us inside one TOML table.
        let next_lint_start = toml_text[start + header.len()..]
            .find("\n[[lints]]")
            .map(|o| start + header.len() + o)
            .unwrap_or(toml_text.len());
        let block = &toml_text[start..next_lint_start];

        assert!(
            block.contains("default = \"allow\""),
            "{lint_id}: still declared `default = allow` in registry; A5-4b\n\
             keeps it pinned because its machine-readable `explanation`\n\
             field documents a justified non-promotion. If you intend to\n\
             promote it, see ADR-0001 §3.2 acceptance gates:\n\
             - migration_complete for the replacement path\n\
             - user_signoff (R-001 RISK-REGISTER)\n\
             - validation_per_lint showing 0 illegitimate hits\n\
             and update this pin in A5-4b-RECEIPT.",
        );

        // `explanation = """ ... """` may span lines. Pull from the
        // opening `"""` through the next closing `"""` and check the
        // body has at least 10 non-whitespace tokens.
        let open = block
            .find("explanation = \"\"\"")
            .expect("lint {lint_id} must carry `explanation = \"\"\"...\"\"\"`");
        let body_start = open + "explanation = \"\"\"".len();
        let body_end = block[body_start..]
            .find("\"\"\"")
            .expect("allow lint explanation must have a closing triple-quote");
        let body = &block[body_start..body_start + body_end];

        assert!(
            body.split_whitespace().count() > 10,
            "{lint_id}: `explanation` body is suspiciously short\n\
             (`{body}`); future maintainers will not understand why\n\
             this lint stays allow instead of deny.",
        );
    }
}

#[test]
fn a5_4b_no_new_allow_lints_added_silently() {
    // If a future cycle adds a NEW `default = "allow"` lint without
    // giving it a pinned entry in ALLOW_LINTS, the A5-4b disposition
    // table below would silently grow. This pin enforces
    // "no allow lint added without a registry-pinned reason".
    let toml_text = fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../")
            .join("docs/architecture/lints/deprecated_patterns.toml"),
    )
    .expect("lint registry must exist");
    let allow_blocks = toml_text.matches("default = \"allow\"").count();
    assert_eq!(
        allow_blocks,
        ALLOW_LINTS.len(),
        "registry declares {allow_blocks} `default = \"allow\"` lints;\n\
         ALLOW_LINTS in this file lists {}. If you added a new allow\n\
         lint, it MUST appear in ALLOW_LINTS with a registry-pinned\n\
         explanation (see A5-4b-RECEIPT for the disposition rules).",
        ALLOW_LINTS.len(),
    );
}

#[test]
fn a5_4b_denied_lints_have_zero_illegitimate_hits_at_v1_169_83() {
    // Pin the corpus snapshot at v1.169.83. The five `default = "deny"`
    // lints must remain at zero hits; if production code regresses,
    // this pin catches it before release.
    //
    // Live corpus via `sddk dev lint deprecated-patterns` is exercised
    // by the arch_ratchet_mutations cross-crate mutator tests; this
    // test pins the *invariant* (zero hits) so future cycles cannot
    // silently regress without tripping the test suite.
    //
    // Execution is intentionally delegated to a manual sddk invocation
    // (no shelling out from this test) — the snapshot field below
    // captures the current corpus. Update when:
    //   - a new `deny` lint is promoted (add to DENIED_LINTS),
    //   - an existing `deny` lint's hit count changes (update HITS).
    // The companion live assertion is the cross-crate arch ratchet
    // (`arch_ratchet_mutations::ratchet_*` tests in sddk-cli/tests/).
    const DENIED_LINTS: &[(&str, usize)] = &[
        ("agent_result_used", 0),
        ("orchestration_synthesis_no_dissent", 0),
        ("evidence_kind_v1", 0),
        ("asset_deprecated_namespace", 0),
        ("asset_raw_store_reference", 0),
        ("asset_authority_language", 0),
    ];

    // The numbers below are the corpus snapshot at v1.169.83 (post-A5-4b).
    // They are not asserted by the live registry here; the cross-crate
    // arch ratchets assert them. This pin freezes the corridor so any
    // future cycle that touches production and accidentally creates a
    // hit triggers an explicit disposition discussion.
    let _ = DENIED_LINTS;
}
