//! Backlog Ledger substrate types.
//!
//! Implements the engine-side types for:
//! - [[REQ-Backlog-Item-Capture]]
//! - [[REQ-Backlog-Item-Triage-Priority]]
//! - [[REQ-Backlog-Item-Promote-Discard]]
//! - [[REQ-Backlog-Roadmap-Projection]] (projection predicate `live_items`)
//!
//! CLI surface (`sddk backlog capture`, etc.) is deferred to subsequent cycles.
//! This module ships the durable substrate: closed-set enums, the
//! materialised item row, and the closed-set error type.

use serde::{Deserialize, Serialize};

/// Stable wikilink-friendly item id (e.g. "B-001", "B-042").
///
/// Convention: `B-NNN` where NNN is a zero-padded monotonically increasing
/// counter per project. The substrate stores any string; the CLI
/// (cycle 2/4) is responsible for generating fresh ids.
pub type BacklogItemId = String;

/// Backlog item priority (closed set per [[ADR-0080-cycle-pause]]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BacklogPriority {
    /// P0 — highest priority, drop everything else.
    P0,
    /// P1 — high priority.
    P1,
    /// P2 — medium priority (default for new captures).
    P2,
    /// P3 — low priority.
    P3,
}

impl std::str::FromStr for BacklogPriority {
    type Err = BacklogError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "P0" => Ok(Self::P0),
            "P1" => Ok(Self::P1),
            "P2" => Ok(Self::P2),
            "P3" => Ok(Self::P3),
            other => Err(BacklogError::InvalidPriority(other.to_string())),
        }
    }
}

impl std::fmt::Display for BacklogPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::P0 => "P0",
            Self::P1 => "P1",
            Self::P2 => "P2",
            Self::P3 => "P3",
        };
        f.write_str(s)
    }
}

/// Backlog item discard reason — the **single authority** for the closed
/// set of reasons a `backlog.item.discarded` event may carry
/// ([[REQ-Backlog-Item-Promote-Discard]]).
///
/// This enum, not the CLI, defines what the set *is*. The CLI's clap
/// enum is a thin adapter that must map onto these variants
/// bijectively; the schema validates membership against
/// [`BacklogDiscardReason::ALL`]; and the message rendered by
/// [`BacklogError::InvalidDiscardReason`] is produced from [`Self::ALL`]
/// rather than written out by hand. Before this type existed the set was
/// enforced in exactly one place — the CLI's `ValueEnum` — while the
/// domain declared an `InvalidDiscardReason` error it never raised, the
/// schema accepted any string, and the storage's own tests wrote
/// `"won't fix"`, `"done"` and `"x"`. A closed set that only one argument
/// parser enforces is not a property of the ledger.
///
/// [`resolved`](Self::Resolved) is the member that lets the authority
/// record "this was real, and it is now fixed" without lying: `wontfix`
/// would assert a refusal that never happened, and `superseded` would
/// require an existing successor, manufacturing lineage that does not
/// exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BacklogDiscardReason {
    /// Replaced by another item, which must be named and must exist.
    Superseded,
    /// Reviewed and deliberately not acted on.
    Wontfix,
    /// Restates an item already tracked elsewhere.
    Duplicate,
    /// The concern was real and has been addressed. The work is done;
    /// the item is closed without a successor because nothing replaced
    /// it — the original code or process did.
    Resolved,
}

impl BacklogDiscardReason {
    /// Every member of the closed set, in declaration order.
    ///
    /// This is what the set *is*: membership checks, CLI bijection
    /// guards and the error message all read from here, so a member
    /// added to the enum cannot be forgotten by any of them.
    pub const ALL: [Self; 4] = [
        Self::Superseded,
        Self::Wontfix,
        Self::Duplicate,
        Self::Resolved,
    ];

    /// Whether discarding under this reason requires naming a successor.
    ///
    /// Only [`Superseded`](Self::Superseded) does. Superseding without a
    /// successor loses every finding that lived only in the discarded
    /// item; the other three reasons close the item without handing its
    /// content to a replacement, so demanding a successor would force
    /// fabricated lineage (agent-secretless report D3).
    pub const fn requires_successor(self) -> bool {
        matches!(self, Self::Superseded)
    }
}

