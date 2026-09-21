# CDD-ROLE-001 — Agent Role Contract Substrate (Proposal ready)

- status: PROPOSAL READY
- cycle: CDD-ROLE-001 (order 261, horizon H4 — CDD Role)
- depends on: CTX-COMPILER-002 (v1.97.0, SHIPPED)
- ADR: `~/.sddk-knowledge/sddk-framework/adrs/ADR-083-AGENT-ROLE-CONTRACT.md` (proposed, P12-a)
- spec: `~/.sddk-knowledge/sddk-framework/specs/engine/REQ-AgentRoleContract.md` (proposed, 286 lines)
- canonical sources: backlog §CDD-ROLE (target invariants)

## What this cycle delivers

A typed `AgentRoleContract` substrate that turns role boundaries
from prompt convention into machine-validatable contracts:

1. **`AgentRoleContract` struct** — role_id, kind, responsibility,
   dispatch_allowlist, read/write/tool scopes, mutation_authority,
   budget_tokens, return_schemas, input_schemas, synthesis_owner,
   forbidden_actions, capabilities.
2. **`RoleKind` enum** — Orchestrator | Coordinator | Leaf |
   Evaluator | Advisor (closed set, `#[non_exhaustive]`).
3. **`MutationAuthority` enum** — Lifecycle | Attempt | None.
4. **`MutationKind` enum** — 6 variants covering workflow,
   planning, attempt, lifecycle, decision emission.
5. **`RoleRegistry` trait** + `InMemoryRoleRegistry` reference impl.
6. **`RoleValidator` struct** with four static validation paths:
   `validate_delegation`, `validate_mutation`,
   `validate_synthesis_owner`, `detect_cycles`.
7. **`RoleError` closed-set taxonomy** — 9 variants, all
   `#[non_exhaustive]`.
8. **Orthogonal to `AgentIdentity`** — adds `role_contract_id`
   field on `AgentIdentity` without breaking existing code.

## Shape chosen: P12-a (recommended, adopted)

- `AgentRoleContract` struct (mirrors `AgentHost`, `ProviderRouter`,
  `ContextCompiler` patterns).
- Two trait seams (`RoleRegistry`, plus a future role-store).
- One validator struct with deterministic per-call API.
- Static validation; runtime enforcement is post-H4.

P12-b (free function) and P12-c (actor) rejected — see ADR-083 §3.

## Acceptance criteria (from spec §Invariants)

All 6 invariants machine-validatable:

1. Leaf cannot delegate
2. Coordinator dispatch only declared workers
3. Orchestrator dispatches to coordinators
4. One synthesis owner per join
5. No authority cycles
6. Workers cannot mutate lifecycle/planning unless authorized

Implemented as 10 RED->GREEN tests in
`agent_role_contract_tests.rs`.

## Test scenarios (10 in spec)

1. Leaf cannot delegate
2. Coordinator dispatch to declared worker
3. Coordinator dispatch to unknown role
4. One synthesis owner per join
5. Authority cycle detection
6. Leaf cannot mutate lifecycle
7. Forbidden action rejection
8. Tool scope check (positive)
9. Tool scope violation
10. Read-only evaluator cannot emit decisions

## Apply-phase deliverables (next session)

- `crates/sddk-engine/src/agent_role_contract.rs` (~450 lines):
  types + `RoleValidator` + `InMemoryRoleRegistry`
- Extend `AgentIdentity` with optional `role_contract_id` field
- Re-export from `lib.rs`
- `crates/sddk-engine/tests/agent_role_contract_tests.rs` (10 scenarios)
- Target version: **v1.98.0** (next minor after v1.97.0)

## Toxicology check (decision sanity)

- Implements all 6 backlog invariants verbatim.
- Closed-set error taxonomy (`#[non_exhaustive]`) prevents
  downstream breakage when new roles are added.
- Static validation matches the exit gate "machine-validatable";
  runtime enforcement is the natural post-H4 follow-up.
- Orthogonal to existing identity / provider / context concerns —
  no cross-cycle coupling.

## State at handoff

- Spec: PROPOSED in `~/.sddk-knowledge/sddk-framework/specs/engine/REQ-AgentRoleContract.md`
- ADR: PROPOSED in `~/.sddk-knowledge/sddk-framework/adrs/ADR-083-AGENT-ROLE-CONTRACT.md`
- Spine: CDD-ROLE-001 PROPOSED (unchanged from baseline)
- Local workspace: clean
- Binary: `sddk 1.97.0`
