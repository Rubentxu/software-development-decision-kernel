//! Cycle Narrative — operator-facing cycle summary + argumented
//! next-step suggestions. Plain-data, deterministic, bounded-length.
//!
//! Pattern: P-a (struct + trait + plain-data evaluator).

use std::collections::hash_map::DefaultHasher;
use std::fmt::Write as _;
use std::hash::Hasher;

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
