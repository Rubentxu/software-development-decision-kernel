# SCOPE-CONTRACT — j4-context-bridge (arch-spec-026, J4)

| Req | Test | Estado |
|---|---|---|
| CDD-001 bootstrap una vez por basis | cdd001_bootstrap_once_per_basis | ✅ |
| CDD-002 delta con from/to + provenance + relevance | cdd002_delta_carries_provenance | ✅ |
| CDD-003 filtro de relevancia (sin inyección) | cdd003_irrelevant_changes_no_injection | ✅ |
| CDD-004 advisory nunca es instruction authority | cdd004_advisory_never_instruction | ✅ |
| CDD-005 duplicado/stale rechazado explícito | cdd005_duplicate_and_stale_rejected | ✅ |
| CDD-006 sin resend de contexto completo | cdd006_no_full_context_resend | ✅ |

NUEVO `crates/sddk-engine/src/context_bridge.rs` (módulo puro sobre
agentic_session_binding). Out of scope: J5 Reactive Verify, J6
AgentWorkRequest, AW-UAT-030..034 (cuando exista la superficie host).

Evidence: 6 PASS, clippy 0 errores.
