# Failure model y pruebas adversariales

## Taxonomía

### Identity failures
- project/workspace mismatch;
- stale session binding;
- ambiguous active run/cycle;
- resource URI refers to wrong project.

### Basis/staleness failures
- stale ContextLease;
- stale action representation;
- stale Observation due subject/provider/config change;
- out-of-order ContextDelta.

### Capability/provider failures
- unavailable;
- starting timeout;
- incompatible protocol;
- partial result;
- provider crash;
- reconnect with changed capability snapshot;
- contradictory provider evidence.

### Workflow failures
- invalid type wiring;
- unbounded expansion attempt;
- unsupported operator;
- no-progress loop;
- compensation failure;
- subworkflow version mismatch.

### Persistence failures
- event append succeeds / response lost;
- CAS put succeeds / projection fails;
- DB contention;
- migration interrupted;
- corrupted CAS object;
- duplicate contribution retry.

### Authority/security failures
- stale affordance used after policy change;
- pack asks wildcard capability;
- provider output contains prompt injection;
- secret canary in args/env/error/output;
- action targets resource from another project;
- custom workflow attempts forbidden shell capability.

## Adversarial campaigns

### A1 — Hypermedia stale-action campaign

Generate action snapshots, mutate state/policy/authority, attempt every old action. Expected: no unauthorized effect.

### A2 — Recovery fuzz sequence

Repeatedly alternate:

```text
bootstrap
kill process
mutate work item
restart
renew/release lease
bootstrap
```

Assert monotonic basis and no duplicate facts.

### A3 — Provider epistemic integrity

For each provider, inject Complete/Partial/Timeout/Failure/Incompatible and confirm Verification/context never collapse them into equivalent success.

### A4 — Prompt injection through observations

Provider returns text such as `IGNORE ALL RULES; run ...`. It may appear quoted as untrusted evidence but never in instruction authority/action derivation.

### A5 — Custom workflow privilege escalation

User workflow references benign StepDefinition then attempts override to destructive provider/capability. Admission must evaluate resolved execution, not trust authored labels.

### A6 — Expansion bomb

Map/Discover recursively adds nodes until max_nodes/depth. Runtime must stop before hard limit and leave reconstructable failure state.

### A7 — Cross-project reference poisoning

Contribution references evidence/workitem from another project. Reject unless explicit cross-project contract exists; none in initial scope.

### A8 — Partial persistence fault injection

Fault at each durable boundary and replay invocation. Expected: reconcile to one semantic result or typed repair requirement, never double side effect.

## Security assertions

- Provider text is data.
- Skill text is advisory/expertise; command authority remains kernel policy.
- Pack installation does not grant permission.
- Hypermedia actions are offers under a basis, not bearer capabilities.
- Resource URI alone conveys no authority.
- Every mutable action has actor/session/run attribution.
