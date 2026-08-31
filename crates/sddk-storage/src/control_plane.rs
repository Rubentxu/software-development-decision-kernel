//! SQLite adapter for the [`sddk_domain::ControlPlane`] port (SDDK2-103).
//!
//! Lives in `sddk-storage` so it can depend on `rusqlite` without pulling
//! that constraint into `sddk-domain`.

use std::path::Path;
use std::result::Result as StdResult;

use rusqlite::{Connection, params};
use sddk_domain::{ControlPlane, MetricsRecord, StorageError as DomainStorageError, UatResultRow};

/// Schema v1 of the control-plane store (copied verbatim from
/// `crates/sddk-cli/src/telemetry.rs`; no semantic changes).
pub const SCHEMA_V1: &str = r#"
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS projects (
    project_id   TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    scope        TEXT NOT NULL DEFAULT '.',
    remote_url   TEXT,
    first_seen   TEXT NOT NULL,
    last_seen    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS cycles (
    cycle_id                  TEXT PRIMARY KEY,
    project_id                TEXT NOT NULL REFERENCES projects(project_id),
    path                      TEXT NOT NULL DEFAULT 'unknown',
    context_quality           TEXT NOT NULL DEFAULT 'C2',
    phase_durations_sec       TEXT NOT NULL DEFAULT '{}',
    coherence_scores          TEXT NOT NULL DEFAULT '[]',
    correction_cycles         INTEGER NOT NULL DEFAULT 0,
    tokens_used               INTEGER NOT NULL DEFAULT 0,
    cost_estimate_usd         REAL NOT NULL DEFAULT 0.0,
    costs                     TEXT NOT NULL DEFAULT '{}',
    first_pass_success        INTEGER NOT NULL DEFAULT 0,
    verify_verdict            TEXT NOT NULL DEFAULT 'UNKNOWN',
    merged_to_main            INTEGER NOT NULL DEFAULT 0,
    tag_version               TEXT,
    lead_time_hours           REAL,
    teleological_coherence_pct REAL,
    recorded_at               TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS aggregates (
    window_days   INTEGER NOT NULL,
    computed_at   TEXT NOT NULL,
    payload_json  TEXT NOT NULL,
    PRIMARY KEY (window_days)
);

CREATE INDEX IF NOT EXISTS idx_cycles_project ON cycles(project_id);
CREATE INDEX IF NOT EXISTS idx_cycles_recorded ON cycles(recorded_at);

CREATE TABLE IF NOT EXISTS uat_results (
    project_id     TEXT NOT NULL REFERENCES projects(project_id),
    tag_version    TEXT NOT NULL,
    verdict        TEXT NOT NULL,
    coverage_pct   REAL NOT NULL DEFAULT 0,
    defects        INTEGER NOT NULL DEFAULT 0,
    session_count  INTEGER NOT NULL DEFAULT 0,
    uat_duration_minutes INTEGER NOT NULL DEFAULT 0,
    recorded_at    TEXT NOT NULL,
    PRIMARY KEY (project_id, tag_version)
);
"#;

/// SQLite-backed [`ControlPlane`] adapter.
#[derive(Debug)]
pub struct SqliteControlPlane(Connection);

/// Row type returned by [`SqliteControlPlane::load_project_status`].
pub type ProjectStatusRow = (String, String, u32, u32, u32, Option<String>);

impl SqliteControlPlane {
    /// Opens (and initializes) the control-plane store at `dir`.
    ///
    /// Creates `dir` if it does not exist and runs `SCHEMA_V1`.
    pub fn open(dir: &Path) -> StdResult<Self, DomainStorageError> {
        std::fs::create_dir_all(dir).map_err(|e| DomainStorageError::Other(e.to_string()))?;
        let db_path = dir.join("control-plane.sqlite");
        let conn =
            Connection::open(&db_path).map_err(|e| DomainStorageError::Database(e.to_string()))?;
        conn.execute_batch(SCHEMA_V1)
            .map_err(|e| DomainStorageError::Database(e.to_string()))?;
        Ok(Self(conn))
    }

    /// Opens an in-memory store without running the schema.
    ///
    /// This preserves the existing `--dry-run` semantics where the store
    /// is used only for validation without persisting anything.
    pub fn open_in_memory() -> StdResult<Self, DomainStorageError> {
        let conn = Connection::open_in_memory()
            .map_err(|e| DomainStorageError::Database(e.to_string()))?;
        Ok(Self(conn))
    }

    /// Loads project status summary (used by `sddk telemetry status`).
    pub fn load_project_status(&self) -> StdResult<Vec<ProjectStatusRow>, DomainStorageError> {
        let mut stmt = self
            .0
            .prepare(
                r#"
                SELECT p.project_id, p.display_name,
                       COUNT(c.cycle_id),
                       COALESCE(SUM(CASE WHEN c.cost_estimate_usd > 0.0 THEN 1 ELSE 0 END), 0),
                       COALESCE(SUM(CASE WHEN c.teleological_coherence_pct IS NOT NULL THEN 1 ELSE 0 END), 0),
                       MAX(c.recorded_at)
                FROM projects p
                LEFT JOIN cycles c ON c.project_id = p.project_id
                GROUP BY p.project_id, p.display_name
                ORDER BY p.display_name
                "#,
            )
            .map_err(|e| DomainStorageError::Database(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)? as u32,
                    row.get::<_, i64>(3)? as u32,
                    row.get::<_, i64>(4)? as u32,
                    row.get::<_, Option<String>>(5)?,
                ))
            })
            .map_err(|e| DomainStorageError::Database(e.to_string()))?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| DomainStorageError::Database(e.to_string()))?);
        }
        Ok(results)
    }
}

