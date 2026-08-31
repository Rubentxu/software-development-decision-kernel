---
name: ui-audit-protocol
description: Trigger: audit UI, review frontend, visual QA, browser verification, responsive check, layout review, frontend verification. Enforces evidence-driven browser auditing with screenshots, viewports, console checks, geometry checks, and severity-based reporting.
license: MIT
metadata:
  author: OpenCode
  version: "2.0"
---

## Activation Contract

Use when the task is to judge a web UI from actual browser behavior rather than source code alone.

## Hard Rules

- Runtime evidence is mandatory for any meaningful UI conclusion.
- Do not approve from DOM shape alone.
- Do not approve from a single viewport when responsiveness matters.
- Horizontal overflow is always a finding unless explicitly intended.
- Important truncation, overlap, or clipped actions are always findings.
- Treat console errors as warnings at minimum unless proven irrelevant.

## Tool Matrix

| Goal | Primary tool | Secondary tool | Notes |
| --- | --- | --- | --- |
| Fast manual recon | `playwright-cli` | browser MCP/session tool | Best for snapshots, console, quick navigation |
| Codified regression guard | repo Playwright tests | `playwright-cli generate-locator` | Put important checks into versioned tests |
| Custom dynamic inspection | `webapp-testing` | repo temporary scripts | Use only when CLI flow becomes too constrained |
| Accessibility automation | `@axe-core/playwright` in repo tests | manual keyboard pass | Automated scans do not replace manual review |
| Geometry evidence | `playwright-cli snapshot --boxes` | Playwright `boundingBox()` | Prefer measured findings over taste |
| Debugging hard failures | Playwright trace | console/requests | Capture traces for flaky or multi-step bugs |

## Required Evidence

Audit these viewports unless the user explicitly narrows scope:

- `390x844`
- `768x1024`
- `1366x768`
- `1920x1080`

Collect as applicable:

- screenshots or snapshots
- console output
- request failures relevant to UI behavior
- measured bounding boxes or widths/heights
- accessibility scan results or keyboard evidence

## Decision Gates

| Situation | Action |
| --- | --- |
| App cannot be started or reached | report blocker, include exact failing step |
| Responsive issue suspected | measure overflow/boxes, do not rely on screenshots alone |
| Accessibility issue suspected | combine automated scan with manual focus/name checks |
| Visual inconsistency only | classify as warning or suggestion unless it blocks use |
| Fix requested | reproduce -> patch minimally -> re-verify -> add regression test |

## Execution Steps

1. Resolve target route and runtime entrypoint.
2. Start or confirm the app is healthy.
3. Stabilize the page state before collecting evidence.
4. Inspect required viewports.
5. Capture browser evidence.
6. Run geometry checks for suspicious areas.
7. Run accessibility checks when relevant.
8. Classify findings by severity.
9. Recommend specific Playwright coverage.

## Severity Rubric

- `critical`: broken navigation, hidden actions, overlap, severe responsive breakage, inaccessible primary flow
- `warning`: inconsistent layout, visible drift, console error with UX impact, recoverable accessibility issue
- `suggestion`: polish, clarity, maintainability, refinement

## Output Contract

Return:

- target URL/route and mode
- evidence collected
- findings grouped by severity
- recommended tests
- final verdict

If no evidence was collected, say so explicitly and explain why.

## References

- `references/tooling-matrix.md`
- `references/evidence-checklist.md`

---

## CUA Test Mode (experimental)

For the `cua-test-orchestrator` agent family: the Output Contract and Severity Rubric above are reused unchanged, but the Tool Matrix is replaced because the orchestration cannot navigate the browser.

### When to use CUA Test Mode

- Caller is `cua-test-orchestrator`, `cua-test-runner`, `cua-test-judge`, or `cua-test-scenarist`.
- Inputs are **static assets only**: PNG screenshots, HTML, CSS, JS source, or text descriptions.
- No URL is fetched; no Playwright; no `control-browser`; no `node_repl`; no `fara-cli` subprocess.
- Fara 1.5 9B is invoked exclusively as an LLM via `POST http://localhost:8082/v1/chat/completions` by `cua-test-runner`.

### Tool Matrix (CUA Test Mode)

| Goal | Tool |
|---|---|
| Inspect static asset (image, HTML, source) | `read`, `glob`, `grep` |
| Generate acceptance criteria | `cua-test-scenarist` subagent |
| Ask Fara to evaluate a single criterion | `cua-test-runner` subagent (model: `llamacpp/Fara1.5-9B`) |
| Synthesize Fara responses against rubric | `cua-test-judge` subagent |
| Persist artifacts | `edit`/`write` under `tests/cua/**` and `docs/cua/**` only |
| Verify Fara server is up | `bash: curl http://localhost:8082/health` |
| Probe Fara response shape | `bash: curl http://localhost:8082/v1/models` |

### Hard rules (CUA Test Mode)

1. **No browser automation** under any subagent. `control-browser`, `playwright-cli`, and `node_repl` are not allowed in the runner/judge/scenarist prompts.
2. **Fara is invoked only by `cua-test-runner`**, only via HTTP, only with `temperature: 0` and `max_tokens: 200`.
3. **The orchestrator must check** `curl /health` before dispatching the runner. If the server is down, abort the entire loop and report `server_down`.
4. **Reuse the same severity rubric** as the standard audit (`critical` / `warning` / `suggestion`). Findings that depend on dynamic interaction are not assignable to a severity and are skipped with a `skip_reason`.
5. **The Output Contract is the same shape**: target URL/route (passed through, not fetched), evidence collected (asset paths), findings grouped by severity, recommended tests (Playwright tests the user should run manually outside this skill), final verdict.

### Return Envelope compatibility

`cua-test-judge` returns a `JudgeVerdictEnvelope` (see `agents/cua-test-judge.md`). The orchestrator normalizes it to the standard Output Contract: findings become `critical_findings`, `overall_verdict` becomes `final verdict`, `overall_score` is appended as a numeric field.
