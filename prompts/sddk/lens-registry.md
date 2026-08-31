# SDDK Lens Registry

This registry belongs to the canonical SDDK flow.

## Purpose

Use this registry to select architecture and design lenses from the problem taxonomy. The goal is not to apply every framework. The goal is to apply the smallest set of lenses that changes a proposal or design decision.

## Registry Contract

This follows the same discipline as the skill-registry pattern:
- The registry is an index and router, not a compiler of skill rules.
- The source of truth for an installed skill remains its `SKILL.md`.
- Do not paste compact skill summaries into phase prompts when delegation is possible.
- When a lens delegates to a skill, pass the exact `SKILL.md` path if known.
- If a matching skill is absent, use the lens' custom kernel heuristic and mark that explicitly.
- If multiple skills match, prefer project-level skills over user-level skills, then prefer the most specific trigger over broad architecture advice.
- If no lens applies, record `None` and the skip reason so agents stop searching blindly.

Delegators may use the installed skill registry (via the `skill-registry` skill) when present. If it is missing, use the known installed skills from the current OpenCode session and the lens rows below.

## Base Discipline

These checks are always active in kernel `propose` and `design`:
- Context discipline: respect project `CONTEXT.md`, `CONTEXT-MAP.md`, ADRs, specs, and code evidence.
- Module depth: use `improve-codebase-architecture` vocabulary: Module, Interface, Seam, Adapter, Depth, Leverage, Locality.
- Entropy envelope: produce a risk-appropriate connascence/SOLID entropy summary.
- Invariant preservation: every known rule needs an enforcement point or explicit unknown.
- Review budget: avoid designs that create unreviewable change sets.

## Effort Gate

Use `Recommended Effort` to throttle lens work:

| Effort | Meaning | Lens Action |
|--------|---------|-------------|
| skip | Context is sufficient and low risk. | No adaptive lens. Record skipped reasons. |
| verify | Context is strong but a claim needs checking. | Verify only the dominant lens evidence. Do not invent new architecture. |
| deepen | Context has gaps or material risk. | Apply one or two dominant lenses. Return decision impact. |
| recommend-lenses | Multiple hard trade-offs or low context quality. | Recommend lens set, escalate unresolved choices, and use auto-grill when justified. |

## Lens Output Contract

Every applied or skipped lens returns this compact shape:

```markdown
### Lens: {lens-id}
- Status: skipped / verified / deepened / escalated
- Delegation: {exact SKILL.md path, installed skill name, or custom kernel heuristic}
- Why Applied: {taxonomy signal or skip reason}
- Evidence: {paths, docs, ADRs, tests, traces, or explicit unknowns}
- Decision Impact: {what changed in proposal/design or None}
- Risks: {remaining risk or None}
- Escalation: {question, ADR candidate, auto-grill, or None}
```

## Lenses

### domain-modeling

- Trigger: new or changed domain concepts, ambiguous terms, invariants, aggregates, subdomains, lifecycle rules, business events.
- Skip when: request is simple CRUD, technical plumbing, or the domain language is unchanged and already C2/C3.
- Delegate: `grill-with-docs`; future DDD strategic/tactical lens if installed.
- Sources: Eric Evans, Vaughn Vernon, Alberto Brandolini, Mathias Verraes, Rebecca Wirfs-Brock, Nick Tune.
- Evidence: `CONTEXT.md`, `CONTEXT-MAP.md`, ADRs, specs, scenarios, code names, event names, business rules.
- Output: resolved terms, unresolved ambiguities, candidate bounded contexts, aggregate/invariant candidates, capability names.
- Escalate when: glossary conflicts with code/user language, invariant ownership is unclear, or a domain decision is hard to reverse.

### boundary-seam-depth

- Trigger: shallow modules, pass-through layers, hard-to-test behavior, unclear seams, adapter variation, scattered caller knowledge.
- Skip when: only one adapter exists and no meaningful variation or test surface problem exists.
- Delegate: `improve-codebase-architecture`.
- Sources: John Ousterhout, Michael Feathers, Alistair Cockburn, Robert C. Martin, Jeffrey Palermo.
- Evidence: affected modules, caller count, tests, adapter count, deletion test, interface burden.
- Output: proposed seam, interface shape, adapter role, deletion-test result, depth/locality/leverage assessment.
- Escalate when: moving the seam changes ownership, persistence model, external contract, or large review budget.

### clean-hexagonal-dependency

- Trigger: dependency direction issues, framework leakage into domain/application policy, multiple entry points, ports/adapters discussion.
- Skip when: existing architecture intentionally follows a simpler convention and the change does not worsen dependency direction.
- Delegate: `improve-codebase-architecture`; optional external architecture-patterns skill only if installed.
- Sources: Robert C. Martin, Alistair Cockburn, Jeffrey Palermo, Ivar Jacobson.
- Evidence: import graph, package/module structure, framework calls, persistence access, adapter interfaces.
- Output: dependency-rule check, port/adapter placement, layer or slice ownership, test boundary.
- Escalate when: enforcing the dependency rule conflicts with current project structure or ADRs.

### grasp-responsibility

- Trigger: responsibility placement is unclear, behavior is split from data, controller/use-case ownership is vague, polymorphism/protected variation decisions appear.
- Skip when: code is primarily functional/dataflow and GRASP vocabulary would force fake objects.
- Delegate: custom kernel heuristic until a trusted GRASP skill exists.
- Sources: Craig Larman, Rebecca Wirfs-Brock.
- Evidence: collaborators, data ownership, behavior ownership, use-case flow, variation points.
- Output: Information Expert, Creator, Controller, Low Coupling, High Cohesion, Polymorphism, Protected Variations assessment.
- Escalate when: responsibility assignment changes public interfaces or aggregate boundaries.