impl ControlPlane for SqliteControlPlane {
    fn store_exists(&self) -> bool {
        // The in-memory connection always "exists" for the purpose of
        // dry-run; the actual file-based store is checked by callers.
        true
    }

    fn upsert_project(
        &mut self,
        project_id: &str,
        display_name: &str,
        scope: &str,
        remote_url: Option<&str>,
        now: &str,
    ) -> StdResult<(), DomainStorageError> {
        self.0
            .execute(
                r#"
                INSERT INTO projects (project_id, display_name, scope, remote_url, first_seen, last_seen)
                VALUES (?1, ?2, ?3, ?4, ?5, ?5)
                ON CONFLICT(project_id) DO UPDATE SET
                    display_name = excluded.display_name,
                    scope = excluded.scope,
                    remote_url = excluded.remote_url,
                    last_seen = excluded.last_seen
                "#,
                params![project_id, display_name, scope, remote_url, now],
            )
            .map_err(|e| DomainStorageError::Database(e.to_string()))?;
        Ok(())
    }

    fn upsert_cycle(
        &mut self,
        project_id: &str,
        record: &MetricsRecord,
    ) -> StdResult<(), DomainStorageError> {
        let phase_durations = serde_json::to_string(&record.phase_durations_sec)
            .map_err(|e| DomainStorageError::Other(e.to_string()))?;
        let coherence_scores = serde_json::to_string(&record.coherence_scores)
            .map_err(|e| DomainStorageError::Other(e.to_string()))?;
        let costs = serde_json::to_string(&record.costs)
            .map_err(|e| DomainStorageError::Other(e.to_string()))?;
        self.0
            .execute(
                r#"
                INSERT INTO cycles (
                    cycle_id, project_id, path, context_quality, phase_durations_sec,
                    coherence_scores, correction_cycles, tokens_used, cost_estimate_usd,
                    costs, first_pass_success, verify_verdict, merged_to_main,
                    tag_version, lead_time_hours, teleological_coherence_pct, recorded_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
                ON CONFLICT(cycle_id) DO UPDATE SET
                    path = excluded.path,
                    context_quality = excluded.context_quality,
                    phase_durations_sec = excluded.phase_durations_sec,
                    coherence_scores = excluded.coherence_scores,
                    correction_cycles = excluded.correction_cycles,
                    tokens_used = excluded.tokens_used,
                    cost_estimate_usd = excluded.cost_estimate_usd,
                    costs = excluded.costs,
                    first_pass_success = excluded.first_pass_success,
                    verify_verdict = excluded.verify_verdict,
                    merged_to_main = excluded.merged_to_main,
                    tag_version = excluded.tag_version,
                    lead_time_hours = excluded.lead_time_hours,
                    teleological_coherence_pct = excluded.teleological_coherence_pct,
                    recorded_at = excluded.recorded_at
                "#,
                params![
                    record.cycle_id,
                    project_id,
                    record.path,
                    record.context_quality,
                    phase_durations,
                    coherence_scores,
                    record.correction_cycles as i64,
                    record.tokens_used as i64,
                    record.cost_estimate_usd,
                    costs,
                    record.first_pass_success,
                    record.verify_verdict,
                    record.merged_to_main,
                    record.tag_version,
                    record.lead_time_hours,
                    record.teleological_coherence_pct,
                    record.recorded_at,
                ],
            )
            .map_err(|e| DomainStorageError::Database(e.to_string()))?;
        Ok(())
    }

    fn upsert_aggregate(
        &mut self,
        window_days: u16,
        computed_at: &str,
        payload_json: &str,
    ) -> StdResult<(), DomainStorageError> {
        self.0
            .execute(
                r#"
                INSERT INTO aggregates (window_days, computed_at, payload_json)
                VALUES (?1, ?2, ?3)
                ON CONFLICT(window_days) DO UPDATE SET
                    computed_at = excluded.computed_at,
                    payload_json = excluded.payload_json
                "#,
                params![window_days as i64, computed_at, payload_json],
            )
            .map_err(|e| DomainStorageError::Database(e.to_string()))?;
        Ok(())
    }

    fn upsert_uat_result(&mut self, result: &UatResultRow) -> StdResult<(), DomainStorageError> {
        self.0
            .execute(
                r#"
                INSERT INTO uat_results (
                    project_id, tag_version, verdict, coverage_pct, defects,
                    session_count, uat_duration_minutes, recorded_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                ON CONFLICT(project_id, tag_version) DO UPDATE SET
                    verdict = excluded.verdict,
                    coverage_pct = excluded.coverage_pct,
                    defects = excluded.defects,
                    session_count = excluded.session_count,
                    uat_duration_minutes = excluded.uat_duration_minutes,
                    recorded_at = excluded.recorded_at
                "#,
                params![
                    result.project_id,
                    result.tag_version,
                    result.verdict,
                    result.coverage_pct,
                    result.defects,
                    result.session_count,
                    result.uat_duration_minutes,
                    result.recorded_at,
                ],
            )
            .map_err(|e| DomainStorageError::Database(e.to_string()))?;
        Ok(())
    }

    fn load_cycles(&self) -> StdResult<Vec<MetricsRecord>, DomainStorageError> {
        let mut stmt = self
            .0
            .prepare(
                "SELECT cycle_id, path, context_quality, phase_durations_sec, \
                 coherence_scores, correction_cycles, tokens_used, cost_estimate_usd, costs, \
                 first_pass_success, verify_verdict, merged_to_main, tag_version, \
                 lead_time_hours, teleological_coherence_pct, recorded_at FROM cycles",
            )
            .map_err(|e| DomainStorageError::Database(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                let parse_u64_map =
                    |value: String| serde_json::from_str(&value).unwrap_or_default();
                let parse_f64_map =
                    |value: String| serde_json::from_str(&value).unwrap_or_default();
                Ok(MetricsRecord {
                    cycle_id: row.get(0)?,
                    path: row.get(1)?,
                    context_quality: row.get(2)?,
                    phase_durations_sec: parse_u64_map(row.get::<_, String>(3)?),
                    coherence_scores: serde_json::from_str(&row.get::<_, String>(4)?)
                        .unwrap_or_default(),
                    correction_cycles: row.get::<_, i64>(5)? as u8,
                    tokens_used: row.get::<_, i64>(6)? as u64,
                    cost_estimate_usd: row.get(7)?,
                    costs: parse_f64_map(row.get::<_, String>(8)?),
                    first_pass_success: row.get(9)?,
                    verify_verdict: row.get(10)?,
                    merged_to_main: row.get(11)?,
                    tag_version: row.get(12)?,
                    lead_time_hours: row.get(13)?,
                    teleological_coherence_pct: row.get(14)?,
                    recorded_at: row.get(15)?,
                })
            })
            .map_err(|e| DomainStorageError::Database(e.to_string()))?;
        let mut records = Vec::new();
        for row in rows {
            records.push(row.map_err(|e| DomainStorageError::Database(e.to_string()))?);
        }
        Ok(records)
    }

    fn load_uat_results(&self) -> StdResult<Vec<UatResultRow>, DomainStorageError> {
        let mut stmt = self
            .0
            .prepare(
                "SELECT project_id, tag_version, verdict, coverage_pct, defects, \
                 session_count, uat_duration_minutes, recorded_at \
                 FROM uat_results ORDER BY recorded_at DESC",
            )
            .map_err(|e| DomainStorageError::Database(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(UatResultRow {
                    project_id: row.get(0)?,
                    tag_version: row.get(1)?,
                    verdict: row.get(2)?,
                    coverage_pct: row.get(3)?,
                    defects: row.get(4)?,
                    session_count: row.get(5)?,
                    uat_duration_minutes: row.get(6)?,
                    recorded_at: row.get(7)?,
                })
            })
            .map_err(|e| DomainStorageError::Database(e.to_string()))?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| DomainStorageError::Database(e.to_string()))?);
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_in_memory_succeeds_and_store_exists() {
        let plane = SqliteControlPlane::open_in_memory().unwrap();
        // dry-run store "exists" by design (no file to check)
        assert!(plane.store_exists());
        // In-memory without schema: verify store_exists still true
        // (actual UPSERTs require schema; the dry-run path validates
        // the connection is open without persisting anything)
        let mut plane2 = SqliteControlPlane::open_in_memory().unwrap();
        // This will fail with "no such table" which confirms the
        // dry-run path correctly does NOT create schema
        let result = plane2.upsert_project("p1", "p1", "uat", None, "2026-01-01");
        assert!(result.is_err()); // no schema → expected failure
    }

    #[test]
    fn upsert_cycle_roundtrips_with_schema() {
        // Use in-memory with schema for the roundtrip test
        let mut plane = SqliteControlPlane::open_in_memory().unwrap();
        plane.0.execute_batch(SCHEMA_V1).unwrap();
        // FK target must exist
        plane
            .upsert_project("p1", "p1", "uat", None, "2026-01-01")
            .unwrap();
        let record = MetricsRecord {
            cycle_id: "c1".into(),
            path: "a-lite".into(),
            context_quality: "C2".into(),
            phase_durations_sec: std::collections::HashMap::new(),
            coherence_scores: Vec::new(),
            correction_cycles: 0,
            tokens_used: 0,
            cost_estimate_usd: 0.0,
            first_pass_success: true,
            verify_verdict: "PASS".into(),
            merged_to_main: false,
            tag_version: None,
            lead_time_hours: None,
            teleological_coherence_pct: None,
            costs: std::collections::HashMap::new(),
            recorded_at: "2026-01-01T00:00:00Z".into(),
        };
        plane.upsert_cycle("p1", &record).unwrap();
        let loaded = plane.load_cycles().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].cycle_id, "c1");
    }
}
