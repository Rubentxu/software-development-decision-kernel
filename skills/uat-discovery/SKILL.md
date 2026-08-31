---
name: uat-discovery
description: "Trigger: uat discover, test discovery, fara discovery, discover flows, actual application model. Explore live app with Fara CUA, discover real UI flows, and generate ActualApplicationModel for uat-planner."
disable-model-invocation: false
user-invocable: true
license: Apache-2.0
metadata:
  author: sddk-framework
  version: "1.0"
---

## Purpose

Explore a live web application using Fara (Computer Use Agent) and Playwright to discover real UI flows. Generate an ActualApplicationModel (AAM) that feeds `uat-planner` with verified flows — not assumptions.

## Prerequisites

1. **Fara server running** at `http://localhost:8082`:
   ```bash
   # Check health
   curl -fsS http://localhost:8082/health
   ```

2. **Application under test** accessible at the target URL

3. **Authentication credentials** (if required) — provided via CLI flags or environment

## Invocation

```bash
# Full discovery pipeline
sddk uat discover \
  --app-url https://staging.app.io \
  --entry /dashboard \
  --goals "Create a project" "Delete a project" "Access settings" \
  --budget 50 \
  --output discovered-flows.yaml

# Health check only
sddk uat discover --health-check

# Goal-directed exploration with specific auth
sddk uat discover \
  --app-url https://staging.app.io \
  --entry /dashboard \
  --auth-method cookie \
  --auth-file ~/.config/sddk/test-credentials.env \
  --goals "Complete a checkout flow" \
  --output checkout-flows.yaml
```

## Exploration Flow

### Phase 1: Health Check

```bash
curl -fsS http://localhost:8082/health
# Expected: {"status":"ok","model":"fara-1.5"}

curl -fsS -o /dev/null -w "%{http_code}" https://staging.app.io
# Expected: 200
```

If health check fails:
```
WARN: Fara not reachable at localhost:8082
INFO: Discovery skipped — pipeline continues with planner-generated flows
```
Exit code: 0 (graceful skip, do NOT fail the pipeline).

### Phase 2: Fara Exploration

The skill invokes `computer_use.mjs` harness for each goal:

```bash
node assets/uat-driver/computer_use.mjs \
  --url https://staging.app.io \
  --goal "Explore all ways to create a project" \
  --output /tmp/discovery-run-001 \
  --fara-url http://localhost:8082 \
  --max-steps 50
```

For each goal, Fara:
1. Navigates to entry URL
2. Observes current state (screenshot + DOM)
3. Decides next action (think phase)
4. Executes action (click, fill, navigate)
5. Repeats until goal achieved or budget exhausted

Output per goal: `trajectory.json` + `screenshot-*.png` files.

### Phase 3: AAM Generation

Post-process all trajectories into `ActualApplicationModel`:

```yaml
# discovered-flows.yaml
schema_version: 1
model: uat-discovery
pages:
  - id: PAGE-001
    path: /dashboard
    semantic: "Main dashboard"
    elements: [...]
flows:
  - id: FLOW-001
    semantic: "Create project"
    steps: [...]
scenario_candidates:
  - title: "Crear proyecto"
    plain_steps: [...]
    provenance:
      generated_by: uat-discovery
      discovered_by: fara
      confidence: 0.94
```

## Integration with Pipeline (E14.5)

In the E14 pipeline:

```
uat-discovery → uat-planner (enriched with AAM) → ux-form → quality → validate → approve
```

The `discovered-flows.yaml` is passed to `uat-planner` as an additional input:

```bash
sddk uat plan \
  --requirements ./docs/ \
  --changelog CHANGELOG.md \
  --last-plan uat-plan-v1.9.0.yaml \
  --discovered-flows discovered-flows.yaml \
  --output uat-plan-enriched.yaml
```

If discovery was skipped (Fara unavailable), `uat-planner` runs without `--discovered-flows` and emits:
```
⚠ Flows not verified against live app — some scenarios may be stale
```

## Output Files

| File | Location | Purpose |
|------|----------|---------|
| `discovered-flows.yaml` | CWD or `--output` path | AAM for uat-planner |
| `trajectory-*.json` | `/tmp/uat-discovery-<session>/` | Raw Fara trajectories |
| `screenshots/` | `/tmp/uat-discovery-<session>/` | Screenshots per step |

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Discovery complete (or gracefully skipped if Fara unavailable) |
| 1 | App unreachable or unexpected error |

## References

- `agents/uat-discovery.md` — full agent definition
- `assets/uat-driver/computer_use.mjs` — Fara harness
- `agents/uat-planner.md` — downstream consumer of AAM
- `skills/cua-test-orchestrator/SKILL.md` — existing Fara integration reference
- `specs/E14-uat-guided-pipeline/E14.4-TEST-DISCOVERY-AGENT.md` — full spec
