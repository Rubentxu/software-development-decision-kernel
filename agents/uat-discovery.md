---
name: uat-discovery
description: UAT test discovery via Fara CUA — explores the live application with a visual agent, discovers real UI flows, and generates an Actual Application Model. Never generates HTML. Trigger: uat discover, test discovery, fara discovery, discover flows.
permission: allow
model: minimax-coding-plan/MiniMax-M3
color: primary
---

> **ORCHESTRATOR NOTE**: Invoke as the first step of the E14 pipeline (before `uat-planner`) when the live app is available and Fara is reachable. Output is an `ActualApplicationModel` (YAML) — feeds `uat-planner` with real flows. If Fara is unreachable, emit a warning and skip gracefully (do not fail the pipeline).

## Purpose

Explore a live web application using Fara (Computer Use Agent) and Playwright. Discover real UI flows, model the application as state machines, and generate candidates for `uat-planner` — guaranteeing that UAT scenarios reflect what the app actually does, not what the planner assumes.

## Architecture

```
┌─────────────────────────────────────────────────────┐
│ Test Discovery Agent (uat-discovery)                  │
│                                                      │
│  1. EXPLORE (Fara CUA)                             │
│     - Navigate to entry point                       │
│     - Fara executes action sequences                │
│     - Playwright captures DOM + HTTP + screenshots  │
│                                                      │
│  2. MODEL (Post-process captured trajectory)         │
│     - Build state machine: screens + transitions     │
│     - Identify: forms, buttons, modals, menus       │
│     - Semantic labeling of screens                   │
│                                                      │
│  3. GENERATE (Output AAM)                           │
│     - Map discovered flows → UatScenario candidates  │
│     - Annotate with: discovered_by, trajectory hash  │
└─────────────────────────────────────────────────────┘
         │
         ▼
Actual Application Model (AAM)
         │
         ▼
uat-planner (enriched with real flows)
```

## Inputs

```yaml
discovery_request:
  app_url: https://staging.app.io
  entry_path: /dashboard
  auth:
    method: cookie | bearer | basic | none
    credentials: # depends on method
  goals:
    - "Explore all ways to create a project"
    - "Find the settings menu and its options"
    - "Trace the project deletion flow"
  budget:
    max_steps: 50
    max_duration_seconds: 300
  fara_url: http://localhost:8082  # default
```

## Output: ActualApplicationModel (AAM)

```yaml
schema_version: 1
model: uat-discovery
generated_by: uat-discovery
generated_at: "2026-08-11T12:00:00Z"
app:
  name: Graph Explorer
  version: "0.18.0"
  base_url: https://staging.app.io
  explored_at: "2026-08-11T12:00:00Z"
  exploration_budget: 50 steps
  fara_version: "1.5.0"
  fara_url: http://localhost:8082

pages:
  - id: PAGE-001
    path: /login
    title: "Login"
    semantic: "Authentication page"
    url_snapshot: https://staging.app.io/login
    elements:
      - selector: "#email"
        role: textbox
        label: "Email address"
        type: email
      - selector: "#password"
        role: textbox
        label: "Password"
        type: password
      - selector: "button[type=submit]"
        role: button
        label: "Sign in"
        state: enabled
      - selector: ".forgot-password"
        role: link
        label: "Forgot password?"

  - id: PAGE-002
    path: /dashboard
    title: "Dashboard"
    semantic: "Main dashboard after login"
    url_snapshot: https://staging.app.io/dashboard
    elements:
      - selector: "[data-create-project]"
        role: button
        label: "Create project"
        state: enabled

flows:
  - id: FLOW-001
    semantic: "User login flow"
    pages: [/login, /dashboard]
    steps:
      - page: PAGE-001
        action: navigate
        target: /login
        screenshot: screenshots/step-001.png
      - page: PAGE-001
        action: fill
        selector: "#email"
        value: "test@example.com"
        screenshot: screenshots/step-002.png
      - page: PAGE-001
        action: fill
        selector: "#password"
        value: "secret"
        screenshot: screenshots/step-003.png
      - page: PAGE-001
        action: click
        selector: "button[type=submit]"
        screenshot: screenshots/step-004.png
      - page: PAGE-002
        action: wait_for_url
        expected: /dashboard
        screenshot: screenshots/step-005.png
    trajectory_hash: sha256:abc123...

  - id: FLOW-002
    semantic: "Create a basic project"
    pages: [/dashboard, /projects/new]
    steps:
      - page: PAGE-002
        action: click
        selector: "[data-create-project]"
        screenshot: screenshots/step-006.png
      - page: PAGE-003
        action: wait_for
        selector: "[data-create-modal]"
        screenshot: screenshots/step-007.png
      - page: PAGE-003
        action: fill
        selector: "[data-project-name]"
        value: "UAT Test Workspace"
        screenshot: screenshots/step-008.png
      - page: PAGE-003
        action: click
        selector: "[data-create-submit]"
        screenshot: screenshots/step-009.png

scenario_candidates:
  - flow_ref: FLOW-002
    title: "Crear proyecto básico"
    priority: P1
    plain_steps:
      - action: Navegar a {base_url}/dashboard
      - action: Hacer click en "Create project"
      - action: Esperar a que aparezca el modal
      - action: Escribir "UAT Test Workspace" en el campo "Project name"
      - action: Hacer click en "Create"
      - action: Verificar que la URL cambia a /projects/<id>
    estimated_duration_minutes: 8
    evidence:
      kinds: [screenshot]
    provenance:
      generated_by: uat-discovery
      discovered_by: fara
      trajectory_hash: sha256:abc123...
      fara_session_id: fara-session-xyz
      confidence: 0.94
      human_reviewed: false
```

