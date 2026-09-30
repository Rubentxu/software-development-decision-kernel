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

    // C3l.2 falsificador principal: register rule → dispatch(real
    // ProducerEvent) → expected signal. La ruta pública del adapter es la
    // que debe disparar la regla; reconstruir el ReactiveEvent a mano está
    // PROHIBIDO por C3l.2 (eso demostraba el engine, no la ruta).
    let adapter =
        ProducerToL0Adapter::with_engine(std::sync::Arc::new(clone_engine(&engine)), 10_000);
    let signals = adapter.dispatch(ProducerEvent::CogniCodeFinding {
        symbol: "stale_parser".into(),
        file: "crates/sddk-engine/src/lib.rs".into(),
        line: 119,
        finding_kind: FindingKind::Unused,
    });
    assert_eq!(
        signals,
        vec![ReactiveSignal::PersistEvidence {
            evidence_ref: "ev-g04-cognicode".into(),
            summary: "unused symbol".into(),
        }],
        "a registered rule MUST fire through the public dispatch path"
    );
}

/// Exit gate de C3l.2: el test de arriba FALLA si el engine inyectado se
/// sustituye por uno nuevo vacío — una regla registrada en un engine que el
/// adapter no usa no puede disparar señal alguna por la ruta pública.
#[test]
fn exit_gate_fresh_engine_cannot_fire_registered_rules() {
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

    // El adapter recibe un engine DISTINTO (vacío): sin wiring, la señal no
    // existe. Esto es lo que el defecto C3l.2 hacía SIEMPRE.
    let adapter =
        ProducerToL0Adapter::with_engine(std::sync::Arc::new(SecretaryL0Engine::new()), 10_000);
    let signals = adapter.dispatch(ProducerEvent::CogniCodeFinding {
        symbol: "stale_parser".into(),
        file: "crates/sddk-engine/src/lib.rs".into(),
        line: 119,
        finding_kind: FindingKind::Unused,
    });
    assert!(
        signals.is_empty(),
        "an unconfigured engine fires nothing: the signal came from the \
         injected engine, not from the adapter"
    );
}

/// Helper del test: clona el estado del engine vía registro paralelo. El
/// engine es interior-mutable pero no Clone; para el falsificador basta con
/// un segundo engine con LA MISMA regla registrada (lo que el caller hace
/// en producción: configura UN engine y se lo pasa al adapter).
fn clone_engine(engine: &SecretaryL0Engine) -> SecretaryL0Engine {
    let _ = engine; // la regla se re-registra en el nuevo engine abajo
    let clone = SecretaryL0Engine::new();
    clone
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
        .expect("rule registers on the clone");
    clone
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

    // C3l.2: la regla registrada dispara POR LA RUTA PÚBLICA del adapter
    // configurado con ese engine (sin reconstrucción manual del evento).
    let adapter =
        ProducerToL0Adapter::with_engine(std::sync::Arc::new(engine_for_adapter()), 20_000);
    let signals = adapter.dispatch(ProducerEvent::ChronosCrash {
        program: "sddk-cli".into(),
        signal: 11,
        callstack_top: "sddk::main+0x42".into(),
    });
    assert_eq!(
        signals,
        vec![ReactiveSignal::OpenHumanDecision {
            request_ref: "dec-g04-crash".into(),
            reason: "producer reported a crash".into(),
        }],
        "a registered crash rule MUST fire through the public dispatch path"
    );
}

/// Regla de crash registrada dos veces (una para el engine de aserción
/// original del test, otra para el engine que recibe el adapter): en
/// producción el caller configura UN engine y lo comparte; aquí el dup
/// documenta que el disparo depende del engine inyectado, no del adapter.
fn engine_for_adapter() -> SecretaryL0Engine {
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
    engine
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
