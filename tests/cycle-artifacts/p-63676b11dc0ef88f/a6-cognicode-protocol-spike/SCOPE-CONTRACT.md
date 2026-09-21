# SCOPE-CONTRACT — A6-CogniCode-protocol-spike (CC-S0)

Cycle: `p-63676b11dc0ef88f/a6-cognicode-protocol-spike`
Baseline (released): `v1.169.88` → `add896d94274a7515254b8c195e2e78c3669108f`
Development head at start: `aa6855e7cea1bf99992c55c9771e2bc51ed7894a`
Plan: `docs/history/proposals/all-proposals/2026-09-19-a6-cognicode-cc-s0-protocol-spike-PROPOSAL.md`
Spec: `docs/architecture/specs/arch-spec-021-intelligence-provider-boundary.md`
Handoff: `docs/history/legacy-packages/SDDK-Production-Readiness-Alignment-2026-09-14/04-COGNICODE-HANDOFF.md`

## Goal

ONE narrow implementation cycle: prove the SDDK-side seam for
static-intelligence providers exists and is contract-clean.
Deliverable is the trait + an in-process fake provider + 6
falsification tests. No production CogniCode adapter, no
Verify/Knowledge integration, no new dependency.

## MUST (acceptance)

- M1. `CodeIntelligencePort` trait exists in
  `crates/sddk-engine/src/code_intelligence_port.rs` with the
  four mandatory operations
  (`analyze_delta`, `analyze_scope`, `analyze_impact`,
  `capabilities`) per `arch-spec-021`.
- M2. Provider lifecycle state enum exists with the seven
  states: `UNAVAILABLE` / `DORMANT` / `STARTING` / `READY` /
  `BUSY` / `INCOMPATIBLE` / `FAILED`.
- M3. Capability snapshot digest exists and is content-stable
  across two calls.
- M4. `AnalysisBasis` carries provider build/commit, negotiated
  protocol version, capability snapshot digest, analyzer set
  digest, source/revision basis, request scope.
- M5. `analyze_delta` is deterministic across two calls (same
  fixture → same digest).
- M6. Protocol-major mismatch → `INCOMPATIBLE` (NOT silent
  degrade to `BASE`).
- M7. Cancellation yields typed refusal (no false PASS).
- M8. Provider restart mid-request → reconnect yields same
  digest OR explicit `Partial` marker.
- M9. With provider `UNAVAILABLE`, SDDK Verify produces the
  same result as without the port (`arch-spec-021 IPB-010`).
- M10. `arch-spec-021` IPB-001..IPB-012 are NOT amended by
  this cycle (the spec remains `proposed`; this cycle
  implements it).
- M11. `context_fitness::no_new_root_level_context_module_without_adr`
  passes — the new module `code_intelligence_port` is
  registered by an ADR (proposed or accepted).
- M12. `context_fitness::no_knowledge_to_provider_sdk` still
  passes — no CogniCode / chronos / prost / tonic token
  appears in the protected knowledge modules.
- M13. All 6 falsification tests GREEN.
- M14. `cargo test -p sddk-engine --offline` GREEN.
- M15. `cargo clippy --workspace --all-targets -- -D warnings`
  GREEN.
- M16. `cargo fmt --check` GREEN.

## MUST NOT

- N1. No new secret manager / log system (SEC-1 discipline
  carried forward).
- N2. No new canonical SDDK domain types derived from a
  provider SDK.
- N3. No production ingestion of provider observations into
  Knowledge — CC-S1+ territory.
- N4. No modification of `arch-spec-021`.
- N5. No `serde` / `prost` / `tonic` dependency added.
- N6. No edits to A5-C cert or SEC-1 contract.
- N7. No retro-active change to A5-C certification or to
  the SEC-1 release gate.

## Surfaces inventory

| Surface | In scope? | Why |
|---|---|---|
| `crates/sddk-engine/src/code_intelligence_port.rs` | YES (new) | The trait module |
| `crates/sddk-engine/src/code_intelligence_port_fake.rs` | YES (new) | In-process fake provider |
| `crates/sddk-engine/src/lib.rs` | YES (1-line `pub mod code_intelligence_port;`) | Module wiring |
| `crates/sddk-engine/tests/a6_cognicode_protocol_spike.rs` | YES (new) | Falsification battery |
| `crates/sddk-engine/src/{semantic_*,evidence_ref,observation*,intelligence_loop,intelligence_advisory,architecture_*,...}` | NO | Knowledge / Alignment / Verify / Architecture — outside CC-S0 |
| `crates/sddk-cli/**` | NO | Adapter call sites are CC-S1+ |
| `crates/sddk-domain/**` | NO | Domain types not derived from providers |
| `Cargo.toml` (workspace.dependencies) | NO | No new deps |
| `crates/sddk-engine/Cargo.toml` | NO | No new deps |