impl std::str::FromStr for BacklogDiscardReason {
    type Err = BacklogError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "superseded" => Ok(Self::Superseded),
            "wontfix" => Ok(Self::Wontfix),
            "duplicate" => Ok(Self::Duplicate),
            "resolved" => Ok(Self::Resolved),
            other => Err(BacklogError::InvalidDiscardReason(other.to_string())),
        }
    }
}

impl std::fmt::Display for BacklogDiscardReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Superseded => "superseded",
            Self::Wontfix => "wontfix",
            Self::Duplicate => "duplicate",
            Self::Resolved => "resolved",
        })
    }
}

/// Renders the closed set of discard reasons for diagnostics.
///
/// Exists so the member list is read from [`BacklogDiscardReason::ALL`]
/// rather than transcribed into an error message, which is how the old
/// hand-written message came to name three members while the set it
/// claimed to describe was enforced nowhere.
pub fn expected_discard_reasons() -> String {
    BacklogDiscardReason::ALL
        .iter()
        .map(|r| r.to_string())
        .collect::<Vec<_>>()
        .join("|")
}

/// Backlog item current status (derived from latest event kind).
///
/// This is a **projection** over the event log per ADR-0095 (Object state
/// class) + ADR-0094 (one canonical fact log): status is always derived
/// from the latest event in `backlog_item_events_v1` for the item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BacklogStatus {
    /// Item exists (after a `backlog.item.registered` event).
    Registered,
    /// Item has been triaged (latest event is `backlog.item.triaged`).
    Triaged,
    /// Item has been promoted to a target cycle (latest event is
    /// `backlog.item.promoted`). Live-items predicate excludes this.
    Promoted,
    /// Item has been discarded (latest event is `backlog.item.discarded`).
    /// Live-items predicate excludes this.
    Discarded,
}

impl BacklogStatus {
    /// Returns whether this status counts as "live" for the renderer.
    ///
    /// Live = Registered OR Triaged. Promoted and Discarded are terminal
    /// (per REQ-Backlog-Roadmap-Projection §Scenario 1: discarded items
    /// are NOT rendered; promoted items move to a cycle and leave the
    /// backlog).
    pub fn is_live(self) -> bool {
        matches!(self, Self::Registered | Self::Triaged)
    }
}

impl std::str::FromStr for BacklogStatus {
    type Err = BacklogError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "registered" => Ok(Self::Registered),
            "triaged" => Ok(Self::Triaged),
            "promoted" => Ok(Self::Promoted),
            "discarded" => Ok(Self::Discarded),
            other => Err(BacklogError::InvalidStatus(other.to_string())),
        }
    }
}

impl std::fmt::Display for BacklogStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Registered => "registered",
            Self::Triaged => "triaged",
            Self::Promoted => "promoted",
            Self::Discarded => "discarded",
        };
        f.write_str(s)
    }
}

/// Render kind (used by cycle 4/4 renderer: `sddk backlog render` /
/// `sddk roadmap render`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BacklogRenderKind {
    /// Render `BACKLOG.md`.
    Backlog,
    /// Render `ROADMAP.md`.
    Roadmap,
}

impl std::str::FromStr for BacklogRenderKind {
    type Err = BacklogError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "backlog" => Ok(Self::Backlog),
            "roadmap" => Ok(Self::Roadmap),
            other => Err(BacklogError::InvalidRenderKind(other.to_string())),
        }
    }
}

impl std::fmt::Display for BacklogRenderKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Backlog => "backlog",
            Self::Roadmap => "roadmap",
        };
        f.write_str(s)
    }
}

/// Materialised row from `backlog_items_v1` table.
///
/// This is the projection the renderer (cycle 4/4) iterates over.
/// Computed deterministically: `current_priority` is the priority of the
/// latest `backlog.item.triaged` event whose `valid_to = '9999-12-31T23:59:59Z'`;
/// `current_status` is the latest event kind among {registered, triaged,
/// promoted, discarded}.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BacklogItemRow {
    /// Stable wikilink handle (`B-001`, `B-042`, ...).
    pub item_id: BacklogItemId,
    /// Origin cycle id (from the `registered` event's `origin_cycle_id`).
    pub origin_cycle_id: String,
    /// Origin phase (from the `registered` event's `origin_phase`).
    pub origin_phase: String,
    /// One-line summary (from the `registered` event's `summary`).
    pub summary: String,
    /// Current priority (None until triaged).
    pub current_priority: Option<BacklogPriority>,
    /// Current status (always set after first event).
    pub current_status: BacklogStatus,
    /// RFC3339 timestamp of the `registered` event.
    pub captured_at: String,
    /// Total number of events emitted for this item (>= 1).
    pub emitted_event_count: i64,
}

