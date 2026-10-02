//! R1-R3 for `cl-vault-html-replica`.
//!
//! `vault export` writes a self-contained HTML file whose `<script>` carries
//! `window.__vault_graph__` — a second declaration of the same graph that
//! `vault graph` declares in its own JSON. `GraphExport` (`export.rs:90-95`) is a
//! hand-written struct with three fields, not a projection of `GraphView`, so the
//! two lists can drift apart and nothing notices.
//!
//! Measured before this file existed, on a vault with two disjoint cycles:
//! `cycle_count`, `multiple_cycles` and `topological_order_absent_because` were
//! all absent from the HTML. The previous cycle's STOP 4 said "`vault export`
//! stops matching `vault graph`" and was satisfied — the HTML asserts nothing
//! *false* about the graph — with the whole defect still present.
//!
//! R1 and R2 are presence checks on their own, and a presence check passes the
//! moment the field exists even if the value is invented. R3 is the one that
//! carries the weight: it compares the values field by field, which is what
//! turns "the field is there" into a shape the two surfaces cannot leave again.

use std::path::Path;

use sddk_vault::{export_html, graph_view, parse_vault};

/// Builds a vault with two disjoint cycles (A→B→C→A and X→Y→X).
///
/// Two cycles is the case that makes the defect visible: with exactly one, the
/// saturated form and the "one cycle" form coincide, and a fixture built on it
/// would pass against a `GraphExport` that declares nothing.
fn two_cycle_vault(root: &Path) {
    for (id, body) in [
        ("CYC-A", "links [[CYC-B]]"),
        ("CYC-B", "links [[CYC-C]]"),
        ("CYC-C", "links [[CYC-A]]"),
        ("CYC-X", "links [[CYC-Y]]"),
        ("CYC-Y", "links [[CYC-X]]"),
    ] {
        let path = root.join(format!("{id}.md"));
        std::fs::write(
            &path,
            format!("---\nid: {id}\ntype: term\nstatus: active\n---\n# {id}\n\n{body}\n"),
        )
        .unwrap();
    }
}

/// Pulls the JSON literal out of `window.__vault_graph__=…;`.
///
/// Written by hand rather than pulled from a test helper, because the whole
/// question is whether a *consumer* can recover these fields from the shipped
/// artifact. A helper that knew the struct's shape would prove nothing about
/// the file.
fn embedded_graph(html: &str) -> serde_json::Value {
    let start = html
        .find("window.__vault_graph__=")
        .expect("the export embeds the graph for the page to consume")
        + "window.__vault_graph__=".len();
    let rest = &html[start..];
    let end = rest
        .find(';')
        .expect("the assignment is terminated by a semicolon");
    serde_json::from_str(&rest[..end]).expect("the embedded literal is valid JSON")
}

fn html_for(root: &Path) -> String {
    let index = parse_vault(root).unwrap();
    let graph = graph_view(&index).unwrap();
    export_html(&index, &graph).unwrap()
}

#[test]
fn r1_embedded_graph_declares_the_cycle_count() {
    let dir = tempfile::tempdir().unwrap();
    two_cycle_vault(dir.path());
    let e = embedded_graph(&html_for(dir.path()));

    // This first version of the assertion was "the key `cycle_count` is
    // present", and it FAILED against a correct implementation. It was the test
    // that was wrong, not the product: the saturated form deliberately omits
    // `cycle_count` when the answer is "more than one", because `None` is the
    // encoding of "2 or more" and `multiple_cycles: true` is what makes it
    // distinguishable from "absent". Requiring the key to be present would
    // re-introduce exactly the ambiguity this cycle set out to remove — and it
    // would have forced a `cycle_count: null` that `vault graph` does not emit,
    // which is a divergence of the opposite sign.
    //
    // So the property is not presence. It is: the embedded graph says how many
    // cycles there are, in one of the two forms the saturated contract allows.
    let exact = e.get("cycle_count").and_then(|v| v.as_u64());
    let multiple = e.get("multiple_cycles").and_then(|v| v.as_bool());
    assert!(
        exact == Some(1) || (exact.is_none() && multiple == Some(true)),
        "the embedded graph must say how many cycles there are, in the saturated \
         form: an exact count when it is 0 or 1, and `multiple_cycles: true` with \
         no count when it is 2 or more. Anything else leaves a reader unable to \
         tell 'more than one' from 'not computed'. Got: {e}"
    );
    assert_eq!(
        multiple,
        Some(true),
        "this fixture has two disjoint cycles, so the saturated form is the only \
         correct one. If this ever reads 2 cycles, `dfs_cycle` found one — and \
         then this test would be measuring the wrong graph, not a wrong export. \
         Got: {e}"
    );
}

#[test]
fn r2_embedded_graph_explains_the_absent_topological_order() {
    let dir = tempfile::tempdir().unwrap();
    two_cycle_vault(dir.path());
    let e = embedded_graph(&html_for(dir.path()));

    assert!(
        e.get("topological_order").is_none() || e["topological_order"].is_null(),
        "precondition: a cyclic graph has no topological order: {e}"
    );
    assert_eq!(
        e.get("topological_order_absent_because")
            .and_then(|v| v.as_str()),
        Some("cyclic"),
        "an absent topological order must SAY that it is absent and why. Today the \
         field is simply not in the embedded JSON, which reads as 'not computed' \
         rather than 'does not exist because the graph is cyclic'. Got: {e}"
    );
}

#[test]
fn r3_embedded_graph_matches_vault_graph_field_by_field() {
    let dir = tempfile::tempdir().unwrap();
    two_cycle_vault(dir.path());
    let root = dir.path();

    let index = parse_vault(root).unwrap();
    let graph = graph_view(&index).unwrap();
    let html = export_html(&index, &graph).unwrap();
    let e = embedded_graph(&html);

    // `GraphView` serialises with skip_serializing_if, so the command's JSON and
    // the struct differ in which optional keys are PRESENT. Comparing the values
    // of the fields that both surfaces claim to declare is what matters; a key
    // absent from one and null in the other is a difference worth reporting, not
    // a difference to paper over.
    for field in [
        "cyclic",
        "cycle_count",
        "multiple_cycles",
        "topological_order",
        "topological_order_absent_because",
    ] {
        let from_view = serde_json::to_value(&graph).unwrap();
        assert_eq!(
            e.get(field),
            from_view.get(field),
            "`vault export` must declare the same thing as `vault graph` for \
             `{field}`. They are two declarations of the same graph, and a field \
             that differs between them is the defect this cycle closes. \
             html={e} graph={from_view}"
        );
    }
}
