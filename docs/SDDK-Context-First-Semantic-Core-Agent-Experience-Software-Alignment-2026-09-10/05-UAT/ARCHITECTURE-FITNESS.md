# Architecture fitness suite

Minimum automated ratchets:

```text
no_alignment_to_governance_impl
no_alignment_to_instruction_compiler
no_domain_to_rpc_types
no_knowledge_to_provider_sdk
no_workbook_canonical_write
no_verify_full_scan_default
no_inferred_as_verified_without_evidence
no_provider_absence_as_pass
no_new_root_level_context_files_after_R0
```

Una nueva abstracción core sólo entra si el PR declara qué path/abstracción reemplaza, consolida o hace innecesaria, salvo ADR explícito de expansión.
