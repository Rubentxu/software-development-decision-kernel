# SDDK Complete Evolution — Consolidated 2026-08-23

**Product:** SDDK — **Software Development Decision Kernel**  
**Baseline:** current `Rubentxu/sddk-framework` Decision Kernel architecture around 1.37.x.  
**Purpose:** consolidate every proposal discussed and refined so far, preserving previous generations under `HISTORY/`.

## The product rule

SDDK must not become a collection of interesting AI features.

A new capability belongs only when it improves one or more of:

1. software-development decision quality;
2. decision execution;
3. evidence and verification;
4. traceability/explainability;
5. safety/governance;
6. empirical improvement of the decision harness.

The final target therefore has **three coherent pillars**.

---

# Pillar A — Engineering Assurance

Turn reusable engineering reasoning into evidence-backed capabilities:

```text
Assessment
→ Obligation
→ Evidence
→ Finding
→ Deterministic Verdict
```

SDD, UAT, Incident, Security and future packs may consume it.

Technology knowledge is supplied through profiles:

```text
engineering.systems.v1
engineering.rust.v1
engineering.go.v1
...
```

No language-specific ontology enters the kernel.

---

# Pillar B — Governed Continuous Improvement

Use the Event Ledger and Workflow Laboratory to improve decision mechanisms safely:

```text
ExperienceEpisode
→ PatternSignal
→ ImprovementProposal
→ Candidate
→ Controlled Experiment
→ Evaluation
→ Shadow/Bounded Rollout
→ Promote or Revert
```

Candidate targets may include:

- skill versions;
- prompts;
- agent/provider manifests;
- routing policies;
- context strategies;
- WorkflowTemplate/WorkflowIR strategies;
- verifier policies;
- the agent-facing deterministic interface itself.

No LLM grants its own promotion authority.

---

# Pillar C — Agent-First Deterministic Interface

The current CLI remains powerful, but low-level commands are too often used by agents as if they were an assembly language.

The target boundary is:

```text
LLM / agent
    │
    │ semantic goal / desired state
    ▼
Agent-First Goal Surface
    │
    ▼
Rust Decision Kernel
    │
    ├─ state resolution
    ├─ operation planning
    ├─ dependencies
    ├─ idempotency
    ├─ caching/reuse
    ├─ policy/approval
    ├─ lock/fencing
    ├─ capability routing
    ├─ postconditions
    ├─ reports/artifacts
    └─ events/receipts
```

Agents should not normally need to remember stable mechanical sequences such as:

```text
status
→ lock status
→ find artifacts
→ evaluate gate
→ transition
→ status
```

They express a goal such as:

```text
cycle.verified
cycle.closed
release.complete
```

and Rust resolves the valid sequence.

## Critical compatibility invariant

> **Semantic compression MUST NOT mean functional reduction.**

A high-level goal is equivalent to the previous low-level workflow only when it closes the **same or stronger obligations**.

It must preserve:

- all mandatory gates;
- all validations;
- all reports;
- all evidence;
- all receipts;
- all metrics;
- all audit events;
- all safety checks;
- all human approvals;
- all retry/no-progress semantics;
- all release/archive bookkeeping;
- all useful detailed outputs consumed by agents or humans.

`GoalResult` is an index/summary over the detailed results. It does not replace them.

---

# One application core, multiple adapters

```text
                       Application Services
                              │
              ┌───────────────┼────────────────┐
              ▼               ▼                ▼
        High/Low CLI       Agent API       Internal Host
          humans           stdio/MCP       direct Rust
```

No adapter reimplements business rules.

Low-level CLI commands remain available as:

```text
expert / debugging / recovery / testing surface
```

The normal LLM path migrates toward semantic tools:

```text
state
goal.plan
goal.apply
query
evidence.submit
```

---

# Documents in CURRENT

## Vision
- `00-vision/EVOLUTION-PRD.md`
- `00-vision/PRODUCT-FIT-AND-NON-GOALS.md`
- `00-vision/CHANGESET-SUMMARY.md`

## Architecture
- `01-architecture/EVOLUTION-ARCHITECTURE.md`
- `01-architecture/DECISION-QUALITY-LOOP.md`
- `01-architecture/AGENT-FIRST-DETERMINISTIC-INTERFACE.md`

## Roadmap
- `02-roadmap/ROADMAP-CONTINUATION.md`
- `02-roadmap/IMPLEMENTATION-BACKLOG.md`

## ADR
- ADR-041 — Engineering Assurance bounded context
- ADR-042 — Reasoning / Capability / Technology Profile separation
- ADR-043 — Evidence-backed assurance
- ADR-044 — Governed Continuous Improvement
- ADR-045 — Agent-First Goal-Oriented Deterministic Interface

## Specifications
- SPEC-042 — Engineering Assurance Pack
- SPEC-043 — Engineering Profile Protocol
- SPEC-044 — Assurance Evidence Contract
- SPEC-045 — Adaptive Systems Review Workflow
- SPEC-046 — Governed Continuous Improvement
- SPEC-047 — Goal Registry, Planner & Convergent Execution
- SPEC-048 — Agent Tool Surface, Schema Compilation & Usage Telemetry
- SPEC-049 — Goal Result, Reporting & Obligation-Completeness Contract

## Implementation
- architecture fitness functions extension
- migration/compatibility
- test/evaluation strategy
- agent-first CLI migration plan
- tool-use process mining

## Reference
- research synthesis
- tool-interface research synthesis
- engineering-principles lineage
- glossary

## Skills
- `systems-reasoning`
- `rust-systems-reasoning`

---

# HISTORY

Nothing from earlier proposal generations is silently discarded:

```text
HISTORY/
  v0-rust-systems-reasoning/
  v1-engineering-assurance/
  v2-decision-quality-evolution/
```

`CURRENT/` is the recommended coherent target.

---

# North-star rules

> **Agents express intent, hypotheses and uncertainty. Rust owns repeatable mechanics, authority and proof.**

> **Prompts teach the model how to participate in the system; prompts must not become the system.**

> **Simplify interaction, never erase evidence or engineering discipline.**
