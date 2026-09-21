# AGENT-HOST-002 — Provider Failover Substrate (Proposal ready)

- status: PROPOSAL READY
- cycle: AGENT-HOST-002 (order 240, horizon H4 — Agent Host)
- depends on: AGENT-HOST-001 (v1.94.0, SHIPPED)
- ADR: `~/.sddk-knowledge/sddk-framework/adrs/ADR-080-PROVIDER-FAILOVER-SUBSTRATE.md` (proposed, P9-a)
- spec: `~/.sddk-knowledge/sddk-framework/specs/engine/REQ-ProviderFailover.md` (proposed, 389 lines)
- canonical sources: ADR-027, SPEC-026, SPEC-032

## What this cycle delivers

A `ProviderRouter` substrate that turns AGENT-HOST-001's `AgentHost`
into a routing-aware executor:

1. **Closed-set `ProviderFailure` taxonomy** adopted verbatim from
   SPEC-026 (RateLimited, QuotaExhausted, AuthenticationFailed,
   AuthorizationFailed, ModelUnavailable, ServiceUnavailable,
   Timeout, TransportFailure, UnknownProviderFailure).
2. **`Provider` trait** + `ProviderIdentity` (circuit breaker key).
3. **`CircuitBreakerStore` trait** + `InMemoryCircuitBreakerStore`
   with `CircuitState { Closed, Open, HalfOpen, Disabled }`.
4. **`ProviderRouter` struct** with configurable `RouterPolicy`
   implementing ADR-027 semantics (retry same-route vs failover).
5. **`TelemetrySink` trait** + `UsageRecord` (strict subset of
   SPEC-032).
6. **Integration with `AgentHost`** via
   `execute_through_router(decision, cycle_id, ctx)`, extending
   `DecisionReceipt.provider_route: Option<Vec<RouteAttempt>>`.

## Shape chosen: P9-a (recommended, adopted)

- `ProviderRouter` struct (mirrors `AgentHost` pattern from
  AGENT-HOST-001).
- Two trait seams (`CircuitBreakerStore`, `TelemetrySink`) and one
  `Provider` trait.
- `InMemoryCircuitBreakerStore` is kernel reference impl.
- Deterministic ordering: providers iterated in registration order;
  first eligible wins with bounded retries.

P9-b (free-function router) and P9-c (actor router) rejected — see
ADR-080 §3 for rationale.

## Acceptance criteria (from spec §Acceptance demo)

Inject a deterministic `QuotaExhausted` for Attempt #1 (provider
`mock://primary`) and prove Attempt #2 (`mock://secondary`) uses
another healthy route and completes the same NodeRun. Implemented as
`provider_failover_tests.rs::scenario_quota_failover` in apply
phase.

## Test scenarios (10 in spec)

1. Quota exhaustion triggers failover (canonical)
2. Transient timeout retries then succeeds
3. All providers fail → `RouteError::AllProvidersFailed`
4. Open circuit is skipped
5. Cooldown transitions Open → HalfOpen
6. No eligible provider
7. Disabled route returns `RouteError::Disabled`
8. Telemetry records both attempts and failovers
9. Deterministic ordering across runs
10. Lease-bound routing (AGENT-HOST-001 integration)

## Apply-phase deliverables (next session)

- `crates/sddk-engine/src/provider_router.rs` (~500 lines)
- `crates/sddk-engine/src/circuit_breaker.rs` (~150 lines)
- `crates/sddk-engine/src/telemetry.rs` (~100 lines)
- New traits re-exported in `lib.rs`
- Extend `crates/sddk-engine/src/agent_host.rs::DecisionReceipt`
  with `provider_route` field
- `crates/sddk-engine/tests/provider_failover_tests.rs` (10 scenarios)
- Target version: **v1.95.0** (next minor after v1.94.0)

## Toxicology check (decision sanity)

- No deviation from SPEC-026 taxonomy (canonical).
- No deviation from SPEC-032 telemetry shape (strict subset).
- P9-a follows the same architectural pattern as AGENT-HOST-001
  (struct + trait + RAII-style ownership).
- Mock-only in apply phase keeps the cycle bounded and deterministic;
  real network adapters deferred to AGENT-HOST-003+.

## State at handoff

- Spec: PROPOSED in `~/.sddk-knowledge/sddk-framework/specs/engine/REQ-ProviderFailover.md`
- ADR: PROPOSED in `~/.sddk-knowledge/sddk-framework/adrs/ADR-080-PROVIDER-FAILOVER-SUBSTRATE.md`
- Spine: AGENT-HOST-002 PROPOSED (unchanged from baseline)
- Local workspace: clean
- Binary: `sddk 1.94.0`
