# CogniCode Handoff — Static Intelligence Provider

## 1. Goal

CogniCode enhances SDDK with static/code intelligence. It does **not** become SDDK's knowledge database, architecture judge, decision memory or canonical graph authority.

```text
SDDK Verify / Knowledge / Alignment
             |
     CodeIntelligencePort       <- SDDK-owned semantic port
             |
        SDDK adapter            <- maps SDDK ADTs <-> provider wire DTOs
             |
     versioned RPC/local IPC
             |
          CogniCode             <- owns analysis engine + heavy graph/index data
```

No LLM intermediary is required between SDDK and CogniCode.

## 2. Ownership boundary

### SDDK owns

- `CodeIntelligencePort` semantic operations and ADTs;
- `KnowledgeAssertion`, KnowledgeBasis/KMT and knowledge status semantics;
- Evidence relation/provenance normalized into SDDK;
- Verify/Deepening decisions;
- Software Alignment interpretation;
- policy/authority;
- provider requirement level (`OPTIONAL | PREFERRED | REQUIRED`);
- capability-profile composition (`BASE`, `STATIC_ENHANCED`, ...).

### CogniCode owns

- parsing/indexing/static-analysis implementation;
- its internal graph/index/database/cache;
- language/analyzer support;
- provider-side result lifecycle and stable references;
- provider protocol DTOs and protocol server;
- analysis performance/caching;
- analyzer-specific diagnostics and raw facts.

### Forbidden ownership inversion

CogniCode MUST NOT:

- write SDDK canonical events/Decision Memory directly;
- return `ALIGNED/MISALIGNED` as canonical architecture truth;
- decide SDDK AuthorityEngine outcomes;
- expose its database/AST types as SDDK domain types;
- require SDDK to copy complete provider graphs into SemanticGraph.

## 3. Minimal semantic capability surface

The final wire names belong to CogniCode, but it MUST be possible for the SDDK adapter to implement these semantic operations without scraping CLI text:

```text
capabilities()
analyze_delta(basis, delta, options)
analyze_scope(basis, scope, options)
analyze_impact(basis, changed_units, options)
unit_summary(basis, unit_ref, depth?)        # if required by D0-D3 deepening
graph_slice(basis, query, budget)            # if required by D2 deepening
```

`analyze_delta`, `analyze_scope`, `analyze_impact` are the mandatory core. `unit_summary` and `graph_slice` should be implemented when the R5 deepening fixtures demonstrate value; they must not be invented merely for API symmetry.

A provider response should be capable of carrying:

```text
request/basis identity
provider build + protocol version
capability snapshot digest
analyzer set/version digest
source/revision basis
observations/diagnostics
stable result refs/digests
confidence/limitations where applicable
partial/incomplete marker
provider timings/cost metadata if useful
```

Raw provider observations map to SDDK `OBSERVED` only when deterministically produced by an identified analyzer over a recorded basis. Model/heuristic inference remains `INFERRED` unless independently verified.

## 4. Provider lifecycle contract

Support explicit states or equivalent observable semantics:

```text
UNAVAILABLE
DORMANT
STARTING
READY
BUSY
INCOMPATIBLE
FAILED
```

Required behavior:

- on-demand startup is allowed;
- UDS/local socket/stdio child are transport details, not domain concepts;
- protocol major incompatibility is explicit;
- cancellation/deadline is supported for long analysis;
- process restart does not invalidate already persisted SDDK Evidence refs without a declared retention/expiry rule;
- capability negotiation is runtime truth.

## 5. Work required in CogniCode

Suggested internal milestones; crate names are candidates, not mandates.

### CC-S0 — Protocol spike

Prove an external Rust client can:

- connect without importing CogniCode internals;
- negotiate protocol + capabilities;
- request delta analysis;
- receive stable typed results;
- cancel/timeout;
- reconnect after provider restart.

### CC-S1 — Stable provider boundary

Create a versioned public protocol boundary (`cognicode-protocol` or equivalent) and isolate it from analyzer internals. Unknown additive fields/events must be forward-tolerant within the negotiated protocol rules.

### CC-S2 — Service runtime

Provide an on-demand service (`cognicode-rpc`/daemon or equivalent) supporting the agreed core operations. Heavy indexes remain provider-side.

### CC-S3 — Basis and reproducibility

Define `AnalysisBasis`-equivalent wire data covering repository/workspace identity, source revision/fingerprint, analyzer set and request scope. Return a result basis/digest that SDDK can put in receipts.

### CC-S4 — Deterministic fixtures

Fixtures must cover at least:

- source-only local change;
- dependency-impact change;
- symbol added/removed/renamed;
- syntax/parse failure;
- unsupported language;
- partial analysis;
- cancelled request;
- incompatible protocol;
- provider restart and cache rebuild.

### CC-S5 — Integration release contract