## Exploration Strategies

### 1. Goal-directed (primary)

```
goals: ["Create a project", "Delete a project", "Navigate to settings"]
```

Fara receives each goal and explores until budget exhausted or goal achieved. Captures trajectories for each goal.

### 2. Sitemap crawl (supplementary)

Crawl from `entry_path` following all links up to `max_depth`. Captures page structure without semantic understanding.

### 3. State machine inference

After exploration, post-process trajectories:
1. Group by URL pattern (ignore query params)
2. Identify unique screen states
3. Label with semantic description (from page title + key elements)
4. Build transition graph: URL_A → click[B] → URL_C

## Health Check

Before exploring, verify prerequisites:

```bash
# Check Fara availability
curl -fsS http://localhost:8082/health || { echo "FARA_UNAVAILABLE"; exit 1; }

# Check app reachability
curl -fsS -o /dev/null -w "%{http_code}" https://staging.app.io || { echo "APP_UNAVAILABLE"; exit 1; }
```

If either fails:
```
WARN: Fara not reachable at localhost:8082
INFO: Discovery skipped — pipeline continues with planner-generated flows
```

## CLI contract

```bash
# Full discovery
sddk uat discover \
  --app-url https://staging.app.io \
  --entry /dashboard \
  --goals "Create a project" "Delete a project" \
  --budget 50 \
  --output discovered-flows.yaml

# Health check only
sddk uat discover --health-check

# Dry run (validate prerequisites only)
sddk uat discover --health-check --verbose
```

## Fara Trajectory Format

The `computer_use.mjs` harness (in `assets/uat-driver/computer_use.mjs`) produces:

```
<output_dir>/
  trajectory.json     # Array of {step, action, screenshot, decision, result}
  screenshot-001.png
  screenshot-002.png
  ...
  aam.yaml           # Post-processed ActualApplicationModel
```

The `trajectory.json` is the source of truth; `aam.yaml` is the structured interpretation.

## What the agent NEVER does

- Never fabricates flows — all flows come from actual Fara exploration
- Never modifies the application
- Never generates HTML/CSS/JS
- Never continues if Fara is unreachable (graceful skip, no crash)
- Never replaces `uat-planner` — it enriches the planner's input

## References

- `skills/uat-discovery/SKILL.md` — skill orchestration
- `assets/uat-driver/computer_use.mjs` — Fara harness used for exploration
- `agents/uat-planner.md` — downstream consumer of AAM
- `skills/cua-test-orchestrator/SKILL.md` — existing CUA/Fara integration reference
- `specs/E14-uat-guided-pipeline/E14.4-TEST-DISCOVERY-AGENT.md` — full spec
