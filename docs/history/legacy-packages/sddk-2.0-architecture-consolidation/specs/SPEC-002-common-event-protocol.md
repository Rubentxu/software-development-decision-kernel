# SPEC-002 — Common Event Protocol (CEP)

**Status:** Proposed

## 1. Goal

Define one durable event envelope that can be shared by workflow, UAT, capabilities, evidence, agents, releases and graph projections without forcing every domain to share one payload schema.

## 2. Event envelope

Every persisted domain event MUST contain:

- `event_id`: globally unique stable ID;
- `event_type`: namespaced verb, e.g. `uat.acceptance.granted`;
- `schema_version`: payload/envelope compatibility version;
- `stream_id`: aggregate/cycle/run stream;
- `sequence`: monotonic sequence within the stream;
- `project_id`;
- `occurred_at` and `recorded_at`;
- `actor`: human/agent/system identity reference;
- optional `causation_id`;
- optional `correlation_id`;
- optional `cycle_id`, `frame_id`, `fork_id`;
- `subjects`: typed entity references;
- `payload`;
- `evidence_refs`;
- `content_hash` over canonical event content excluding transport metadata.

See `schemas/event-envelope.schema.json`.

## 3. Event naming

Format:

```text
<context>.<entity-or-process>.<past-tense-outcome>
```

Examples:

- `workflow.phase.entered`
- `capability.execution.requested`
- `capability.execution.denied`
- `capability.execution.completed`
- `uat.scenario.started`
- `uat.check.failed`
- `uat.acceptance.granted`
- `evidence.item.recorded`
- `agent.execution.completed`
- `graph.staleness.detected`
- `release.gate.passed`

## 4. Events vs exceptions

Expected operational outcomes MUST be modeled as events when they are meaningful domain facts:

- denial;
- rejection;
- failed test;
- failed UAT check;
- blocked capability;
- stale evidence;
- approval denied.

Programming misuse/invariant corruption SHOULD remain typed errors:

- invalid lifecycle transition API usage;
- corrupted ledger sequence;
- content hash mismatch;
- impossible schema state;
- unsupported migration.

## 5. Deterministic serialization

The event hash algorithm and canonical serialization format MUST be versioned. The initial implementation SHOULD use deterministic canonical JSON and SHA-256. A compatibility test MUST ensure two equivalent payload maps produce the same content hash.

## 6. Idempotency

External command handlers SHOULD accept an `idempotency_key`. Duplicate execution attempts MUST either return the original receipt or emit an explicit duplicate-suppressed event.

## 7. Evolution

Breaking payload changes MUST increment `schema_version`. Upcasters MAY normalize older events for projections but MUST preserve raw original bytes/hash for audit.