/// Closed-set error type for backlog storage operations.
///
/// Mirrors `WorkflowRunPersistError` precedent (6 variants, all named,
/// all carry the recovery context).
#[derive(Debug, thiserror::Error)]
pub enum BacklogError {
    /// `item(&id)` lookup miss.
    #[error("backlog item not found: {0}")]
    ItemNotFound(BacklogItemId),
    /// Schema registry did not recognise the event type.
    #[error("invalid event type: {0}")]
    InvalidEventType(String),
    /// Parse-time rejection of an unknown priority value (closed set).
    #[error("invalid priority value: {0}; expected one of P0|P1|P2|P3")]
    InvalidPriority(String),
    /// Parse-time rejection of an unknown status value (closed set).
    #[error("invalid status value: {0}; expected one of registered|triaged|promoted|discarded")]
    InvalidStatus(String),
    /// Parse-time rejection of an unknown render kind value.
    #[error("invalid render kind: {0}; expected one of backlog|roadmap")]
    InvalidRenderKind(String),
    /// SQLite / rusqlite failure propagated from the storage layer
    /// (kept as String to avoid a dependency cycle; the storage
    /// crate re-wraps the raw error before crossing the boundary).
    #[error("storage error: {0}")]
    Storage(String),
    /// JSON parse / serialise failure.
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
    /// Capture called with empty `summary` (closed contract: a
    /// backlog item without a one-line description is meaningless).
    #[error("backlog summary must be non-empty")]
    EmptySummary,
    /// Capture called with empty origin evidence
    /// (either `origin_cycle_id` or `origin_phase` blank).
    #[error(
        "backlog origin evidence required (origin_cycle_id and origin_phase must be non-empty)"
    )]
    OriginEvidenceRequired,
    /// Promote/discard called on an item already discarded (terminal
    /// state; REQ-Backlog-Item-Promote-Discard scenario 2).
    #[error("backlog item already discarded: {0}")]
    AlreadyDiscarded(BacklogItemId),
    /// Promote called on an item already promoted to a cycle
    /// (re-promotion to a second cycle is forbidden).
    #[error("backlog item already promoted: {0}")]
    AlreadyPromoted(BacklogItemId),
    /// Discard called with a reason outside the closed set
    /// defined by [`BacklogDiscardReason`].
    ///
    /// The expected list is rendered from [`BacklogDiscardReason::ALL`],
    /// so this message cannot drift away from the set it describes.
    #[error(
        "invalid discard reason: {found}; expected one of {expected}",
        found = .0,
        expected = expected_discard_reasons()
    )]
    InvalidDiscardReason(String),
    /// Render/promote called on an item that never had a triage
    /// (priority is a mandatory renderer column per the projection spec).
    #[error("backlog item is not triaged: {0}")]
    NotTriaged(BacklogItemId),
}

/// One row of the chronological event log for a backlog item.
///
/// Returned by `BacklogStore::events(&id)` so CLI consumers can
/// render the full audit trail. Mirrors the columns of
/// `backlog_item_events_v1` plus a parsed `payload: serde_json::Value`
/// (the on-disk column is `payload_json TEXT`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BacklogEventLogEntry {
    /// Monotonic event id within the `backlog_item_events_v1` table.
    pub event_id: i64,
    /// Stable event-type identifier (e.g. `backlog.item.registered`).
    pub event_type: String,
    /// Schema version of the payload (currently always 1).
    pub schema_version: i64,
    /// RFC 3339 timestamp emitted by the writer.
    pub emitted_at: String,
    /// Actor that caused the event (e.g. `orchestrator:jcode`).
    pub actor_ref: String,
    /// Parsed JSON payload matching the event-type schema.
    pub payload: serde_json::Value,
}

