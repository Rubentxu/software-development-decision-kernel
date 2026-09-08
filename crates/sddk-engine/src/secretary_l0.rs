//! Secretary L0 Reactive Rules substrate — deterministic, fully
//! reactive; never orchestrator/memory/authority path.
//!
//! Spec: ~/.sddk-knowledge/sddk-framework/specs/engine/REQ-SecretaryL0ReactiveRules.md
//! ADR:  ~/.sddk-knowledge/sddk-framework/adrs/ADR-089-SECRETARY-L0-REACTIVE-RULES.md

use std::collections::BTreeMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

/// What kind of event the rule reacts to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ReactiveTrigger {
    CandidateArrived,
    HumanDecisionSubmitted,
    CyclePhaseChanged,
    FrontierBlockerChanged,
    RehydrationStepFinished,
    ManualHeartbeat,
}

/// What the rule emits when fired.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[non_exhaustive]
pub enum ReactiveSignal {
    SurfaceContext {
        context_ref: String,
    },
    OpenHumanDecision {
        request_ref: String,
        reason: String,
    },
    PersistEvidence {
        evidence_ref: String,
        summary: String,
    },
    EmitEnvelope {
        envelope_id: String,
        summary: String,
    },
    Silent,
}

/// Predicate the rule uses against the incoming event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ReactiveMatcher {
    MatchAll,
    MatchCycleRef(String),
    MatchRoleId(String),
    MatchDecisionKind(String),
    CustomKey { key: String, value: String },
}

impl ReactiveMatcher {
    fn matches(&self, ev: &ReactiveEvent) -> bool {
        match self {
            ReactiveMatcher::MatchAll => true,
            ReactiveMatcher::MatchCycleRef(c) => ev.cycle_ref.as_deref() == Some(c.as_str()),
            ReactiveMatcher::MatchRoleId(r) => ev.role_id.as_deref() == Some(r.as_str()),
            ReactiveMatcher::MatchDecisionKind(k) => {
                ev.decision_kind.as_deref() == Some(k.as_str())
            }
            ReactiveMatcher::CustomKey { key, value } => {
                ev.custom.get(key).map(|v| v == value).unwrap_or(false)
            }
        }
    }
}

/// Rule definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReactiveRule {
    pub rule_id: String,
    pub trigger: ReactiveTrigger,
    pub matcher: ReactiveMatcher,
    pub signal: ReactiveSignal,
    pub cooldown_ms: i64,
}

impl ReactiveRule {
    pub fn new(
        rule_id: impl Into<String>,
        trigger: ReactiveTrigger,
        matcher: ReactiveMatcher,
        signal: ReactiveSignal,
    ) -> Self {
        Self {
            rule_id: rule_id.into(),
            trigger,
            matcher,
            signal,
            cooldown_ms: 0,
        }
    }

    pub fn with_cooldown_ms(mut self, ms: i64) -> Self {
        self.cooldown_ms = ms;
        self
    }
}

/// Per-event payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReactiveEvent {
    pub trigger: ReactiveTrigger,
    pub cycle_ref: Option<String>,
    pub role_id: Option<String>,
    pub decision_kind: Option<String>,
    pub custom: BTreeMap<String, String>,
    pub at_ms: i64,
}

impl ReactiveEvent {
    pub fn new(trigger: ReactiveTrigger, at_ms: i64) -> Self {
        Self {
            trigger,
            cycle_ref: None,
            role_id: None,
            decision_kind: None,
            custom: BTreeMap::new(),
            at_ms,
        }
    }

    pub fn with_cycle_ref(mut self, c: impl Into<String>) -> Self {
        self.cycle_ref = Some(c.into());
        self
    }
    pub fn with_role_id(mut self, r: impl Into<String>) -> Self {
        self.role_id = Some(r.into());
        self
    }
    pub fn with_custom(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom.insert(key.into(), value.into());
        self
    }
}

/// Error taxonomy (closed-set, `#[non_exhaustive]`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[non_exhaustive]
pub enum SecretaryL0Error {
    EmptyRuleId,
    InvalidMatcher,
    NegativeCooldown,
}

impl std::fmt::Display for SecretaryL0Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecretaryL0Error::EmptyRuleId => write!(f, "rule_id is empty"),
            SecretaryL0Error::InvalidMatcher => write!(f, "matcher is invalid"),
            SecretaryL0Error::NegativeCooldown => write!(f, "cooldown_ms is negative"),
        }
    }
}

impl std::error::Error for SecretaryL0Error {}

