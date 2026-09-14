# A1.2 — Agent Asset Inventory and Obsolete-Path Closure

**Audit basis:** `main@438e4de` (regenerated with the C7 receipt at the certified HEAD).
**Closes:** SPEC-013 (`PARTIAL → PASS`), M9 *obsolete monolithic prompt paths*.

## Authoritative assets (active)

| Asset class | Location | Count | Role | Consumers |
|---|---|---|---|---|
| Orchestrator | `prompts/sddk/orchestrator.md` | 1 | entry point | agent runtime / Jcode overlay |
| Phase prompts | `prompts/sddk/**` | 22 | modular phase/contract prompts | sddk-* agents |
| Agents | `agents/*.md` (incl. `agents/sddk-*.md`) | 70 | agent definitions | host runtimes |
| Skills | `skills/*/SKILL.md` | 96 | invocation patterns | host runtimes |
| Schemas/templates | `assets/**` | 6 dirs | JSON schemas, templates | CLI + agents |
| Global contract | `AGENTS.md` | 1 | repo operating rules | humans + agents |
| Typed substrate | `crates/sddk-cli/src/command_spec.rs` | 1 | command/agent-surface source | `sddk introspect`, agent surface |

The canonical prompt boundary is declared in `prompts/sddk/README.md`; the single executable surface is `orchestrator` + `commands/sddk-*` + `prompts/sddk/**` + `agents/sddk-*.md` + `skills/sddk-*`.

## Derived / generated assets (not independent authority)

- `AgentCommandSurface` and its golden fixture — derived from `all_command_specs()`.
- Cheat sheets / examples — `ExampleSpec` carried by `CommandSpec`, walked by the M7.1B walker.
- Allowed-command sets — `SkillRegistry::admits_command(&CommandSpec)`, `AgentProfile::admits(&CommandSpec)`.
- Instruction material — compiled by `instruction_compiler.rs` (prompt text is not architecture; `ADR-0106`).

## Historical assets (superseded, non-authoritative)

The consolidated historical packages under `docs/` listed in `docs/architecture/README.md` (decision-kernel-architecture, 2.0-consolidation, complete-evolution, human-agent-collaboration, etc.). They are provenance only and do not claim current authority.

## Obsolete monolithic prompt paths

- **None active.** `prompts/` contains only the modular `sddk/` tree; no repository-root monolithic prompt (`PROMPT.md`, `prompt.md`, `SDDK_PROMPT.md`) exists.
- Legacy paths may survive only if: (a) needed for compatibility, (b) explicitly marked, (c) non-authoritative, (d) with a demonstrated consumer, (e) with an objective removal trigger. No such monolithic path currently qualifies, so none is retained.

## Negative fixtures (executable)

- `crates/sddk-cli/src/dev/lint/deprecated_patterns.rs::tests::obsolete_monolithic_prompt_paths_are_absent` — asserts `prompts/` has no stray/monolithic file, the boundary doc exists, and no root monolithic prompt exists.
- `...::asset_lints_have_zero_hits_on_active_corpus` — the `agent_assets` lints (`asset_deprecated_namespace`, `asset_raw_store_reference`, `asset_authority_language`) are `default = deny`; 0 hits on the live corpus.
- `...::asset_lint_detects_injected_deprecated_asset` — an injected `SDD-kernel` / authority-bypass asset is detected (proves the path cannot silently regain authority).
- `crates/sddk-cli/tests/dev_lint_e2e.rs::uat20_deprecated_asset_is_rejected_by_enforce_without_mutation` — the real enforcement path (`--enforce`) rejects the workspace deterministically with zero asset mutation.

## Conclusion

SPEC-013 is satisfied: Agent Experience consumes the current typed assets/contracts; no hidden monolithic prompt is authoritative; obsolete paths are provably absent and guarded by `deny` lints with negative fixtures.
