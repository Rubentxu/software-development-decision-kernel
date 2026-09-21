# Chronos Handoff — Runtime Intelligence Provider

## 1. Goal

Chronos enhances SDDK with runtime/behavioral evidence. It does **not** become SDDK's runtime authority, verification truth, decision memory or policy engine.

```text
SDDK Verify / Knowledge / Alignment
             |
   RuntimeIntelligencePort        <- SDDK-owned semantic port
             |
        SDDK adapter              <- maps SDDK ADTs <-> provider wire DTOs
             |
     versioned RPC/local IPC
             |
           Chronos                <- owns instrumentation, traces, scenarios, analysis
```

Runtime evidence is especially valuable when static evidence is insufficient or contradicted. It is not mandatory for every Verify.

## 2. Ownership boundary

### SDDK owns

- `RuntimeIntelligencePort` semantic operations and ADTs;
- decision about when runtime deepening is warranted;
- KnowledgeAssertion/KnowledgeBasis/KMT semantics;
- Evidence normalization and provenance;
- Verify/DebVerify orchestration;
- Software Alignment interpretation;
- policy/authority and capability requirement level;
- cross-provider contradiction handling.

### Chronos owns

- runtime instrumentation/collection mechanisms;
- scenario execution/observation implementation;
- eBPF/OpenTelemetry/SDK/compile-time strategies where applicable;
- raw traces, event streams, timing data and provider-heavy datasets;
- runtime analysis algorithms/fingerprints;
- provider-side retention/cache;
- versioned provider protocol and service runtime.

### Forbidden ownership inversion

Chronos MUST NOT:

- write SDDK canonical facts/Decision Memory directly;
- decide an Alignment verdict or Governance gate;
- turn a transient trace into canonical truth without SDDK provenance/status handling;
- force one instrumentation technology into the SDDK domain model;
- require SDDK to persist full raw traces merely to preserve evidence provenance.

## 3. Minimal semantic capability surface

Final wire names belong to Chronos. The SDDK adapter must be able to implement these semantic operations without parsing CLI/log prose:

```text
capabilities()
observe_scenario(basis, scenario, options)
compare_scenario(basis, left_ref, right_ref, options)
behavior_summary(basis, evidence_ref, options)
```

If controlled execution becomes a stable Chronos responsibility, `run_scenario(...)` may be separated from `observe_scenario(...)`; do not split them merely for symmetry before the execution/observation boundary is demonstrated.

A result should be capable of carrying:

```text
scenario/basis identity
provider build + protocol version
capability snapshot digest
instrumentation/analyzer basis
runtime environment fingerprint
stable evidence/trace references + digests
behavior fingerprint
observations/anomalies
partial/incomplete marker
clock/timing limitations where relevant
resource/cancellation outcome
```

Raw runtime observations may become SDDK `OBSERVED` only with recorded scenario + instrumentation basis. Interpretive conclusions remain `INFERRED` unless independently verified.

## 4. Provider lifecycle and execution safety

Chronos must expose explicit lifecycle/capability behavior equivalent to:

```text
UNAVAILABLE
DORMANT
STARTING
READY
BUSY
INCOMPATIBLE
FAILED
```

Additionally runtime work requires:

- request deadline/timeout;
- cancellation;
- bounded CPU/memory/disk/trace volume or an explicit resource policy;
- scenario process cleanup on cancellation/failure;
- stable correlation IDs;
- provider restart/reconnect behavior;
- clear handling of partial traces;
- clock/source limitations recorded in evidence where temporal claims depend on them.

A failed or cancelled scenario MUST NOT result in a green Verify/Alignment receipt unless the requested property can legitimately be established from other independent evidence.

## 5. Work required in Chronos

Suggested internal milestones; names are candidates, not mandates.

### CH-S0 — External protocol spike

Prove an external Rust client can:

- negotiate protocol/capabilities;
- start/observe one controlled fixture scenario;
- receive typed stable result refs/fingerprint;
- cancel/timeout;
- reconnect after provider restart.

### CH-S1 — Stable runtime protocol

Create a versioned public boundary (`chronos-protocol` or equivalent) isolated from internal trace/store types.

### CH-S2 — Scenario service

Expose the agreed observe/run/compare/summary operations over an on-demand service (`chronos-rpc`/daemon or equivalent). Raw traces stay provider-side by default.

### CH-S3 — ScenarioBasis and reproducibility

Define a `ScenarioBasis`-equivalent wire model including:

- workspace/revision identity;
- executable/build identity;
- scenario definition/version;
- environment fingerprint;
- instrumentation/analyzer set;
- provider build/protocol;
- time/resource policy where relevant.

### CH-S4 — Golden scenarios

Provide deterministic or tolerance-bounded fixtures for at least:

- expected call/behavior path;
- unexpected branch/path;
- latency regression/timing comparison where valid;
- exception/error path;
- concurrent execution fixture;
- partial instrumentation;
- timeout;
- cancellation;
- incompatible protocol;
- provider restart/cache recovery.

Where strict determinism is impossible, expected tolerances and nondeterminism sources must be explicit rather than hidden by unstable snapshots.

