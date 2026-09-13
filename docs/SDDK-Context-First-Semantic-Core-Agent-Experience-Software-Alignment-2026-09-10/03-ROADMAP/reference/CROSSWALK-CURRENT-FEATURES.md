# Crosswalk of current product features into the consolidated architecture

| Current CLI/domain | New home | User-facing direction |
|---|---|---|
| project/adopt | Identity/Application | keep |
| cycle | Delivery compatibility | reduce prominence |
| plan | Planning | keep porcelain |
| run/run-view | Runtime | consolidate under `run/status/next` |
| capability | Governed execution plumbing | keep low-level |
| permission | Authority internals | `policy explain/check` or plumbing |
| approval | Authority facts/workflow | contextual UX, not separate daily domain |
| rules | Policy/architecture rules | split product policy vs dev architecture rules |
| ledger | Canonical facts plumbing | keep debug/export/verify |
| artifact | CAS plumbing | keep debug |
| graph | SemanticGraph plumbing | `why/impact` porcelain |
| stale | Why/impact profile | deprecate top-level once parity achieved |
| explore | View adapter | optional presentation |
| vault | KnowledgeSource | keep human/wiki tooling |
| knowledge | source/context integration | simplify; avoid duplicate store semantics |
| fork | Experiment/replay | keep advanced namespace |
| UAT | Pack | keep pack UX/targets |
| debt | Planning/semantic graph profile | `why debt`, report target |
| metrics | Observability projection | group under observe/plumbing |
| analytics | Observability query | group under observe/plumbing |
| telemetry | Observability ingestion/control-plane | advanced/plumbing |
| release/ship | Target + release capability | porcelain `run ship` / alias `ship` |
| dev doctor/lint | Developer diagnostics | consolidate `doctor`, `dev` advanced |

## Principle

A feature can remain fully capable while losing top-level CLI prominence. Simplification is primarily **semantic and navigational**, not arbitrary feature deletion.


## Agent-facing crosswalk additions

| Current responsibility pattern | Target owner | Disposition |
|---|---|---|
| AGENTS/project rules | ProjectInstructionSet + normative architecture docs | MIGRATE/KEEP source |
| role-specific monolithic prompts | AgentProfile + Skill/Task instructions | SPLIT |
| provider-specific agent prompt identity | AgentProfile + ProviderAdapter | MIGRATE |
| hard-coded CLI recipes | CommandRegistry examples → AgentCommandSurface | ABSORB |
| skill/tool bundle with implicit permissions | SkillDefinition + separate Capability/Authority | SPLIT |
| prompt-level approval/risk behavior | Authority Policy | ABSORB |
| prompt-level workflow sequencing | Target/Task/Runtime | ABSORB |
| chat transcript as resume source | Decision Memory + ContextCompiler | DEPRECATE as authority |
