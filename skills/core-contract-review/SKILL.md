---
name: core.contract-review
description: "Trigger: contract review — auditing command examples, agent surface, registry invariants, or skill/CommandSpec claims against the canonical SPEC documents. Use before declaring a feature shipped or before merging into `main`."
disable-model-invocation: true
user-invocable: false
license: Apache-2.0
metadata:
  author: SDDK Team
  version: "1"
  delegate_only: true
---

## Activation Contract

Activated automatically by the `sddk lint` and `sddk capability`
CommandSpecs via `with_required_skill("core.contract-review@v1")`.
The runtime admission gate requires this skill to be present on
disk; without it, the gate emits a `MissingPlaceholderSkill`
warning.

## When this skill applies

- Auditing whether a CommandSpec example still parses after a CLI
  change (SPEC-015 "examples are executable contracts" invariant).
- Auditing whether a `SkillDefinition` honors Skill ≠ Capability
  (SPEC-016 anti-patterns).
- Auditing whether the `SemanticGraphProjection` includes the
  required nodes/edges for a cycle (SPEC-005).
- Reviewing architecture drift after a release (AGENTS.md §2.10
  "exactly one current roadmap").

## Evidence it expects

- `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-016-SKILL-CONTRACT.md`
  (skill contract — the authority this skill enforces)
- `docs/architecture/specs/arch-spec-015-command-registry-and-agent-surface.md`
  (CLI docs from one source — invariant #10)
- `docs/architecture/specs/arch-spec-016-skill-contract.md` (mirror)
- `docs/architecture/specs/arch-spec-013-agent-experience-contract.md`
  (AgentExperienceContract)
- `AGENTS.md` §2.7..§2.10 (semantic ownership, agent experience,
  extension discipline, consolidation rule)

## Pattern of review

1. **Identify the contract under review** (SPEC-NNN, ADR-NNNN,
   AGENTS.md §X.Y) and read it in full before touching code.
2. **Run the executable evidence first**: `cargo test --workspace`,
   `cargo clippy --workspace --all-targets -- -D warnings`,
   `cargo fmt --check`, `sddk dev skills verify`,
   `sddk cycle status`. Capture receipts as evidence.
3. **Spot-check at least one representative example** per claim
   the contract makes (SPEC-015 invariant #11).
4. **Cross-reference the canonical roadmap** at
   `docs/architecture/README.md` to ensure the row you are
   touching is consistent with the latest shipped release.
5. **Report findings as a contract delta**: what is honored, what
   is broken, what is missing. Never silently patch around a
   contract violation — fix the contract or fix the code, never
   both at once without a deliberate proposal.

## Hard rules

- Skill ≠ Capability (SPEC-016): this skill reviews contracts; it
  does not grant the right to relax them.
- The reviewer MUST cite the exact SPEC / ADR / AGENTS.md section
  for every finding. Findings without a citation are not findings.
- Do not invent or fabricate ADRs/SPECs that don't exist. If the
  contract is missing, open a vault INC (e.g.
  `INC-VAULT-PHANTOM-...`) and file a real follow-up cycle.
