//! Cycle Narrative — operator-facing cycle summary + argumented
//! next-step suggestions. Plain-data, deterministic, bounded-length.
//!
//! Pattern: P-a (struct + trait + plain-data evaluator).

use std::collections::hash_map::DefaultHasher;
use std::fmt::Write as _;
use std::hash::Hasher;

use sddk_domain::CycleStatus;

// ── Audit guard constants ────────────────────────────────────────────────

/// Closed-set size of [`NarrativeAudience`]. Bump when a variant is added.
pub const NARRATIVE_AUDIENCE_VARIANT_COUNT: usize = 3;
/// Closed-set size of [`NarrativeTone`]. Bump when a variant is added.
pub const NARRATIVE_TONE_VARIANT_COUNT: usize = 3;
/// Closed-set size of [`SuggestionPriority`]. Bump when a variant is added.
pub const SUGGESTION_PRIORITY_VARIANT_COUNT: usize = 3;
/// Closed-set size of [`EffortEstimate`]. Bump when a variant is added.
pub const EFFORT_ESTIMATE_VARIANT_COUNT: usize = 3;
/// Closed-set size of [`AnchorKind`]. Bump when a variant is added.
pub const ANCHOR_KIND_VARIANT_COUNT: usize = 5;

/// Default bound, RNF-HX-004 (≤150 words by default).
pub const DEFAULT_MAX_WORDS: usize = 150;

// ── Enums ────────────────────────────────────────────────────────────────

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NarrativeAudience {
    /// Author of the cycle (technical, no preamble).
    SelfAuthor,
    /// Active maintainer of the framework.
    Maintainer,
    /// External stakeholder; reads to understand "what happened".
    Stakeholder,
}

