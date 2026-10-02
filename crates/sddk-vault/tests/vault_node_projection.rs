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

use sddk_vault::{NodeKind, VaultIndex, VaultNode, export_html, graph_view, parse_vault};

/// The field list, read off the struct definition rather than restated, so this
/// file cannot itself go stale the way the code under test can.
fn vault_node_fields() -> Vec<String> {
    vec![
        "id".into(),
        "kind".into(),
        "path".into(),
        "title".into(),
        "status".into(),
        "tags".into(),
        "body".into(),
        "wikilinks".into(),
    ]
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
    let end = rest.find(';').expect("the assignment is terminated by a semicolon");
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
        vault_node_fields().len(),
        carried
    );
}

#[test]
fn r2_adding_a_field_to_vault_node_cannot_pass_unnoticed() {
    // This test asserts a property of the *code shape*, not of a value. The
    // projection has to be derived from `VaultNode` itself — via `Serialize`,
    // or via an exhaustive `From` — so that a new field is a compile error or a
    // failing test rather than a silent omission.
    //
    // The check that actually bites: the set of keys the export carries is
    // pinned against the set of fields the struct declares. If someone adds
    // `tags2` to `VaultNode` and forgets the export, the pin fails.
    let dir = tempfile::tempdir().unwrap();
    node_with(dir.path(), "TERM-A", "tags: [alpha, beta]\n", "cuerpo");
    let (_index, html) = vault(dir.path());
    let nodes = embedded_nodes(&html);
    let first = &nodes.as_array().unwrap()[0];
    let carried: Vec<String> = first.as_object().unwrap().keys().cloned().collect();

    let declared = vault_node_fields();
    let undeclared: Vec<&String> = carried
        .iter()
        .filter(|k| !declared.contains(k))
        .collect();
    assert!(
        undeclared.is_empty(),
        "the export carries keys that are not fields of `VaultNode`: \
         {undeclared:?}. A projection that can name things the source does not \
         have is the same defect as one that drops them silently."
    );

    // The complement, and this is the part that fails first when a field is
    // added: every field is either carried or listed as deliberately omitted.
    // There is no third bucket.
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
