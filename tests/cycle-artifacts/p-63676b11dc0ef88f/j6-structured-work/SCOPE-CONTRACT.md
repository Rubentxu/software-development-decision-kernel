# SCOPE-CONTRACT — j6-structured-work (arch-spec-027, J6)

| Req | Test | Estado |
|---|---|---|
| SAW-001 typed request host-agnostic | saw001_002_typed_request_schema_result | ✅ |
| SAW-002 schema-constrained ContributionV2 | saw001_002_typed_request_schema_result | ✅ |
| SAW-003 output inválido distinguible, sin success fabricado | saw003_invalid_output_visible_not_fabricated | ✅ |
| SAW-004 mismo adapter companion/orchestrated | saw004_same_adapter_two_modes | ✅ |
| SAW-005 contribution no es authority | saw005_contribution_not_authority | ✅ |
| SAW-006 AgentExecutionReceipt con provenance | saw006_receipt_provenance | ✅ |

NUEVO `crates/sddk-engine/src/structured_work.rs`: AgentWorkRequest,
ReturnSchema, ContributionV2, StructuredRunOutcome, RawHostOutput,
StructuredWorkExecutor (run_structured), AgentExecutionReceipt.
Out of scope: crates sdk/api (arch-spec-030), wiring con hosts
reales, AW-UAT-060..063.

Evidence: 5 PASS, clippy 0.