impl NarrativeAudience {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            NarrativeAudience::SelfAuthor => "self",
            Self::Maintainer => "maintainer",
            Self::Stakeholder => "stakeholder",
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NarrativeTone {
    /// Plain, terse.
    Concise,
    /// Explain WHY before WHAT.
    Explanatory,
    /// End with a probe.
    Socratic,
}

impl NarrativeTone {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Concise => "concise",
            Self::Explanatory => "explanatory",
            Self::Socratic => "socratic",
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum SuggestionPriority {
    /// Blocks the next phase or is critical debt.
    High,
    /// Important, optional in next few cycles.
    #[default]
    Medium,
    /// Nice-to-have.
    Low,
}

impl SuggestionPriority {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::High => "HIGH",
            Self::Medium => "MEDIUM",
            Self::Low => "LOW",
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum EffortEstimate {
    /// < 1 cycle.
    #[default]
    Small,
    /// 1-3 cycles.
    Medium,
    /// > 3 cycles.
    Large,
}

impl EffortEstimate {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Small => "small",
            Self::Medium => "medium",
            Self::Large => "large",
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnchorKind {
    /// Evidence card from a phase.
    EvidenceCard,
    /// Decision record in the Decision Plane.
    DecisionRecord,
    /// Open debt ticket from `docs/debt/`.
    DebtInc,
    /// Item from the canonical execution spine.
    SpineItem,
    /// External reference (file:line, URL, ADR-... , SPEC-...).
    ExternalRef,
}

impl AnchorKind {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::EvidenceCard => "evidence",
            Self::DecisionRecord => "decision",
            Self::DebtInc => "debt",
            Self::SpineItem => "spine",
            Self::ExternalRef => "ref",
        }
    }
}

// ── Records ──────────────────────────────────────────────────────────────

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReasoningAnchor {
    pub kind: AnchorKind,
    pub ref_id: String,
    pub note: String,
}

impl ReasoningAnchor {
    pub fn new(kind: AnchorKind, ref_id: impl Into<String>, note: impl Into<String>) -> Self {
        Self {
            kind,
            ref_id: ref_id.into(),
            note: note.into(),
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextStepSuggestion {
    pub title: String,
    /// 1-2 sentences describing what would happen if taken.
    pub description: String,
    pub reasoning_anchors: Vec<ReasoningAnchor>,
    pub priority: SuggestionPriority,
    pub effort: EffortEstimate,
    pub blockers: Vec<String>,
    /// Optional pointer to a `EXECUTION-SPINE.yaml` item id.
    pub canonical_ref: Option<String>,
    /// Witness references (file:line, test paths, etc).
    pub witness_refs: Vec<String>,
}

impl NextStepSuggestion {
    pub fn builder(title: impl Into<String>) -> NextStepSuggestionBuilder {
        NextStepSuggestionBuilder {
            title: title.into(),
            rest: NextStepSuggestionRest::default(),
        }
    }
}

#[derive(Debug, Default)]
struct NextStepSuggestionRest {
    description: String,
    reasoning_anchors: Vec<ReasoningAnchor>,
    priority: SuggestionPriority,
    effort: EffortEstimate,
    blockers: Vec<String>,
    canonical_ref: Option<String>,
    witness_refs: Vec<String>,
}

#[derive(Debug)]
pub struct NextStepSuggestionBuilder {
    title: String,
    rest: NextStepSuggestionRest,
}

impl NextStepSuggestionBuilder {
    #[must_use]
    pub fn description(mut self, s: impl Into<String>) -> Self {
        self.rest.description = s.into();
        self
    }
    #[must_use]
    pub fn priority(mut self, p: SuggestionPriority) -> Self {
        self.rest.priority = p;
        self
    }
    #[must_use]
    pub fn effort(mut self, e: EffortEstimate) -> Self {
        self.rest.effort = e;
        self
    }
    #[must_use]
    pub fn anchor(
        mut self,
        kind: AnchorKind,
        ref_id: impl Into<String>,
        note: impl Into<String>,
    ) -> Self {
        self.rest
            .reasoning_anchors
            .push(ReasoningAnchor::new(kind, ref_id, note));
        self
    }
    #[must_use]
    pub fn blocker(mut self, b: impl Into<String>) -> Self {
        self.rest.blockers.push(b.into());
        self
    }
    #[must_use]
    pub fn canonical_ref(mut self, r: impl Into<String>) -> Self {
        self.rest.canonical_ref = Some(r.into());
        self
    }
    #[must_use]
    pub fn witness(mut self, w: impl Into<String>) -> Self {
        self.rest.witness_refs.push(w.into());
        self
    }
    #[must_use]
    pub fn build(self) -> NextStepSuggestion {
        NextStepSuggestion {
            title: self.title,
            description: self.rest.description,
            reasoning_anchors: self.rest.reasoning_anchors,
            priority: self.rest.priority,
            effort: self.rest.effort,
            blockers: self.rest.blockers,
            canonical_ref: self.rest.canonical_ref,
            witness_refs: self.rest.witness_refs,
        }
    }
}

// ── Operator-view claims: derived, never asserted ───────────────────────
//
// `human_action_required` existed with NO PRODUCER: it was declared, set to
// `None` in the constructor, read once by the footer, and never received a
// `Some(..)` anywhere in the workspace. The footer therefore could not print
// anything but "Nada por ahora.", by construction — and a narrative that
// cannot say "an action is required" says "no action is required" to every
// operator, including the ones who are about to leave a cycle waiting
// forever on a decision only they can make.
//
// Worse than an absent derivation: `sddk cycle narrative` never opened the
// store. It rendered "Cycle completed." with exit 0 for a cycle id that does
// not exist, because the command is a template, not a view.
//
// So the two claims the operator view makes are now DERIVED from facts the
// caller reads out of the ledger, and the derivation lives here — in the
// engine — rather than in the command that needs it, for the same reason
// `remote_urls_equivalent` lives in the domain (session-76): two callers with
// private copies diverge the moment one is touched and the other is not.

/// Lease liveness, as the operator view needs to judge it.
///
/// `Expired` is separated from `Absent` because they are different facts with
/// different recoveries: an expired lease needs renewing, an absent one needs
/// taking. Collapsing them would tell the operator the wrong remedy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NarrativeLease {
    /// A lease whose expiry is in the future at the reference instant.
    Live {
        /// Lease owner recorded in the ledger.
        owner: String,
        /// Expiry, epoch milliseconds.
        expires_at_ms: i64,
    },
    /// A lease exists but had already expired at the reference instant.
    Expired {
        /// Lease owner recorded in the ledger.
        owner: String,
        /// Expiry, epoch milliseconds.
        expired_at_ms: i64,
    },
    /// No lease row for this cycle.
    Absent,
}

/// The facts the operator view is allowed to assert.
///
/// Every field is read from the ledger by the caller. The narrative derives
/// its two central claims from these and from nothing else — that is the whole
/// point: a claim that is not derivable from a fact is a guess, and a guess
/// rendered as a certainty is the defect this type exists to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NarrativeFacts {
    /// Persisted cycle status, from the cycle record.
    pub status: CycleStatus,
    /// Human-readable phase from the cycle record.
    pub phase: String,
    /// Derived runtime state (e.g. `approval-waiting`); empty when none apply.
    pub runtime_state: String,
    /// Capabilities with an unresolved approval request.
    pub approval_waiting_on: Vec<String>,
    /// Lease liveness at [`Self::now_ms`].
    pub lease: NarrativeLease,
    /// Reference instant for lease liveness, epoch milliseconds.
    ///
    /// Supplied by the caller, never read from the clock here: the narrative
    /// is a deterministic artifact and two renders of the same facts must be
    /// byte-identical.
    pub now_ms: i64,
}

/// The two claims the operator view makes, derived.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NarrativeClaims {
    /// One sentence describing what the cycle actually is.
    pub what_was_done: String,
    /// What the operator must do, when something is pending. `None` only when
    /// the facts show nothing is pending.
    pub human_action_required: Option<String>,
}

/// Derives the operator view's two central claims from ledger facts.
///
/// The precedence below is fixed and total: for the same facts there is
/// exactly one answer, and the first matching rule wins. An operator view
/// whose answer depends on evaluation order is a view whose answer depends on
/// nothing.
#[must_use]
pub fn derive_claims(facts: &NarrativeFacts) -> NarrativeClaims {
    // ── Claim 1: what the cycle IS ──────────────────────────────────────
    // A live cycle is never described as completed. Before this, every
    // status rendered the same sentence, so a CLOSED cycle and an OPEN one
    // were indistinguishable in the one surface a human reads.
    let what_was_done = match facts.status {
        CycleStatus::Closed => format!("The cycle closed at phase \"{}\".", facts.phase),
        CycleStatus::Abandoned => format!("The cycle was abandoned at phase \"{}\".", facts.phase),
        CycleStatus::Released => format!("The cycle was released at phase \"{}\".", facts.phase),
        other => format!(
            "The cycle is {} at phase \"{}\"{}.",
            wire_status(other),
            facts.phase,
            if facts.runtime_state.is_empty() {
                String::new()
            } else {
                format!(" (runtime: {})", facts.runtime_state)
            }
        ),
    };

    // ── Claim 2: what the operator must do ──────────────────────────────
    // First match wins, in the same precedence the runtime summary already
    // uses (approval > uat > remediation), so the footer and the status view
    // cannot disagree about what is blocking.
    let mut pending: Vec<String> = facts.approval_waiting_on.clone();
    pending.sort();
    pending.dedup();

    let human_action_required = if !pending.is_empty() {
        let n = pending.len();
        let plural = if n == 1 { "request" } else { "requests" };
        Some(format!(
            "Decide {n} pending approval {plural}: {}.",
            pending.join(", ")
        ))
    } else if facts.runtime_state.contains("uat-waiting") {
        Some("Give the pending UAT verdict for this cycle.".to_string())
    } else if matches!(facts.status, CycleStatus::Open)
        && !lease_is_live(&facts.lease, facts.now_ms)
    {
        // Total by construction: every arm of the lease is handled, and
        // none of them panics. A lease LABELLED live whose expiry is
        // already past at the reference instant is a reachable state — the
        // label is a claim and the expiry is the fact, and this function
        // judges by the fact. An `unreachable!` here was reached by a test
        // that passed a stale label, which is exactly the kind of input a
        // public function has to survive.
        Some(match &facts.lease {
            NarrativeLease::Absent => {
                "This cycle holds no lease; take one before applying.".to_string()
            }
            NarrativeLease::Expired { owner, .. } => {
                format!("The lease held by {owner} expired; take a fresh one before applying.")
            }
            NarrativeLease::Live { owner, .. } => {
                format!(
                    "The lease held by {owner} is no longer live; take a fresh one before applying."
                )
            }
        })
    } else {
        None
    };

    NarrativeClaims {
        what_was_done,
        human_action_required,
    }
}

/// Whether the lease is live at `now_ms`.
fn lease_is_live(lease: &NarrativeLease, now_ms: i64) -> bool {
    match lease {
        NarrativeLease::Live { expires_at_ms, .. } => *expires_at_ms > now_ms,
        NarrativeLease::Expired { .. } => false,
        NarrativeLease::Absent => false,
    }
}

/// Lowercase wire form of a [`CycleStatus`], matching `sddk cycle status`.
fn wire_status(status: CycleStatus) -> String {
    format!("{status:?}").to_lowercase()
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CycleNarrative {
    pub cycle_id: String,
    pub horizon: String,
    /// A-min / A-lite / A-full / B-direct.
    pub path: String,
    pub audience: NarrativeAudience,
    pub tone: NarrativeTone,
    /// Short title, e.g. "Apply phase 4/7".
    pub title: String,
    /// 1-2 sentences: what was done.
    pub what_was_done: String,
    /// 1 sentence: why it matters.
    pub why_it_matters: String,
    pub what_changed: Vec<String>,
    pub what_to_validate: Vec<String>,
    pub risk_caveats: Vec<String>,
    pub next_step_suggestions: Vec<NextStepSuggestion>,
    /// Optional: "todo", "approve", etc. None when no action needed.
    pub human_action_required: Option<String>,
    /// Caller-supplied timestamp (no `SystemTime::now`).
    pub generated_at: String,
    /// Deterministic fingerprint of the inputs.
    pub cycle_digest: u64,
}

impl CycleNarrative {
    /// Build a narrative from identity-bearing fields. Default-deny on
    /// future expansion: callers SHOULD use this constructor rather than
    /// a struct literal because the struct is `#[non_exhaustive]`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        cycle_id: impl Into<String>,
        horizon: impl Into<String>,
        path: impl Into<String>,
        audience: NarrativeAudience,
        tone: NarrativeTone,
        title: impl Into<String>,
        what_was_done: impl Into<String>,
        generated_at: impl Into<String>,
    ) -> Self {
        Self {
            cycle_id: cycle_id.into(),
            horizon: horizon.into(),
            path: path.into(),
            audience,
            tone,
            title: title.into(),
            what_was_done: what_was_done.into(),
            why_it_matters: String::new(),
            what_changed: Vec::new(),
            what_to_validate: Vec::new(),
            risk_caveats: Vec::new(),
            next_step_suggestions: Vec::new(),
            human_action_required: None,
            generated_at: generated_at.into(),
            cycle_digest: 0,
        }
    }

    /// Compute the canonical digest over identity fields.
    #[must_use]
    pub fn compute_digest(
        cycle_id: &str,
        title: &str,
        what_was_done: &str,
        next: &[NextStepSuggestion],
    ) -> u64 {
        let mut h = DefaultHasher::new();
        h.write(cycle_id.as_bytes());
        h.write(title.as_bytes());
        h.write(what_was_done.as_bytes());
        for s in next {
            h.write(s.title.as_bytes());
            h.write(s.description.as_bytes());
            h.write(s.priority.label().as_bytes());
            h.write(s.effort.label().as_bytes());
        }
        h.finish()
    }

    /// Recompute the digest from the current identity fields.
    ///
    /// The digest covers `what_was_done`, so anything that mutates it must
    /// come through here. A digest computed before a mutation fingerprints a
    /// document that is not the one the operator is handed, and a digest
    /// that fingerprints the wrong document is worse than no digest.
    pub fn recompute_digest(&mut self) {
        self.cycle_digest = Self::compute_digest(
            &self.cycle_id,
            &self.title,
            &self.what_was_done,
            &self.next_step_suggestions,
        );
    }

    /// Apply derived operator-view claims to this narrative.
    ///
    /// This is the producer `human_action_required` never had.
    pub fn apply_claims(&mut self, claims: &NarrativeClaims) {
        self.what_was_done = claims.what_was_done.clone();
        self.human_action_required = claims.human_action_required.clone();
        self.recompute_digest();
    }

    /// Replace the derived sentence with a caller-supplied one.
    ///
    /// The override wins over a DERIVED default, never over a constant —
    /// that ordering is the whole difference between an escape hatch and the
    /// reason the field had no producer.
    pub fn override_what_was_done(&mut self, text: &str) {
        self.what_was_done = text.to_string();
        self.recompute_digest();
    }

    pub fn sort_suggestions(&mut self) {
        self.next_step_suggestions.sort_by(|a, b| {
            (a.priority, a.effort, a.title.as_str()).cmp(&(b.priority, b.effort, b.title.as_str()))
        });
        self.cycle_digest = Self::compute_digest(
            &self.cycle_id,
            &self.title,
            &self.what_was_done,
            &self.next_step_suggestions,
        );
    }
}

// ── Errors ───────────────────────────────────────────────────────────────

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NarrativeError {
    EmptyCycleId,
    EmptyTitle,
}

impl std::fmt::Display for NarrativeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::EmptyCycleId => "cycle_id is required",
            Self::EmptyTitle => "title is required",
        };
        f.write_str(s)
    }
}

