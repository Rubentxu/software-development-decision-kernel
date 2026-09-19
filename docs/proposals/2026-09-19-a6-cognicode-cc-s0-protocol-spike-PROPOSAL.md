# Proposal: A6 CogniCode STATIC_ENHANCED — first implementable cycle (CC-S0 protocol spike)

**Status:** PROPOSAL — not opened as a cycle. Awaiting operator
decision.

**Author:** SDDK orchestrator session, 2026-09-19.

**Predecessor:** SEC-1 (closed 2026-09-19, commits `2c63aff` /
`1374951` / `f6e1906` / `b442635` / `50c7d0f` /
`b93b50d`). A6 historical cycles (a6-0..a6-4) closed R4-B
(decide-then-act TOCTOU).

## Why this proposal exists

The live roadmap (`docs/SDDK-Production-Readiness-Alignment-2026-09-14/02-MINI-ROADMAP.md`)
reserves **A6 CogniCode STATIC_ENHANCED** as the next P1
parallel track after A5 BASE_READY:

> "AC10 consumes CogniCode call/dependency/impact/graph
> observations as Evidence through `CodeIntelligencePort`;
> provider types never enter Knowledge/Alignment/Verification
> domain. Static evidence can verify or contradict architecture
> claims but CogniCode never owns status. Exit: Base stays
> green with provider absent; pinned enhanced UAT yields
> `STATIC_ENHANCED` receipt."

The capability contract is fully specified
(`docs/architecture/specs/arch-spec-021-intelligence-provider-boundary.md`
— IPB-001..IPB-012, `proposed`). The handoff is detailed
(`04-COGNICODE-HANDOFF.md` — CC-S0..CC-S5). What is missing
is **any Rust implementation** of `CodeIntelligencePort` —
the trait is referenced in the spec but does not exist in
`crates/sddk-engine`. The closest code is a single
informative comment in
`crates/sddk-engine/src/completion_provider_router.rs`.

The user mandate was explicit:
> "La tarea correcta es identificar qué capacidad operativa
> falta después de A6-4, contrastarla con el roadmap vigente
> y abrir el siguiente ciclo de implementación, no otro ciclo
> de certificación."

So this proposal defines the **first implementable cycle** of
A6 CogniCode — narrow, with falsifiable evidence, no SDDK
core rewiring.

## Naming

The repo carries cycle IDs `a6-0..a6-4` (R4-B closure). The
roadmap warns against collision:

> "A6 CogniCode / STATIC_ENHANCED provider integration —
> NOT STARTED — reserved. **Not to be confused with the
> historical a6-0..a6-4 cycle IDs above.**"

This proposal uses the name **CC-S0** (per the CogniCode
handoff §5 internal milestones) and avoids any `a6-` numeric
prefix. If opened, the cycle id would be
`p-63676b11dc0ef88f/a6-cognicode-protocol-spike` (kebab-case
without numeric suffix).

## What CC-S0 delivers

A thin **protocol skeleton** inside the SDDK workspace that
proves an external Rust client (the SDDK adapter candidate)
can:

- connect without importing CogniCode internals;
- negotiate protocol + capabilities (`BASE` vs
  `STATIC_ENHANCED`);
- request `analyze_delta` on a fixture and receive a stable
  typed result + digest;
- observe `INCOMPATIBLE` on protocol-major mismatch;
- observe typed refusal on cancellation / timeout;
- reconnect after a provider restart and obtain a stable
  digest (or an explicit partial marker);
- coexist with Base mode: with the provider `UNAVAILABLE`,
  SDDK Verify produces the same result as without the port.

It is **not** the production CogniCode adapter — that is
CC-S2 / CC-S3. CC-S0 produces the protocol version and the
capability-snapshot digest that downstream cycles need.

## Scope budget (one cycle)

1. **Trait in code** —
   `crates/sddk-engine/src/code_intelligence_port.rs`:
   - `CodeIntelligencePort` trait per `arch-spec-021`:
     `analyze_delta`, `analyze_scope`, `analyze_impact`,
     `capabilities`.
   - Provider lifecycle enum:
     `UNAVAILABLE` / `DORMANT` / `STARTING` / `READY` / `BUSY`
     / `INCOMPATIBLE` / `FAILED`.
   - Capability snapshot digest: stable hash of the
     negotiated (protocol_major, protocol_minor,
     capability_set, analyzer_set_digest).
   - Result basis: `AnalysisBasis` carrying provider build /
     commit, negotiated protocol version, capability
     snapshot digest, analyzer set digest, source/revision
     basis, request scope.

2. **In-process fake provider** —
   `crates/sddk-engine/src/code_intelligence_port_fake.rs`:
   deterministic offline provider for fixtures
   (`source-only local change`, `dependency-impact change`,
   `syntax/parse failure`, `unsupported language`,
   `partial analysis`, `cancelled request`,
   `incompatible protocol`, `provider restart`).
   - Returns only stable digests — no heavy AST/graph data
     enters SDDK (IPB-003).
   - The fake provider has a `--fake-incompatible` toggle
     used by T3 (protocol-major mismatch).
   - The fake has a `--fake-restart-after <n>` toggle used by
     T5 (restart mid-request).

