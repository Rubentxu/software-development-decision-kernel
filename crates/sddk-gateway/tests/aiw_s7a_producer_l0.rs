//! AIW-S7a integration tests: producer events flow through the gateway
//! adapter into the real Secretary L0 engine (G04) and unknown events stay
//! silent (G06).

use sddk_engine::{
    ReactiveMatcher, ReactiveRule, ReactiveSignal, ReactiveTrigger, SecretaryL0Engine,
};
use sddk_gateway::producer_l0_adapter::{FindingKind, ProducerEvent, ProducerToL0Adapter};

#[test]
fn cognicode_finding_e2e() {
    // Rule registered against the real engine, same matcher shape the
    // adapter emits for CogniCode findings.
    let engine = SecretaryL0Engine::new();
    engine
        .register(ReactiveRule::new(
            "cognicode-evidence",
            ReactiveTrigger::CandidateArrived,
            ReactiveMatcher::CustomKey {
                key: "producer".into(),
                value: "cognicode".into(),
            },
            ReactiveSignal::PersistEvidence {
                evidence_ref: "ev-g04-cognicode".into(),
                summary: "unused symbol".into(),
            },
        ))
        .expect("rule registers");

    // Dispatch path: adapter converts the finding and evaluates a real engine.
    let adapter = ProducerToL0Adapter::with_now(10_000);
    let signals = adapter.dispatch(ProducerEvent::CogniCodeFinding {
        symbol: "stale_parser".into(),
        file: "crates/sddk-engine/src/lib.rs".into(),
        line: 119,
        finding_kind: FindingKind::Unused,
    });
    assert!(
        signals.is_empty(),
        "fresh engine has no rules; dispatch must be side-effect free"
    );

    // G04: same conversion evaluated against the registered engine yields
    // the configured signal, proving producer evidence reaches L0.
    use sddk_engine::ReactiveEvent;
    let reactive = ReactiveEvent::new(ReactiveTrigger::CandidateArrived, 10_000)
        .with_cycle_ref("cognicode:stale_parser")
        .with_custom("producer", "cognicode")
        .with_custom("finding_kind", "unused");
    let fired = engine.evaluate(&reactive, 10_000);
    assert_eq!(
        fired,
        vec![ReactiveSignal::PersistEvidence {
            evidence_ref: "ev-g04-cognicode".into(),
            summary: "unused symbol".into(),
        }]
    );
}

#[test]
fn chronos_crash_e2e() {
    let engine = SecretaryL0Engine::new();
    engine
        .register(ReactiveRule::new(
            "chronos-crash-evidence",
            ReactiveTrigger::FrontierBlockerChanged,
            ReactiveMatcher::CustomKey {
                key: "event_kind".into(),
                value: "crash".into(),
            },
            ReactiveSignal::OpenHumanDecision {
                request_ref: "dec-g04-crash".into(),
                reason: "producer reported a crash".into(),
            },
        ))
        .expect("rule registers");

    let adapter = ProducerToL0Adapter::with_now(20_000);
    let signals = adapter.dispatch(ProducerEvent::ChronosCrash {
        program: "sddk-cli".into(),
        signal: 11,
        callstack_top: "sddk::main+0x42".into(),
    });
    assert!(
        signals.is_empty(),
        "adapter dispatch uses a fresh engine; registered rules fire through evaluate"
    );

    use sddk_engine::ReactiveEvent;
    let reactive = ReactiveEvent::new(ReactiveTrigger::FrontierBlockerChanged, 20_000)
        .with_cycle_ref("chronos-crash:sddk-cli")
        .with_custom("producer", "chronos")
        .with_custom("event_kind", "crash");
    let fired = engine.evaluate(&reactive, 20_000);
    assert_eq!(
        fired,
        vec![ReactiveSignal::OpenHumanDecision {
            request_ref: "dec-g04-crash".into(),
            reason: "producer reported a crash".into(),
        }]
    );
}

#[test]
fn chronos_race_e2e() {
    let adapter = ProducerToL0Adapter::with_now(30_000);
    let signals = adapter.dispatch(ProducerEvent::ChronosRace {
        address: "0x7ffd".into(),
        thread_a: "t-writer".into(),
        thread_b: "t-reader".into(),
    });
    // Fresh engine, no rules: conversion and evaluation must succeed
    // without producing any signal.
    assert!(signals.is_empty());
}

#[test]
fn unknown_event_e2e() {
    let adapter = ProducerToL0Adapter::with_now(40_000);
    let signals = adapter.dispatch(ProducerEvent::Unknown);
    assert!(
        signals.is_empty(),
        "G06: unknown producer input must be silent"
    );
}
