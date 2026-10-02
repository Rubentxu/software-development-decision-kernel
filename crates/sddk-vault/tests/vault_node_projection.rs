//! R1-R4 for `cl-vault-node-projection`.
//!
//! `VaultNode` (`index.rs:61-78`) derives `Serialize` and has eight fields.
//! `export_node` (`export.rs:75-84`) is a hand-written `serde_json::json!` with
//! six. It drops `tags` and `body`, and — measured against the real binary with a
//! vault whose frontmatter carries `tags` — **the artifact declares nothing**.
//!
//! That is the same shape `GraphExport` had before `cl-vault-html-replica`: a
//! field list written by hand over a type that already knows how to serialize
//! itself, with no compile-time coupling. Adding a field to `VaultNode` does not
//! reach the export and nothing notices.
//!
//! What is *not* being claimed: that `body` should travel. It almost certainly
//! should not — it is the whole document, and the export is a self-contained
//! page. The claim is narrower and harder to argue with: **the artifact does not
//! say what it omits**, so a consumer cannot answer "is this everything?" without
//! reading the code.
//!
//! R2 is the one carrying the weight. R1, R3 and R4 are all satisfiable by
//! writing a sentence in the HTML; only R2 asks whether the coupling can be made
//! to fail the build.

use std::path::Path;

use sddk_vault::{NodeKind, VaultIndex, export_html, graph_view, parse_vault};

/// The field list, **derived from the type** by asking serde for it.
///
/// Two earlier versions of this helper were wrong, and both wrong in the way
/// that matters here:
///
/// 1. It returned a **restated literal** of the eight field names. That is the
///    defect under test wearing a different hat — a hand-written list of what
///    the struct has, in the one place whose job is to notice when the
///    hand-written list stops matching.
/// 2. It built a `VaultNode { … }` literal to serialise. Then a field added to
///    the struct made **this test file stop compiling**, so the mutation was
///    "caught" — by a `missing field` error in a helper, before any assertion
///    ran. A guard that fires for the wrong reason is the same shape as a guard
///    that does not fire, and it hides the message that would have been useful.
///
/// So the probe is a node **parsed from a real fixture**, never a literal: the
/// list always reflects whatever the struct currently has, and the test always
/// compiles so the assertion can do the reporting.
fn vault_node_fields(root: &Path) -> Vec<String> {
    let index = parse_vault(root).expect("the fixture vault parses");
    let node = index
        .nodes
        .first()
        .expect("the fixture vault has at least one node");
    serde_json::to_value(node)
        .expect("a parsed node serialises")
        .as_object()
        .expect("a node serialises to an object")
        .keys()
        .cloned()
        .collect()
}

fn node_with(directory: &Path, id: &str, frontmatter_extra: &str, body: &str) {
    let path = directory.join(format!("{id}.md"));
    std::fs::write(
        &path,
        format!(
            "---\nid: {id}\ntype: term\nstatus: active\n{frontmatter_extra}---\n# {id}\n\n{body}\n"
        ),
    )
    .unwrap();
}

fn vault(dir: &Path) -> (VaultIndex, String) {
    let index = parse_vault(dir).unwrap();
    let graph = graph_view(&index).unwrap();
    let html = export_html(&index, &graph).unwrap();
    (index, html)
}

/// Pulls the JSON literal out of `window.__vault_nodes__=[…];`.
fn embedded_nodes(html: &str) -> serde_json::Value {
    let start = html
        .find("window.__vault_nodes__=")
        .expect("the export embeds the nodes for the page to consume")
        + "window.__vault_nodes__=".len();
    let rest = &html[start..];
    let end = rest
        .find(';')
        .expect("the assignment is terminated by a semicolon");
    serde_json::from_str(&rest[..end]).expect("the embedded literal is valid JSON")
}

#[test]
fn r1_artifact_declares_the_projection_it_makes() {
    let dir = tempfile::tempdir().unwrap();
    node_with(dir.path(), "TERM-A", "tags: [alpha, beta]\n", "[[TERM-B]]");
    node_with(dir.path(), "TERM-B", "", "cuerpo");
    let (_index, html) = vault(dir.path());

    let nodes = embedded_nodes(&html);
    let first = &nodes.as_array().unwrap()[0];
    let carried: Vec<&String> = first.as_object().unwrap().keys().collect();

    // The declaration has to be *findable*, not merely present: a consumer
    // reading the page has to be able to ask the question and get an answer
    // without parsing the whole document.
    let lower = html.to_lowercase();
    let says_total = lower.contains("fields carried")
        || lower.contains("of 8 fields")
        || lower.contains("fields omitted");
    assert!(
        says_total,
        "the export must DECLARE that it carries a projection of the node, not \
         the whole node. Today it carries {} of {} fields ({:?}) and says \
         nothing, so a consumer cannot answer 'is this everything?' without \
         reading the code.",
        carried.len(),
        vault_node_fields(dir.path()).len(),
        carried
    );
}

