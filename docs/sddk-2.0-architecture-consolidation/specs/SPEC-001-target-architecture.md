# SPEC-001 — Target Architecture

**Status:** Proposed

## 1. Architectural shape

```text
Agent Clients / Humans
        |
        v
+-------------------------+
| Protocol / CLI / UI     |
+------------+------------+
             |
             v
+---------------------------------------------+
| SDDK Application + Kernel                   |
| Identity | Workflow | Policy | Capability   |
| Ledger   | Receipts | CAS    | Pack Runtime |
+----------------------+----------------------+
                       |
             append domain events
                       v
                +-------------+
                | Event Ledger|
                +------+------+ 
                       |
          +------------+-------------+
          |            |             |
          v            v             v
    FSM Projection  Graph       Analytics/Search
                       |
                  pattern match
                       v
                  Behaviors
                       |
                   Proposals
                       |
                       +------> Kernel policy/capability path
```

## 2. Proposed Rust workspace boundaries

The exact crate names may be staged, but dependency direction MUST converge to:

```text
sddk-domain
    ^
    |
sddk-app             # use cases + ports
    ^
    |
+---+-------------------------------+
| adapters                           |
| sddk-storage                      |
| sddk-gateway                      |
| sddk-vault                        |
| sddk-graph                        |
| sddk-ledger                       |
+-----------------------------------+
    ^
    |
sddk-cli / hosts / explorer
```

Optional capability crates/packs:

```text
sddk-evidence
sddk-pack
sddk-reactor
sddk-uat-domain
sddk-uat-app
sddk-uat-adapters
sddk-uat-web
```

The migration MAY reuse existing crate names to avoid churn, but the dependency rules are normative.

## 3. Compile-time dependency rules

- Domain MUST NOT depend on adapters, CLI or infrastructure.
- Application/use-case layer MUST depend on domain and port traits only.
- Storage/gateway/vault/graph MUST implement ports owned by domain/application boundaries.
- CLI MUST NOT instantiate SQL queries or encode persistence rules.
- Domain packs MUST NOT call another pack's implementation functions directly for coordination; cross-pack coordination occurs via events, core contracts or declared application ports.
- Any exception requires an ADR with expiry/revisit trigger.

See `data/architecture-rules.yaml`.

## 4. Kernel responsibilities

The kernel owns:

- project identity and stable IDs;
- workflow/cycle transitions;
- authorization and policy evaluation;
- capability dispatch;
- append-only event recording;
- receipts and evidence references;
- artifact CAS;
- pack lifecycle and compatibility checks;
- deterministic validation of agent outputs.

The kernel DOES NOT own domain-specific ontology such as UAT scenarios, C4 containers, PR objects or research papers.

## 5. Reactive plane responsibilities

The reactive plane MAY:

- rebuild graph state from events;
- detect graph patterns;
- produce observations/findings;
- derive staleness and impact;
- schedule deterministic or agentic reasoning behaviors;
- propose actions;
- produce projection-specific materialized views.

It MUST NOT perform governed external side effects directly.

## 6. Minimal universal graph vocabulary

Recommended initial universal node types:

- `actor`
- `intent`
- `work_item`
- `artifact`
- `observation`
- `evidence`
- `decision`

Recommended universal relations:

- `caused_by`
- `derived_from`
- `produced_by`
- `supports`
- `contradicts`
- `depends_on`
- `satisfies`

Detailed domain semantics belong in packs. `Evidence` is universal as a graph primitive while richer evidence rules live in the evidence bounded context.

## 7. Command/event separation

Commands express intent and may fail validation. Events record accepted outcomes.

```text
Command -> validate -> authorize -> execute -> verify -> Event(s) + Receipt
```

Graph behaviors generally produce `Proposal` commands rather than invoking capabilities themselves.

## 8. Compatibility

The migration MUST maintain current v1.9.1 user-visible commands until replacement commands reach parity. Transitional facades are preferred over flag-day rewrites.
