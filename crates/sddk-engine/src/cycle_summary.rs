//! Derived cycle runtime summary (WU-C3 cutover, DELTA-CONF-004).
//!
//! Since the cycle/run lifecycle cutover, runtime-derived states
//! (approval-wait, UAT-wait, remediation, recovery) are NOT canonical
//! Cycle truth: the Cycle record stays at its last delivery-level status
//! and the runtime detail is derived from durable facts on the ledger —
//!
//! - approval waits: `approval.capability.requested` events without a
//!   matching `approval.capability.granted`/`denied` decision (same fact
//!   family as the `ApprovalProjection` in the CLI);
//! - UAT waits: a succeeded `phase.verify.uat.sync` transition
//!   (legacy `UAT_WAITING` entry) without a later succeeded
//!   `phase.uat.complete`;
//! - remediation/recovery: failed `cycle.transitioned` outcomes not yet
//!   closed by a succeeded remediation transition (`phase.verify.remediate`
//!   / `phase.build.remediate`). Recovery rounds (`release.recover`) fold
//!   into the same counter — both are failure-closure work.
//!
//! Everything here is READ-ONLY derivation over the `Ledger` port. No
//! function in this module writes a cycle row or constructs a
//! runtime-derived `CycleStatus` value (decode-only since C3).

use std::collections::{BTreeMap, BTreeSet};

use sddk_domain::{CycleStatus, Ledger, LedgerEvent, StorageError};

/// Derived runtime state of one cycle, computed from durable facts.
///
/// This is a projection summary, never persisted on the Cycle record.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CycleRuntimeSummary {
    /// Persisted delivery-level status of the Cycle record.
    pub persisted_status: CycleStatus,
    /// True while at least one approval request awaits a decision.
    pub approval_waiting: bool,
    /// Capabilities with an unresolved approval request, sorted.
    pub approval_waiting_on: Vec<String>,
    /// True while a UAT sync has succeeded without a later completion.
    pub uat_waiting: bool,
    /// True while the latest failure has no remediation completion after it.
    pub remediating: bool,
    /// Number of failed-gate (remediation/recovery round) transitions.
    pub remediation_rounds: u32,
    /// The single label this summary derives (empty when none apply).
    ///
    /// Precedence (deterministic, no-contradiction rule):
    /// approval-waiting > uat-waiting > remediating > persisted status.
    pub derived_state: String,
}

/// Transition whose succeeded outcome starts a UAT wait.
const UAT_START_TRANSITION: &str = "phase.verify.uat.sync";
/// Transition whose succeeded outcome closes a UAT wait.
const UAT_COMPLETE_TRANSITION: &str = "phase.uat.complete";
/// Transitions whose succeeded outcome closes an open remediation.
const REMEDIATION_DONE_TRANSITIONS: [&str; 2] = ["phase.verify.remediate", "phase.build.remediate"];

/// Derives the runtime summary for one cycle from ledger facts.
///
/// Read-only: walks the cycle's ledger events; never touches cycle rows.
pub fn derive_cycle_summary(
    ledger: &dyn Ledger,
    cycle_id: &str,
) -> Result<CycleRuntimeSummary, StorageError> {
    let record = ledger.get_cycle(cycle_id)?;
    let events = ledger.list_cycle_events(cycle_id)?;
    Ok(summarize_cycle_events(record.manifest.status, &events))
}