### connascence-coupling

- Trigger: changes must be coordinated across modules, shared names/types/values/order/timing/algorithms, hidden assumptions, brittle integration.
- Skip when: coupling is local, explicit, low degree, and inside one module implementation.
- Delegate: `entropy-sdd`; custom connascence lens for focused analysis.
- Sources: Meilir Page-Jones, Jim Weirich.
- Evidence: call graph, data flow, schema fields, message contracts, test failures, magic values, execution order.
- Output: connascence pairs with type, strength, degree, locality, and risk; critical pairs over threshold.
- Escalate when: hidden meaning/timing connascence crosses seams or external contracts.

### solid-entropy

- Trigger: extension points, switch growth, substitutability, interface pollution, dependency inversion, unstable abstractions.
- Skip when: SOLID would create ceremony around simple procedural or data-transform code.
- Delegate: `entropy-sdd`; optional SOLID skill only if installed and relevant to the language/style.
- Sources: Robert C. Martin, Bertrand Meyer, Barbara Liskov.
- Evidence: variation axes, interface consumers, subtype contracts, dependency direction, change history.
- Output: SRP/OCP/LSP/ISP/DIP risk summary and entropy budget impact.
- Escalate when: OCP/LSP/ISP risk would make the design hard to evolve or test.

### api-interface-contract

- Trigger: public API, schema, event contract, CLI, SDK, external caller, compatibility, versioning, error model.
- Skip when: interface is private and caller count is low.
- Delegate: `design-an-interface` when multiple interface shapes matter.
- Sources: David Parnas, Martin Fowler, John Ousterhout.
- Evidence: caller needs, compatibility constraints, schemas/types, error modes, performance expectations.
- Output: 2-3 interface options when useful, caller burden, compatibility risk, recommended contract.
- Escalate when: changing the interface breaks external users or persisted data.

### event-cqrs-temporal

- Trigger: temporal history, domain events, async workflows, eventual consistency, read/write model tension, outbox/saga decisions.
- Skip when: synchronous CRUD satisfies the invariant and no temporal audit/history is needed.
- Delegate: `grill-with-docs` for event language; future event/CQRS lens if installed.
- Sources: Alberto Brandolini, Vaughn Vernon, Martin Fowler, Gregor Hohpe, Martin Kleppmann, Adam Bellemare.
- Evidence: commands, events, state transitions, consistency requirements, integration points, failure modes.
- Output: command/event vocabulary, aggregate boundary impact, read model needs, outbox/saga/compensation recommendation.
- Escalate when: consistency model or event meaning requires product/domain decision.

### refactor-legacy-migration

- Trigger: legacy module, big-ball-of-mud, strangler opportunity, risky migration, feature flag, incremental replacement.
- Skip when: change is small and can be completed in one safe work unit.
- Delegate: `improve-codebase-architecture`; `auto-grill` for risky migration choices.
- Sources: Martin Fowler, Michael Feathers, Sam Newman, Neal Ford, Rebecca Parsons, Patrick Kua, Pramod Sadalage.
- Evidence: affected paths, characterization tests, seams, rollout constraints, rollback path, review budget.
- Output: migration slice plan, characterization tests, strangler seam, rollback and review strategy.
- Escalate when: migration changes data ownership, deployment topology, or public behavior.

### socio-technical-flow

- Trigger: ownership seams, team cognitive load, platform vs stream alignment, bounded context/team mismatch, flow bottlenecks.
- Skip when: change is local and team ownership is obvious.
- Delegate: custom heuristic unless a trusted flow/team-topologies skill is installed.
- Sources: Matthew Skelton, Manuel Pais, Simon Wardley, Susanne Kaiser, Nick Tune.
- Evidence: owners, deploy boundaries, on-call responsibility, platform dependencies, cognitive load signals.
- Output: ownership risk, platform/stream interaction, candidate team-aligned seam, flow risk.
- Escalate when: architecture choice requires team or ownership decision.

### adr-governance

- Trigger: hard-to-reverse decision, surprising choice, real trade-off, conflict with existing ADR.
- Skip when: decision is obvious, reversible, or only implementation detail.
- Delegate: `grill-with-docs`; `auto-grill` when unresolved options need ranking.
- Sources: Michael Nygard, Martin Fowler, Michael Keeling, Joel Parker Henderson.
- Evidence: alternatives, constraints, consequences, ADR history, risk, reversibility.
- Output: ADR candidate summary or skip reason.
- Escalate when: user approval is required to accept or supersede an ADR.

### c4-communication

- Trigger: architecture needs communication across audiences, system/container/component boundaries are unclear, diagrams would reduce ambiguity.
- Skip when: the design is local and text is clearer than a diagram.
- Delegate: `skills/sddk-c4-likec4/SKILL.md` when architecture impact or an explicit C4 request justifies it.
- Sources: Simon Brown.
- Evidence: audience, system scope, containers, modules, runtime/deployment relationships.
- Output: recommended C4 level and scope, or validated/fallback output from the
  optional skill. Preserve observed/planned/actual state and evidence coverage.
- Escalate when: diagram exposes disputed ownership or system boundary.

## Skill Trust Notes

- Installed and preferred: `improve-codebase-architecture`, `grill-with-docs`, `auto-grill`, `entropy-sdd`, `design-an-interface` when available.
- Public skills with meaningful adoption can be considered later: `mattpocock/skills@improve-codebase-architecture`, `wshobson/agents@architecture-patterns`, `ccheney/robust-skills@clean-ddd-hexagonal`, `ramziddin/solid-skills@solid`.
- No trusted public skill was found for `connascence` or `GRASP`; keep those as kernel-owned lenses until a better community skill exists.
