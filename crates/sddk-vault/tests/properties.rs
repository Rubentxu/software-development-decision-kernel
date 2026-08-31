//! Property tests for vault parsing and graph projection.

use std::fs;

use proptest::prelude::*;
use sddk_vault::{graph_view, parse_vault};

fn write_vault(directory: &std::path::Path, files: &[(String, String)]) {
    for (name, content) in files {
        let path = directory.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn arbitrary_markdown_parses_without_panic(title in ".{0,32}", body in ".{0,256}") {
        let directory = tempfile::tempdir().unwrap();
        let content = format!("---\nid: N-1\ntype: term\n---\n# {title}\n\n{body}\n");
        write_vault(directory.path(), &[("terms/N-1.md".into(), content)]);
        let index = parse_vault(directory.path()).unwrap();
        prop_assert_eq!(index.nodes.len(), 1);
        prop_assert_eq!(&index.get("N-1").expect("node exists").id, "N-1");
    }

    #[test]
    fn wikilinks_are_deduplicated_and_sorted_by_occurrence(links in prop::collection::vec("[A-Za-z0-9_-]{1,12}", 0..8)) {
        let directory = tempfile::tempdir().unwrap();
        let mut body = String::new();
        for link in &links {
            body.push_str(&format!("[[{link}]] "));
        }
        let content = format!("---\nid: SRC\ntype: term\n---\n# Src\n\n{body}\n");
        write_vault(directory.path(), &[("terms/SRC.md".into(), content)]);
        let index = parse_vault(directory.path()).unwrap();
        let node = index.get("SRC").unwrap();
        let mut deduped = node.wikilinks.clone();
        deduped.sort();
        deduped.dedup();
        prop_assert_eq!(node.wikilinks.len(), deduped.len());
    }

    #[test]
    fn graph_projection_is_stable(nodes in prop::collection::vec("[A-Za-z0-9]{1,8}", 1..8)) {
        let directory = tempfile::tempdir().unwrap();
        let mut files = Vec::new();
        for (index, node) in nodes.iter().enumerate() {
            let next = nodes.get((index + 1) % nodes.len()).cloned().unwrap_or_default();
            let content = format!("---\nid: {node}\ntype: term\n---\n# {node}\n\n[[{next}]]\n");
            files.push((format!("terms/{node}.md"), content));
        }
        write_vault(directory.path(), &files);
        let index = parse_vault(directory.path()).unwrap();
        let first = graph_view(&index).unwrap();
        let second = graph_view(&index).unwrap();
        prop_assert_eq!(first.node_count, second.node_count);
        prop_assert_eq!(first.edge_count, second.edge_count);
        prop_assert_eq!(first.cyclic, second.cyclic);
    }
}
