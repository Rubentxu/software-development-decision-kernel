# SPEC-018 — Documentation and Planning Governance

**Status:** Proposed

## 1. Problem

Operational history, architecture rules, current plans and old audit notes should not compete in the same files. Agent-facing permanent instructions must remain concise and stable.

## 2. Document roles

Recommended separation:

- `AGENTS.md` — stable agent/runtime invariants and navigation only;
- `ARCHITECTURE.md` — current architectural model;
- `DEVELOPMENT.md` — contributor workflow;
- `CONTRACT.md` or ADRs — locked decisions;
- `CHANGELOG.md` — shipped user-facing changes;
- `ROADMAP.md` — current scoped planning cycle;
- `FUTURE_IDEAS.md` — valid unscheduled ideas with revisit triggers;
- `docs/history/` — historical audits/plans;
- XDG session/handoff state — ephemeral agent/session notes.

## 3. Drift checks

Docs that contain generated inventories, command lists, schemas or pack catalogs SHOULD be generated/validated from machine-readable sources. CI/local lint SHOULD fail on deterministic drift where generation is possible.

## 4. Agent surface limits

Keep agent/skill/prompt surfaces short enough to be read reliably. Briefness thresholds may remain as maintainability gates, but reducing prompt size MUST NOT justify moving uncontrolled complexity into giant runtime modules.

## 5. Deferred ideas

Every deferred idea record must include:

- idea;
- why deferred;
- revisit trigger;
- owner/context;
- dependencies/evidence needed.
