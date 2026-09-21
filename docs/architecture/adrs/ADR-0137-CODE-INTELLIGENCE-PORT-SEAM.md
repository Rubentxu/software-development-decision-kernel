---
id: ADR-0137-CODE-INTELLIGENCE-PORT-SEAM
status: accepted
supersedes_history: false
proposed_at: 2026-09-19
accepted_at: 2026-09-19
accepted_by_cycle: p-63676b11dc0ef88f/a6-cognicode-protocol-spike
---

# ADR-0137 — Code Intelligence Port Seam (CC-S0)

## Context

`arch-spec-021-intelligence-provider-boundary` (status `proposed`)
defines the SDDK-owned semantic port for static-intelligence
providers (CogniCode / future equivalent). The contract exists
in prose and IPB-001..IPB-012 requirements; the **trait does
not** exist in `crates/sddk-engine/src/`.

The live mini-roadmap
(`docs/history/legacy-packages/SDDK-Production-Readiness-Alignment-2026-09-14/02-MINI-ROADMAP.md`)
reserves **A6 CogniCode STATIC_ENHANCED** as the next P1
parallel track after A5 BASE_READY. AC10 ("consume CogniCode
call/dependency/impact/graph observations as Evidence through
`CodeIntelligencePort`") cannot be met without the trait.

A historical R4-B closure cycle series (`a6-0..a6-4`,
2026-09-18) used the `a6-` numeric prefix for ticket /
authority migration. The roadmap explicitly warns against
collision:

> "A6 CogniCode / STATIC_ENHANCED provider integration — NOT
> STARTED — reserved. Not to be confused with the historical
> a6-0..a6-4 cycle IDs above."

This ADR names the new SDDK-engine root-level module
`code_intelligence_port` (and its companion
`code_intelligence_port_fake`) so that the architectural lint
`no_new_root_level_context_module_without_adr`
(`crates/sddk-cli/tests/context_fitness.rs:207`) admits them.

## Decision

CC-S0 ships:

1. `crates/sddk-engine/src/code_intelligence_port.rs` — the
   `CodeIntelligencePort` trait, the `ProviderLifecycle` state
   enum, the `CapabilityProfile` enum, the
   `CapabilitySnapshot` digest type, the `AnalysisBasis`
   struct, and the result / digest types.
2. `crates/sddk-engine/src/code_intelligence_port_fake.rs` —
   the deterministic in-process fake provider used by the
   falsification battery. Returns only stable digests
   (`IPB-003`).
3. `crates/sddk-engine/tests/a6_cognicode_protocol_spike.rs` —
   6 falsification tests (T1..T6) covering negotiation,
   determinism, INCOMPATIBLE on protocol mismatch,
   cancellation typing, restart-mid-request, and Base-mode
   first-class.

The cycle is **strangler**, not flag-day: no existing SDDK
engine surface is rewritten; the trait is additive. The fake
provider lives next to the trait (not behind a feature flag)
because CC-S0 is a spike, not a production integration. CC-S1+
will move the fake to `sddk-engine/tests/` (or a sibling
`fixtures` crate) once the production CogniCode adapter exists.

The two new modules are listed in
`context_fitness.rs::BASELINE_ROOT_MODULES` only if / when they
become durable (CC-S1+). Until then, this ADR's mention of
`code_intelligence_port` in its body satisfies the lint
(`adr_text.contains(&stem.to_lowercase())`).

## Requirements addressed (per `arch-spec-021`)

| Requirement | Demonstration in CC-S0 |
|---|---|
| IPB-001 Anti-corruption adapter | The trait is the SDDK-owned port; the fake returns SDDK ADTs only. The lint `no_knowledge_to_provider_sdk` continues to pass. |
| IPB-002 Provider is evidence source, not truth authority | Trait returns `ObservationSet`-shaped results; no canonical event log writes happen in the spike. |
| IPB-003 Heavy data remains provider-side | Fake returns only stable digests. |
| IPB-004 Capability negotiation is runtime authority | `CapabilitySnapshot` is negotiated at request time, not build time. T1 verifies. |
| IPB-008 Failure / cancellation is epistemically visible | `ProviderLifecycle::FAILED` and the `Cancelled` error variant are distinct. T4 verifies. |
| IPB-010 Base mode is first-class | Provider `UNAVAILABLE` ⇒ `CapabilityProfile::BASE`; the adapter `observe()` is a no-op. T6 verifies. |

The remaining IPB requirements (IPB-005 requirement
semantics, IPB-006 lifecycle integration with the engine
event log, IPB-007 reproducible basis full support,
IPB-009 contradiction preservation, IPB-011 completion /
model routing distinction, IPB-012 no-LLM intermediary) are
**out of scope for CC-S0** and become the budget of CC-S1+.

## Honesty markers

1. **Spec remains `proposed`** during this cycle. Acceptance
   (`arch-spec-021` → `accepted`) happens when CC-S1 ships the
   production adapter and the full UAT.
2. **The fake is not CogniCode.** CC-S0 proves the SDDK-side
   seam; CC-S2 / CC-S3 will introduce the real provider.
3. **AC10 (static evidence into Verify) is not this cycle.**
   CC-S0 produces the seam; AC10 is CC-S1.
4. **Trait lives in `sddk-engine`**, not `sddk-domain`,
   because the provider semantics are evidence-source, not
   evidence-truth. A future decision may move it to `domain`
   if IPB semantics tighten.
5. **No `serde` / `prost` / `tonic` dependency is added** in
   this cycle. The trait uses SDDK-internal types only.

## Implementation surface

| File | Status | Purpose |
|---|---|---|
| `crates/sddk-engine/src/code_intelligence_port.rs` | new | Trait + types |
| `crates/sddk-engine/src/code_intelligence_port_fake.rs` | new | In-process deterministic provider |
| `crates/sddk-engine/src/lib.rs` | +1 line | `pub mod code_intelligence_port;` (re-export `fake` is test-only, behind `#[cfg(test)]`) |
| `crates/sddk-engine/tests/a6_cognicode_protocol_spike.rs` | new | 6 falsification tests |

No other file is modified.

## Risks

1. **Lint regression.** The new module must not trigger
   `no_new_root_level_context_module_without_adr` post-fix.
   Mitigation: this ADR mentions `code_intelligence_port` in
   its body (already done).
2. **Provider SDK leakage.** The trait must not import any
   CogniCode / chronos / prost / tonic type. Mitigation: the
   `no_knowledge_to_provider_sdk` lint guards
   `sddk-engine/src/semantic_*.rs`, `evidence_ref.rs`,
   `vault_boundary.rs`, `why_queries.rs`. The new module is
   not on that list; a separate check (`grep -r cognicode
   crates/sddk-engine/src/code_intelligence_port*`) is added
   to the cycle's local test plan.
3. **Future spec drift.** If `arch-spec-021` is amended by a
   later cycle, this ADR must be reconciled. Cross-references
   both ways (spec ↔ ADR) are added in both documents.

## See also

- `docs/architecture/specs/arch-spec-021-intelligence-provider-boundary.md`
- `docs/history/legacy-packages/SDDK-Production-Readiness-Alignment-2026-09-14/04-COGNICODE-HANDOFF.md`
- `docs/history/legacy-packages/SDDK-Production-Readiness-Alignment-2026-09-14/02-MINI-ROADMAP.md`
  (A6 paragraph)
- `docs/history/proposals/all-proposals/2026-09-19-a6-cognicode-cc-s0-protocol-spike-PROPOSAL.md`
- `tests/cycle-artifacts/p-63676b11dc0ef88f/a6-cognicode-protocol-spike/SCOPE-CONTRACT.md`

## Addendum (AIW-S1, 2026-09-19): CogniCode MCP stdio adapter

The first concrete `CodeIntelligencePort` implementation is
`crates/sddk-engine/src/code_intelligence_port_mcp.rs`
(`CogniCodeMcpAdapter`). Decisions recorded here so the context_fitness
gate stays truthful:

- **Transport:** stdio JSON-RPC (initialize → tools/call) against the
  external `cognicode-mcp` binary. No daemon, no HTTP; AIS-004 holds.
- **Provider isolation:** all CogniCode-specific knowledge (handshake
  protocol 2025-03-26, JSON-in-`content[0].text` envelope, tool names)
  lives inside the adapter, never in the trait or its consumers.
- **Digests:** SHA-256 over canonical material
  (`DigestSha256::of`), no provider-controlled strings enter digests
  unhashed.
- **Observation basis:** results derive `ObservationBasis` canonically
  via `ObservationBasis::for_provider_result`, so the VerifyKernel can
  check staleness without trusting the provider's own claims.
