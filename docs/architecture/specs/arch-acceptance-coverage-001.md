---
id: arch-acceptance-coverage-001
status: proposed
proposed_at: 2026-09-20
related_specs:
  - arch-spec-021-intelligence-provider-boundary
  - arch-spec-021 IPB-004 (capability negotiation is runtime authority)
  - arch-spec-021 IPB-005 (requirement semantics)
related_adrs:
  - ADR-0139-STATIC-ENHANCED-COVERAGE-CONTRACT
cycle:
  - p-63676b11dc0ef88f/a6-cc-s1-static-graph-completeness
references:
  - tests/cycle-artifacts/p-63676b11dc0ef88f/a6-cc-s1-static-graph-completeness/SCOPE-CONTRACT.md
---

# Acceptance Contract — Coverage Evaluation (`STATIC_ENHANCED`)

This acceptance contract operationalises ADR-0139 against arch-spec-021 IPB-004.
It exists so that the obligation introduced by ADR-0139 ("a static enhanced
capability may be advertised only when an approved `CoverageContract` is
satisfied against a reproducible `CoverageBasis`") is normative and not
hidden in code or in an ADR alone.

## §1 Vocabulary (canonical, normative)

| Term | Definition |
|------|------------|
| `CoverageContract` | A versioned policy owned by SDDK, identified by `(contract_id, contract_version)`, bound to a single `consumer` and a single `scope`, declaring `required_capabilities[]`. |
| `CoverageBasis` | A reproducible anchor for an evaluation: `(revision, inventory, provider_strategy, provider_version, contract_revision)`. |
| `CoverageEvaluation` | The verdict over a contract applied to a basis with concrete provider observations: a triple `(inventory, semantics, operational)` of `DimensionValue`s plus `gaps[]`. |
| `DimensionValue` | One of `Demonstrated` / `Partial` / `Incomplete` / `Unknown`. |
| `CoverageVerdict` | Either `Satisfied` (all three dimensions `Demonstrated`) or `Incomplete { gaps: [...] }` (at least one dimension `< Demonstrated`). |
| `EvidenceGap { reason }` | The non-success state when the provider is unavailable; replaces `Satisfied` with explicit reason. |

## §2 Normative rules

### AR-1. Contract approval order

A `CoverageContract` is approved **before** the evaluation runs. The
`contract_revision` recorded in the `CoverageBasis` is the version that was
approved; no retroactive substitution is permitted.

### AR-2. Verdict determinism

Given the same `(contract_id, contract_version, contract_revision,
revision, provider_strategy, provider_version)` and the same provider
observations, `evaluate()` returns the same `CoverageVerdict` and the same
`gaps[]`. Two consecutive evaluations must report equal results.

### AR-3. Verdicts

- **`Satisfied`** iff all three dimensions are `Demonstrated`.
- **`Incomplete { gaps }`** if at least one dimension is `Partial`,
  `Incomplete`, or `Unknown`. `gaps[]` enumerates exactly what is missing
  (file path, semantic class, error kind, or "unknown: <dimension>").

### AR-4. Provider-unavailable state

When the provider lifecycle is `UNAVAILABLE` / `DORMANT` / `STARTING` /
`INCOMPATIBLE` / `FAILED` / `BUSY`-with-timeout, the evaluation does not
proceed. The consumer receives `EvidenceGap { reason: provider_unavailable }`.
This is **not** `Satisfied`. It is **not** `STATIC_ENHANCED=true`.

### AR-5. Strategy matching

A `lightweight` strategy is **not blanket-rejected**. It may satisfy a
contract whose `required_capabilities[]` are within its declared
`available_strategies[]` and `semantic_classes[]`. It may not satisfy a
contract that requires a semantic class it does not declare.

### AR-6. No global thresholds

There is no global constant of the form `coverage_ratio >= T`. Any number
published in an evaluation result is qualified by `CoverageContract.contract_id`
and `CoverageBasis.revision`. The runtime applies the contract approved; it
does not choose another threshold after observing the results.

### AR-7. Inventory independence

The `inventory` recorded in `CoverageBasis` is generated from a Git revision
pin and explicit inclusion/exclusion rules. It is not derived by `grep` over
text. The inventory command is deterministic and the inventory artefact is
durable (regenerable).

## §3 Mandatory tests (acceptance battery)

The cycle's `tests/a6_cc_s1_static_graph_completeness.rs` MUST include, at
minimum:

- **T-AR-1 (Satisfied path).** Given a contract with `required_capabilities`
  that the fake provider's `CapabilitySnapshot` declares, the evaluation is
  `Satisfied` and `gaps[]` is empty.
- **T-AR-2 (Incomplete path).** Given a contract that requires a file the
  fake provider omits, the evaluation is `Incomplete` and `gaps[]` contains
  the omitted file path.
- **T-AR-3 (Unknown strategy).** Given a contract whose
  `required_capabilities` include a semantic class the fake provider does
  not declare, the evaluation is `Incomplete` with `gaps[]` containing the
  semantic class.
- **T-AR-4 (Provider unavailable).** Given `ProviderLifecycle::Unavailable`,
  `evaluate()` returns `EvidenceGap { reason: provider_unavailable }`.
- **T-AR-5 (Determinism).** Two consecutive calls with identical inputs
  return byte-equal `CoverageEvaluation` (modulo `timestamp`).
- **T-AR-6 (EXT, env-gated).** With `COGNICODE_MCP_BIN` set, an evaluation
  runs against the real CogniCode MCP and reports a `CoverageBasis` whose
  `provider_strategy` and `provider_version` match the handshake. Without
  the env var, the test is `#[ignore]`-marked and listed in the receipt's
  scope-limited gate.

## §4 Forbidden constructs (enforced by lint)

The cycle MUST NOT introduce any of the following; `cargo clippy
--workspace --all-targets -- -D warnings` and the
`no_knowledge_to_provider_sdk` context-fitness lint enforce this in
practice:

- A `coverage_ratio: f32` field on `CapabilitySnapshot` or
  `CodeIntelligencePort`.
- A constant of the form `const COVERAGE_THRESHOLD: f32 = 0.9;`.
- A type with the name `CogniCode*` in `sddk-domain`.
- A provider wire DTO in `crates/sddk-engine/src/semantic_*.rs` or
  `evidence_ref.rs` or `vault_boundary.rs` or `why_queries.rs`.

## §5 Linkage

This contract is bound to:

- `arch-spec-021 IPB-004` — capability negotiation is runtime authority.
- `arch-spec-021 IPB-005` — requirement semantics (OPTIONAL/PREFERRED/
  REQUIRED).
- `arch-spec-021 IPB-002` — provider is evidence source, not truth authority.
- `ADR-0139-STATIC-ENHANCED-COVERAGE-CONTRACT` — architectural decision.

A change to this contract requires a new ADR; an amendment to
`arch-spec-021` requires an ADR citing this contract as a dependent.

## §6 Status

This acceptance contract is `proposed` until `cross-acceptance-coverage-001`
or equivalent test suite from a downstream consumer passes. The cycle
`p-63676b11dc0ef88f/a6-cc-s1-static-graph-completeness` is the first
consumer; its `RECEIPT.md` records the activation.