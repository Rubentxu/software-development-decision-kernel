---
id: ADR-0129-SINGLE-CONFIG-RESOLVER
status: accepted
supersedes_history: false
proposed_at: 2026-09-17
accepted_at: 2026-09-17
accepted_by_cycle: p-63676b11dc0ef88f/a5-config-resolver
implementation_evidence:
  - "crates/sddk-engine/src/orchestration_config.rs (single resolver: model keys, laws, forbidden keys, index v1, profile chain, writer)"
  - "crates/sddk-cli/src/config_cmd.rs (sddk config resolve|laws|profiles|set|clear)"
  - "crates/sddk-engine/tests (17 pins in orchestration_config) + config_cmd tests (11 pins)"
  - "~/.jcode/bin/sddk-mode + ~/.jcode/bin/sddk-config (delegation shims, no local resolution)"
  - "docs/architecture/specs/arch-spec-049-sddk-configuration-model-v1.md (model authority)"
superseded_by: []
---

# ADR-0129 — Single configuration resolver; the prompt consumes, never decides

> Contract of record: `arch-spec-049-sddk-configuration-model-v1`.

## Context

SDDK orchestration behaviour (adoption mode, autonomy, reporting, workflow,
parallelism, git policy) had grown across several surfaces: a bash adoption
script, a prompt overlay with hardcoded rules, and ad-hoc expectations in
agent/skill documents. Three resolvers of the same question is a divergence
waiting to happen.

## Decision

1. **One resolver.** `sddk_engine::orchestration_config` is the only place
   that resolves mode + profile + policy keys. `sddk config resolve` exposes
   it; `~/.jcode/bin/sddk-mode` and `~/.jcode/bin/sddk-config` are shims that
   delegate and add no logic.
2. **Law: consumers MUST NOT independently resolve SDDK configuration.** The
   prompt (jcode overlay, orchestrator prompt, skills) *consumes* the resolved
   values; it never re-decides them.
3. **Prompt layers.** (1) core orchestrator laws, non-configurable;
   (2) effective config, already resolved; (3) workflow context. The prompt
   defines **how to obey**, the configuration defines **what**.
4. **Provenance is visible.** Every resolved key reports its source
   (`system-law`, `profile:<name>`, `builtin`), so a `system-law` value is
   visibly non-overridable.
5. **Laws sit above profiles.** Non-overridable keys (git push/tag/release/
   history-rewrite = `human_gate`, `evidence.*`) are imposed by the resolver;
   a profile that sets one is rejected (fail closed).
6. **Subagent scoping.** Workers receive only task, ownership, stage,
   verification policy and architectural laws — not onboarding, global
   personality, or unrelated git policy. The orchestrator stays the authority.

## Consequences

- Behaviour is changed through the declaration (`sddk config set` / profiles),
  never by editing prompt text or a shell script.
- `resolve` makes configuration inspectable end to end; no magic defaults.
- Adding a new option requires extending arch-spec-049 **and** the resolver
  together, which prevents options accreting one-by-one in the overlay.
- Compatibility: the `mode-index` v1 format is backward compatible
  (`<id> <mode>` still resolves), and the mode token is `undeclared`
  (previously written `no-declarado`).

## Non-goals

Nested-YAML profiles, dynamic reload mid-session, per-key workspace overrides,
and the reserved namespaces (`persistence.*`, `observability.*`, `tooling.*`,
`security.*` beyond the laws) are explicitly out of scope for v1.
