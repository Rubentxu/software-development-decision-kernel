//! Experience Episodes & Process Mining
//!
//! Records what actually happened during a cycle (events, decisions,
//! delegations, lab runs, promotion verdicts) into a plain-data
//! `ExperienceEpisode`. Output is deterministic; the caller supplies
//! every timestamp.
//!
//! Pattern: P-a (struct + trait + plain-data evaluator).

use std::collections::BTreeMap;

// ── Audit guard constants ────────────────────────────────────────────────

/// Closed-set size of [`EpisodeEventKind`]. Bump when a variant is added.
pub const EPISODE_EVENT_KIND_COUNT: usize = 5;

/// Closed-set size of [`EpisodeOutcome`]. Bump when a variant is added.
pub const EPISODE_OUTCOME_VARIANT_COUNT: usize = 4;

// ── Enums ────────────────────────────────────────────────────────────────

/// Closed-set taxonomy of episode-event kinds.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EpisodeEventKind {
    Event,
    Decision,
    Delegation,
    LabRun,
    Promotion,
}

impl EpisodeEventKind {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Event => "event",
            Self::Decision => "decision",
            Self::Delegation => "delegation",
            Self::LabRun => "lab_run",
            Self::Promotion => "promotion",
        }
    }
}

/// Closed-set taxonomy of episode outcomes.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EpisodeOutcome {
    Ok,
    Fail { reason: String },
    Skipped { reason: String },
    Pending,
}

impl EpisodeOutcome {
    #[must_use]
    pub fn kind_label(&self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Fail { .. } => "fail",
            Self::Skipped { .. } => "skipped",
            Self::Pending => "pending",
        }
    }
}

// ── Records ─────────────────────────────────────────────────────────────

/// One recorded event in an episode.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpisodeEvent {
    pub id: String,
    pub kind: EpisodeEventKind,
    pub actor: String,
    pub target: String,
    pub outcome: EpisodeOutcome,
    pub recorded_at: String,
}

/// Counts of outcomes for one event kind.
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EpisodeOutcomeCounts {
    pub ok: usize,
    pub fail: usize,
    pub skipped: usize,
    pub pending: usize,
    pub total: usize,
}

/// Aggregate of an episode.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperienceEpisode {
    pub episode_id: String,
    pub subject: String,
    pub events: Vec<EpisodeEvent>,
    pub outcomes_by_kind: BTreeMap<EpisodeEventKind, EpisodeOutcomeCounts>,
    pub started_at: String,
    pub closed_at: String,
    pub generated_at: String,
}

// ── Raw input + builder ─────────────────────────────────────────────────

/// Raw input for one event (plain-data, no builder).
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawEpisodeEvent {
    pub id: String,
    pub kind: EpisodeEventKind,
    pub actor: String,
    pub target: String,
    pub outcome: EpisodeOutcome,
    pub recorded_at: String,
}

impl From<RawEpisodeEvent> for EpisodeEvent {
    fn from(r: RawEpisodeEvent) -> Self {
        EpisodeEvent {
            id: r.id,
            kind: r.kind,
            actor: r.actor,
            target: r.target,
            outcome: r.outcome,
            recorded_at: r.recorded_at,
        }
    }
}

