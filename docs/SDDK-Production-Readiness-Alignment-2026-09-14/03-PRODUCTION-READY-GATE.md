# Production-Ready Gate

This document defines what must be true before a release or commit may carry an SDDK production-readiness declaration.

## 1. Principle

A readiness label is a **receipt over evidence**, not a roadmap status and not a test-count claim.

Every receipt MUST bind to:

- SDDK commit SHA and version;
- readiness profile (`BASE`, `STATIC_ENHANCED`, `RUNTIME_ENHANCED`, `FULLY_ENHANCED`);
- configuration/policy schema versions;
- migration/schema version;
- command-registry digest;
- architecture/spec set or manifest digest;
- exact verification commands and CI/run references;
- unresolved accepted risks and their revisit triggers;
- provider compatibility basis when the profile uses providers.

## 2. Gate G0 — Baseline semantic conformance

Required for every profile.

- 09/09 C7 conformance receipt exists and is green.
- SPEC-001..018 are `PASS` or `PASS_WITH_COMPAT`.
- UAT-01..22 are executable and green.
- one canonical event append authority;
- one universal Evidence write model;
- Cycle is not runtime execution truth;
- one generic revision/CAS/ref implementation per invariant;
- one AuthorityEngine decision path for governed effects;
- no reachable legacy bypass that can mutate canonical state.

A compatibility exception requires:

```text
compatibility_id
scope
read_only_or_write_behavior
owner
fixture
detection_guard
removal_trigger
latest_review_date
```

Writable compatibility that duplicates canonical authority is not eligible for `PASS_WITH_COMPAT`.

## 3. Gate G1 — Ownership and dependency integrity

Required for Base and above.

- R0 bounded-context ownership is physically represented or an ADR proves an equivalent boundary.
- Dependency fitness checks block forbidden edges.
- Domain types contain no protobuf/gRPC/provider SDK types.
- Knowledge and Alignment depend on semantic ports, not CogniCode/Chronos implementations.
- Alignment cannot call Governance/Authority implementation paths.
- Alignment cannot compile into EffectiveInstructions.
- Workbooks/projections cannot write canonical facts directly.
- storage schema migrations have one explicit owner.

## 4. Gate G2 — Knowledge and epistemic integrity

Required for Base and above.

- `KnowledgeAssertion` status is explicit.
- deterministic observations are distinguishable from inference.
- `INFERRED` is never silently upgraded to `OBSERVED` or `VERIFIED`.
- `DECIDED` requires DecisionRef provenance.
- `VERIFIED` requires verifier/reproducible evidence.
- stale/contradicted/superseded knowledge remains queryable with provenance.
- KnowledgeBasis/KMT invalidation is deterministic for the tested dimensions.
- SemanticGraph is rebuildable; it is not a second authority store.

## 5. Gate G3 — Alignment boundary

Required for Base and above.

- Software Alignment observes/compares/explains/suggests; it never orders.
- project Architectural Intent is explicit and versioned.
- selected lens set is recorded.
- missing evidence yields `UNKNOWN` / `NOT_APPLICABLE` as appropriate.
- no universal architecture-quality score exists.
- accepted tradeoffs/debt preserve DecisionRef and revisit semantics.
- advisory context changes the ContextCapsule hash but never the EffectiveInstructionSet hash.

## 6. Gate G4 — Verify and DebVerify

Required for Base and above.

### Verify

- default scope is change/delta-driven;
- localized change does not cause unconditional full-repository scan;
- staleness/impact and EvidenceGap are explicit;
- optional deepening is risk/uncertainty driven;
- missing optional intelligence does not fabricate PASS.

### DebVerify

- independently questions the accumulated baseline;
- can discover debt/contradiction outside the current Git delta;
- records ProjectKnowledgeBaselineReceipt/ReconciliationSummary equivalents;
- is not implemented as an alias for `verify --full`.