/// Deterministic reactive engine.
#[derive(Debug, Default)]
pub struct SecretaryL0Engine {
    rules: Mutex<Vec<ReactiveRule>>,
    last_fire_ms: Mutex<BTreeMap<String, i64>>,
}

impl SecretaryL0Engine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Validate and register a rule.
    pub fn register(&self, rule: ReactiveRule) -> Result<(), SecretaryL0Error> {
        if rule.rule_id.trim().is_empty() {
            return Err(SecretaryL0Error::EmptyRuleId);
        }
        if rule.cooldown_ms < 0 {
            return Err(SecretaryL0Error::NegativeCooldown);
        }
        if let ReactiveMatcher::CustomKey { value, .. } = &rule.matcher
            && value.is_empty()
        {
            return Err(SecretaryL0Error::InvalidMatcher);
        }
        let mut g = self.rules.lock().expect("SecretaryL0Engine rules poisoned");
        g.push(rule);
        Ok(())
    }

    pub fn rule_count(&self) -> usize {
        let g = self.rules.lock().expect("SecretaryL0Engine rules poisoned");
        g.len()
    }

    /// Deterministic evaluation: rules sorted by `rule_id` lex order;
    /// per-rule cooldown enforced.
    pub fn evaluate(&self, ev: &ReactiveEvent, now_ms: i64) -> Vec<ReactiveSignal> {
        let g = self.rules.lock().expect("SecretaryL0Engine rules poisoned");
        let mut sorted: Vec<&ReactiveRule> = g.iter().collect();
        sorted.sort_by(|a, b| a.rule_id.cmp(&b.rule_id));

        let mut out: Vec<ReactiveSignal> = Vec::new();
        let mut last_g = self
            .last_fire_ms
            .lock()
            .expect("SecretaryL0Engine last_fire_ms poisoned");

        for rule in sorted {
            if rule.trigger != ev.trigger
                && !matches!(rule.trigger, ReactiveTrigger::ManualHeartbeat)
            {
                continue;
            }
            if !rule.matcher.matches(ev) {
                continue;
            }
            // Cooldown: skip if last fire was within cooldown window
            if rule.cooldown_ms > 0
                && let Some(&last) = last_g.get(&rule.rule_id)
                && now_ms - last < rule.cooldown_ms
            {
                continue;
            }
            last_g.insert(rule.rule_id.clone(), now_ms);
            if !matches!(rule.signal, ReactiveSignal::Silent) {
                out.push(rule.signal.clone());
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sig_surface(c: &str) -> ReactiveSignal {
        ReactiveSignal::SurfaceContext {
            context_ref: c.to_string(),
        }
    }

    fn rule_for_cycle(rule_id: &str, c: &str, cooldown: i64) -> ReactiveRule {
        ReactiveRule::new(
            rule_id,
            ReactiveTrigger::CyclePhaseChanged,
            ReactiveMatcher::MatchCycleRef(c.to_string()),
            sig_surface("ctx-1"),
        )
        .with_cooldown_ms(cooldown)
    }

    #[test]
    fn empty_rule_id_rejected() {
        let e = SecretaryL0Engine::new();
        let r = ReactiveRule::new(
            "",
            ReactiveTrigger::ManualHeartbeat,
            ReactiveMatcher::MatchAll,
            sig_surface("x"),
        );
        let err = e.register(r).unwrap_err();
        assert!(matches!(err, SecretaryL0Error::EmptyRuleId));
    }

    #[test]
    fn negative_cooldown_rejected() {
        let e = SecretaryL0Engine::new();
        let r = ReactiveRule::new(
            "r1",
            ReactiveTrigger::ManualHeartbeat,
            ReactiveMatcher::MatchAll,
            sig_surface("x"),
        )
        .with_cooldown_ms(-1);
        let err = e.register(r).unwrap_err();
        assert!(matches!(err, SecretaryL0Error::NegativeCooldown));
    }

    #[test]
    fn match_cycle_ref_fires_once() {
        let e = SecretaryL0Engine::new();
        e.register(rule_for_cycle("r-cycle", "c1", 0)).unwrap();
        let ev = ReactiveEvent::new(ReactiveTrigger::CyclePhaseChanged, 1_000).with_cycle_ref("c1");
        let sigs = e.evaluate(&ev, 1_000);
        assert_eq!(sigs.len(), 1);
        assert!(matches!(sigs[0], ReactiveSignal::SurfaceContext { .. }));
    }

    #[test]
    fn cooldown_skips_second_fire() {
        let e = SecretaryL0Engine::new();
        e.register(rule_for_cycle("r-cd", "c1", 1_000)).unwrap();
        let ev = ReactiveEvent::new(ReactiveTrigger::CyclePhaseChanged, 1_000).with_cycle_ref("c1");
        let s1 = e.evaluate(&ev, 1_000);
        let s2 = e.evaluate(&ev, 1_500);
        assert_eq!(s1.len(), 1);
        assert_eq!(s2.len(), 0);
    }

    #[test]
    fn manual_heartbeat_always_matches() {
        let e = SecretaryL0Engine::new();
        e.register(ReactiveRule::new(
            "r-hb",
            ReactiveTrigger::ManualHeartbeat,
            ReactiveMatcher::MatchAll,
            sig_surface("x"),
        ))
        .unwrap();
        let ev = ReactiveEvent::new(ReactiveTrigger::CyclePhaseChanged, 1_000);
        let sigs = e.evaluate(&ev, 1_000);
        assert_eq!(sigs.len(), 1);
    }

    #[test]
    fn non_matching_trigger_returns_silent() {
        let e = SecretaryL0Engine::new();
        e.register(ReactiveRule::new(
            "r-decision",
            ReactiveTrigger::HumanDecisionSubmitted,
            ReactiveMatcher::MatchAll,
            sig_surface("x"),
        ))
        .unwrap();
        let ev = ReactiveEvent::new(ReactiveTrigger::CyclePhaseChanged, 1_000);
        let sigs = e.evaluate(&ev, 1_000);
        assert_eq!(sigs.len(), 0);
    }

    #[test]
    fn multiple_rules_lex_order() {
        let e = SecretaryL0Engine::new();
        e.register(ReactiveRule::new(
            "r-b",
            ReactiveTrigger::CyclePhaseChanged,
            ReactiveMatcher::MatchAll,
            ReactiveSignal::PersistEvidence {
                evidence_ref: "B".into(),
                summary: "b".into(),
            },
        ))
        .unwrap();
        e.register(ReactiveRule::new(
            "r-a",
            ReactiveTrigger::CyclePhaseChanged,
            ReactiveMatcher::MatchAll,
            ReactiveSignal::PersistEvidence {
                evidence_ref: "A".into(),
                summary: "a".into(),
            },
        ))
        .unwrap();
        let ev = ReactiveEvent::new(ReactiveTrigger::CyclePhaseChanged, 1_000);
        let sigs = e.evaluate(&ev, 1_000);
        assert_eq!(sigs.len(), 2);
        if let ReactiveSignal::PersistEvidence { evidence_ref, .. } = &sigs[0] {
            assert_eq!(evidence_ref, "A");
        } else {
            panic!("expected PersistEvidence A first");
        }
    }

    #[test]
    fn custom_key_empty_value_rejected() {
        let e = SecretaryL0Engine::new();
        let r = ReactiveRule::new(
            "r-custom",
            ReactiveTrigger::ManualHeartbeat,
            ReactiveMatcher::CustomKey {
                key: "k".into(),
                value: "".into(),
            },
            sig_surface("x"),
        );
        let err = e.register(r).unwrap_err();
        assert!(matches!(err, SecretaryL0Error::InvalidMatcher));
    }

    #[test]
    fn engine_returns_multiple_signals() {
        let e = SecretaryL0Engine::new();
        e.register(ReactiveRule::new(
            "r-a",
            ReactiveTrigger::ManualHeartbeat,
            ReactiveMatcher::MatchAll,
            sig_surface("x"),
        ))
        .unwrap();
        e.register(ReactiveRule::new(
            "r-b",
            ReactiveTrigger::ManualHeartbeat,
            ReactiveMatcher::MatchAll,
            ReactiveSignal::OpenHumanDecision {
                request_ref: "R".into(),
                reason: "test".into(),
            },
        ))
        .unwrap();
        let ev = ReactiveEvent::new(ReactiveTrigger::ManualHeartbeat, 1_000);
        let sigs = e.evaluate(&ev, 1_000);
        assert_eq!(sigs.len(), 2);
    }

    #[test]
    fn last_fire_stays_empty_until_fire() {
        let e = SecretaryL0Engine::new();
        e.register(ReactiveRule::new(
            "r1",
            ReactiveTrigger::ManualHeartbeat,
            ReactiveMatcher::MatchAll,
            sig_surface("x"),
        ))
        .unwrap();
        let ev = ReactiveEvent::new(ReactiveTrigger::ManualHeartbeat, 1_000);
        e.evaluate(&ev, 1_000);
        assert!(e.last_fire_ms.lock().unwrap().contains_key("r1"));
    }
}
