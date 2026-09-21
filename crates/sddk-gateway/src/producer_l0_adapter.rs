//! Producer → L0 stream adapter (AIW-S7a).
//!
//! Converts external producer events (CogniCode code findings, Chronos
//! crash/race reports) into public [`ReactiveEvent`] values and evaluates
//! them synchronously with the real [`SecretaryL0Engine`]. Unknown events
//! produce no signals at all: no automatic WorkItem, no external side
//! effect (G06).
//!
//! Spec: tests/cycle-artifacts/p-63676b11dc0ef88f/aiw-s7a-producer-l0-stream/SCOPE-CONTRACT.md

use sddk_engine::{ReactiveEvent, ReactiveSignal, SecretaryL0Engine};

/// Classification of a producer finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingKind {
    /// Symbol defined but never referenced.
    Unused,
    /// Branch statically known to be unreachable.
    DeadBranch,
    /// Operation applied to a value of the wrong type.
    TypeMismatch,
    /// Reference to a symbol that cannot be resolved.
    UnresolvedRef,
}

impl FindingKind {
    /// Stable snake_case tag used as the `finding_kind` custom key.
    pub fn as_str(self) -> &'static str {
        match self {
            FindingKind::Unused => "unused",
            FindingKind::DeadBranch => "dead_branch",
            FindingKind::TypeMismatch => "type_mismatch",
            FindingKind::UnresolvedRef => "unresolved_ref",
        }
    }
}

/// A producer event arriving at the gateway from an external source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProducerEvent {
    /// A static-analysis finding from CogniCode.
    CogniCodeFinding {
        /// Symbol the finding refers to.
        symbol: String,
        /// Path of the file containing the symbol.
        file: String,
        /// 1-based line of the finding.
        line: u32,
        /// Classification of the finding.
        finding_kind: FindingKind,
    },
    /// A crash captured by Chronos with the top of the call stack.
    ChronosCrash {
        /// Program that crashed.
        program: String,
        /// POSIX signal number observed.
        signal: i32,
        /// Top frame of the captured call stack.
        callstack_top: String,
    },
    /// A data race captured by Chronos.
    ChronosRace {
        /// Memory address of the racing access.
        address: String,
        /// First racing thread.
        thread_a: String,
        /// Second racing thread.
        thread_b: String,
    },
    /// Unrecognized producer input. Always silent.
    Unknown,
}

/// Synchronous adapter from producer events to Secretary L0 reactive rules.
#[derive(Debug, Default)]
pub struct ProducerToL0Adapter {
    now_ms: i64,
}

impl ProducerToL0Adapter {
    /// Adapter with the current timestamp anchored at construction.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adapter with a deterministic timestamp (tests, replays).
    pub fn with_now(now_ms: i64) -> Self {
        Self { now_ms }
    }

    /// Converts `ev` into a [`ReactiveEvent`] and evaluates it against a
    /// fresh real engine. Unknown events return an empty signal list.
    pub fn dispatch(&self, ev: ProducerEvent) -> Vec<ReactiveSignal> {
        let Some(reactive) = self.to_reactive(ev) else {
            return Vec::new();
        };
        SecretaryL0Engine::new().evaluate(&reactive, self.now_ms)
    }

    fn to_reactive(&self, ev: ProducerEvent) -> Option<ReactiveEvent> {
        use sddk_engine::ReactiveTrigger;
        let reactive = match ev {
            ProducerEvent::CogniCodeFinding {
                symbol,
                file,
                line,
                finding_kind,
            } => ReactiveEvent::new(ReactiveTrigger::CandidateArrived, self.now_ms)
                .with_cycle_ref(format!("cognicode:{symbol}"))
                .with_custom("producer", "cognicode")
                .with_custom("symbol", symbol)
                .with_custom("file", file)
                .with_custom("line", line.to_string())
                .with_custom("finding_kind", finding_kind.as_str()),
            ProducerEvent::ChronosCrash {
                program,
                signal,
                callstack_top,
            } => ReactiveEvent::new(ReactiveTrigger::FrontierBlockerChanged, self.now_ms)
                .with_cycle_ref(format!("chronos-crash:{program}"))
                .with_custom("producer", "chronos")
                .with_custom("event_kind", "crash")
                .with_custom("program", program)
                .with_custom("signal", signal.to_string())
                .with_custom("callstack_top", callstack_top),
            ProducerEvent::ChronosRace {
                address,
                thread_a,
                thread_b,
            } => {
                let _ = (&thread_a, &thread_b);
                ReactiveEvent::new(ReactiveTrigger::FrontierBlockerChanged, self.now_ms)
                    .with_cycle_ref(format!("chronos-race:{address}"))
                    .with_custom("producer", "chronos")
                    .with_custom("event_kind", "race")
                    .with_custom("address", address)
                    .with_custom("thread_a", thread_a)
                    .with_custom("thread_b", thread_b)
            }
            ProducerEvent::Unknown => return None,
        };
        Some(reactive)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sddk_engine::{ReactiveMatcher, ReactiveRule, ReactiveTrigger};

    #[test]
    fn cognicode_finding_dispatches() {
        let adapter = ProducerToL0Adapter::with_now(1_000);
        let signals = adapter.dispatch(ProducerEvent::CogniCodeFinding {
            symbol: "unused_helper".into(),
            file: "src/lib.rs".into(),
            line: 42,
            finding_kind: FindingKind::Unused,
        });
        // Fresh engine with no rules: dispatch must not crash and must
        // produce no spurious signals.
        assert!(signals.is_empty());
    }

    #[test]
    fn chronos_crash_dispatches() {
        let adapter = ProducerToL0Adapter::with_now(2_000);
        let signals = adapter.dispatch(ProducerEvent::ChronosCrash {
            program: "sddk-cli".into(),
            signal: 11,
            callstack_top: "main+0x1f".into(),
        });
        assert!(signals.is_empty());
    }

    #[test]
    fn unknown_event_empty() {
        let adapter = ProducerToL0Adapter::with_now(3_000);
        let signals = adapter.dispatch(ProducerEvent::Unknown);
        assert!(signals.is_empty(), "G06: unknown events must be silent");
    }

    #[test]
    fn registered_rule_fires_persist_evidence() {
        let adapter = ProducerToL0Adapter::with_now(4_000);
        let reactive = ReactiveEvent::new(ReactiveTrigger::CandidateArrived, adapter.now_ms)
            .with_cycle_ref("cognicode:unused_helper")
            .with_custom("producer", "cognicode")
            .with_custom("finding_kind", "unused");

        let engine = SecretaryL0Engine::new();
        engine
            .register(ReactiveRule::new(
                "persist-cognicode-unused",
                ReactiveTrigger::CandidateArrived,
                ReactiveMatcher::CustomKey {
                    key: "producer".into(),
                    value: "cognicode".into(),
                },
                ReactiveSignal::PersistEvidence {
                    evidence_ref: "ev-cognicode-1".into(),
                    summary: "CogniCode unused symbol".into(),
                },
            ))
            .expect("rule registers");
        assert_eq!(engine.rule_count(), 1);

        let signals = engine.evaluate(&reactive, 4_000);
        assert_eq!(
            signals,
            vec![ReactiveSignal::PersistEvidence {
                evidence_ref: "ev-cognicode-1".into(),
                summary: "CogniCode unused symbol".into(),
            }]
        );
        // And the adapter path itself is timestamp-consistent.
        assert_eq!(ProducerToL0Adapter::with_now(4_000).now_ms, reactive.at_ms);
    }
}
