---
name: core.workflow-orchestration
description: "Trigger: orchestrating an SDDK cycle through explore → specify → build → verify → release → archive. Use when planning, sequencing, or supervising a multi-phase workflow that must end in a durable release receipt."
disable-model-invocation: true
user-invocable: false
license: Apache-2.0
metadata:
  author: SDDK Team
  version: "1"
  delegate_only: true
---

## Activation Contract

Activated automatically by the `sddk cycle` CommandSpec via
`with_required_skill("core.workflow-orchestration@v1")`. The runtime
admission gate requires this skill to be present on disk; without
it, the gate emits a `MissingPlaceholderSkill` warning.

## When this skill applies

- Any agent or operator is about to start, transition, pause, or
  archive a cycle.
- Any workflow needs to be sequenced against the canonical
  Goal → WorkItem → WorkflowDefinition → ExecutablePlan → Run chain
  (per ADR-0096 / SPEC-002).

## Evidence it expects

- `agents/orchestrator.md` (the canonical orchestrator protocol)
- `AGENTS.md` §1 (session pre-flight + orchestrator gate)
- `docs/architecture/specs/arch-spec-002-lifecycle-model.md`
  (the slim Cycle contract)
- `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/03-ROADMAP/ROADMAP.md`
  (M0..M9 sequencing — the live authority)

## Pattern of orchestration

1. **Pre-flight**: `sddk version`, `sddk adopt status --root . --scope .`,
   `git fetch && git checkout main && git pull --ff-only`. Reject if
   not adopted or HEAD diverged.
2. **Cycle start**: `sddk cycle start --name <slug> --path <a-min|a-lite|a-full|b-direct>`.
3. **Phase progression**: for each transition, fetch the frontier
   with `sddk cycle next`, authorize the gate with
   `sddk cycle evaluate-gate`, apply with `sddk cycle transition`
   passing `--artifact` and `--gate-receipt` evidence.
4. **Burst mode**: when many cycles ship in sequence (M7.x..M9.x style),
   document any ledger gap explicitly in a handoff and do NOT
   retroactively backfill events.
5. **Closure**: `sddk-archive` writes the archive-manifest,
   `sddk-release` ships the binary, `sddk dev install` updates
   the local bundle, `sddk dev update --prune-only` cleans stale
   versions.

## Hard rules

- Skill ≠ Capability (SPEC-016): enabling this skill does NOT grant
  any new side-effect permission. The runtime AuthorityEngine gate
  still applies.
- Prompt text is never architecture (SPEC-014): if the orchestrator
  protocol needs to change, edit `agents/orchestrator.md` and the
  InstructionCompiler pipeline, not this skill.
- Never fabricate `cycle.start` / `cycle.transition` ledger events
  retroactively. If the cycle was skipped during a burst, document
  it in the handoff and move on.