impl std::error::Error for NarrativeError {}

// ── Writer trait ─────────────────────────────────────────────────────────

pub trait CycleNarrativeWriter {
    /// Render `n` as Markdown. Implementations should respect
    /// `max_words` and surface truncation as part of the returned
    /// string or `Err`.
    fn write(&self, n: &CycleNarrative, max_words: usize) -> Result<String, NarrativeError>;
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DefaultCycleNarrativeWriter;

impl DefaultCycleNarrativeWriter {
    /// Construct a default writer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl CycleNarrativeWriter for DefaultCycleNarrativeWriter {
    fn write(&self, n: &CycleNarrative, max_words: usize) -> Result<String, NarrativeError> {
        if n.cycle_id.trim().is_empty() {
            return Err(NarrativeError::EmptyCycleId);
        }
        if n.title.trim().is_empty() {
            return Err(NarrativeError::EmptyTitle);
        }

        let mut out = String::new();
        let _ = writeln!(
            out,
            "`{} · {} · {} · audience={} tone={}`\n",
            n.cycle_id,
            n.horizon,
            n.path,
            n.audience.label(),
            n.tone.label()
        );

        let _ = writeln!(out, "**{}**\n", n.title);
        let _ = writeln!(out, "{}", n.what_was_done.trim());
        if !n.why_it_matters.trim().is_empty() {
            let _ = writeln!(out, "\n_Why_: {}", n.why_it_matters.trim());
        }

        if !n.what_changed.is_empty() {
            let _ = writeln!(out, "\n**What changed:**");
            for change in &n.what_changed {
                let _ = writeln!(out, "- {change}");
            }
        }

        if !n.what_to_validate.is_empty() {
            let _ = writeln!(out, "\n**Validate:**");
            for v in &n.what_to_validate {
                let _ = writeln!(out, "- {v}");
            }
        }

        if !n.risk_caveats.is_empty() {
            let _ = writeln!(out, "\n**Caveats:**");
            for r in &n.risk_caveats {
                let _ = writeln!(out, "- {r}");
            }
        }

        // Sorted suggestions.
        let mut sorted: Vec<&NextStepSuggestion> = n.next_step_suggestions.iter().collect();
        sorted.sort_by(|a, b| {
            (a.priority, a.effort, a.title.as_str()).cmp(&(b.priority, b.effort, b.title.as_str()))
        });

        if !sorted.is_empty() {
            let _ = writeln!(out, "\n────────────────── Next steps ──────────────────");
            for s in sorted {
                let _ = writeln!(
                    out,
                    "\n▶ [{} · {}] {}\n  {}\n  Anclajes:",
                    s.priority.label(),
                    s.effort.label(),
                    s.title,
                    s.description
                );
                if s.reasoning_anchors.is_empty() {
                    let _ = writeln!(out, "  - (none provided)");
                } else {
                    for a in &s.reasoning_anchors {
                        let _ = writeln!(
                            out,
                            "  - [{kind}] {ref_id} — {note}",
                            kind = a.kind.label(),
                            ref_id = a.ref_id,
                            note = a.note
                        );
                    }
                }
                if let Some(cr) = &s.canonical_ref {
                    let _ = writeln!(out, "  Ref: {cr}");
                }
                for w in &s.witness_refs {
                    let _ = writeln!(out, "  Witness: {w}");
                }
                for b in &s.blockers {
                    let _ = writeln!(out, "  Blocker: {b}");
                }
            }
        }

        // Human action footer.
        if let Some(action) = &n.human_action_required {
            let _ = writeln!(
                out,
                "\n────────────────── Necesito de ti ──────────────────\n{action}"
            );
        } else {
            let _ = writeln!(
                out,
                "\n────────────────── Necesito de ti ──────────────────\nNada por ahora."
            );
        }

        let _ = writeln!(out, "\n_generated: {}_", n.generated_at);

        // Word-bound enforcement.
        let words: Vec<&str> = out.split_whitespace().collect();
        if words.len() > max_words {
            let truncated = words
                .iter()
                .take(max_words)
                .copied()
                .collect::<Vec<_>>()
                .join(" ");
            let mut out_trunc = truncated;
            let _ = writeln!(
                out_trunc,
                "\n\n_(truncado a {max_words} words; output had {words_len})_",
                words_len = words.len()
            );
            return Ok(out_trunc);
        }

        Ok(out)
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Facts for a cycle that is open, at phase `design`, with a live
    /// lease and nothing pending — the only combination in which the
    /// operator view is allowed to say "nothing to do".
    fn quiet_open() -> NarrativeFacts {
        NarrativeFacts {
            status: CycleStatus::Open,
            phase: "design".into(),
            runtime_state: String::new(),
            approval_waiting_on: Vec::new(),
            lease: NarrativeLease::Live {
                owner: "mvs-test".into(),
                expires_at_ms: 2_000,
            },
            now_ms: 1_000,
        }
    }

    /// A live cycle is never described as completed. Before the producer
    /// existed, every status rendered the same sentence, so a CLOSED cycle
    /// and an OPEN one were byte-identical in the one surface a human reads.
    ///
    /// The sweep is driven by the PERSISTED statuses plus every label
    /// `derive_cycle_summary` can derive, rather than by naming the
    /// runtime-derived variants. That is a constraint with a reason: since the
    /// WU-C3 cutover those variants are decode-only — wait/remediation/recovery
    /// truth lives in Run/Authority facts and the label is DERIVED — so a
    /// `conf09b` ratchet forbids referencing them from here. Enumerating them
    /// in a test would have been asserting against a status the domain no
    /// longer treats as canonical Cycle truth.
    ///
    /// Sweeping the derived labels instead is the stronger claim: the
    /// narrative stays honest for every combination the authority can
    /// actually produce, including ones a hand-written list would miss.
    #[test]
    fn a_live_cycle_is_never_described_as_completed() {
        // Every label `derive_cycle_summary` derives, in its own precedence
        // order, plus a representative not-derived one.
        let runtime_labels = [
            "",                 // nothing pending: only the persisted status counts
            "approval-waiting", // highest precedence
            "uat-waiting",
            "remediating",
            "recovering",
        ];
        let persisted = [
            CycleStatus::Open,
            CycleStatus::Blocked,
            CycleStatus::ReleasePending,
        ];

        for status in persisted {
            for label in runtime_labels {
                let facts = NarrativeFacts {
                    status,
                    runtime_state: label.to_string(),
                    ..quiet_open()
                };
                let claims = derive_claims(&facts);
                let what = &claims.what_was_done;
                assert!(
                    !what.contains("completed"),
                    "{status:?}/{label:?} rendered as completed: {what}"
                );
                assert!(
                    !what.contains("closed"),
                    "{status:?}/{label:?} rendered as closed: {what}"
                );
            }
        }
    }

    /// Two cycles whose truth differs must not render the same sentence.
    /// This is the property the debt record measured as violated across
    /// three cycles: CLOSED, OPEN, and OPEN+approval-waiting all rendered
    /// identically.
    #[test]
    fn cycles_in_different_states_derive_different_sentences() {
        let closed = derive_claims(&NarrativeFacts {
            status: CycleStatus::Closed,
            ..quiet_open()
        });
        let open = derive_claims(&quiet_open());
        let waiting = derive_claims(&NarrativeFacts {
            status: CycleStatus::Open,
            runtime_state: "approval-waiting".into(),
            approval_waiting_on: vec!["surface.cycle_state#cycle_supersede".into()],
            ..quiet_open()
        });
        assert_ne!(closed.what_was_done, open.what_was_done);
        assert_ne!(open.what_was_done, waiting.what_was_done);
        assert_ne!(closed.what_was_done, waiting.what_was_done);
        assert!(closed.what_was_done.contains("closed"));
        assert!(open.what_was_done.contains("is open"));
        assert!(waiting.what_was_done.contains("approval-waiting"));
    }

    /// A cycle waiting on a human decision must NAME the decision, and must
    /// not say nothing is needed. Before, `human_action_required` had no
    /// producer at all, so the footer could not print anything else.
    #[test]
    fn a_pending_approval_names_what_has_to_be_decided() {
        let claims = derive_claims(&NarrativeFacts {
            approval_waiting_on: vec!["surface.cycle_state#cycle_supersede".into()],
            runtime_state: "approval-waiting".into(),
            ..quiet_open()
        });
        let action = claims
            .human_action_required
            .expect("a pending approval must produce a human action");
        assert!(
            action.contains("surface.cycle_state#cycle_supersede"),
            "{action}"
        );
    }

    /// Precedence is total and deterministic: approval beats the lease,
    /// because the operator's decision is the blocking one. Two
    /// derivations of the same facts must be equal, always.
    #[test]
    fn derivation_is_deterministic_and_total() {
        let facts = NarrativeFacts {
            approval_waiting_on: vec!["b".into(), "a".into(), "b".into()],
            runtime_state: "approval-waiting".into(),
            lease: NarrativeLease::Absent,
            ..quiet_open()
        };
        let first = derive_claims(&facts);
        let second = derive_claims(&facts);
        assert_eq!(first, second, "the same facts must derive the same claims");
        // Duplicates collapse and the list is sorted, so the operator sees
        // one item per pending decision in a stable order.
        let action = first.human_action_required.unwrap();
        assert!(action.contains("2 pending approval requests"), "{action}");
        assert!(action.contains("a, b"), "{action}");
    }

    /// An OPEN cycle without a live lease needs one before anything is
    /// applied. Expired and absent are different facts with different
    /// remedies, so they must not collapse into one sentence.
    #[test]
    fn an_open_cycle_without_a_live_lease_says_so() {
        let expired = derive_claims(&NarrativeFacts {
            lease: NarrativeLease::Expired {
                owner: "mvs-old".into(),
                expired_at_ms: 500,
            },
            ..quiet_open()
        });
        let absent = derive_claims(&NarrativeFacts {
            lease: NarrativeLease::Absent,
            ..quiet_open()
        });
        let e = expired
            .human_action_required
            .expect("an expired lease must produce an action");
        let a = absent
            .human_action_required
            .expect("an absent lease must produce an action");
        assert!(
            e.contains("mvs-old"),
            "the expired owner must be named: {e}"
        );
        assert!(
            a.contains("no lease"),
            "the absent lease reads differently: {a}"
        );
        assert_ne!(e, a);
    }

    /// A lease is live only while its expiry is in the future. The boundary
    /// is exclusive: at exactly the expiry instant it is not live.
    ///
    /// The last two cases are the ones that matter for totality: a caller
    /// that LABELS a lease `Live` while its expiry is already past must get
    /// a sentence, not a panic. That case was reached by this test and hit
    /// an `unreachable!` that the guard `lease_is_live` did not actually
    /// exclude — the guard re-judges liveness, so a stale label contradicts
    /// it rather than being excluded by it.
    #[test]
    fn lease_liveness_boundary_is_exclusive() {
        let at = |expires: i64, now: i64| {
            derive_claims(&NarrativeFacts {
                lease: NarrativeLease::Live {
                    owner: "o".into(),
                    expires_at_ms: expires,
                },
                now_ms: now,
                ..quiet_open()
            })
            .human_action_required
        };
        assert!(
            at(2_000, 1_999).is_none(),
            "before expiry the lease is live"
        );
        let at_boundary = at(2_000, 2_000)
            .expect("at the expiry instant the lease is not live and must still answer");
        let after = at(2_000, 2_001).expect("after expiry the lease is not live");
        assert_eq!(at_boundary, after, "both must name the same remedy");
    }

    /// The claim the operator actually reads must not depend on the runtime
    /// summary being available. A summary that failed to derive leaves an
    /// empty runtime state, and the sentence must still be true.
    #[test]
    fn an_absent_runtime_summary_still_yields_a_true_sentence() {
        let mut facts = quiet_open();
        facts.runtime_state = String::new();
        let claims = derive_claims(&facts);
        assert!(claims.what_was_done.contains("is open"));
        assert!(
            !claims.what_was_done.contains("()"),
            "an empty runtime state must not render empty parentheses: {}",
            claims.what_was_done
        );
    }

    /// Applying the claims is what gives `human_action_required` its
    /// producer, and it must recompute the digest: a digest computed
    /// before the claims were applied would fingerprint a document that
    /// is not the one the operator is handed.
    #[test]
    fn applying_claims_moves_the_field_and_the_digest() {
        let mut n = CycleNarrative::new(
            "C-1",
            "H13 (Narration)",
            "operator",
            NarrativeAudience::Maintainer,
            NarrativeTone::Explanatory,
            "Cycle C-1",
            "The cycle is open at phase \"design\".",
            "2026-10-04T00:00:00Z",
        );
        assert!(
            n.human_action_required.is_none(),
            "the constructor still leaves it unset; the producer is apply_claims"
        );
        let before = n.cycle_digest;

        let mut facts = quiet_open();
        facts.approval_waiting_on = vec!["surface.cycle_state#cycle_supersede".into()];
        n.apply_claims(&derive_claims(&facts));

        assert_eq!(
            n.human_action_required.as_deref(),
            Some("Decide 1 pending approval request: surface.cycle_state#cycle_supersede.")
        );
        assert_ne!(
            n.cycle_digest, before,
            "the digest must cover the claims that were applied"
        );
    }

    fn narrative() -> CycleNarrative {
        CycleNarrative {
            cycle_id: "C-018".into(),
            horizon: "H12".into(),
            path: "A-FULL".into(),
            audience: NarrativeAudience::Stakeholder,
            tone: NarrativeTone::Explanatory,
            title: "Apply phase 4/7".into(),
            what_was_done: "Bloqueo optimista añadido tras descubrir una carrera concurrente."
                .into(),
            why_it_matters: "Sin coordinación, perdíamos writes.".into(),
            what_changed: vec!["repo.rs: 3 sites now use compare-and-swap".into()],
            what_to_validate: vec!["cargo test -p sddk-engine".into()],
            risk_caveats: vec!["thrashing risk on heavy writes (see ADR-0079)".into()],
            next_step_suggestions: vec![
                NextStepSuggestion::builder("Verificar bajo carga")
                    .description(
                        "Locking bajo concurrencia no se ha medido fuera de los tests unitarios.",
                    )
                    .priority(SuggestionPriority::High)
                    .effort(EffortEstimate::Medium)
                    .anchor(
                        AnchorKind::ExternalRef,
                        "EXECUTION-SPINE.yaml",
                        "H6-DW-OPERATORS-002 listed in roadmap",
                    )
                    .anchor(AnchorKind::DebtInc, "INC-OK", "Performance budget open")
                    .canonical_ref("EXECUTION-SPINE:H6-DW-OPERATORS-002")
                    .witness("crates/sddk-engine/src/run_view.rs:160")
                    .build(),
            ],
            human_action_required: None,
            generated_at: "t-A".into(),
            cycle_digest: 0,
        }
    }

    fn writer() -> DefaultCycleNarrativeWriter {
        DefaultCycleNarrativeWriter
    }

    #[test]
    fn s1_minimal_narrative_renders() {
        let mut n = narrative();
        n.sort_suggestions();
        assert!(
            n.cycle_digest != 0,
            "sort_suggestions must compute cycle_digest"
        );
        let out = writer().write(&n, 200).expect("write");
        assert!(out.contains("C-018"));
        assert!(out.contains("Apply phase 4/7"));
        assert!(out.contains("Verificar bajo carga"));
    }

    #[test]
    fn s2_three_suggestions_priority_order() {
        let mut n = narrative();
        n.next_step_suggestions.push(
            NextStepSuggestion::builder("Lower priority win")
                .description("nice to have")
                .priority(SuggestionPriority::Low)
                .effort(EffortEstimate::Small)
                .build(),
        );
        n.next_step_suggestions.push(
            NextStepSuggestion::builder("Medium win")
                .description("mid term")
                .priority(SuggestionPriority::Medium)
                .effort(EffortEstimate::Small)
                .build(),
        );
        n.sort_suggestions();
        let out = writer().write(&n, 500).expect("write");
        let pos_high = out.find("Verificar bajo carga").unwrap_or(usize::MAX);
        let pos_med = out.find("Medium win").unwrap_or(usize::MAX);
        let pos_low = out.find("Lower priority win").unwrap_or(usize::MAX);
        assert!(
            pos_high < pos_med && pos_med < pos_low,
            "expected HIGH < MEDIUM < LOW by render order"
        );
    }

    #[test]
    fn s3_deterministic_byte_equal() {
        let out1 = writer().write(&narrative(), 500).expect("write");
        let out2 = writer().write(&narrative(), 500).expect("write");
        assert_eq!(
            out1, out2,
            "rendered output must be byte-stable for same inputs"
        );
    }

    #[test]
    fn s4_voice_profile_changes_render_shape() {
        // The writer is voice-profile-agnostic for the substrate; the
        // mapping to Bender-friendly vs concise happens in the CLI layer.
        // Verify the substrate preserves profile-neutral data.
        let mut n = narrative();
        n.audience = NarrativeAudience::SelfAuthor;
        n.tone = NarrativeTone::Concise;
        let out = writer().write(&n, 500).expect("write");
        assert!(out.contains("audience=self"));
        assert!(out.contains("tone=concise"));
    }

    #[test]
    fn s5_bounded_length_truncates() {
        // Build a very long narrative with many suggestions.
        let mut n = narrative();
        for i in 0..20 {
            n.next_step_suggestions.push(
                NextStepSuggestion::builder(format!("Suggestion {i:02}"))
                    .description("x".repeat(50))
                    .priority(if i % 3 == 0 {
                        SuggestionPriority::High
                    } else {
                        SuggestionPriority::Low
                    })
                    .effort(EffortEstimate::Small)
                    .build(),
            );
        }
        n.sort_suggestions();
        let out = writer().write(&n, 30).expect("write");
        let words: Vec<&str> = out.split_whitespace().collect();
        // Marker phrase adds ~12 words on top of the truncated body,
        // so we allow a slack of 12 over the cap.
        assert!(
            words.len() <= 30 + 12,
            "truncation must shrink output (got {} words)",
            words.len()
        );
        assert!(
            out.contains("truncado"),
            "truncation marker must be present"
        );
    }

    #[test]
    fn s6_priority_effort_stable_sort() {
        let mut n = narrative();
        // Two suggestions with same (priority, effort) but different titles.
        n.next_step_suggestions.push(
            NextStepSuggestion::builder("Z-title same priority")
                .description("zzz")
                .priority(SuggestionPriority::High)
                .effort(EffortEstimate::Small)
                .build(),
        );
        n.next_step_suggestions.push(
            NextStepSuggestion::builder("A-title same priority")
                .description("aaa")
                .priority(SuggestionPriority::High)
                .effort(EffortEstimate::Small)
                .build(),
        );
        n.sort_suggestions();
        let out = writer().write(&n, 500).expect("write");
        let za = out.find("Z-title").unwrap();
        let aa = out.find("A-title").unwrap();
        assert!(aa < za, "tie-break by title lexicographic");
    }

    #[test]
    fn s7_anchor_kind_closed_set_audit() {
        assert_eq!(ANCHOR_KIND_VARIANT_COUNT, 5);
        let all = [
            AnchorKind::EvidenceCard,
            AnchorKind::DecisionRecord,
            AnchorKind::DebtInc,
            AnchorKind::SpineItem,
            AnchorKind::ExternalRef,
        ];
        assert_eq!(all.len(), ANCHOR_KIND_VARIANT_COUNT);
    }

    #[test]
    fn s8_cycle_digest_stable_across_rerun() {
        let d1 = CycleNarrative::compute_digest(
            "C-018",
            "title",
            "what",
            &[NextStepSuggestion::builder("a")
                .priority(SuggestionPriority::High)
                .effort(EffortEstimate::Small)
                .build()],
        );
        let d2 = CycleNarrative::compute_digest(
            "C-018",
            "title",
            "what",
            &[NextStepSuggestion::builder("a")
                .priority(SuggestionPriority::High)
                .effort(EffortEstimate::Small)
                .build()],
        );
        assert_eq!(d1, d2);
    }

    #[test]
    fn s9_empty_suggestions_section_hidden() {
        let mut n = narrative();
        n.next_step_suggestions.clear();
        n.sort_suggestions();
        let out = writer().write(&n, 500).expect("write");
        assert!(
            !out.contains("Next steps"),
            "empty suggestions should suppress section header"
        );
    }

    #[test]
    fn s10_audit_constants_match_enum_sizes() {
        assert_eq!(NARRATIVE_AUDIENCE_VARIANT_COUNT, 3);
        assert_eq!(NARRATIVE_TONE_VARIANT_COUNT, 3);
        assert_eq!(SUGGESTION_PRIORITY_VARIANT_COUNT, 3);
        assert_eq!(EFFORT_ESTIMATE_VARIANT_COUNT, 3);
    }

    #[test]
    fn s11_empty_cycle_id_rejected() {
        let mut n = narrative();
        n.cycle_id.clear();
        let r = writer().write(&n, 100);
        assert!(r.is_err());
    }

    #[test]
    fn s12_anchor_kinds_emit_label_text() {
        let n = narrative();
        let out = writer().write(&n, 500).expect("write");
        assert!(
            out.contains("[ref]") || out.contains("[spine]") || out.contains("[debt]"),
            "rendered output should include at least one anchor label"
        );
    }
}