## Falsification plan

Each T is RED-pinned against the pre-fix code (no trait, no
fake) and becomes GREEN after the implementation.

| T | Test | Expected behaviour post-fix |
|---|------|------------------------------|
| T1 | `connect_negotiate_returns_static_enhanced_snapshot` | Negotiate `BASE` capability → adapter reports `BASE`. Negotiate `STATIC_ENHANCED` (provider advertises it) → adapter reports `STATIC_ENHANCED`. |
| T2 | `analyze_delta_is_deterministic_across_runs` | Same fixture (analyzer set, source basis, options) → same digest twice. |
| T3 | `protocol_major_mismatch_yields_incompatible` | Fake provider with `--fake-incompatible` → adapter returns `INCOMPATIBLE`, NOT `BASE`. |
| T4 | `cancellation_yields_typed_refusal_no_false_evidence` | Mid-request cancel → typed `Cancelled` variant; no `ObservationSet` produced; no false PASS. |
| T5 | `restart_mid_request_yields_replay_or_partial_marker` | Provider restart mid-request → reconnect yields same digest OR explicit `Partial` marker (with what-was-completed basis). |
| T6 | `base_mode_first_class_with_provider_unavailable` | Provider `UNAVAILABLE` → `CodeIntelligencePort::capabilities()` returns `BASE`; the adapter's `observe()` is a no-op. SDDK Verify path with the port absent produces identical output to the port-absent path (asserted by comparing two invocations of the same fixture: one with `UNAVAILABLE`, one with no port at all). |

## Diagnostic preservation check

CC-S0 introduces no observable outputs that an operator would
read. There is no new log, no new CLI surface, no new receipt
field. Diagnostics preservation is therefore vacuously
satisfied for this cycle; it is the obligation of CC-S1+
cycles that actually emit observations.

## Acceptance criteria

PASS requires:

1. RED tests committed before the fix (falsification pinned).
2. GREEN tests committed after the fix (proof of closure).
3. All 6 tests pass.
4. `arch-spec-021` is not modified.
5. `context_fitness` lints pass.
6. `cargo test -p sddk-engine --offline` GREEN.
7. `cargo clippy --workspace --all-targets -- -D warnings`
   GREEN.
8. `cargo fmt --check` GREEN.

## Disposition at exit

Will report:

- Which `arch-spec-021` requirements are demonstrated by the
  spike (IPB-001, IPB-002, IPB-003, IPB-004, IPB-008,
  IPB-010).
- Which IPB requirements remain for downstream cycles
  (IPB-005 requirement semantics, IPB-006 lifecycle
  integration, IPB-007 reproducible provider basis full
  support, IPB-009 contradiction preservation, IPB-011
  completion/model routing distinction, IPB-012 no-LLM).
- Honesty markers carried forward (e.g. spec remains
  `proposed`).
- Follow-up list (CC-S1 / CC-S2 / CC-S3).

## Pre-push hook reminder

This cycle introduces:

- 1 new module in `crates/sddk-engine/src/` (`code_intelligence_port`).
- 1 new module in `crates/sddk-engine/src/` (`code_intelligence_port_fake`).

The pre-push hook requires either:

1. A `[workspace.package] version` bump in `Cargo.toml`
   (release flow), OR
2. A NON-EMPTY range whose changed paths are all under
   `docs/**` or `.sddk/followups/**`.

The two new `.rs` files are NOT under `docs/**`. So the
push of the implementation commit MUST go through a release
bump (preferred) or be deferred until the next release that
includes this cycle.

The ADR is `docs/architecture/adrs/ADR-0137-...md` and lives
under `docs/`, so it CAN be pushed independently as
docs-only.

## Anti-corruption check

- The fake provider lives in `sddk-engine` (no new
  external crate, no provider SDK import).
- Knowledge / Alignment / Verify modules are untouched.
- The trait's return types are SDDK ADTs (no provider
  types cross the adapter boundary).
- The lint `no_knowledge_to_provider_sdk` continues to pass.
