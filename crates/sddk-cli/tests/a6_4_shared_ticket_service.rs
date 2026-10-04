// A6-4 §5 / §6 — shared-state proof: both High-band ticket surfaces share
// the SAME `AuthorityTicketService` singleton (process-wide), so a
// `framework_bundle` ticket and a `github_releases` ticket observed on
// the same process increment the SAME monotonic seq, share the SAME
// fence domain, and are visible to each other through
// `AuthorityTicketService::process_service()`.
//
// Plus a cross-surface policy-transition proof (§6): a ticket issued on
// one surface is invalidated when a policy is re-registered on the
// service (fence advances) regardless of which surface issued it.

use sddk_cli::dev::framework_bundle_ticket::{
    framework_bundle_policy, with_framework_bundle_ticket,
};
use sddk_cli::dev::github_releases_ticket::github_releases_policy;
use sddk_engine::authority_engine::{ActionKind, ActionProposal, Actor, ActorKind, Facts};
use sddk_engine::authority_ticket_service::{AuthorityTicketServiceError, process_service};

// SEC-WORKSPACE-FLAKE fix (2026-09-19): the redundant
// `cross_surface_facades_share_the_service_instance` test was
// removed because it raced with parallel tests touching the
// same `process_service()` singleton — see
// `tests/cycle-artifacts/p-63676b11dc0ef88f/sec-workspace-flake-fix/SCOPE-CONTRACT.md`.
// `github_releases_policy` is still referenced by the retained
// `cross_surface_policy_transition_invalidates_either_surface_ticket`
// test.
//
// SEC-WORKSPACE-FLAKE, segunda vuelta (session-72, medido): **el arreglo de
// 2026-09-19 quito UNO de los dos corredores y dejo el otro.** Los dos tests
// que quedan en este fichero se pisan el `process_service()` entre si, porque
// `process_service()` es un singleton de proceso y Rust ejecuta los `#[test]`
// de un mismo binario en hilos:
//
//   `cross_surface_shared_seq_strictly_monotonic`
//       hace `set_last_policy_digest(framework_bundle_policy())`
//   `cross_surface_policy_transition_invalidates_either_surface_ticket`
//       hace `set_last_policy_digest(github_releases_policy())`
//
// Si el segundo se cuela entre el `issue` y el `consume_at_live_now` del
// primero —o al reves— el `consume_at_live_now` ya no ve un cambio de politica
// y devuelve Ok, y el `.expect_err("must refuse")` revienta con
// `must refuse: ()`.
//
// **Medido, no supuesto:** 8/8 verde aislado
// (`cargo test -p sddk-cli --test a6_4_shared_ticket_service` repetido), y
// FALLA bajo `cargo test --workspace`. O sea: la causa es la concurrencia, y
// el fallo no depende de nada que yo hubiera cambiado — cero commits tocaban
// `crates/` en ese rango. Es un gate de release no determinista: cada release
// tiene una probabilidad independiente de morir aqui.
//
// El candado es de FICHERO, y tiene que serlo: los binarios de test corren en
// procesos separados, luego el unico que comparte el singleton son los tests de
// este fichero. Un candado global de crate no evitaria nada y serializaria
// tests que no compiten.
fn serial_guard() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn sys_actor(service: &str) -> Actor {
    Actor {
        kind: ActorKind::System {
            service: service.to_string(),
        },
        capabilities: vec!["cli.execute".to_string()],
        lease: None,
    }
}

#[test]
fn shared_service_singleton_is_same_instance_for_both_surfaces() {
    // process_service() must return the same static ref every time.
    let a = process_service() as *const _;
    let b = process_service() as *const _;
    assert_eq!(a, b, "singleton must be the same instance");
}

#[test]
fn cross_surface_shared_seq_strictly_monotonic() {
    // A6-4 §5: framework_bundle ticket ⇒ seq=N, github_releases ticket
    // ⇒ seq=N+1. They share the process-wide monotonic seq.
    //
    // El candado serializa este test con el de la transicion de politica: los
    // dos escriben `last_policy_digest` en el MISMO singleton de proceso, y sin
    // el uno se cuela entre el `issue` y el `consume_at_live_now` del otro.
    let _serial = serial_guard();
    let svc = process_service();
    let before = svc.next_seq();

    // Force the service's last_policy_digest back to canonical for both
    // surfaces (the facades do this anyway; doing it explicitly here
    // keeps the test independent of test ordering).
    svc.set_last_policy_digest(framework_bundle_policy().policy_digest.clone());
    let ticket_a = svc
        .issue(
            &sys_actor("a6-4-shared"),
            &ActionProposal {
                kind: ActionKind::CliRelease,
                target_id: "shared/fb".to_string(),
                payload_digest: None,
                created_at: time::OffsetDateTime::now_utc(),
            },
            &framework_bundle_policy(),
            &Facts::default(),
        )
        .expect("issue fb");

    svc.set_last_policy_digest(github_releases_policy().policy_digest.clone());
    let ticket_b = svc
        .issue(
            &sys_actor("a6-4-shared"),
            &ActionProposal {
                kind: ActionKind::CliRelease,
                target_id: "shared/gr".to_string(),
                payload_digest: None,
                created_at: time::OffsetDateTime::now_utc(),
            },
            &github_releases_policy(),
            &Facts::default(),
        )
        .expect("issue gr");

    let after = svc.next_seq();
    assert!(
        after >= before + 2,
        "seq must advance by at least 2 across surfaces (was {before}, now {after})"
    );
    assert_ne!(
        ticket_a.ticket_id, ticket_b.ticket_id,
        "different surfaces produce different ticket ids"
    );
}

