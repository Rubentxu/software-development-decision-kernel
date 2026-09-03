# STATUS — SDDK Complete Evolution Pack 2026-08-23

> **Original validated baseline:** approximately v1.37.x
> **Current assessment baseline:** v1.70.0 / 2026-09-03
> **Canonical roadmap:** `../sddk-decision-kernel-architecture/02-roadmap/ROADMAP.md`
> **Crosswalk:** `../sddk-decision-kernel-architecture/02-roadmap/EVOLUTION-CROSSWALK.md`

## Current disposition

This pack is **retained as a source dossier**. It must not be executed as one monolithic continuation roadmap.

Its three pillars now have different dispositions:

1. **Agent-First Interface (AFI):** largely `ABSORBED` by later facade, goal, state-driven CLI and recovery work; remaining AgentHost semantics are remapped.
2. **Engineering Assurance (EA):** still valuable; retained as H6 cross-cutting assurance capability.
3. **Governed Continuous Improvement (GCI):** still valuable, but intentionally deferred until workflow/event/replay semantics are stable.

## AFI classification

| Original intent | Status | Canonical destination |
|---|---|---|
| semantic facade/project inputs | `ABSORBED` | existing facade/project-input implementation |
| deterministic goal semantics | `ABSORBED/PARTIAL` | current goal/facade/runtime contracts |
| reduce low-level CLI probing by agents | substantially `ABSORBED` | state-driven CLI + H3 Decision Plane |
| DecisionSnapshot | intent retained, original shape not canonical | H3 `DEC-PLANE-004` |
| semantic AgentHost tool surface | `REMAINING` | North Star `AGENT-HOST` + H3 parity |
| cross-project semantic tooling | `REMAINING` | later AgentHost work |
| telemetry for semantic tool use | `PARTIAL` | H5/H6 |
| process mining over agent/tool activity | `REMAINING/DEFERRED` | H6 `GCI-LEARNING` |

### AFI decision

Do **not** implement AFI-001..010 verbatim. Re-open an AFI item only when its acceptance intent is demonstrably missing from current abstractions and can be mapped to a semantic Work Item in the canonical backlog.

## Engineering Assurance classification

**Status:** `REMAINING`, strategically valid.

Canonical epic: H6 `EA-ASSURANCE`.

Retain the useful concepts:

- assurance profiles and rules;
- typed evidence;
- resolvers;
- deterministic evaluators;
- capability/risk-specific assurance;
- UAT integration;
- explainable verdicts;
- policy/provenance outputs.

Refinement: EA should consume the common runtime, evidence/event model and Decision Plane. It should not introduce a second execution engine or competing workflow lifecycle.

Contracts may be specified earlier if they help H0-H5 design, but runtime integration should target the architecture proven by H2-H5.

## Governed Continuous Improvement classification

**Status:** `REMAINING`, deliberately `BLOCKED` by runtime/event maturity.

Canonical epic: H6 `GCI-LEARNING`.

Retain:

- ExperienceEpisode-like projections;
- process mining;
- strategy quality/cost comparison;
- bounded experiments;
- evidence-backed promotion;
- rollback and policy ratchets.

Refinement: learning observes canonical execution evidence. It must never become an alternate source of runtime truth or bypass deterministic policy/human authority.

## Why the original sequence is no longer canonical

Since the pack baseline, SDDK has shipped several capabilities that collapse parts of the old AFI ladder:

- first-class facade/project-input behavior;
- goal/facade parity work;
- state-driven context inference;
- workflow-driven `cycle next`;
- actionable recovery contracts;
- pause/resume lifecycle substrate.

Executing the old AFI ladder literally would therefore duplicate already-delivered intent and distract from the current architectural bottleneck: durable generated workflow execution and a unified Decision Plane.

## Execution rule

Use this directory for rationale, detailed examples and acceptance ideas, but schedule work only through the canonical backlog:

- remaining semantic AgentHost work → later North Star / H3 integration;
- EA → H6 `EA-ASSURANCE`;
- GCI/process mining → H6 `GCI-LEARNING` after H5 runtime/replay stability.
