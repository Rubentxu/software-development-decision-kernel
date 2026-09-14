---
id: arch-spec-021-intelligence-provider-boundary
status: proposed
proposed_at: 2026-09-14
source: docs/SDDK-Production-Readiness-Alignment-2026-09-14/
supersedes_history: false
---

# arch-spec-021 — Intelligence Provider Boundary

## Intent

CogniCode and Chronos SHALL enhance SDDK through stable semantic ports without becoming SDDK domain authorities or forcing provider implementation types into Knowledge, Alignment, Verification or Governance.

This specification refines the accepted context-first Extension Platform provider SPI for production integration.

## Semantic ports owned by SDDK

Conceptually:

```rust
trait CodeIntelligencePort {
    // SDDK-owned request/result ADTs
    fn analyze_delta(/* ... */);
    fn analyze_scope(/* ... */);
    fn analyze_impact(/* ... */);
    fn capabilities(/* ... */);
}

trait RuntimeIntelligencePort {
    // SDDK-owned request/result ADTs
    fn observe_scenario(/* ... */);
    fn compare_scenario(/* ... */);
    fn behavior_summary(/* ... */);
    fn capabilities(/* ... */);
}
```

Exact async/runtime signatures may evolve. Ownership and semantics may not.

## Requirements

### IPB-001 — Anti-corruption adapter

Provider wire DTOs, protobuf messages, transport errors and internal graph/trace types SHALL terminate at the gateway/extension adapter boundary.

Knowledge/Alignment/Verification SHALL depend on SDDK ADTs/ports only.

### IPB-002 — Provider is evidence source, not truth authority

Provider results become attributable Evidence/Knowledge inputs.

CogniCode does not decide Alignment. Chronos does not decide Verify. Neither decides AuthorityEngine outcomes or writes canonical facts directly.

### IPB-003 — Heavy data remains provider-side by default

Large AST/index/graph/trace datasets SHALL remain provider-owned. SDDK persists only the semantic facts/projections and stable provider refs/digests necessary for its own contracts.

Any materialized copy requires an explicit use-case and ownership decision.

### IPB-004 — Capability negotiation is runtime authority

The integration SHALL negotiate protocol + capabilities. Product version/tag is reproducibility metadata and compatibility guidance, not proof that a capability is available.

Capability profiles include:

```text
BASE
STATIC_ENHANCED
RUNTIME_ENHANCED
FULLY_ENHANCED
```

A profile may only be advertised while its required negotiated capabilities are available.

### IPB-005 — Requirement semantics

Each consuming Task/Policy may classify provider capability as:

```text
OPTIONAL
PREFERRED
REQUIRED
```

- unavailable OPTIONAL/PREFERRED => `NOT_EVALUATED` / EvidenceGap equivalent;
- unavailable REQUIRED => explicit failure for that operation;
- absence SHALL NOT become PASS.

### IPB-006 — Provider lifecycle

Adapters SHALL represent provider lifecycle with semantics covering at least:

```text
UNAVAILABLE
DORMANT
STARTING
READY
BUSY
INCOMPATIBLE
FAILED
```

Equivalent ADTs are acceptable if no material state is lost.

On-demand local activation is allowed. UDS, local socket, stdio child, gRPC/tonic or future remote transport are adapter details.

### IPB-007 — Reproducible provider basis

Enhanced evidence/receipts SHALL identify:

- provider build/version/commit where available;
- negotiated protocol version;
- capability snapshot/digest;
- analyzer/instrumentation set/basis;
- source/revision/scenario basis;
- stable provider result/evidence refs/digests.

### IPB-008 — Failure/cancellation is epistemically visible

Timeout, cancellation, partial analysis, incompatible protocol and provider failure SHALL remain distinguishable. A failed provider request cannot be normalized into successful evidence merely because SDDK continues operating in Base mode.

### IPB-009 — Static/runtime contradiction is preserved

CogniCode and Chronos evidence may corroborate or contradict. SDDK Knowledge/Alignment reconciliation SHALL preserve source identity and contradiction rather than average results into a universal score or pick one provider as canonical truth.

### IPB-010 — Base mode is first-class

SDDK SHALL pass its Base production-readiness suite with both providers absent. Enhanced adapters cannot introduce startup, migration or domain dependencies that make provider installation a prerequisite for Base operation.

### IPB-011 — Completion/model routing is a different responsibility

LLM/model/completion-provider routing SHALL NOT implicitly own Code/Runtime Intelligence merely because both use the word provider. Naming/module ownership MUST make the distinction observable.

### IPB-012 — No LLM intermediary required

SDDK SHALL be able to invoke CogniCode/Chronos directly through their typed integrations. An LLM may consume resulting context/evidence but is not the transport or semantic mediator required to access provider intelligence.

## Provider-specific handoffs

- CogniCode: `docs/SDDK-Production-Readiness-Alignment-2026-09-14/04-COGNICODE-HANDOFF.md`
- Chronos: `docs/SDDK-Production-Readiness-Alignment-2026-09-14/05-CHRONOS-HANDOFF.md`

## Compatibility and publication

Provider product SemVer and integration protocol SemVer MAY evolve independently. SDDK SHALL record tested compatibility tuples, while runtime capability negotiation remains authoritative.

Provider-specific handoff snapshots copied to external repositories SHALL carry the originating SDDK commit/path and a manifest digest. The complete SDDK architecture package SHALL NOT be used as the provider's mutable working contract.

## Acceptance

This spec is satisfied per profile only when the relevant rows in `07-UAT-EVIDENCE-MATRIX.md` are green and the readiness receipt records the negotiated provider basis.
