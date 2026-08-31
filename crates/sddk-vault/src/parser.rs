//! Vault directory parsing: frontmatter, titles, and wikilinks.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use regex::Regex;
use serde_json::Value;
use thiserror::Error;
use walkdir::WalkDir;

use crate::index::{NodeKind, VaultIndex, VaultNode};

/// Errors emitted while parsing a vault directory.
#[derive(Debug, Error)]
pub enum VaultError {
    /// A Markdown file could not be read.
    #[error("failed to read vault node {path:?}: {source}")]
    Read {
        /// Affected node path.
        path: PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },
    /// Frontmatter YAML could not be parsed.
    #[error("invalid vault frontmatter: {source}")]
    Parse {
        /// Parse failure.
        source: serde_saphyr::Error,
    },
}

/// Parses every Markdown node under a vault directory.
pub fn parse_vault(directory: &Path) -> Result<VaultIndex, VaultError> {
    let mut nodes = Vec::new();
    let mut by_id = HashMap::new();
    let mut wikilinks = Vec::new();

    for entry in WalkDir::new(directory)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file()
            || entry.path().extension().and_then(|ext| ext.to_str()) != Some("md")
        {
            continue;
        }
        let node = parse_node(directory, entry.path())?;
        by_id.insert(node.id.clone(), nodes.len());
        wikilinks.push((node.id.clone(), node.wikilinks.clone()));
        nodes.push(node);
    }

    let mut backlinks: HashMap<String, Vec<String>> = HashMap::new();
    for (source, links) in &wikilinks {
        for target in links {
            backlinks
                .entry(target.clone())
                .or_default()
                .push(source.clone());
        }
    }
    for sources in backlinks.values_mut() {
        sources.sort();
        sources.dedup();
    }

    Ok(VaultIndex {
        nodes,
        by_id,
        backlinks,
    })
}

fn parse_node(directory: &Path, path: &Path) -> Result<VaultNode, VaultError> {
    let source = std::fs::read_to_string(path).map_err(|source| VaultError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let (frontmatter, body) = split_frontmatter(&source);
    let meta = frontmatter
        .and_then(|raw| parse_frontmatter(raw).ok())
        .unwrap_or_default();

    let relative = path
        .strip_prefix(directory)
        .expect("walked paths stay under the vault")
        .to_string_lossy()
        .replace('\\', "/");
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .to_owned();

    let id = meta
        .get("id")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| {
            frontmatter
                .and_then(|raw| raw_scalar(raw, "id"))
                .map(str::to_owned)
        })
        .unwrap_or_else(|| stem.clone());
    let kind = meta
        .get("type")
        .and_then(Value::as_str)
        .map(NodeKind::from_type)
        .or_else(|| {
            path.parent()
                .and_then(|parent| parent.file_name())
                .and_then(|name| name.to_str())
                .map(NodeKind::from_folder)
        })
        .unwrap_or(NodeKind::Other);
    let title = meta
        .get("title")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| {
            frontmatter
                .and_then(|raw| raw_scalar(raw, "title"))
                .map(str::to_owned)
        })
        .or_else(|| first_heading(body))
        .unwrap_or_else(|| id.clone());
    let status = meta
        .get("status")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let tags = meta
        .get("tags")
        .and_then(Value::as_array)
        .map(|tags| {
            tags.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();

    Ok(VaultNode {
        id,
        kind,
        path: relative,
        title,
        status,
        tags,
        body: body.to_owned(),
        wikilinks: extract_wikilinks(body),
    })
}

fn split_frontmatter(source: &str) -> (Option<&str>, &str) {
    let Some(rest) = source.strip_prefix("---") else {
        return (None, source);
    };
    let Some(end) = rest.find("\n---") else {
        return (None, source);
    };
    (Some(&rest[..end]), &rest[end + 4..])
}

fn parse_frontmatter(raw: &str) -> Result<HashMap<String, Value>, VaultError> {
    serde_saphyr::from_str::<HashMap<String, Value>>(raw)
        .map_err(|error| VaultError::Parse { source: error })
}

/// Reads a `key:` scalar from raw frontmatter without typed parsing.
fn raw_scalar<'a>(frontmatter: &'a str, key: &str) -> Option<&'a str> {
    frontmatter
        .lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix(&format!("{key}:")))
        .map(str::trim)
        .map(|value| value.trim_matches('"').trim_matches('\''))
        .filter(|value| !value.is_empty())
}

fn first_heading(body: &str) -> Option<String> {
    body.lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix("# "))
        .map(str::to_owned)
}

fn extract_wikilinks(body: &str) -> Vec<String> {
    let pattern = Regex::new(r"\[\[([^\]|#]+)").expect("wikilink pattern is valid");
    let mut links = Vec::new();
    for capture in pattern.captures_iter(body) {
        let target = capture[1].trim().to_owned();
        if !target.is_empty() && !links.contains(&target) {
            links.push(target);
        }
    }
    links
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::index::NodeKind;

    use super::parse_vault;

    fn node(file: &str, content: &str) {
        fs::create_dir_all(std::path::Path::new(file).parent().unwrap()).unwrap();
        fs::write(file, content).unwrap();
    }

    #[test]
    fn parses_frontmatter_title_and_wikilinks() {
        let directory = tempfile::tempdir().unwrap();
        node(
            &directory
                .path()
                .join("adrs/ADR-001-auth.md")
                .to_string_lossy(),
            "---\nid: ADR-001\ntype: adr\nstatus: accepted\ntags: [auth]\n---\n# Auth\n\nSee [[REQ-Session]] and [[M-001]]\n",
        );
        node(
            &directory
                .path()
                .join("specs/REQ-Session.md")
                .to_string_lossy(),
            "---\nid: REQ-Session\ntype: requirement\n---\n# Session\n",
        );

        let index = parse_vault(directory.path()).unwrap();
        assert_eq!(index.nodes.len(), 2);
        let adr = index.get("ADR-001").unwrap();
        assert_eq!(adr.kind, NodeKind::Adr);
        assert_eq!(adr.title, "Auth");
        assert_eq!(adr.status.as_deref(), Some("accepted"));
        assert_eq!(adr.wikilinks, vec!["REQ-Session", "M-001"]);
        assert_eq!(index.backlinks_of("REQ-Session"), vec!["ADR-001"]);
    }

    #[test]
    fn derives_id_kind_and_title_without_frontmatter() {
        let directory = tempfile::tempdir().unwrap();
        node(
            &directory.path().join("terms/TERM-JWT.md").to_string_lossy(),
            "# JWT\n\nToken format.",
        );
        let index = parse_vault(directory.path()).unwrap();
        let term = index.get("TERM-JWT").unwrap();
        assert_eq!(term.kind, NodeKind::Term);
        assert_eq!(term.title, "JWT");
        assert!(term.status.is_none());
    }
}

#[cfg(test)]
mod robustness_tests {
    use std::fs;

    use super::parse_vault;

    #[test]
    fn scientific_notation_ids_parse_without_error() {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir_all(directory.path().join("terms")).unwrap();
        fs::write(
            directory.path().join("terms/1e848.md"),
            "---\nid: 1e848\ntype: term\n---\n# Float id\n",
        )
        .unwrap();
        let index = parse_vault(directory.path()).unwrap();
        let node = index.get("1e848").expect("id derived from raw frontmatter");
        assert_eq!(node.id, "1e848");
        assert_eq!(node.title, "Float id");
    }
}
