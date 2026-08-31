# SDDK Prompts

This folder is the canonical prompt tree for the SDDK flow coordinated by `orchestrator`.

## Boundary

SDDK uses one executable surface:
- `orchestrator`
- `commands/sddk-*.md`
- `prompts/sddk/**`
- `agents/sddk-*.md`
- `skills/sddk-*`

Phase prompts and shared runtime contracts must use these canonical names directly. Do not add aliases or parallel flow namespaces.

## Design Goal

SDDK uses an explicit decision kernel:

```text
session gates
  -> context quality gate
  -> problem taxonomy
  -> mandatory protocols
  -> adaptive lenses
  -> escalation engine
  -> delivery gates
```

The objective is not to run more agents. The objective is to decide when extra context, entropy analysis, architecture lenses, or grilling are worth their cost.

## Non-Goals

- Do not put kernel rules in global `AGENTS.md`.
- Do not make `entropy-sdd`, `auto-grill`, or `grill-with-docs` globally mandatory.
- Do not use `CONTEXT.md` as a spec, scratch pad, or architecture report.
