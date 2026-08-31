# Test Pyramid Audit Checklist

Run this checklist against an existing suite to find gaps, misplacement, and flakiness risk. Do not check items off if you are not sure — investigate first.

This checklist is **stack-agnostic**. Use the stack-specific reference for the layer you're auditing.

## 1. Pyramid shape (read coverage report, not just lines)

- [ ] **Unit tests** are the majority by count and runtime.
- [ ] **Integration tests** cover the seams (HTTP, DB, network) without being E2E.
- [ ] **E2E tests** number in the dozens, not hundreds. One per critical user journey, max.
- [ ] **Component tests** cover every interactive UI component.
- [ ] **Property tests** exist for parsers, serializers, and state machines.
- [ ] **Benchmarks** exist for any code path called out as perf-critical.

## 2. Per layer — pick the reference for your stack

The checklist items below are stack-agnostic. For per-language tool choices, see:
- `references/rust-testing.md`
- `references/js-ts-testing.md`
- `references/python-testing.md`
- `references/go-testing.md`
- `references/e2e-playwright.md` or `references/e2e-cypress.md`

### 2.1 Unit / component

- [ ] Tests live next to the code (or in `__tests__` / `tests/` per stack convention).
- [ ] No shared mutable state between tests.
- [ ] `describe` / `mod tests` / `TestXxx` per unit; sub-tests by scenario.
- [ ] No `unwrap()` in production paths; acceptable in tests when failure should be loud.
- [ ] No `fireEvent` in JS — use `userEvent`.
- [ ] No CSS class selectors in queries; use semantic / `getByRole` / `getByLabelText` / `getByTestId` (last resort).
- [ ] No `waitForTimeout` / `time.sleep` in tests.
- [ ] No real filesystem paths under `~`; use `tmp_path` / `t.TempDir` / `tempfile`.
- [ ] No real network; use `MSW` / `responses` / `respx` / `httptest` / `wiremock`.
- [ ] No `#[ignore]` / `t.Skip` / `xit` / `test.skip` without a TODO + owner.
- [ ] Coverage threshold set (≥ 80% lines) and enforced in CI.

### 2.2 Integration / API

- [ ] HTTP tests use the framework's in-process client (`TestClient` for FastAPI, `httptest` for Go, `tower::ServiceExt::oneshot` for axum, MSW for JS).
- [ ] DB tests use ephemeral DBs (`sqlx::test`, `testcontainers`, `pytest-postgresql`, `dockertest`), not a shared dev DB.
- [ ] Migrations are run as part of the test setup, not assumed.
- [ ] No test hits a real third-party API.
- [ ] Async tests use the right event loop per stack (`#[tokio::test]`, `pytest.mark.asyncio`, `t.Parallel`, etc.).
- [ ] One runtime / event loop per test — no shared global runtime.

### 2.3 E2E (Playwright / Cypress)

- [ ] `webServer` config spins up the full stack (backend + frontend).
- [ ] `getByRole` / `getByLabelText` everywhere; CSS selectors only with a comment.
- [ ] No `waitForTimeout` (Playwright auto-waits).
- [ ] Tests are tagged (`@smoke`, `@critical`, `@slow`) and runnable via grep.
- [ ] At least one cross-browser project (chromium + firefox or webkit).
- [ ] At least one mobile project for any layout-sensitive page.
- [ ] `axe-playwright` runs on every critical page; `critical` + `serious` violations fail the build.
- [ ] Visual regression (`toHaveScreenshot`) covers the chrome (header, sidebar).
- [ ] Traces are saved on retry.
- [ ] Flake budget: < 1% per build. Track and triage.

## 3. Cross-cutting

- [ ] Vulnerability audit in CI (`cargo audit` / `npm audit` / `pip-audit` / `govulncheck`); high/critical block the build.
- [ ] A regression test lands with every bug fix (in the layer it actually broke).
- [ ] Tests run in CI on every PR and on `main` after merge.
- [ ] Test report is published (Playwright HTML, `cargo test --no-fail-fast`, junit output).
- [ ] LCP/INP/CLS budgets are measured at the Playwright layer and tracked over time.
- [ ] The test pyramid is reviewed at every sprint planning, not just on incident.
- [ ] Property tests exist for parsers and serializers (`proptest` / `fast-check` / `hypothesis`).
- [ ] Mutation testing is run on critical modules (`cargo mutants` / `stryker-js` / `mutmut`).
- [ ] Race detection enabled in CI for the languages that support it (`-race` in Go, `tokio` in Rust).

## 4. Process

- [ ] Tests live in the same commit as the behavior they verify (see `work-unit-commits`).
- [ ] PRs > 400 lines are split into chained PRs (see `chained-pr`).
- [ ] New contributors can run the test pyramid locally in < 5 minutes (cold cache).
- [ ] There is a "what to do when a test fails in CI" runbook.
- [ ] Test flake is tracked (e.g., `flaky-tests.md`) and triaged weekly.
- [ ] A test is removed when the behavior is removed (no orphan tests).

## Scoring (rough)

- All items checked → A+ (likely over-engineered somewhere; look for low-value tests).
- 1–3 unchecked → B (good; pick the lowest-cost one to fix this sprint).
- 4–8 unchecked → C (acceptable for early-stage; pick the three that hurt most).
- 9+ unchecked → D (do not ship features until fixed).

## Re-running

Re-audit when:
- A new module or service is added.
- A new framework or test runner is adopted.
- A significant incident reveals a testing gap.
- After every major release.

Use `auto-grill-loop` (load the `auto-grill-loop` skill) to surface gaps you may have missed in this checklist.
