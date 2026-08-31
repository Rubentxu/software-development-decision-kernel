---
name: uat-dashboard
description: "Trigger: uat-dashboard, generar dashboard UAT, render dashboard. Generate or extend the self-contained UAT HTML dashboard from uat-plan.yaml using the bundle dashboard kit."
disable-model-invocation: true
user-invocable: false
license: MIT
metadata:
  author: sddk-framework
  version: "1.0"
  delegate_only: true
---

> **ORCHESTRATOR GATE**: If you loaded this skill, STOP. Delegate to `uat-planner`.

## Purpose

Generate the self-contained UAT dashboard (ADR-013) from a canonical `uat-plan.yaml`. Rendering is deterministic. The guided view is served from a loopback-only same-origin server so it can ingest the exported session; other views remain standalone HTML.

## Commands

```bash
# Render the dashboard (writes HTML, opens nothing).
sddk uat dashboard --plan uat-plan.yaml --view guided --output uat-guided.html

# Render AND open the dashboard in the system browser. Guided mode starts a
# loopback-only server for same-origin ingest and stays alive until Ctrl+C.
sddk uat open --plan uat-plan.yaml --view guided
sddk uat open --release v1.5.0           # auto-resolves uat-plan-v1.5.0.yaml
sddk uat open --release v1.5.0 --browser firefox   # override launcher
sddk uat open --release v1.5.0 --view matrix --theme light

# Matrix (senior, TestRail-style table)
sddk uat dashboard --plan uat-plan.yaml --view matrix --output uat-matrix.html

# Report view (architect traceability; uses uat-report.yaml when present)
sddk uat dashboard --plan uat-plan.yaml --view traceability --output uat-report-view.html

# Theme
sddk uat dashboard --plan uat-plan.yaml --theme light --output uat-guided.html
```

## Platform launchers (`uat open`)

`uat open` resolves the platform launcher via `cfg!(target_os)`:

| OS | Command |
|----|---------|
| Linux | `xdg-open <path>` |
| macOS | `open <path>` |
| Windows | `cmd /c start "" <path>` |

Override with `--browser <cmd>` (useful in containers or to pin a browser).

If the launcher fails (no display server, headless, missing tool), the command prints the resolved HTML path so the user can open it manually.

## Kit layout (bundle, ADR-013)

```
assets/uat-dashboard/
├── kit/          tokens.css, components.css, components.js, storage.js
├── views/        guided.html, interactive.html, report.html
└── themes/       dark.css, light.css
```

## Rules

- The plan is the single source of truth; the dashboard is derived, never edited by hand.
- `flags` are semantic (smoke/warning/optional/data-verify) — the template decides the style. Never put style in the YAML.
- Validate first: `sddk uat validate --file <plan>` — a failing plan must not render.
- Evidence pasted in the browser (Ctrl+V) is stored in localStorage and exported as session JSON; guided mode posts it to the local ingest endpoint.
- Finalization exports every scenario. Unvalued scenarios use `NOT_RUN`, never an implicit PASS.

## References

- ADR-013 (dashboard kit) in the knowledge vault
- `agents/uat-planner.md` — the plan contract