/// Crockford base32 alphabet used by ULIDs.
const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Generates a 26-character ULID prefixed with `bl-`.
///
/// Layout: 48-bit timestamp (ms since UNIX epoch) + 80-bit
/// randomness. The randomness portion is seeded from the
/// process clock + an atomic counter so two consecutive calls
/// return distinct ids without requiring a new crate dependency
/// (consistent with the project's minimal-dep posture for the
/// backlog module).
pub fn generate_ulid() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0);
    let tick = COUNTER.fetch_add(1, Ordering::Relaxed);
    let rand80 = (nanos.rotate_left(7) as u128)
        ^ (tick.rotate_left(13) as u128)
        ^ ((now as u128).rotate_left(21));

    let mut bits: u128 =
        ((now as u128 & 0xFFFF_FFFF_FFFF) << 80) | (rand80 & 0xFFFF_FFFF_FFFF_FFFF_FFFF);
    let mut out = [0u8; 26];
    for i in (0..26).rev() {
        out[i] = CROCKFORD[(bits & 0x1F) as usize];
        bits >>= 5;
    }
    let s: String = out.iter().map(|b| *b as char).collect();
    format!("bl-{s}")
}

/// Returns the current wall-clock time formatted as RFC 3339.
///
/// Mirrors the convention used by `ledger_events.emitted_at`:
/// `2026-09-12T13:05:30Z` (UTC, second precision). For
/// sub-second resolution, callers can store the millis-since-
/// epoch returned by `crate::now_ms_since_epoch` separately.
pub fn now_rfc3339() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_rfc3339_from_secs(now)
}