3. **Falsification battery** —
   `crates/sddk-engine/tests/a6_cognicode_protocol_spike.rs`:

   | T | Test | Expectation |
   |---|------|-------------|
   | T1 | `connect_negotiate_returns_static_enhanced_snapshot` | negotiate → adapter receives `STATIC_ENHANCED` advertised only when negotiation succeeds |
   | T2 | `analyze_delta_is_deterministic_across_runs` | same fixture → same digest twice (determinism per IPB-007) |
   | T3 | `protocol_major_mismatch_yields_incompatible` | `--fake-incompatible` → `INCOMPATIBLE`, NOT silent degrade to `BASE` |
   | T4 | `cancellation_yields_typed_refusal_no_false_evidence` | cancel mid-request → typed refusal, no false PASS |
   | T5 | `restart_mid_request_yields_replay_or_partial_marker` | restart during `analyze_delta` → reconnect returns same digest OR explicit partial marker |
   | T6 | `base_mode_first_class_with_provider_unavailable` | provider `UNAVAILABLE` → SDDK Verify result is identical to no-port case (IPB-010) |

4. **Scope contract + plan** —
   `tests/cycle-artifacts/p-63676b11dc0ef88f/a6-cognicode-protocol-spike/SCOPE-CONTRACT.md`
   with MUST / MUST_NOT, surfaces inventory (in/out), and the
   falsification plan above.

5. **Receipt** —
   `docs/architecture/a6/A6-COGNICODE-CC-S0-RECEIPT.md` with
   gate evidence, honest limits, and follow-up list (links
   to CC-S1 / CC-S2).

## MUST (acceptance)

- M1. SDDK does not depend on any CogniCode-specific type
  outside the adapter module. (IPB-001.) Enforced by lint:
  `sddk-engine` imports from `cognicode-*` are forbidden; the
  fake provider lives in its own module.
- M2. Provider is evidence source, not truth authority.
  (IPB-002.) Enforced by typing: the port returns
  `ObservationSet` (an SDDK ADT), not a CogniCode DTO.
- M3. Heavy data remains provider-side. (IPB-003.) The fake
  returns digests only — no AST or graph data crosses the
  adapter.
- M4. Capability negotiation is runtime authority. (IPB-004.)
  The adapter consults the negotiated snapshot at every
  `request`, not the build-time capability claim.
- M5. Failure / cancellation is epistemically visible.
  (IPB-008.) Distinct error variants for cancellation /
  timeout / incompatible / failed / partial — none collapse
  into success.
- M6. Base mode is first-class. (IPB-010.) T6 above.
- M7. No LLM intermediary between SDDK and the provider
  port. (IPB-012.)
- M8. All 6 falsification tests GREEN.

## MUST NOT

- N1. No new secret manager / log system (carried forward
  from SEC-1 discipline).
- N2. No new canonical SDDK domain types derived from
  CogniCode internals.
- N3. No production ingestion of provider observations
  into Knowledge. The spike is evidence-of-the-port, not
  evidence-into-Knowledge (that is CC-S1+ territory).
- N4. No modification of `arch-spec-021` (it remains
  `proposed`; this cycle implements the spec but does not
  amend it).
- N5. No retro-active change to A5-C certification or
  SEC-1 contract.

## Honest limits (named, not silent)

1. The fake provider is **not** a CogniCode binary. CC-S0
   proves the SDDK-side seam; production ingestion is CC-S1+.
2. `arch-spec-021` remains `proposed` (not `accepted`)
   during this cycle. Acceptance happens when CC-S1 ships.
3. AC10 (static evidence into Verify) is **not** this
   cycle. CC-S0 produces the seam; AC10 is CC-S1.
4. The trait is added to `sddk-engine`, not to
   `sddk-domain`, because the provider semantics are
   evidence-source not evidence-truth. The trait could
   move to domain later if IPB semantics tighten; that's
   a separate decision.
5. No `serde`/`prost` is added to `sddk-engine`; the fake
   uses Rust structs only. Wire DTOs (if any) live in a
   future `sddk-extension-platform` crate (out of scope).

## Out of scope (named explicitly)

- Adapters for the real CogniCode binary (CC-S2 / CC-S3).
- Static evidence integration into Verify (AC10) —
  CC-S1.
- Chronos RUNTIME_ENHANCED — separate A7 track.
- Agentic Workspace / JCode track — J2 proposal at
  `docs/proposals/2026-09-19-jcode-anti-corruption-adapter-PROPOSAL.md`.
- Workspace flake (`SEC-WORKSPACE-FLAKE`) — separate
  cycle; would unblock `cargo test --workspace` runs.

## Estimated scope

Small cycle. Roughly:

- 1 new trait + 1 new lifecycle enum + 1 new basis struct
  + 1 fake provider module (~250 lines).
- 6 falsification tests (~150 lines).
- 1 scope contract + 1 plan + 1 receipt.
- 0 changes to existing SDDK engine surfaces.

## Why not open this cycle now

The user mandate requires explicit operator authorisation
for the first implementable cycle of A6. This proposal is
the contract; the operator opens it or picks a different
cycle.

## Status

PROPOSAL. Awaiting operator decision.

A5-C cert, SEC-1 cycle, and the SEC-1 release gate are
unaffected by this proposal.