Publish/tag a provider build usable by SDDK integration tests and document protocol compatibility independent of CogniCode product SemVer.

## 6. Work required in SDDK

Do not implement this before the R2/R5 semantic contracts are stable enough to consume it.

1. finalize SDDK `CodeIntelligencePort` ADTs;
2. add Extension Platform provider registry/capability negotiation;
3. implement CogniCode adapter in gateway/infrastructure;
4. map provider result refs/observations to SDDK Evidence + KnowledgeAssertions;
5. integrate only through Verify/Deepening/Alignment consumers;
6. add fallback behavior proving Base mode remains first-class;
7. add end-to-end static-enhanced receipt.

The current completion/model `provider_router` is not the owner of this port. If it remains, rename/document it so completion-provider routing and engineering-intelligence providers cannot be conflated.

## 7. Documents to transfer to CogniCode

Do **not** copy the complete `SDDK-Context-First-...zip` into CogniCode as normative documentation. It is too broad and will drift.

Create a narrow folder in CogniCode:

```text
docs/integrations/sddk/
  README.md
  CONTRACT.md
  COMPATIBILITY.md
  UAT.md
  SOURCE-MANIFEST.md
  reference/
```

The handoff source set should contain copies/snapshots of exactly these SDDK documents:

### Common provider contract

1. `02-BOUNDED-CONTEXTS/extension-platform/INTELLIGENCE-PROVIDER-PORTS.md`
2. `02-BOUNDED-CONTEXTS/extension-platform/specs/SPEC-030-INTELLIGENCE-PROVIDER-SPI.md`
3. `02-BOUNDED-CONTEXTS/extension-platform/specs/SPEC-031-ENHANCED-MODES-AND-CAPABILITY-NEGOTIATION.md`
4. `02-BOUNDED-CONTEXTS/extension-platform/specs/SPEC-035-PROVIDER-RPC-LIFECYCLE.md`
5. `docs/architecture/specs/arch-spec-021-intelligence-provider-boundary.md`

### Knowledge/verification semantics CogniCode must respect

6. `02-BOUNDED-CONTEXTS/knowledge/specs/SPEC-021-PROGRESSIVE-KNOWLEDGE.md`
7. `02-BOUNDED-CONTEXTS/knowledge/specs/SPEC-022-KNOWLEDGE-MERKLE-TREE.md`
8. `02-BOUNDED-CONTEXTS/knowledge/specs/SPEC-023-KNOWLEDGE-DEPENDENCY-OVERLAY.md`
9. `02-BOUNDED-CONTEXTS/knowledge/specs/SPEC-032-DRIFT-STALENESS-CONTRADICTIONS.md`
10. `02-BOUNDED-CONTEXTS/verification/specs/SPEC-027-VERIFY-DELTA-INTELLIGENCE.md`

### Provider-specific handoff

11. this file, `04-COGNICODE-HANDOFF.md`.
12. provider-specific rows from `07-UAT-EVIDENCE-MATRIX.md`.

Every copied snapshot MUST state the SDDK source commit and original path. CogniCode may add implementation notes but must not silently edit the copied normative intent.

### Recommended ZIP

When the protocol spike begins, generate a small handoff artifact:

`SDDK-CogniCode-Static-Intelligence-Handoff-<date>.zip`

containing only the 12 items above plus `SOURCE-MANIFEST.sha256`. The existing broad integration ZIP remains SDDK archival/reference material and should not be the provider's day-to-day contract.

## 8. Static Enhanced UAT

`STATIC_ENHANCED_PRODUCTION_READY` requires all of:

1. Base profile passes with CogniCode stopped/uninstalled.
2. Optional CogniCode absence returns EvidenceGap/NOT_EVALUATED, not PASS.
3. Required CogniCode absence fails the requested task explicitly.
4. Capability negotiation succeeds against the pinned provider build.
5. Delta analysis avoids a forced full repository scan for localized fixtures.
6. Provider result maps to SDDK Evidence with basis/provider/analyzer provenance.
7. Provider heavy graph/index remains provider-side; SDDK persists only semantic knowledge + stable refs/digests needed by its contracts.
8. Restart/reconnect succeeds.
9. Cancellation/deadline is observable and leaves no false verification receipt.
10. Provider protocol incompatibility is reported as incompatible, not generic success/failure ambiguity.
11. The same source+basis+analyzer set produces stable semantic result digests where deterministic analysis promises determinism.
12. A contradiction between provider evidence and existing SDDK knowledge is preserved as contradiction/reconciliation work, not silently overwritten.

## 9. Compatibility receipt

SDDK should record a tested tuple such as:

```text
SDDK version + commit
CodeIntelligencePort schema/version
CogniCode protocol major/minor
CogniCode build/tag + commit
capability snapshot digest
analyzer set digest
integration adapter version
```

Runtime capability negotiation remains authoritative; the compatibility matrix is reproducibility/documentation, not a substitute for negotiation.