## 7. Gate G5 — Authority, safety and failure behavior

Required for every profile.

- governed side effects cannot bypass AuthorityEngine;
- approval/deny/require-approval outcomes are deterministic from the same proposal/actor/facts/policy snapshot;
- crash/restart does not duplicate an already committed canonical fact or silently lose an acknowledged one;
- CAS/ref races are covered by tests;
- invalid/corrupt/incompatible state fails explicitly;
- provider failure never gives provider code canonical write authority;
- sensitive material is not written to receipts/logs/evidence without an explicit redaction policy.

## 8. Gate G6 — Operational recovery and compatibility

Required for every profile.

At minimum test:

1. fresh repository;
2. migrated repository from supported prior schema/version;
3. projection deletion and rebuild;
4. process crash/reopen around append/CAS boundaries;
5. installed binary rather than only `cargo run`;
6. config precedence and explainability;
7. unsupported future schema/protocol behavior;
8. deterministic replay of representative receipts.

Compatibility policy MUST state supported upgrade paths. “Works on current fixtures” without a migration contract is insufficient.

## 9. Gate G7 — Provider-specific readiness

Only for enhanced profiles.

### Common

- capability negotiation is runtime truth;
- version string is metadata, not authority for capabilities;
- provider lifecycle distinguishes at least unavailable/dormant/starting/ready/busy/incompatible/failed or equivalent;
- protocol major incompatibility is explicit;
- request timeout/cancellation is defined;
- stable refs/digests allow SDDK to persist provenance without copying provider-heavy datasets;
- provider process may restart independently of SDDK;
- provider absence semantics follow requirement level `OPTIONAL | PREFERRED | REQUIRED`.

### Static Enhanced

Must additionally pass CogniCode contract tests and end-to-end Verify/Alignment fixtures.

### Runtime Enhanced

Must additionally pass Chronos scenario/runtime contract tests, resource-bound tests and golden scenarios.

### Fully Enhanced

Must additionally prove static/runtime contradiction preservation and provider-independent degradation.

## 10. Gate G8 — Documentation and release hygiene

Required for every profile.

- one normative architecture entry point;
- every active specification has status/owner/evidence linkage;
- historical/superseded packages are clearly marked;
- proposal disposition register contains no important accepted proposal silently missing from implementation or an explicit defer/reject/supersede decision;
- CLI command documentation/examples derive from the typed command surface;
- release notes name the readiness profile actually certified.

## 11. Receipt template

A release/certification receipt SHOULD use this shape:

```yaml
schema_version: 1
sddk:
  version: "..."
  commit: "..."
profile: BASE | STATIC_ENHANCED | RUNTIME_ENHANCED | FULLY_ENHANCED
spec_manifest: "sha256:..."
command_registry: "sha256:..."
storage_schema: "..."
policy_basis: "sha256:..."
providers:
  cognicode: null
  chronos: null
gates:
  G0: PASS
  G1: PASS
  G2: PASS
  G3: PASS
  G4: PASS
  G5: PASS
  G6: PASS
  G7: NOT_APPLICABLE
  G8: PASS
accepted_risks: []
evidence:
  - command: "..."
    result_ref: "..."
```

For enhanced profiles, provider entries additionally record protocol version, provider build identity, capability snapshot digest and analyzer/instrumentation basis.

## 12. Release claim rules

- `BASE_PRODUCTION_READY` requires G0–G6 and G8.
- `STATIC_ENHANCED_PRODUCTION_READY` requires Base + CogniCode part of G7.
- `RUNTIME_ENHANCED_PRODUCTION_READY` requires Base + Chronos part of G7.
- `FULLY_ENHANCED_PRODUCTION_READY` requires all gates including cross-provider G7.

R9 control-tower polish, R11 crate splitting and Agentic Workspace/JCode are not permitted to delay a truthful Base production-ready claim unless a later ADR explicitly promotes one of them to a required gate.