#[test]
fn cross_surface_policy_transition_invalidates_either_surface_ticket() {
    // A6-4 §6: register a fresh policy on the service after issuing a
    // fb ticket. The fence advances; the fb ticket can no longer be
    // consumed against the live now. Demonstrates that the fence is no
    // longer local to the helper.
    // Mismo candado, y por el mismo motivo: este test escribe
    // `last_policy_digest` y el anterior tambien, sobre el mismo singleton.
    let _serial = serial_guard();
    let svc = process_service();
    let fb_policy = framework_bundle_policy();
    svc.set_last_policy_digest(fb_policy.policy_digest.clone());
    let fb_ticket = svc
        .issue(
            &sys_actor("a6-4-cross"),
            &ActionProposal {
                kind: ActionKind::CliRelease,
                target_id: "shared/cross-fb".to_string(),
                payload_digest: None,
                created_at: time::OffsetDateTime::now_utc(),
            },
            &fb_policy,
            &Facts::default(),
        )
        .expect("issue fb");

    // Now register a NEW policy under a different policy_id on the same
    // service. This is what `github_releases` would do on entry.
    let mut gr_policy = github_releases_policy();
    gr_policy.policy_version = 99; // distinct from any prior registration
    svc.register_policy(gr_policy).expect("register gr");
    svc.set_last_policy_digest(github_releases_policy().policy_digest.clone());

    // The fb ticket can no longer be consumed at live_now: either the
    // digest changed (PolicyChanged) or the fence moved (FenceExpired).
    let err = svc
        .consume_at_live_now(&fb_ticket, &fb_policy)
        .expect_err("must refuse");
    match err {
        AuthorityTicketServiceError::Ticket(
            sddk_engine::authority_admission_ticket::AdmissionTicketError::PolicyChanged { .. }
            | sddk_engine::authority_admission_ticket::AdmissionTicketError::FenceExpired { .. },
        ) => {}
        other => panic!("expected PolicyChanged or FenceExpired, got {:?}", other),
    }
}

#[test]
fn facade_deny_yields_zero_side_effects_on_real_service() {
    // A6-4 §3: deny ⇒ no ticket ⇒ no body execution. We confirm this
    // against the real shared service by constructing an actor without
    // capabilities and observing that `Denied` propagates through the
    // facade and the body closure never runs.
    //
    // MEDIDO: este test NO tomaba `serial_guard()` y los otros tres del fichero
    // si. `with_framework_bundle_ticket` va al MISMO `process_service()`
    // singleton, luego este test corria en paralelo con
    // `cross_surface_policy_transition_invalidates_either_surface_ticket`, que
    // escribe `last_policy_digest` y decide en consecuencia: la carrera hacia
    // que el consume devolviera Ok donde deberia devolver PolicyChanged, y el
    // aserto reventaba con `must refuse: ()`. Solo se manifestaba en el run
    // completo del workspace —con los 4 tests en hilo, la ventana es mayor— y
    // no al correr el fichero solo, que es justo la forma de fallo que hace
    // que uno crea que el suite esta verde.
    let _serial = serial_guard();
    let actor = Actor {
        kind: ActorKind::System {
            service: "no-cap-shared".to_string(),
        },
        capabilities: Vec::new(),
        lease: None,
    };
    let invoked = std::sync::Mutex::new(false);
    let result = with_framework_bundle_ticket::<_, ()>(actor, "shared/deny", || {
        *invoked.lock().unwrap() = true;
        Ok(())
    });
    match result {
        Err(sddk_cli::dev::framework_bundle_ticket::FrameworkBundleTicketError::Denied(_)) => {}
        other => panic!("expected Denied, got {:?}", other),
    }
    assert!(
        !*invoked.lock().unwrap(),
        "body must NOT run when the shared service denies"
    );
}
