# DISCOVERY — CogniCode v0.97.1 Coverage Information (CC-S1)

Date: 2026-09-20
Provider: `cognicode-mcp` v0.97.1
AIW-S1 reference: `tests/cycle-artifacts/p-63676b11dc0ef88f/aiw-s1-cognicode-real/DISCOVERY.md` (predecessor)
CC-S0 reference: `docs/architecture/a6/A6-COGNICODE-CC-S0-RECEIPT.md` (seam)

## §1 What CogniCode actually reports (OBSERVED, against this repo)

From the AIW-S1 discovery's handshake table and the CC-S0 falsification battery:

### Inventory

- `build_graph {strategy: lightweight}` → `{"success":true,"symbols_found":41607,"relationships_found":36548,...}` (~35 s, cold graph).
- `build_graph {strategy: full}` → OK (precondition for graph tools).

These two numbers are the **only** cardinality information CogniCode returns
at the workspace level. There is no per-file count, no per-symbol
classification, and no per-relationship-kind breakdown in the
`build_graph` response surface observed by the AIW-S1 discovery.

### Semantics (capabilities)

- Symbol kinds: not enumerated in `build_graph`; `get_file_symbols` returns
  per-file lists but we did not observe a top-level taxonomy.
- Relationship kinds: not enumerated globally; `find_usages`,
  `analyze_impact`, `get_call_hierarchy`, `trace_path`, `build_call_subgraph`
  each return results, but the *kind* of relationship returned is implicit in
  the tool name, not a typed field in the response.
- Languages: CogniCode targets Rust on this repo (its supported set is not
  declared in the handshake).

### Operational completeness

- Latency: ~35 s for `build_graph {strategy: lightweight}` cold; warm
  graphs are faster (not measured here).
- Errors: `build_graph` returns `success:true`; no error/cancellation field
  is reported.
- Truncation: not observed; `find_usages` paginated correctly.
- Limits: not exposed in handshake; an upper bound on graph size is
  unknown.

## §2 What is NOT in CogniCode's response (gaps)

Three information dimensions that CC-S1's evaluation needs are **absent**
from CogniCode's wire surface:

1. **Per-file / per-symbol coverage.** No `analysed_files[]` or
   `expected_files[]`. No `coverage_ratio`.
2. **Semantic-class taxonomy.** No top-level enumeration of symbol kinds or
   relationship kinds the provider can represent. We cannot know in
   advance whether `lightweight` covers `macros` or `derive()` blocks.
4. **Truncation/exclusion disclosure.** No `excluded_paths[]`, no
   `truncated: bool`, no `errors[]` per file.

Consequence: any `CoverageEvaluation` against CogniCode's wire surface must
mark those three dimensions `Unknown` for now. SDDK does not invent them
(M6 of the SCOPE-CONTRACT).

## §3 Inventory independence (M1 of SCOPE-CONTRACT)

The cycle's `tests/fixtures/static_enhanced/inventory_v1.json` is generated
**independently** of CogniCode, by enumerating the repository tree at the
pinned revision and applying explicit inclusion/exclusion rules:

- Pin: `git rev-parse HEAD` at the close of this cycle.
- Include: `crates/*/src/**/*.rs`, `crates/*/tests/**/*.rs`,
  `tests/**/*.rs` (only Rust sources under `crates/` and the root
  `tests/` tree).
- Exclude: `target/**`, `.git/**`, `**/mod.rs` (counted via `lib.rs` only
  when the parent crate declares `pub mod`), `**/tests/fixtures/**`
  (fixtures are not analysed units).
- Exclude paths (explicit list): any path listed in `exclude_paths[]` of
  the inventory command (e.g. `crates/sddk-engine/tests/fixtures/`).

The inventory command is `just inventory-static-enhanced` (or equivalent
deterministic command). It emits:

```json
{
    "revision": "<pinned SHA>",
    "rule_version": "1.0.0",
    "expected_files": ["crates/sddk-cli/src/main.rs", ...],
    "exclude_patterns": ["target/**", ".git/**", "**/tests/fixtures/**"],
    "exclude_paths": ["crates/sddk-engine/tests/fixtures/"],
    "generated_at": "2026-09-20T..."
}
```

The inventory is reproducible: the same `(revision, rule_version)` produces
byte-identical `expected_files[]`. `rule_version` is bumped on rule changes;
each bump is a contract change.

## §4 The contract shape (derived from M2 of SCOPE-CONTRACT)

For CC-S1's closure, the `CoverageContract` is:

```text
contract_id    = "static-enhanced-workspace-v1"
contract_version = 1.0.0
consumer       = "verify-kernel::static_evidence"
scope = ScopeRef {
    repo:    "sddk-framework",
    revision: "<HEAD of CC-S1 close>",
    include_globs: ["crates/**/src/**/*.rs", "crates/**/tests/**/*.rs",
                    "tests/**/*.rs"],
    exclude_globs: ["target/**", ".git/**", "**/tests/fixtures/**"],
}
required_capabilities = [
    RequiredCapability { kind: SymbolKind, name: "fn" },
    RequiredCapability { kind: SymbolKind, name: "struct" },
    RequiredCapability { kind: SymbolKind, name: "trait" },
    RequiredCapability { kind: RelationClass, name: "find_usages" },
    RequiredCapability { kind: RelationClass, name: "analyze_impact" },
]
```

The provider MUST declare all five capabilities in its `CapabilitySnapshot`
for the contract to be considered. CogniCode v0.97.1's handshake does
**not** enumerate capabilities in a machine-readable way; the adapter
(via `capabilities()`) has to surface them from the tool catalog. This is
a known adapter gap, listed as `Unknown` for the semantics dimension of
the dimension evaluation if the adapter can't answer.

## §5 What CC-S1 declares and does NOT declare

**Declares:** the contract of coverage evaluation. The `evaluate()` function
in `crates/sddk-engine/src/code_intelligence_port.rs` exists and returns
`CoverageEvaluation` per the rules in `arch-acceptance-coverage-001`.

**Does NOT declare:** `STATIC_ENHANCED=true` for any consumer, anywhere.
That requires a separate `CoverageContract` with `consumer = "..."`, an
execution against the real provider at the pinned revision, and a
`Satisfied` verdict. CC-S1 produces the scaffolding for that future
declaration; it does not perform it.

## §6 Limits of this discovery

- The handshake response shape is what AIW-S1 captured on 2026-09-19. We
  have not re-executed the handshake against a fresh `cognicode-mcp`
  instance today. If the wire surface has changed (e.g. a new field), the
  adapter's `capabilities()` and `coverage_evaluation()` implementations
  will need to be updated and the cycle reopened.
- The inventory command has not been implemented yet — it is part of
  CC-S1's code deliverables (M1). This DISCOVERY documents the *target*
  shape; the `inventory_v1.json` artefact is generated by the cycle's
  implementation.

## §7 References

- AIW-S1 DISCOVERY (`tests/cycle-artifacts/.../aiw-s1-cognicode-real/DISCOVERY.md`) — handshake, tool catalog, latency.
- CC-S0 RECEIPT (`docs/architecture/a6/A6-COGNICODE-CC-S0-RECEIPT.md`) — falsification matrix T1..T6.
- arch-spec-021 IPB-002, IPB-004, IPB-005.
- ADR-0139-STATIC-ENHANCED-COVERAGE-CONTRACT.
- arch-acceptance-coverage-001.