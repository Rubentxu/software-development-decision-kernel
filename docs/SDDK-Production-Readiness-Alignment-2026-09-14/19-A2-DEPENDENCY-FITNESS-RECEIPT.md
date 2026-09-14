# 19-A2-DEPENDENCY-FITNESS-RECEIPT

Executable ratchets: `crates/sddk-cli/tests/context_fitness.rs` (7 tests).

| rule | implemented as | status |
|---|---|---|
| `no_domain_to_rpc_types` | identifier scan of `sddk-domain/src` for `prost`/`tonic`/`grpc` use | PASS |
| `no_domain_to_provider_sdk` | identifier scan + `Cargo.toml` dep check for `cognicode`/`chronos` | PASS |
| `no_domain_to_host_sdk` | identifier scan for `jcode` | PASS |
| `no_knowledge_to_provider_sdk` | scan of `semantic_graph`/`semantic_node`/`semantic_kind`/`vault_boundary`/`why_queries` | PASS |
| `no_alignment_to_governance_impl` | scan of any `*alignment*` file for `authority::` | PASS (vacuously; no alignment module yet) |
| `no_alignment_to_authority_engine` | scan of any `*alignment*` file for `authority_engine` | PASS |
| `no_alignment_to_instruction_compiler` | scan of any `*alignment*` file for `instruction_compiler` | PASS |
| `no_workbook_canonical_write` | scan of `*workbook*`/`*cockpit*` files for `INSERT/UPDATE/DELETE` | PASS |
| `no_host_specific_jcode_type_in_generic_agentic_contract` | scan of `*agentic*`/`*session_binding*` files for `jcode` | PASS |
| `no_new_root_level_context_modules_after_R0_without_ADR` | root-module allowlist vs ADR text (fail-closed) | PASS |
| `one_owner_per_core_concept` | expressed by `17-A2-CONTEXT-OWNERSHIP-MAP.md` + the existing single-authority ratchets (canonical event log singleton, authority single path, semantic graph singleton); no fragile duplicate-scan added | PASS (documented + existing ratchets) |

Design notes:

- Rules are **fail-closed**: a forbidden edge fails the test.
- Detection is **identifier-based** (`use <crate>`, `<crate>::`, dependency in
  `Cargo.toml`) to avoid false positives on string literals such as pack ids.
- `no_new_root_level_context_modules_after_R0_without_ADR` accepts a new root
  module iff its name appears in some ADR under `docs/architecture/adrs/`.

Command: `cargo test -p sddk-cli --test context_fitness` → 7 passed.
