# Implementation backlog propuesto

> IDs locales a este paquete. Al integrar, crear WorkItems canónicos sólo para el slice que se vaya a ejecutar; no importar todo como backlog activo.

## C3i

### HWP-C3I-01 — Pin active-cycle inference contradiction
- RED test: resume without cycle ID + one active lease.
- Expected current mismatch between skill contract and CLI capability.

### HWP-C3I-02 — Bootstrap application service
- compose project/adoption/cycle/knowledge resolution;
- machine JSON contract;
- explicit ambiguity.

### HWP-C3I-03 — Adoption ensure semantics
- no prompt/no mutation when Complete;
- converge safe Partial;
- block Conflicting.

### HWP-C3I-04 — Update agent skill and workflow contracts
- remove stale recipe;
- add mutation test preventing regression.

## C3j

### HWP-C3J-01 — Storage-shape spike
Determine if capsules/deltas/bindings are events, projections, CAS refs or tables. Exit with one-authority decision.

### HWP-C3J-02 — Persistent CapsuleStore
- create/read latest by basis/run/binding;
- digest/idempotency;
- restart tests.

### HWP-C3J-03 — Persistent SessionBindingStore
- session != run;
- multi-session isolation;
- reattach.

### HWP-C3J-04 — Context bootstrap/expand
- compact capsule;
- resource refs;
- ContextReadRecord.

### HWP-C3J-05 — Hypermedia read-only MVP
- Project/Run/Step/Context representations;
- no mutating actions yet except existing safe read navigation.

## C6a

### HWP-C6A-01 — ResourceRef grammar + versioning
### HWP-C6A-02 — Affordance ADT
### HWP-C6A-03 — StepDefinition ADT + registry
### HWP-C6A-04 — WorkflowDefinition ADT/parser
### HWP-C6A-05 — Contract/type checker skeleton

## C6b

### HWP-C6B-01 — Legacy manifest → normalized WorkflowDefinition adapter
### HWP-C6B-02 — WorkflowDefinition → WorkflowIR compiler
### HWP-C6B-03 — Equivalence model/tests
### HWP-C6B-04 — Close minimum operator gaps for first vertical
### HWP-C6B-05 — Restart/replay on compiled default slice

## C6c

### HWP-C6C-01 — AugmentationPlan ADT
### HWP-C6C-02 — Skill selector integration
### HWP-C6C-03 — Capability merge lattice
### HWP-C6C-04 — ProviderRegistry and resolution receipt
### HWP-C6C-05 — CogniCode capability mappings
### HWP-C6C-06 — Chronos capability mappings
### HWP-C6C-07 — Explain augmentation

## C6d

### HWP-C6D-01 — Observation normalized contract
### HWP-C6D-02 — ContributionReconciler
### HWP-C6D-03 — Tasks → WorkItems/dependencies
### HWP-C6D-04 — Decisions → DecisionRecords
### HWP-C6D-05 — Test/provider outputs → Evidence
### HWP-C6D-06 — ContextDelta from persisted contributions
### HWP-C6D-07 — First software vertical cutover

## C6e

### HWP-C6E-01 — Affordance resolver
### HWP-C6E-02 — Typed problem/recovery model
### HWP-C6E-03 — `resource get/actions` porcelain
### HWP-C6E-04 — generic MCP resource/action tools
### HWP-C6E-05 — Prompt slimming for migrated vertical
### HWP-C6E-06 — Harness black-box hypermedia UAT

## C6f

### HWP-C6F-01 — Workflow/Step registry scope precedence
### HWP-C6F-02 — SubWorkflow compilation/execution contract
### HWP-C6F-03 — Project custom workflow discovery
### HWP-C6F-04 — compatibility/version resolver
### HWP-C6F-05 — custom workflow security/admission UAT

## C7

### HWP-C7A-01 — Software-specific dependency inventory
### HWP-C7A-02 — Software augmentation profile extraction
### HWP-C7A-03 — Software workflows registered as pack resources
### HWP-C7B-01 — Authoring StepDefinitions minimum set
### HWP-C7B-02 — Book WorkflowDefinition
### HWP-C7B-03 — Research/source capability adapter minimum
### HWP-C7B-04 — Book E2E restart/map/loop UAT
### HWP-C7C-01 — Core portability audit
### HWP-C7C-02 — Generality receipt or falsification report

## Priorización

No abrir C6 tasks mientras C3i/C3j blockers de continuity sigan reproduciéndose. Dentro de C6, evitar completar todo el catalog antes del primer vertical E2E: contracts → vertical → generalize.
