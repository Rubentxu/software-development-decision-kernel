# Glossary

**AgentHost** — adapter boundary for an agentic IDE/runtime such as OpenCode.

**Attempt** — one physical execution route for a logical NodeRun.

**Active Graph** — event-derived graph projection of current/historical knowledge and causality, inspired by ActiveGraph principles.

**Behavior** — reactive unit that consumes events/projection state and emits commands/events/signals.

**Capability** — semantic action requested by a workflow, independent of concrete agent/tool.

**Circuit Breaker** — deterministic state preventing repeated requests to a failing route/provider.

**Context Capsule** — structured, bounded and traceable context package for an execution.

**Control Plane / Cockpit** — independent operational visibility layer over persisted SDDK data.

**Event Journal** — human-readable temporal projection of canonical events.

**Event Ledger** — append-only operational history and source of truth for SDDK runtime state.

**Execution Route** — concrete combination of host, logical agent, provider/model/credentials and constraints.

**Logical Agent** — stable role/capability profile, independent of model/provider.

**Moldable View** — task-specific projection/lens over the same underlying graph/history.

**Negative Knowledge** — explicitly recorded disproven hypotheses/rejected alternatives to avoid repeated work.

**NodeRun** — logical execution instance of a WorkflowDefinition node.

**OrchestratorSignal** — typed event-derived request for cognitive Supervisor reasoning.

**Pack** — extension bundle containing workflows, capabilities, agents, schemas, policies, behaviors and views.

**Receipt** — verified immutable record that a governed side effect was authorized and occurred.

**Supervisor** — global cognitive coordinator; reasons about goals/replanning but does not own deterministic scheduling.

**Workflow IR** — generic versioned workflow representation executed by the kernel runtime.

## Dynamic workflow refinement

**WorkflowTemplate** — Stable declaration of workflow intent, invariants, policies and allowed capability surface.

**WorkflowIR** — Validated provider-neutral executable plan compiled from a template and planning context.

**ExecutionGraphRevision** — Versioned durable revision of a running workflow graph after validated dynamic expansion.

**ExpansionProposal** — Typed proposal to add/alter allowed runtime work; not executable authority until validated.

**ChangeContract** — Semantic source of truth for one software change: intent, requirements, acceptance, constraints, decisions, verification and work decomposition.

**WorkUnit** — Logical unit discovered/planned for execution; may map to one NodeRun, a map branch or a subworkflow.

**SHAPE / BUILD / CONVERGE / INTEGRATE** — Four macro-stages of experimental `sdd-adaptive`.

**Workflow Laboratory** — Evaluation layer for baseline/adaptive/fork/ablation comparison using common quality and efficiency metrics.

## Test-tooling boundary (per ADR-0069, ADR-042)

**Test-tooling ownership** — Policy declaring which language owns which test surface. Four cells: Rust (binary behavior/CLI contracts), Shell (bootstrap/installer/Podman/TUI smoke), Python (external golden/evaluation/analytical assets), JavaScript (frontend, reserved). See [ADR-0069-test-tooling-ownership.md](../adr/ADR-0069-test-tooling-ownership.md).

**Bats reassessment** — ADR-0069 finding that Bats is not a strategic default for this repository; reconsider only when a genuine shell-boundary test cannot be expressed in shell + Rust without losing measurable coverage.

**Evaluator-optimizer pattern** — Workflow pattern where a sizing advisory (evaluator) projects forecast/budget and an optimizer recommends next steps, without blocking. Replaces the prior `circuit-advisor`/`circuit-breaker` pattern. Per `[[ADR-0070-sizing-budgets-advisory]]` (reside en el vault XDG; resuélvelo con `sddk knowledge path`).

**Instruction contract matrix** — Canonical table in [`skills/_shared/cli-usage-contract.md`](../../skills/_shared/cli-usage-contract.md) defining CLI command intent, owner_role, command, required_inputs, expected_output, side_effects, idempotence, and next_handoff. All phase prompts reference matrix rows by anchor.

**Circuit-advisor** — Deprecated pattern name. Superseded by `evaluator-optimizer` per ADR-0070 and the sizing advisory routing in A-* workflows. References to `circuit-advisor` in phase prompts should be replaced with `evaluator-optimizer`.