### CH-S5 — Integration release contract

Tag/publish a build usable by SDDK integration tests and document protocol compatibility independently of Chronos product SemVer.

## 6. Work required in SDDK

1. finalize `RuntimeIntelligencePort` SDDK ADTs;
2. use the same Extension Platform registry/lifecycle concepts as static intelligence;
3. implement the Chronos adapter outside domain/Knowledge/Alignment;
4. normalize Chronos refs/fingerprints to Evidence + KnowledgeAssertions;
5. invoke Chronos only when Verify/DebVerify/Alignment deepening warrants it or when policy/task explicitly requires it;
6. keep Base and Static Enhanced modes valid with Chronos absent;
7. reconcile static/runtime contradiction without choosing a provider as universal truth;
8. produce runtime-enhanced and fully-enhanced receipts.

## 7. Documents to transfer to Chronos

Do **not** copy the complete SDDK context-first ZIP into Chronos as normative material.

Create:

```text
docs/integrations/sddk/
  README.md
  CONTRACT.md
  COMPATIBILITY.md
  UAT.md
  SOURCE-MANIFEST.md
  reference/
```

The handoff source set should contain snapshots of exactly these SDDK documents:

### Common provider contract

1. `02-BOUNDED-CONTEXTS/extension-platform/INTELLIGENCE-PROVIDER-PORTS.md`
2. `02-BOUNDED-CONTEXTS/extension-platform/specs/SPEC-030-INTELLIGENCE-PROVIDER-SPI.md`
3. `02-BOUNDED-CONTEXTS/extension-platform/specs/SPEC-031-ENHANCED-MODES-AND-CAPABILITY-NEGOTIATION.md`
4. `02-BOUNDED-CONTEXTS/extension-platform/specs/SPEC-035-PROVIDER-RPC-LIFECYCLE.md`
5. `docs/architecture/specs/arch-spec-021-intelligence-provider-boundary.md`

### Runtime/knowledge/verification semantics Chronos must respect

6. `02-BOUNDED-CONTEXTS/knowledge/specs/SPEC-021-PROGRESSIVE-KNOWLEDGE.md`
7. `02-BOUNDED-CONTEXTS/knowledge/specs/SPEC-032-DRIFT-STALENESS-CONTRADICTIONS.md`
8. `02-BOUNDED-CONTEXTS/verification/specs/SPEC-027-VERIFY-DELTA-INTELLIGENCE.md`
9. `02-BOUNDED-CONTEXTS/verification/specs/SPEC-028-DEBVERIFY-GLOBAL-RECONCILIATION.md`
10. `02-BOUNDED-CONTEXTS/software-alignment/specs/SPEC-034-STATIC-RUNTIME-ALIGNMENT.md`

### Provider-specific handoff

11. this file, `05-CHRONOS-HANDOFF.md`.
12. provider-specific rows from `07-UAT-EVIDENCE-MATRIX.md`.

Every copied snapshot MUST record SDDK source commit and original path.

### Recommended ZIP

When CH-S0 starts, generate:

`SDDK-Chronos-Runtime-Intelligence-Handoff-<date>.zip`

containing only the 12 items above plus `SOURCE-MANIFEST.sha256`.

The broad existing SDDK/CogniCode/Chronos bundle remains archival/reference material in SDDK, not Chronos's mutable working contract.

## 8. Runtime Enhanced UAT

`RUNTIME_ENHANCED_PRODUCTION_READY` requires:

1. Base passes with Chronos stopped/uninstalled.
2. OPTIONAL/PREFERRED absence produces EvidenceGap/NOT_EVALUATED, not PASS.
3. REQUIRED absence fails the requested operation explicitly.
4. Capability negotiation succeeds against the pinned build.
5. Scenario basis includes revision/build/environment/instrumentation provenance.
6. Stable trace/evidence refs are persisted instead of compulsory raw trace duplication into SDDK.
7. timeout/cancellation leaves explicit incomplete/cancelled evidence, not false success.
8. provider restart/reconnect works.
9. incompatible protocol is explicit.
10. golden scenario results are stable within declared deterministic/tolerance contracts.
11. a runtime observation can contradict static/declared knowledge and the contradiction survives reconciliation.
12. resource bounds prevent an unbounded trace/scenario from destabilizing SDDK.
13. no runtime provider operation can bypass AuthorityEngine for an SDDK-governed side effect.

## 9. Fully Enhanced cross-provider UAT

With CogniCode and Chronos together:

- static evidence and runtime evidence are independently attributable;
- corroboration increases evidence diversity without changing source identities;
- contradiction creates a reconciliation need, not a forced average score;
- stopping either provider degrades only the capabilities that depended on it;
- `FULLY_ENHANCED` is advertised only while negotiated capabilities actually support it.

## 10. Compatibility receipt

Record:

```text
SDDK version + commit
RuntimeIntelligencePort schema/version
Chronos protocol major/minor
Chronos build/tag + commit
capability snapshot digest
instrumentation/analyzer basis digest
scenario basis/version
integration adapter version
```

Runtime capability negotiation remains authoritative.
