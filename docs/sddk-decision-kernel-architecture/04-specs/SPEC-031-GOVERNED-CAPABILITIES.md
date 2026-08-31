# SPEC-031 — Governed Capabilities

**Status:** Proposed

## Flow

```text
Proposal → Policy → Approval? → Capability Grant → Execute → Verify → Receipt
```

## Proposal
Contains desired semantic effect, scope, inputs, reason, actor, workflow/node and evidence refs.

## PolicyDecision
`allow`, `deny`, `require_approval`, optionally with constraints.

## CapabilityGrant
Short-lived and scoped:

```yaml
capability: git.apply-patch
scope:
  paths: ["crates/sddk-engine/**"]
operations: ["modify"]
expires_at: ...
workflow_run: wf-...
node_run: nr-...
```

## Verification
Postcondition-specific verifier checks whether intended effect occurred and invariants remain valid.

## Receipt
Records:
- proposal hash;
- policy hash/decision;
- approval ref;
- capability/version;
- actor/agent/model hashes where relevant;
- before/after evidence;
- verification outcome.

## Default deny examples
- git push;
- destructive filesystem operations outside workspace;
- production deployment;
- network destinations not allowed by policy;
- secret export.

## Replay safety
Replaying ledger/projections never re-executes a capability. A new execution requires a new proposal/grant even if based on a previous event.