/// Builder trait: aggregate raw events into an `ExperienceEpisode`.
pub trait ExperienceEpisodeBuilder {
    fn build(
        &self,
        episode_id: &str,
        subject: &str,
        raw: &[RawEpisodeEvent],
        started_at: &str,
        closed_at: &str,
        generated_at: &str,
    ) -> ExperienceEpisode;
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DefaultExperienceEpisodeBuilder;

impl ExperienceEpisodeBuilder for DefaultExperienceEpisodeBuilder {
    fn build(
        &self,
        episode_id: &str,
        subject: &str,
        raw: &[RawEpisodeEvent],
        started_at: &str,
        closed_at: &str,
        generated_at: &str,
    ) -> ExperienceEpisode {
        // Convert + sort canonically.
        let mut events: Vec<EpisodeEvent> = raw.iter().cloned().map(Into::into).collect();
        events.sort_by(|a, b| {
            a.recorded_at
                .as_str()
                .cmp(b.recorded_at.as_str())
                .then(a.id.as_str().cmp(b.id.as_str()))
        });

        // Aggregate outcomes per kind.
        let mut outcomes_by_kind: BTreeMap<EpisodeEventKind, EpisodeOutcomeCounts> =
            BTreeMap::new();
        for ev in &events {
            let counts = outcomes_by_kind.entry(ev.kind).or_default();
            match &ev.outcome {
                EpisodeOutcome::Ok => counts.ok += 1,
                EpisodeOutcome::Fail { .. } => counts.fail += 1,
                EpisodeOutcome::Skipped { .. } => counts.skipped += 1,
                EpisodeOutcome::Pending => counts.pending += 1,
            }
            counts.total += 1;
        }

        ExperienceEpisode {
            episode_id: episode_id.to_string(),
            subject: subject.to_string(),
            events,
            outcomes_by_kind,
            started_at: started_at.to_string(),
            closed_at: closed_at.to_string(),
            generated_at: generated_at.to_string(),
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn builder() -> DefaultExperienceEpisodeBuilder {
        DefaultExperienceEpisodeBuilder
    }

    fn raw(
        id: &str,
        kind: EpisodeEventKind,
        recorded_at: &str,
        outcome: EpisodeOutcome,
    ) -> RawEpisodeEvent {
        RawEpisodeEvent {
            id: id.to_string(),
            kind,
            actor: "actor".to_string(),
            target: "target".to_string(),
            outcome,
            recorded_at: recorded_at.to_string(),
        }
    }

    // ── S-1: empty input ───────────────────────────────────────────────

    #[test]
    fn s1_empty_input_yields_empty_episode() {
        let ep = builder().build("e1", "subj", &[], "t0", "t1", "t2");
        assert_eq!(ep.episode_id, "e1");
        assert!(ep.events.is_empty());
        // All five kinds exist with zero counts.
        assert_eq!(ep.outcomes_by_kind.len(), 0);
    }

    // ── S-2: outcome counts per kind ──────────────────────────────────

    #[test]
    fn s2_outcome_counts_per_kind() {
        let raw = vec![
            raw("d1", EpisodeEventKind::Decision, "t0", EpisodeOutcome::Ok),
            raw("d2", EpisodeEventKind::Decision, "t1", EpisodeOutcome::Ok),
            raw(
                "d3",
                EpisodeEventKind::Decision,
                "t2",
                EpisodeOutcome::Fail {
                    reason: "bad".to_string(),
                },
            ),
            raw("x1", EpisodeEventKind::Delegation, "t3", EpisodeOutcome::Ok),
        ];
        let ep = builder().build("e2", "subj", &raw, "t0", "t3", "t4");
        let decision = ep
            .outcomes_by_kind
            .get(&EpisodeEventKind::Decision)
            .expect("decision counts");
        assert_eq!(decision.ok, 2);
        assert_eq!(decision.fail, 1);
        assert_eq!(decision.total, 3);
        let delegation = ep
            .outcomes_by_kind
            .get(&EpisodeEventKind::Delegation)
            .expect("delegation counts");
        assert_eq!(delegation.ok, 1);
        assert_eq!(delegation.total, 1);
    }

    // ── S-3: events sorted by (recorded_at, id) ───────────────────────

    #[test]
    fn s3_events_sorted_by_recorded_at_and_id() {
        let raw = vec![
            raw("b", EpisodeEventKind::Event, "t-B", EpisodeOutcome::Ok),
            raw("a", EpisodeEventKind::Event, "t-A", EpisodeOutcome::Ok),
            raw("c", EpisodeEventKind::Event, "t-A", EpisodeOutcome::Ok),
        ];
        let ep = builder().build("e3", "subj", &raw, "t0", "t1", "t2");
        assert_eq!(ep.events.len(), 3);
        assert_eq!(ep.events[0].id, "a");
        assert_eq!(ep.events[1].id, "c");
        assert_eq!(ep.events[2].id, "b");
    }

    // ── S-4: closed-set audit guards ──────────────────────────────────

    #[test]
    fn s4_closed_set_audit_guards() {
        assert_eq!(EPISODE_EVENT_KIND_COUNT, 5);
        let all_kinds = [
            EpisodeEventKind::Event,
            EpisodeEventKind::Decision,
            EpisodeEventKind::Delegation,
            EpisodeEventKind::LabRun,
            EpisodeEventKind::Promotion,
        ];
        assert_eq!(all_kinds.len(), EPISODE_EVENT_KIND_COUNT);

        assert_eq!(EPISODE_OUTCOME_VARIANT_COUNT, 4);
        // Smoke-check each outcome kind has a label.
        let outcomes = [
            EpisodeOutcome::Ok,
            EpisodeOutcome::Fail {
                reason: "x".to_string(),
            },
            EpisodeOutcome::Skipped {
                reason: "x".to_string(),
            },
            EpisodeOutcome::Pending,
        ];
        let labels: Vec<&str> = outcomes.iter().map(|o| o.kind_label()).collect();
        assert_eq!(labels.len(), EPISODE_OUTCOME_VARIANT_COUNT);
    }

    // ── S-5: determinism ──────────────────────────────────────────────

    #[test]
    fn s5_deterministic_output() {
        let raw = vec![
            raw("a", EpisodeEventKind::Event, "t0", EpisodeOutcome::Ok),
            raw("b", EpisodeEventKind::Event, "t1", EpisodeOutcome::Pending),
        ];
        let ep1 = builder().build("e5", "subj", &raw, "t0", "t1", "t-A");
        let ep2 = builder().build("e5", "subj", &raw, "t0", "t1", "t-B");
        assert_eq!(ep1.events, ep2.events);
        assert_eq!(ep1.outcomes_by_kind, ep2.outcomes_by_kind);
        assert_ne!(ep1.generated_at, ep2.generated_at);
    }

    // ── S-6: BTreeMap iteration order ─────────────────────────────────

    #[test]
    fn s6_outcomes_iteration_order_canonical() {
        let raw = vec![
            raw("x", EpisodeEventKind::Promotion, "t0", EpisodeOutcome::Ok),
            raw("y", EpisodeEventKind::LabRun, "t0", EpisodeOutcome::Ok),
            raw("z", EpisodeEventKind::Event, "t0", EpisodeOutcome::Ok),
        ];
        let ep = builder().build("e6", "subj", &raw, "t0", "t1", "t2");
        let kinds: Vec<EpisodeEventKind> = ep.outcomes_by_kind.keys().copied().collect();
        // Enum variant declaration order:
        // Event(0), Decision(1), Delegation(2), LabRun(3), Promotion(4)
        // But only 3 present ⇒ Event, LabRun, Promotion.
        assert_eq!(
            kinds,
            vec![
                EpisodeEventKind::Event,
                EpisodeEventKind::LabRun,
                EpisodeEventKind::Promotion,
            ]
        );
    }

    // ── Bonus: From<RawEpisodeEvent> ─────────────────────────────────

    #[test]
    fn s7_raw_to_event_conversion() {
        let r = raw("a", EpisodeEventKind::Decision, "t0", EpisodeOutcome::Ok);
        let e: EpisodeEvent = r.clone().into();
        assert_eq!(e.id, r.id);
        assert_eq!(e.kind, r.kind);
        assert_eq!(e.outcome, r.outcome);
    }
}
