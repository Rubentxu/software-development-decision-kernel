# Test Strategy — One-Page Template

> Copy this file to `docs/test-strategy.md` at the project root. Fill in the brackets. This is the single page the team points to when asked "what's our testing approach?".

# Test Strategy — {project_name}

**Last updated**: {YYYY-MM-DD}
**Owner**: {team or person}

## Stack

{Language(s), framework(s), test runner(s), DB(s), infra.}

Example: "Rust workspace with `axum` + `sqlx` + `tokio`; React 18 + Vite + Jest; Playwright E2E; Postgres + NATS via docker-compose."

## The pyramid (target shape)

- **Unit**: {target %} of suite count, {target} runtime, tool: {cargo test / jest / pytest / go test}.
- **Component**: {target %} of suite count, tool: {RTL / Leptos view! / Testing Library}.
- **Integration**: {target %} of suite count, tool: {in-process HTTP client / MSW / testcontainers}.
- **E2E**: {target %} of suite count, tool: {Playwright / Cypress}, one test per critical user journey.

## Commands (canonical)

| Need | Command |
|---|---|
| Run all unit + component + integration | `{command}` |
| Run a single test by name | `{command with pattern}` |
| Run E2E smoke | `{command with @smoke grep}` |
| Run E2E full | `{command}` |
| Run with coverage | `{command}` |
| Run benchmarks | `{command}` |

## Per-layer rules

- **Unit**: pure functions, no IO, deterministic.
- **Component**: real rendering, no real network (use {MSW / Nock / msw-py}).
- **Integration**: real DB (ephemeral) or fake at the trait boundary.
- **E2E**: real stack, mocked third parties, tagged for grep-based subset runs.

## Coverage gates

- Unit: ≥ {80}% lines, ≥ {75}% branches.
- Component: ≥ {80}% lines.
- Integration: scenario coverage (named list).
- E2E: journey coverage (named list).

## Forbidden patterns

- `waitForTimeout` / `time.sleep` / `setTimeout` in tests.
- Real network, real DB, real third-party APIs in unit / component.
- Snapshot tests for entire pages.
- Test that depends on test execution order.
- `mock_called()` assertions that prove nothing about the user.

## Edge-case discovery

- For new modules / services: run `auto-grill-loop-orchestrator` with a topic from `assets/grill-test-coverage.md`.
- For bug fixes: write the regression test first, then fix.
- For coverage audits: use `assets/pyramid-checklist.md`.

## Performance budgets

- LCP: ≤ {N}ms on 4G / Moto G4.
- INP: ≤ {N}ms.
- CLS: ≤ {N}.
- Backend p95: ≤ {N}ms per endpoint.

## A11y gate

Every interactive component has a `jest-axe` / `axe-playwright` test. No `critical` or `serious` violations block the build.

## CI

- {list of CI jobs that run on PR}
- {list of CI jobs that run on main}
- {flaky-test policy}

## References

- Skill: `test-pyramid` (this skill)
- Specialist skills: {list of skills the team uses for testing}
- Internal docs: {link to ADRs, runbooks, examples}

## Change log

- {YYYY-MM-DD} — {change} — {author}
- {YYYY-MM-DD} — {change} — {author}
