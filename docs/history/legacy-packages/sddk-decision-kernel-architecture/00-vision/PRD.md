# PRD — SDDK Software Development Decision Kernel

## Product statement
SDDK is a local-first, event-sourced, reactive control plane that coordinates software-engineering workflows across humans, agentic IDEs, models, providers and deterministic tools while preserving decisions, evidence, provenance and operational history.

The 2026 dynamic-workflow refinement adds a further product rule: **SDDK governs engineering invariants, while the concrete execution strategy may be generated and expanded dynamically at runtime**.

## Primary users
- software engineers using one or more agentic IDEs;
- platform/DevOps/AI engineers governing agent workflows;
- technical leads reviewing decisions and evidence;
- QA/UAT participants performing guided acceptance;
- teams that need reproducibility, supply-chain provenance and controlled automation.

## Core jobs to be done
1. Continue important work when a model/provider/IDE route fails or exhausts quota.
2. Understand what ran during a work session and why.
3. Execute different engineering workflows on one reusable runtime.
4. Adapt workflow depth and parallelism to goal, risk, uncertainty and runtime discoveries without giving an LLM unrestricted execution authority.
5. Give agents only the context and authority they actually require.
6. Validate outcomes with deterministic checks, agents and humans.
7. Reconstruct, replay, fork and compare execution histories.
8. Inspect all of this locally without operating another server.

## Functional requirements

### FR1 Workflow runtime
Generic durable workflow definitions, node runs, attempts, dynamic graph expansion, parallelism, map/fan-out, joins, choices, loops, waits, approvals, retries, cancellation, subworkflows and compensation.

### FR2 Workflow templates & compiler
Stable workflow templates define goals, invariants, policies and allowed capabilities. A validated Workflow Compiler produces a provider-neutral `WorkflowIR`. The runtime may expand the execution graph from validated expansion commands/events.

### FR3 Supervisor
Cognitive goal interpretation, workflow shaping and replanning separated from deterministic orchestration. The Supervisor may propose an execution plan; it does not become the scheduler.

### FR4 Multi-host agent execution
Bidirectional adapters for agentic IDEs; OpenCode is first reference integration.

### FR5 Resilient routing
Logical agent independent from host/provider/model; health-aware route selection and failover.

### FR6 Reactive events
Canonical Event Ledger, reaction levels, deterministic behaviors and cognitive signals. Dynamic workflow expansion is evented and replayable.

### FR7 Context compiler
Context Capsules, deltas, staleness, read tracing and negative knowledge.

### FR8 Evidence/governance
Proposal → Policy → Approval → Capability → Verify → Receipt.

### FR9 UAT
Execution, evidence, oracles, human decision, defect/retest, sign-off and release gates.

### FR10 Control plane
Static Cockpit with Journal, timelines, graph lenses, workflow execution graph, usage, provider health and causal traces.

### FR11 Supply chain
Track source → build → artifact → SBOM → attestation → promotion/deployment/release lifecycle.

### FR12 Evaluation & Workflow Laboratory
Per-capability outcome metrics plus controlled comparison of workflow strategies. Support baseline-vs-adaptive runs, ablation tests, fork/replay/diff and routing/workflow policy feedback.

### FR13 Invariant-driven SDD
Provide `sdd-adaptive` as an experimental alternative to the phase-heavy SDD path. It MUST preserve intent, acceptance, architecture constraints, implementation verification, evidence and governance while allowing fewer handoffs and dynamically selected specialists.

## Non-functional requirements
- local-first and offline-capable for core persistence/inspection;
- deterministic control semantics where an LLM is unnecessary;
- event replay and projection rebuildability;
- explicit idempotency and effect governance;
- no kernel dependency on a specific IDE/provider/storage adapter;
- no arbitrary LLM-generated shell/script execution as the workflow authority;
- bounded context/token usage;
- transparent unknown metrics rather than fabricated estimates;
- high testability using fakes and fault injection;
- backwards-compatible migration path from current SDDK.

## Success metrics
- provider failure recovery rate;
- percentage of operational failures resolved without Supervisor LLM call;
- mean time to understand a failed workflow from Cockpit/Journal;
- first-pass verification by capability/route;
- duplicated context/read reduction on retries;
- percentage of privileged effects with verified receipts;
- projection replay correctness;
- number of workflow packs running without kernel special cases;
- adaptive-vs-baseline quality delta;
- tokens, latency and cost per accepted change;
- number of agent handoffs per accepted change;
- convergence rounds before acceptance;
- invariant/evidence coverage at workflow completion.

## Non-goals for first releases
- replacing Git as code authority;
- building a hosted SaaS control plane;
- implementing every IDE at once;
- turning every event into autonomous agent activity;
- storing private chain-of-thought;
- using the graph as the authoritative transaction store;
- replacing all canonical workflows with generated workflows before evaluation proves the benefit;
- executing unconstrained JavaScript/Python produced by a model as kernel workflow logic.
