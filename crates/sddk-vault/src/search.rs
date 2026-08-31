//! Incremental, rebuildable SQLite FTS5 search index over vault content.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::index::VaultIndex;

/// One full-text search hit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SearchHit {
    /// Matched node id.
    pub id: String,
    /// Node kind.
    pub kind: String,
    /// Node title.
    pub title: String,
    /// Node path.
    pub path: String,
}

/// Counts of rows touched by one incremental sync.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SyncSummary {
    /// Rows inserted because the node is new.
    pub inserted: usize,
    /// Rows updated because the node content changed.
    pub updated: usize,
    /// Rows removed because the node disappeared.
    pub deleted: usize,
}

/// Errors emitted by the search index.
#[derive(Debug, Error)]
pub enum SearchIndexError {
    /// SQLite rejected an index operation.
    #[error("vault search index error: {0}")]
    Database(#[from] rusqlite::Error),
    /// A filesystem operation failed.
    #[error("vault search index I/O failure: {0}")]
    Io(#[from] std::io::Error),
}

const FTS_TABLE: &str = "vault_fts";
const STATE_TABLE: &str = "vault_index_state";

/// Destroys and rebuilds the FTS index from a parsed vault.
///
/// The index is fully derivable from the vault, so rebuilding is the canonical
/// recovery path: drop, re-create, re-insert everything.
pub fn rebuild_search_index(
    connection: &Connection,
    index: &VaultIndex,
) -> Result<SyncSummary, SearchIndexError> {
    connection.execute_batch(&format!(
        "DROP TABLE IF EXISTS {FTS_TABLE};
         DROP TABLE IF EXISTS {STATE_TABLE};
         CREATE VIRTUAL TABLE {FTS_TABLE} USING fts5(
             id, kind, title, path, tags, links, backlinks, status, body
         );
         CREATE TABLE IF NOT EXISTS {STATE_TABLE} (
             path TEXT PRIMARY KEY,
             id TEXT NOT NULL,
             content_hash TEXT NOT NULL
         );"
    ))?;
    sync_search_index(connection, index)
}

/// Synchronizes the FTS index incrementally by node content hash.
///
/// Only nodes whose indexed content changed are rewritten; removed nodes are
/// deleted. Rerunning without changes touches nothing.
pub fn sync_search_index(
    connection: &Connection,
    index: &VaultIndex,
) -> Result<SyncSummary, SearchIndexError> {
    connection.execute_batch(&format!(
        "CREATE VIRTUAL TABLE IF NOT EXISTS {FTS_TABLE} USING fts5(
             id, kind, title, path, tags, links, backlinks, status, body
         );
         CREATE TABLE IF NOT EXISTS {STATE_TABLE} (
             path TEXT PRIMARY KEY,
             id TEXT NOT NULL,
             content_hash TEXT NOT NULL
         );"
    ))?;

    let mut known = BTreeMap::new();
    let mut statement =
        connection.prepare(&format!("SELECT path, id, content_hash FROM {STATE_TABLE}"))?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    for row in rows {
        let (path, id, hash) = row?;
        known.insert(path, (id, hash));
    }

    let mut summary = SyncSummary::default();
    let mut insert = connection.prepare(&format!(
        "INSERT INTO {FTS_TABLE} (id, kind, title, path, tags, links, backlinks, status, body)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"
    ))?;
    let mut delete_row = connection.prepare(&format!("DELETE FROM {FTS_TABLE} WHERE id = ?1"))?;
    let mut upsert_state = connection.prepare(&format!(
        "INSERT INTO {STATE_TABLE} (path, id, content_hash) VALUES (?1, ?2, ?3)
         ON CONFLICT(path) DO UPDATE SET id = excluded.id, content_hash = excluded.content_hash"
    ))?;

    for node in &index.nodes {
        let hash = node_hash(index, node);
        if known
            .get(&node.path)
            .is_some_and(|(_, existing)| existing == &hash)
        {
            continue;
        }
        delete_row.execute(params![node.id])?;
        insert.execute(params![
            node.id,
            serde_json::to_string(&node.kind).unwrap_or_default(),
            node.title,
            node.path,
            node.tags.join(" "),
            node.wikilinks.join(" "),
            index.backlinks_of(&node.id).join(" "),
            node.status.as_deref().unwrap_or_default(),
            node.body,
        ])?;
        upsert_state.execute(params![node.path, node.id, hash])?;
        if known.contains_key(&node.path) {
            summary.updated += 1;
        } else {
            summary.inserted += 1;
        }
    }

    let current_paths: BTreeSet<&str> = index.nodes.iter().map(|node| node.path.as_str()).collect();
    let mut delete_state =
        connection.prepare(&format!("DELETE FROM {STATE_TABLE} WHERE path = ?1"))?;
    for (path, (id, _)) in known {
        if !current_paths.contains(path.as_str()) {
            delete_row.execute(params![id])?;
            delete_state.execute(params![path.as_str()])?;
            summary.deleted += 1;
        }
    }
    Ok(summary)
}

/// Searches the FTS index with a sanitized query.
///
/// The query is wrapped in quotes so FTS5 operators in user input are treated
/// as literal text.
pub fn search_index(
    connection: &Connection,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchHit>, SearchIndexError> {
    let sanitized = query.replace('"', "\"\"");
    let mut statement = connection.prepare(&format!(
        "SELECT id, kind, title, path FROM {FTS_TABLE}
         WHERE {FTS_TABLE} MATCH ?1 ORDER BY rank LIMIT ?2"
    ))?;
    let rows = statement.query_map(params![format!("\"{sanitized}\""), limit as i64], |row| {
        Ok(SearchHit {
            id: row.get(0)?,
            kind: row.get(1)?,
            title: row.get(2)?,
            path: row.get(3)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Opens (or creates) the index database at a path.
pub fn open_index(path: &Path) -> Result<Connection, SearchIndexError> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    Ok(Connection::open(path)?)
}

/// Reports whether the FTS table has rows.
pub fn index_has_rows(connection: &Connection) -> Result<bool, SearchIndexError> {
    let count: Option<i64> = connection
        .query_row(&format!("SELECT COUNT(*) FROM {FTS_TABLE}"), [], |row| {
            row.get(0)
        })
        .optional()?;
    Ok(count.unwrap_or(0) > 0)
}

fn node_hash(index: &VaultIndex, node: &crate::index::VaultNode) -> String {
    let material = format!(
        "{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}",
        node.id,
        serde_json::to_string(&node.kind).unwrap_or_default(),
        node.title,
        node.path,
        node.tags.join("\u{1e}"),
        node.wikilinks.join("\u{1e}"),
        index.backlinks_of(&node.id).join("\u{1e}"),
        node.status.as_deref().unwrap_or_default(),
        node.body,
    );
    format!("sha256:{:x}", Sha256::digest(material.as_bytes()))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use rusqlite::Connection;

    use crate::parser::parse_vault;

    use super::{
        index_has_rows, open_index, rebuild_search_index, search_index, sync_search_index,
    };

    fn node(file: &str, content: &str) {
        fs::create_dir_all(std::path::Path::new(file).parent().unwrap()).unwrap();
        fs::write(file, content).unwrap();
    }

    #[test]
    fn index_is_rebuildable_after_deletion() {
        let directory = tempfile::tempdir().unwrap();
        node(
            &directory.path().join("terms/TERM-A.md").to_string_lossy(),
            "---\nid: TERM-A\ntype: term\n---\n# Auth\n\nOAuth token exchange\n",
        );
        let db_path = directory.path().join("index.sqlite");
        let index = parse_vault(directory.path()).unwrap();

        let connection = open_index(&db_path).unwrap();
        rebuild_search_index(&connection, &index).unwrap();
        let hits = search_index(&connection, "token", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "TERM-A");

        connection.execute_batch("DROP TABLE vault_fts").unwrap();

        rebuild_search_index(&connection, &index).unwrap();
        assert!(index_has_rows(&connection).unwrap());
        assert_eq!(search_index(&connection, "token", 10).unwrap().len(), 1);
    }

    #[test]
    fn query_operators_are_treated_as_literals() {
        let directory = tempfile::tempdir().unwrap();
        node(
            &directory.path().join("terms/TERM-A.md").to_string_lossy(),
            "---\nid: TERM-A\ntype: term\n---\n# Auth\n\nOAuth token\n",
        );
        let connection = Connection::open_in_memory().unwrap();
        let index = parse_vault(directory.path()).unwrap();
        rebuild_search_index(&connection, &index).unwrap();
        let hits = search_index(&connection, "NEAR(token OR auth", 10).unwrap();
        assert!(hits.is_empty());
    }

    #[test]
    fn sync_is_incremental_and_idempotent() {
        let directory = tempfile::tempdir().unwrap();
        node(
            &directory.path().join("terms/TERM-A.md").to_string_lossy(),
            "---\nid: TERM-A\ntype: term\ntags: [auth]\n---\n# Auth\n\nToken [[TERM-B]]\n",
        );
        node(
            &directory.path().join("terms/TERM-B.md").to_string_lossy(),
            "---\nid: TERM-B\ntype: term\n---\n# B\n",
        );
        let connection = Connection::open_in_memory().unwrap();

        let first =
            sync_search_index(&connection, &parse_vault(directory.path()).unwrap()).unwrap();
        assert_eq!(first.inserted, 2);
        assert_eq!(first.updated, 0);
        assert_eq!(first.deleted, 0);

        let second =
            sync_search_index(&connection, &parse_vault(directory.path()).unwrap()).unwrap();
        assert_eq!(second, super::SyncSummary::default());

        node(
            &directory.path().join("terms/TERM-A.md").to_string_lossy(),
            "---\nid: TERM-A\ntype: term\ntags: [auth, updated]\n---\n# Auth\n\nToken exchange\n",
        );
        let third =
            sync_search_index(&connection, &parse_vault(directory.path()).unwrap()).unwrap();
        assert_eq!(third.inserted, 0);
        // TERM-A changed and TERM-B's backlink list changed with it.
        assert_eq!(third.updated, 2);
        assert_eq!(third.deleted, 0);
        let hits = search_index(&connection, "exchange", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "TERM-A");

        fs::remove_file(directory.path().join("terms/TERM-B.md")).unwrap();
        let fourth =
            sync_search_index(&connection, &parse_vault(directory.path()).unwrap()).unwrap();
        assert_eq!(fourth.inserted, 0);
        assert_eq!(fourth.updated, 0);
        assert_eq!(fourth.deleted, 1);
        // The deleted row is gone and no remaining node mentions the id.
        let hits = search_index(&connection, "TERM-B", 10).unwrap();
        assert!(hits.is_empty());
    }

    #[test]
    fn tags_links_and_backlinks_are_indexed() {
        let directory = tempfile::tempdir().unwrap();
        node(
            &directory.path().join("terms/TERM-A.md").to_string_lossy(),
            "---\nid: TERM-A\ntype: term\ntags: [auth]\n---\n# Auth\n\n[[TERM-JWT]]\n",
        );
        node(
            &directory.path().join("terms/TERM-JWT.md").to_string_lossy(),
            "---\nid: TERM-JWT\ntype: term\n---\n# JWT\n",
        );
        let connection = Connection::open_in_memory().unwrap();
        rebuild_search_index(&connection, &parse_vault(directory.path()).unwrap()).unwrap();

        let by_tag = search_index(&connection, "auth", 10).unwrap();
        assert!(by_tag.iter().any(|hit| hit.id == "TERM-A"));

        let by_link = search_index(&connection, "TERM-JWT", 10).unwrap();
        assert!(by_link.iter().any(|hit| hit.id == "TERM-A"));
        assert!(by_link.iter().any(|hit| hit.id == "TERM-JWT"));

        let by_body = search_index(&connection, "JWT", 10).unwrap();
        assert!(by_body.iter().any(|hit| hit.id == "TERM-JWT"));
    }
}
