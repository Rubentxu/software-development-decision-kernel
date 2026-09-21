# Conformance Fitness Ratchets

These rules become blocking during the closeout. Names are illustrative; implementation may use Rust tests, cargo metadata scans, Semgrep/tree-sitter rules or dedicated architecture tests.

```text
conf09_one_event_append_authority
conf09_no_legacy_event_writes
conf09_universal_evidence_only
conf09_no_planning_evidence_new_writes
conf09_no_runtime_cycle_truth
conf09_one_generic_revision_primitive_set
conf09_all_governed_effects_use_authority_engine
conf09_no_legacy_authority_new_consumers
conf09_command_contract_single_source
conf09_agent_assets_no_deprecated_semantics
conf09_semantic_graph_only_authoritative_graph
conf09_active_graph_projection_only
conf09_every_projection_has_rebuild_contract
conf09_every_storage_model_has_state_class
conf09_no_deprecated_production_path
conf09_docs_status_matches_delivery
conf09_uat_01_22_traceable
```

## Allowlist policy

Any temporary allowlist entry MUST include:

```text
symbol/path
reason
canonical_replacement
read_or_write
owner
removal_trigger
expiry/version
parity_test
```

Write-capable legacy entries are forbidden at C7.

## Mutation tests

At least these intentional regressions SHOULD be injected in CI or a dedicated architecture test suite:

- add second EventStore authority -> fail;
- construct deprecated planning evidence in production module -> fail;
- add `CycleStatus::ApprovalPending` write -> fail;
- bypass AuthorityEngine from effect adapter -> fail;
- change CLI option without command registry update -> fail;
- add deprecated command to active agent prompt -> fail.

A fitness rule is not trusted until at least one fixture proves it fails for the prohibited change.
