# SCOPE-CONTRACT — j2-j3-agentic-session-binding (arch-spec-024, J2/J3)

## Goal

Primera implementación del binding semántico de sesiones host↔SDDK
(JCODE_CORE_GA, track J2/J3 del mini-roadmap).

| Req | Test | Estado |
|---|---|---|
| ASB-001 session ≠ run (attach no sintetiza Run) | attach_project_creates_no_run | ✅ |
| ASB-002 modos PROJECT/WORK_ITEM/RUN/TASK/EPHEMERAL | BindingTarget (cerrado) | ✅ |
| ASB-003 ContextBasis observable, secuencia monótona | basis_sequence_is_monotonic | ✅ |
| ASB-004 transcript queda en host (solo semantic_refs) | reattach_from_persisted… | ✅ |
| ASB-005 reattach desde estado semántico persistido | reattach_from_persisted… | ✅ |
| ASB-006 aislamiento multi-sesión mismo proyecto | multi_session_isolation… | ✅ |
| AW-UAT-021 rebind con receipt + invalidación de basis | rebind_records_receipt… | ✅ |

## Cambios

- NUEVO `crates/sddk-engine/src/agentic_session_binding.rs`
  (módulo puro, sin I/O): AgenticSessionRef, RunRef, BindingTarget,
  ContextBasis, AgenticBinding, BindingStore, errores cerrados.
- Tests de unidad incluidos (7).

## Out of scope (siguientes slices)

- AW-UAT-023 parte remota (J8: localidad lógica vs física).
- J4 ContextBridge (bootstrap + deltas, arch-spec-026).
- J5 Reactive Verify pipeline + AC9.
- J6 AgentWorkRequest/run_structured (arch-spec-027/030).

## Evidence

- `cargo test -p sddk-engine --lib agentic_session_binding` → 7 PASS / 0 FAIL.
- clippy -D warnings = 0.