/// The event-derived half of [`CycleRuntimeSummary`], without the persisted
/// status.
///
/// The split exists because the two halves fail differently. The persisted
/// status lives in a cycle ROW, and a row this build cannot deserialize is a
/// real population — 81 of 179 on the project that motivated `list_cycles`
/// (INC-DEBT-060, D1). The derived facts live in the event log, and the log
/// costs one read regardless of how many cycles are asked about.
///
/// An earlier version of [`derive_all_cycle_facts`] read the row per cycle
/// (`get_cycle`) and propagated its error with `?`. On a ledger holding two
/// unreadable manifests that turns ONE unreadable row into the whole
/// enumeration reading `unknown` — strictly worse than the per-row behaviour
/// it replaced, and worse precisely where the product is thinnest. Hence: the
/// derivation here never reads a cycle row.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CycleRuntimeFacts {
    /// True while at least one approval request awaits a decision.
    pub approval_waiting: bool,
    /// Capabilities with an unresolved approval request, sorted.
    pub approval_waiting_on: Vec<String>,
    /// True while a UAT sync has succeeded without a later completion.
    pub uat_waiting: bool,
    /// True while the latest failure has no remediation completion after it.
    pub remediating: bool,
    /// Number of failed-gate (remediation/recovery round) transitions.
    pub remediation_rounds: u32,
    /// The single label these facts derive (empty when none apply).
    ///
    /// Same precedence as [`CycleRuntimeSummary::derived_state`], because it
    /// is the same function computing it.
    pub derived_state: String,
}

/// Derives the runtime facts for every REQUESTED cycle, reading the event log
/// **once**.
///
/// Why one read and not one per cycle, measured: `list_cycle_events` is
/// "materialise the whole event log, then filter by cycle", so calling it per
/// cycle re-reads the entire log once per cycle. On `p-63676b11dc0ef88f` that
/// is 109 reads of a 651-event log to answer a question about 188 rows, and
/// it took `cycle list` from **0.05 s to 2.38 s** in a *release* build — on a
/// surface that release gates and scripts call on every run.
///
/// Why the caller names the cycles instead of this function discovering them:
/// the enumeration's rows come from `cycles`, not from the event log, so only
/// the caller knows the full set. Returning one entry per requested id —
/// including for a cycle that has no events at all — means the caller never
/// has to read an ABSENCE as "nothing pending". Absence-as-proof is sound (the
/// read either succeeded, or this function returned `Err` and no entry exists
/// for anyone), but it is an argument the reader has to reconstruct. A row
/// that is explicitly present, with a label, is a fact.
///
/// The output is exactly one entry per distinct requested id: the request is
/// collected into a `BTreeSet` first, so a repeated id cannot produce a
/// half-answered map, and the only fallible call happens before any entry is
/// written, so a failure can never leave a partial answer behind.
pub fn derive_all_cycle_facts<'a, I>(
    ledger: &dyn Ledger,
    cycle_ids: I,
) -> Result<BTreeMap<String, CycleRuntimeFacts>, StorageError>
where
    I: IntoIterator<Item = &'a str>,
{
    // The single read. If it fails this returns `Err` and NO row is derivable,
    // which the caller must render as "unknown" on every row rather than as
    // "nothing pending" on any.
    let events = ledger.list_events_after(0, i64::MAX)?;
    let mut by_cycle: BTreeMap<&str, Vec<&LedgerEvent>> = BTreeMap::new();
    for event in &events {
        if let Some(cycle_id) = event.cycle_id.as_deref() {
            by_cycle.entry(cycle_id).or_default().push(event);
        }
    }
    let requested: BTreeSet<&str> = cycle_ids.into_iter().collect();
    let mut out = BTreeMap::new();
    for cycle_id in requested {
        let mut cycle_events: Vec<LedgerEvent> = by_cycle
            .get(cycle_id)
            .map(|events| events.iter().map(|event| (*event).clone()).collect())
            .unwrap_or_default();
        // Ordered per cycle. The `remediating` label is last-writer-wins over
        // transitions, so the rule genuinely needs a sequence order.
        // `list_events_after` returns the log globally ordered and a
        // per-cycle subsequence of an ordered sequence is ordered — but this
        // function is handed a `&dyn Ledger` it does not control, so it
        // re-establishes the invariant locally instead of inheriting it.
        cycle_events.sort_by_key(|event| event.sequence);
        out.insert(cycle_id.to_owned(), derive_runtime_facts(&cycle_events));
    }
    Ok(out)
}

