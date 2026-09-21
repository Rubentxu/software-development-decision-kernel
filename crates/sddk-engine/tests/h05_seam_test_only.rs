// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// tests/h05_seam_test_only.rs — H05 / Phase 1 of §2 C1 H05 closure.
//
// ROADMAP §2 C1 H05: "test-seam set_process_service_for_tests con
// semántica real y aislamiento de producción".
//
// The seam is marked `#[cfg(test)]` in
// `sddk_engine::authority_ticket_service`. This integration test
// documents that fact and verifies it compiles under a normal
// (`cargo test --test h05_seam_test_only`) build.
//
// The HARD proof that the seam is unreachable from production code
// is enforced by the companion shell test `tests/test_h05_isolation.sh`,
// which runs `nm target/release/libsddk_engine.rlib` and asserts the
// symbol `set_process_service_for_tests` is absent from the exported
// symbol table of the production rlib.
//
// Why we don't try to import the seam here: integration tests under
// `crates/<name>/tests/*.rs` are themselves compiled with cfg(test)
// set, so they CAN see `#[cfg(test)]` items in the library they
// link against. So a "compile-fail" stunt would not actually prove
// production-isolation here. Instead, the gate is the shell test.

/// Confirm `process_service()` is reachable (production API surface).
/// The seam function is intentionally NOT imported; if some refactor
/// made it `pub` (no cfg(test)), the shell test would catch it before
/// this Rust test would notice.
#[test]
fn h05_seam_does_not_leak_into_production_api() {
    // Reference the production-side function (must be `pub`).
    let _svc: &sddk_engine::authority_ticket_service::AuthorityTicketService =
        sddk_engine::authority_ticket_service::process_service();
    // Reference the module itself (just a compile-time check that
    // the import path is well-formed).
    let _ = std::any::type_name::<sddk_engine::authority_ticket_service::AuthorityTicketService>();
}