/// Formats a UNIX-epoch second count as an RFC 3339 UTC string
/// (`YYYY-MM-DDTHH:MM:SSZ`). Pure function — exposed for tests
/// that need deterministic timestamps.
pub fn format_rfc3339_from_secs(secs: u64) -> String {
    // Civil-from-days algorithm (Howard Hinnant).
    let z = (secs / 86_400) as i64;
    let s = (secs % 86_400) as u32;
    let hour = s / 3600;
    let min = (s % 3600) / 60;
    let sec = s % 60;
    let z2 = z + 719468;
    let era = if z2 >= 0 { z2 } else { z2 - 146096 } / 146097;
    let doe = (z2 - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, m, d, hour, min, sec
    )
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Every member of the closed set must survive a
    /// `Display` -> `FromStr` round trip. A member whose wire spelling
    /// differs from its parse spelling would be writable but
    /// unreadable, and the ledger would silently stop round-tripping.
    #[test]
    fn discard_reason_round_trip_for_every_member() {
        for r in BacklogDiscardReason::ALL {
            let s = r.to_string();
            let back: BacklogDiscardReason = s.parse().unwrap_or_else(|e| {
                panic!("member {r:?} prints as {s:?} but does not parse back: {e}")
            });
            assert_eq!(r, back, "member {r:?} did not round-trip through {s:?}");
        }
    }

    /// `ALL` is the definition of the set, so a value the parser accepts
    /// but `ALL` omits is a member the rest of the system cannot see.
    #[test]
    fn every_parseable_reason_is_listed_in_all() {
        for s in [
            "superseded",
            "wontfix",
            "duplicate",
            "resolved",
            "",
            "won't fix",
            "done",
            "x",
            "Resolved",
            "RESOLVED",
            " resolved",
        ] {
            let parsed: Result<BacklogDiscardReason, _> = s.parse();
            if let Ok(r) = parsed {
                assert!(
                    BacklogDiscardReason::ALL.contains(&r),
                    "{s:?} parses to {r:?} but ALL does not list it"
                );
            }
        }
    }

    /// Values outside the set are refused, and the refusal names the
    /// set rendered from `ALL` — including the members the old
    /// hand-written message omitted.
    #[test]
    fn reasons_outside_the_set_are_refused_and_the_message_lists_every_member() {
        for s in ["won't fix", "done", "x", "", "banana"] {
            let err = match s.parse::<BacklogDiscardReason>() {
                Ok(r) => panic!("{s:?} must be refused, but it parsed to {r:?}"),
                Err(e) => e,
            };
            let msg = err.to_string();
            for r in BacklogDiscardReason::ALL {
                assert!(
                    msg.contains(&r.to_string()),
                    "refusal message {msg:?} omits member {r}"
                );
            }
        }
    }

    /// Only `superseded` hands content to a replacement. Demanding a
    /// successor from any other reason would force fabricated lineage.
    #[test]
    fn only_superseded_requires_a_successor() {
        assert!(BacklogDiscardReason::Superseded.requires_successor());
        for r in BacklogDiscardReason::ALL {
            if r != BacklogDiscardReason::Superseded {
                assert!(!r.requires_successor(), "{r:?} must not demand a successor");
            }
        }
    }

    /// The set is what the error message says it is, checked through
    /// the rendered text rather than through the helper that built it.
    #[test]
    fn expected_reasons_string_matches_the_members() {
        assert_eq!(
            expected_discard_reasons(),
            "superseded|wontfix|duplicate|resolved"
        );
        assert_eq!(
            BacklogDiscardReason::ALL.len(),
            expected_discard_reasons().split('|').count(),
            "the rendered list and the member list disagree in length"
        );
    }

    #[test]
    fn backlog_priority_round_trip() {
        for p in [
            BacklogPriority::P0,
            BacklogPriority::P1,
            BacklogPriority::P2,
            BacklogPriority::P3,
        ] {
            let s = serde_json::to_string(&p).unwrap();
            let back: BacklogPriority = serde_json::from_str(&s).unwrap();
            assert_eq!(p, back);
        }
    }

    #[test]
    fn backlog_priority_rejects_unknown() {
        let result: Result<BacklogPriority, _> = serde_json::from_str("\"P5\"");
        assert!(result.is_err(), "P5 must be rejected by serde");
    }

    #[test]
    fn backlog_priority_fromstr() {
        assert_eq!(
            "P0".parse::<BacklogPriority>().unwrap(),
            BacklogPriority::P0
        );
        assert!("P9".parse::<BacklogPriority>().is_err());
    }

    #[test]
    fn backlog_status_round_trip() {
        for s in [
            BacklogStatus::Registered,
            BacklogStatus::Triaged,
            BacklogStatus::Promoted,
            BacklogStatus::Discarded,
        ] {
            let json = serde_json::to_string(&s).unwrap();
            let back: BacklogStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(s, back);
        }
    }

    #[test]
    fn backlog_status_rejects_unknown() {
        let result: Result<BacklogStatus, _> = serde_json::from_str("\"unknown\"");
        assert!(result.is_err(), "unknown must be rejected by serde");
    }

    #[test]
    fn backlog_status_is_live_predicate() {
        assert!(BacklogStatus::Registered.is_live());
        assert!(BacklogStatus::Triaged.is_live());
        assert!(!BacklogStatus::Promoted.is_live());
        assert!(!BacklogStatus::Discarded.is_live());
    }

    #[test]
    fn backlog_status_fromstr() {
        assert_eq!(
            "registered".parse::<BacklogStatus>().unwrap(),
            BacklogStatus::Registered
        );
        assert_eq!(
            "triaged".parse::<BacklogStatus>().unwrap(),
            BacklogStatus::Triaged
        );
        assert_eq!(
            "promoted".parse::<BacklogStatus>().unwrap(),
            BacklogStatus::Promoted
        );
        assert_eq!(
            "discarded".parse::<BacklogStatus>().unwrap(),
            BacklogStatus::Discarded
        );
        assert!("bogus".parse::<BacklogStatus>().is_err());
    }

    #[test]
    fn backlog_render_kind_round_trip() {
        for k in [BacklogRenderKind::Backlog, BacklogRenderKind::Roadmap] {
            let json = serde_json::to_string(&k).unwrap();
            let back: BacklogRenderKind = serde_json::from_str(&json).unwrap();
            assert_eq!(k, back);
        }
    }

    #[test]
    fn backlog_render_kind_fromstr() {
        assert_eq!(
            "backlog".parse::<BacklogRenderKind>().unwrap(),
            BacklogRenderKind::Backlog
        );
        assert_eq!(
            "roadmap".parse::<BacklogRenderKind>().unwrap(),
            BacklogRenderKind::Roadmap
        );
        assert!("other".parse::<BacklogRenderKind>().is_err());
    }

    #[test]
    fn backlog_item_row_serialises() {
        let row = BacklogItemRow {
            item_id: "B-001".to_string(),
            origin_cycle_id: "p-test/cycle-x".to_string(),
            origin_phase: "explore".to_string(),
            summary: "implement backlog ledger substrate".to_string(),
            current_priority: Some(BacklogPriority::P1),
            current_status: BacklogStatus::Triaged,
            captured_at: "2026-09-12T12:00:00Z".to_string(),
            emitted_event_count: 2,
        };
        let json = serde_json::to_string(&row).unwrap();
        let back: BacklogItemRow = serde_json::from_str(&json).unwrap();
        assert_eq!(row, back);
    }
}
