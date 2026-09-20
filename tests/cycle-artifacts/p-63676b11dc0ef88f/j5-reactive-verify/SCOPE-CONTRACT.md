# SCOPE-CONTRACT — j5-reactive-verify (arch-spec-025 + arch-spec-037, AC9)

| Req | Test | Estado |
|---|---|---|
| RHB-002 materiality (reads ephemeral) | rhb003_004_coalescing_and_ephemeral | ✅ |
| RHB-003 no canonical flood / AC-037-002 | ac037002_reads_never_become_facts | ✅ |
| RHB-004 semantic debounce con turn_done | rhb003_004_coalescing_and_ephemeral | ✅ |
| RHB-005/AC9 delta-scoped changed-unit loop | ac9_changed_unit_contract_loop | ✅ |
| RHB-006 provider ausente = EvidenceGap | rhb006_provider_absent_is_gap_not_failure | ✅ |
| RHB-008 violation = ATTENTION, nunca INTERRUPT | rhb008_violation_attention_not_interrupt | ✅ |
| AC-037-004 KMT affected units | ac9_changed_unit_contract_loop (via KmtIndex) | ✅ |
| AC-037-005 delta genérico (no host types) | ArchitectureConformanceDelta serializable | ✅ |

NUEVO `crates/sddk-engine/src/reactive_verify.rs`: HostEvent,
ChangeSetCoalescer, WorkspaceChangeSet, KmtIndex,
ArchitecturalContractRef, run_reactive_verify. Out of scope: wiring
a CLI/adapter real, AW-UAT-040..053, provider deepening real.

Evidence: 5 PASS, clippy 0.