/// The derivation itself, over events already in hand.
///
/// THE single implementation of the rule. Every entry point in this module —
/// `derive_cycle_summary` for one cycle, `derive_all_cycle_facts` for an
/// enumeration — reaches its label through here. Two private copies of a
/// precedence rule is exactly how the operator view and `cycle status` end up
/// disagreeing, which is what INC-DEBT-067 existed to stop.
fn derive_runtime_facts(events: &[LedgerEvent]) -> CycleRuntimeFacts {
    let mut requested: Vec<(String, String)> = Vec::new();
    let mut decided: BTreeSet<(String, String)> = BTreeSet::new();
    for event in events {
        let Some(capability) = event.payload.get("capability").and_then(|v| v.as_str()) else {
            continue;
        };
        let hash = event
            .payload
            .get("request_hash")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        match event.event_type.as_str() {
            "approval.capability.requested" => {
                requested.push((capability.to_owned(), hash.to_owned()));
            }
            "approval.capability.granted" | "approval.capability.denied" => {
                decided.insert((capability.to_owned(), hash.to_owned()));
            }
            _ => {}
        }
    }
    let mut approval_waiting_on: Vec<String> = requested
        .iter()
        .filter(|(capability, hash)| !decided.contains(&(capability.clone(), hash.clone())))
        .map(|(capability, _)| capability.clone())
        .collect();
    approval_waiting_on.sort();
    approval_waiting_on.dedup();
    let approval_waiting = !approval_waiting_on.is_empty();

    // ── Transition outcome facts ────────────────────────────────────────
    // `cycle.transitioned` (engine apply path) and
    // `workflow.transition.succeeded`/`workflow.transition.failed`
    // (event-bus canonical events) both carry transition_id + outcome.
    let mut uat_started = false;
    let mut uat_completed = false;
    let mut remediation_open = false;
    let mut remediation_rounds: u32 = 0;
    for event in events {
        let transition_id = event
            .payload
            .get("transition_id")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let outcome = event
            .payload
            .get("outcome")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let succeeded = outcome == "succeeded";
        match event.event_type.as_str() {
            "cycle.transitioned" | "workflow.transition.succeeded" if succeeded => {
                if REMEDIATION_DONE_TRANSITIONS.contains(&transition_id) {
                    remediation_open = false;
                }
                if transition_id == UAT_START_TRANSITION {
                    uat_started = true;
                }
                if transition_id == UAT_COMPLETE_TRANSITION {
                    uat_completed = true;
                }
            }
            "cycle.transitioned" | "workflow.transition.failed" if outcome == "failed" => {
                remediation_rounds += 1;
                remediation_open = true;
            }
            _ => {}
        }
    }
    let uat_waiting = uat_started && !uat_completed;

    // ── Deterministic single label (no-contradiction rule) ──────────────
    let derived_state = if approval_waiting {
        "approval-waiting".to_owned()
    } else if uat_waiting {
        "uat-waiting".to_owned()
    } else if remediation_open {
        "remediating".to_owned()
    } else {
        String::new()
    };

    CycleRuntimeFacts {
        approval_waiting,
        approval_waiting_on,
        uat_waiting,
        remediating: remediation_open,
        remediation_rounds,
        derived_state,
    }
}

/// [`derive_runtime_facts`] plus the persisted status of the cycle row.
///
/// Mechanical, and carrying no rule of its own: the mapping is six field
/// copies so that adding a derived fact cannot silently leave this
/// constructor behind, but the VALUE of every field comes from
/// `derive_runtime_facts`.
fn summarize_cycle_events(
    persisted_status: CycleStatus,
    events: &[LedgerEvent],
) -> CycleRuntimeSummary {
    let facts = derive_runtime_facts(events);
    CycleRuntimeSummary {
        persisted_status,
        approval_waiting: facts.approval_waiting,
        approval_waiting_on: facts.approval_waiting_on,
        uat_waiting: facts.uat_waiting,
        remediating: facts.remediating,
        remediation_rounds: facts.remediation_rounds,
        derived_state: facts.derived_state,
    }
}