#[test]
fn r2_adding_a_field_to_vault_node_cannot_pass_unnoticed() {
    // This is the test that carries the weight, and it is honest about what it
    // does and does not buy.
    //
    // **What it does NOT do:** make the build fail. The `From<&VaultNode> for
    // NodeProjection` mapping is between two different types, so adding a field
    // to `VaultNode` does not break compilation — measured, not assumed: a
    // `mutant_field` was added to the struct, the parser was updated to satisfy
    // it, and `cargo build` succeeded. An earlier version of this file and of
    // the doc comment in `export.rs` both claimed the opposite. A guard that
    // claims a guarantee it does not provide is worse than no guard.
    //
    // **What it DOES do:** pin the two field sets against each other, so the
    // invariant is `carried == declared − omitted` and nothing else. There is
    // no third bucket: a field is either carried, or it is named in the
    // artifact as omitted. Add a field to `VaultNode` and, unless somebody
    // decides for it, this fails with the field's name in the message.
    let dir = tempfile::tempdir().unwrap();
    node_with(dir.path(), "TERM-A", "tags: [alpha, beta]\n", "cuerpo");
    let (_index, html) = vault(dir.path());
    let nodes = embedded_nodes(&html);
    let first = &nodes.as_array().unwrap()[0];
    let carried: Vec<String> = first.as_object().unwrap().keys().cloned().collect();

    let declared = vault_node_fields(dir.path());
    let undeclared: Vec<&String> = carried.iter().filter(|k| !declared.contains(k)).collect();
    assert!(
        undeclared.is_empty(),
        "the export carries keys that are not fields of `VaultNode`: \
         {undeclared:?}. A projection that can name things the source does not \
         have is the same defect as one that drops them silently."
    );

    // The complement: every field is carried or named as omitted, and the two
    // together account for the whole struct. Derived from the type on both
    // sides, so this cannot pass by agreeing with a stale list.
    let omitted: Vec<String> = declared
        .iter()
        .filter(|f| !carried.contains(f))
        .cloned()
        .collect();
    for field in &omitted {
        assert!(
            html.contains(field.as_str()),
            "`{field}` is a field of `VaultNode` that the export drops, and the \
             artifact does not mention it. A dropped field has to be named \
             somewhere a consumer can see, otherwise 'is this everything?' has \
             no answer. Carried: {carried:?}, omitted: {omitted:?}"
        );
    }
    assert_eq!(
        carried.len() + omitted.len(),
        declared.len(),
        "every field of `VaultNode` is either carried or named as omitted, and \
         the two together must account for all {} of them. Carried: {carried:?}, \
         omitted: {omitted:?}",
        declared.len()
    );
}

#[test]
fn r3_status_and_tags_are_treated_by_one_rule() {
    let dir = tempfile::tempdir().unwrap();
    node_with(dir.path(), "TERM-A", "tags: [alpha, beta]\n", "cuerpo");
    let (index, html) = vault(dir.path());
    let nodes = embedded_nodes(&html);

    let node = index
        .get("TERM-A")
        .expect("the fixture node is indexed")
        .clone();
    let carried = nodes.as_array().unwrap()[0].clone();

    let status_carried = carried.get("status").is_some();
    let tags_carried = carried.get("tags").is_some();
    assert_eq!(
        status_carried, tags_carried,
        "`status` and `tags` are both frontmatter metadata and were treated by \
         two different rules: status_carried={status_carried} \
         tags_carried={tags_carried}. The source node has status={:?} and \
         tags={:?}, so neither of them is empty by accident.",
        node.status, node.tags
    );
}

#[test]
fn r4_body_does_not_travel() {
    // Characterization, and it passes today. It is the guard against the
    // obvious wrong repair: "declare the projection" can be satisfied by
    // embedding every document body in the page, and the result would be
    // honest and useless.
    let dir = tempfile::tempdir().unwrap();
    node_with(dir.path(), "TERM-A", "", "UN CUERPO QUE NO DEBERIA VIAJAR");
    let (_index, html) = vault(dir.path());
    let nodes = embedded_nodes(&html);
    let first = &nodes.as_array().unwrap()[0];

    assert!(
        first.get("body").is_none(),
        "the body of every document must stay out of a self-contained inspector \
         page: it is the document, not a field of it. If this ever fails, the \
         page grew by the size of the vault and the repair traded a silent \
         omission for a silent cost — which is the same defect wearing the \
         other hat. Got: {}",
        first
    );
    assert!(
        !html.contains("UN CUERPO QUE NO DEBERIA VIAJAR"),
        "the body text must not appear anywhere in the page, not even escaped."
    );
}

// Compile-time reminder of what `NodeKind` contributes, so a change to its
// serialised form is visible in this file rather than only in the JSON.
#[test]
fn node_kind_round_trips_through_the_projection() {
    let dir = tempfile::tempdir().unwrap();
    node_with(dir.path(), "TERM-A", "", "cuerpo");
    let (_index, html) = vault(dir.path());
    let nodes = embedded_nodes(&html);
    let kind = &nodes.as_array().unwrap()[0]["kind"];
    assert_eq!(
        serde_json::to_value(NodeKind::Term).unwrap(),
        *kind,
        "the exported `kind` must be the serialised form of the enum, not a \
         string built by hand — a hand-built string is how the projection and \
         the type would drift."
    );
}
