---
name: test-pyramid
description: "Trigger: testing pyramid, test strategy, write tests, add tests, integration tests, e2e tests, TDD, test coverage, flaky test, test audit, test plan, edge cases, test design, test quality. Generic testing-pyramid skill that works for any stack (Rust, JS/TS, Python, Go, multi-stack) and any project. Detects the stack, picks the right tools per layer, uses auto-grill-loop to discover edge cases systematically, and fixes bugs found during testing. Use when designing, implementing, or auditing a test pyramid."
license: Apache-2.0
metadata:
  author: gentle-ai
  version: "2.1"
  scope: global
---

# Test Pyramid — Generic Stack-Aware Skill

Single entry point for designing, implementing, and auditing a complete test pyramid across **any stack**. Pairs with the `test-pyramid-builder` primary agent, which combines this skill with `auto-grill-loop` to discover edge cases and fix bugs autonomously.

## Activation Contract

Load this skill whenever the work touches tests, test infrastructure, coverage, or test quality. Activate on any of:

- Designing or auditing a test pyramid.
- Adding tests to a module, component, service, or user flow.
- Choosing a mocking strategy, a test database, or a fixture pattern.
- Debugging flaky tests, slow tests, or weak assertions.
- Reviewing a PR where tests are missing, weak, or wrong-level.
- Generating a list of edge cases to cover.
- **Fixing bugs discovered during testing** — the testing cycle is the prime moment to catch and correct production code issues.

## Stack Auto-Detection (run before planning)

The skill is stack-agnostic. The **agent** (not the skill) detects the stack in the first cycle. The skill lists the patterns to look for:

| Signal in repo | Stack |
|---|---|
| `Cargo.toml` only | Pure Rust |
| `Cargo.toml` with `leptos = ...` or `dioxus = ...` | Rust + WASM frontend |
| `Cargo.toml` with `axum` / `actix-web` / `rocket` | Rust + HTTP API |
| `Cargo.toml` with `tauri` | Rust + desktop app |
| `package.json` with `react` | JS/TS + React |
| `package.json` with `vue` | JS/TS + Vue |
| `package.json` with `svelte` or `sveltekit` | JS/TS + Svelte |
| `package.json` with `next` | Next.js (React + SSR) |
| `package.json` with `nuxt` | Nuxt (Vue + SSR) |
| `package.json` with `astro` | Astro (multi-framework) |
| `requirements.txt` / `pyproject.toml` | Python |
| `pyproject.toml` with `django` | Python + Django |
| `pyproject.toml` with `fastapi` / `flask` | Python + HTTP API |
| `go.mod` | Go |
| `mix.exs` | Elixir |
| `Gemfile` | Ruby |
| `Cargo.toml` + `package.json` + `playwright.config.*` | Full-stack: Rust or Node API + JS frontend + E2E |
| `vercel.json` / `netlify.toml` | JS frontend deployed to edge |
| `docker-compose.yml` with `postgres` / `redis` / `nats` | Polyrepo with infra |
| `testcontainers-*` | Polyrepo with containerized test deps |

For each detected stack, the references in this skill guide the per-layer tool choices.

## The Pyramid (top to bottom)

```
                   ┌─────────────┐
                   │   E2E / a11y│  Few, slow, high-fidelity
                   ├─────────────┤
                   │  API / Int. │  Service boundaries, real DB or fake
                   ├─────────────┤
                   │  Component  │  UI units with real rendering
                   ├─────────────┤
                   │    Unit     │  Pure functions, domain logic
                   └─────────────┘
```

**Rule of thumb**: 70% unit, 20% integration, 10% E2E. If E2E > unit, the suite is slow and brittle. If unit > 95%, you are not testing seams.

## Hard Rules

1. **Test behavior, not implementation.** Assert on public observable output (return value, rendered DOM, network effect, persisted state), not on private fields or call counts on internal helpers.
2. **One assertion concept per test.** Multiple `expect`s are fine if they describe the same behavior; split when you change the system under test.
3. **Tests must be deterministic.** No real `now()`, no real filesystem paths under `~`, no real network, no real third-party APIs. Inject clocks, use temp dirs, use mocks/fakes.
4. **Tests must be isolated.** No shared mutable state between tests. Fresh fixtures per test.
5. **No `unwrap()` in production paths**; `unwrap()` in tests is acceptable when failure should be loud.
6. **Property tests for parsers, serializers, invariants.** Generate inputs, verify invariants, prefer shrinking.
7. **Every bug fix lands with a regression test.** Write the test first, watch it fail, then fix.
8. **Coverage is a floor, not a goal.** 80% line coverage is a starting point; mutation testing is the real signal.
9. **A11y and performance are tests, not afterthoughts.** Every interactive UI has an a11y test; every user-facing page has a perf budget.
10. **No `waitForTimeout` in JS / Playwright tests.** Use `findBy*`, `expect(...).toBeVisible()`, `wait_for` — auto-waiting always.

## Bug Fixing During Testing

Testing is the BEST time to find and fix production code bugs. When tests expose a problem in production code:

### Classification

| Severity | Description | Action |
|---|---|---|
| **Trivial** | Missing field assignment, inconsistent constructor, off-by-one | Fix directly. Write regression test first, watch it fail, then fix. |
| **Moderate** | Logic error, missing validation, race condition | Fix directly. Optionally invoke `auto-grill` for one-shot adversarial review. |
| **Systemic** | Design flaw across modules, duplicated data, missing error handling chain | Invoke `auto-grill-loop-orchestrator` to find ALL related occurrences. Then fix each, delegating complex refactors (>50 LOC) to `sddk-apply`. |

### Workflow

1. **Write a failing regression test** that demonstrates the bug.
2. **Classify severity** using the table above.
3. **If systemic**: invoke `auto-grill-loop-orchestrator` with a topic like "Systemic impact of [bug] in [module/crate]". Read the grill report and fix ALL related issues.
4. **Fix the root cause**, not the symptom. Keep fixes minimal — don't refactor unrelated code.
5. **Verify**: run the failing test → must pass. Run the full suite → must stay green.
6. **Persist**: save bugfix to Engram (`type: bugfix`, `topic_key: test-pyramid/bugs-found`).

### What NOT to do

- Do NOT just document the bug and move on.
- Do NOT write a test that "works around" the buggy behavior.
- Do NOT refactor >50 LOC of production code without delegating to `sddk-apply`.
- Do NOT fix bugs in code you haven't tested — the test proves the fix.

## Edge-Case Discovery with `auto-grill-loop`

This is the **most powerful** capability of the system. Before writing tests for a medium / large scope, invoke the grill.

### When to grill

- Before adding tests to a **new module, service, or feature**.
- After a **bug report** OR **bug found during testing** — the grill surfaces related bugs you didn't think of.
- Before a **release** — coverage audit through the grill.
- When the **test suite is stale** and you want to know what's missing.

### How to grill for edge cases

Formulate a focused topic for the grill. Good topics:

- "Edge cases and failure modes of `UserService::register` in `src/services/user.rs`."
- "Adversarial inputs for the JSON parser in `web-app/src/lib/parse-flow.ts`."
- "Failure modes of the payment webhook handler under retries and partial failures."
- "Systemic impact of duplicated `agent_name` in `ExecutionEvent::AgentReasoning` across hodei-types and consumers."
- "Accessibility gaps in the login form on the web-app."

The `auto-grill-loop-orchestrator` will:
1. Generate questions about claims, decisions, terms, relationships, assumptions.
2. Resolve them via codebase, CONTEXT.md, ADRs, docs, internet (if needed).
3. Challenge each answer with a Skeptic.
4. Judge each with a Judge.
5. Audit coverage and continue until COMPLETE / BLOCKED / MAX_PASSES_REACHED.

The output is a **final report** in `{grill-reports-dir}/{date}-{topic}.report.md` (XDG `cycle-artifacts/{cycle_id}/grill/` under SDDK adoption; `docs/grill/` only standalone) with:
- Accepted decisions (auto-resolved).
- Modified decisions.
- Decisions requiring user validation.
- Rejected alternatives.
- Risks and proposed CONTEXT.md patches.

**Use that report as the input to the test plan.** Each "decision requiring validation" or "rejected alternative" maps to a test case.

### How much to grill

- **Small change** (< 200 LOC): grill 1–2 questions, do it inline.
- **Medium feature** (200–1000 LOC): 1 short grill session (≤ 3 passes).
- **Large module / refactor / new service / bug found**: 1 full grill session (4–6 passes).

If the grill returns BLOCKED, surface it to the user. Do not invent answers.

## Decision Gates (per layer, stack-aware)

The agent chooses the tool by **layer + stack**:

| What you are testing | Rust | JS/TS (React/Vue/Svelte) | Python | Go |
|---|---|---|---|---|
| Pure function | `#[cfg(test)] mod tests` | `*.test.ts` | `*.py` + pytest | `*_test.go` |
| Domain rule / state machine | `#[cfg(test)]` + `proptest!` | `*.test.ts` + `fast-check` | hypothesis | testing + quickcheck |
| HTTP handler / route | `tests/` + `tower::ServiceExt::oneshot` | `*.test.ts` + MSW/Nock | pytest + httpx ASGI client | `httptest` |
| DB integration | `sqlx::test` / testcontainers / `pg_tmp` | prisma/drizzle test client | pytest + testcontainers | dockertest / testfixtures |
| Async with Tokio | `#[tokio::test]` | waitFor / findBy* | pytest-asyncio | goroutine + channel tests |
| UI component | n/a | Testing Library / Vue Test Utils / @testing-library/svelte | pytest-selenium / Playwright Python | n/a |
| User flow (cross-page) | n/a | Playwright / Cypress | Playwright Python | n/a |
| Accessibility | HTML check in unit test | `jest-axe` / `axe-playwright` | `axe-playwright` Python | n/a |
| Visual regression | n/a | Playwright `toHaveScreenshot` | Playwright Python | n/a |
| Performance | `criterion` | Web Vitals + Lighthouse | pytest-benchmark + Locust | go test -bench + pprof |

## Execution Protocol (for the agent)

1. **Detect stack.** Run the auto-detection table. Identify language(s), framework(s), test runner(s), DB(s).
2. **Classify the request.** Where in the pyramid does this test belong? State in one sentence.
3. **Discover edge cases.** If the scope is medium/large, run `auto-grill-loop-orchestrator` on the topic. Read the final report. Otherwise, list 5–10 edge cases inline. Use parallel `explore` sub-agents for reconnaissance across multiple crates.
4. **Pick the smallest public boundary** that exercises the behavior. Test public API, not privates.
5. **Build the fixture** using the layer-appropriate tool. Reuse fixtures.
6. **Name the test by scenario, not by input mechanics.** `rejects_expired_token`, not `test_with_past_date`.
7. **Run the narrowest suite first**, then the layer's suite, then the full pyramid.
8. **For snapshot / golden tests**: regenerate explicitly, inspect the diff, re-run without the flag.
9. **For TDD**: write the test, watch it fail for the right reason, then implement.
10. **Triage failures with bug-fixing workflow.** When a test fails, classify: bug in test → fix test; bug in code → apply Bug Fixing During Testing workflow; environmental → load `diagnose`.
11. **Persist** findings: bugs, fixes, decisions, gotchas to Engram.

## Parallelism Strategy

The testing agent should use parallelism aggressively:

| Work | Strategy |
|---|---|
| Reconnaissance (multiple crates) | Launch multiple `explore` sub-agents simultaneously |
| Test writing (independent files) | Write multiple test files in parallel |
| Test execution (independent crates) | Run `cargo test -p A` and `cargo test -p B` simultaneously |
| Grill + recon | Launch `auto-grill-loop-orchestrator` while exploring code in parallel |

**Rule of thumb**: If two tasks don't depend on each other's output, do them in parallel.

## Anti-Patterns (forbid)

- Testing `mock_called()` instead of real behavior.
- Snapshot-testing everything (snapshots drift, lose signal).
- E2E test for what a 5-line unit test proves.
- `await delay(500)` in JS tests — use `findBy*` with auto-wait.
- Reusing one `tokio::runtime` across tests — use `#[tokio::test]` per test.
- Test that depends on test execution order.
- Hitting a real database, real S3, or real auth provider in CI.
- Unit test for layout/styling, Playwright for serialization edge cases.
- "100% coverage" or "test everything" — negotiate the right shape, not the largest suite.
- `flaky.skip = true` instead of fixing the flake.
- **Documenting a bug instead of fixing it** — when tests find a bug, FIX IT.

## Output Contract

When invoked, produce or update:

- A **layered test plan** mapped to the pyramid.
- A **list of edge cases** sourced from grill output (if used) or inline reasoning.
- **Files created/modified**, one bullet per file with its layer and purpose. Mark production fixes with 🔧.
- **Commands run** to verify each layer (test runner invocations).
- **Bugs found and fixed**: what was wrong, why, what was changed. Severity classification.
- **New fixtures, helpers, or fakes** introduced.
- A short **risk note** if a layer is skipped (e.g., "no E2E for admin panel — covered by integration + manual QA").
- **Engram saves** for significant decisions, gotchas, patterns, bugfixes (architecture, decision, bugfix, pattern types).

## References (on demand, stack-agnostic where possible)

- `references/stack-detection.md` — Detailed auto-detection heuristics.
- `references/edge-case-grill.md` — How to write a grill topic that yields good edge cases.
- `references/quality-and-a11y.md` — Coverage, mutation, a11y, performance budgets.
- `references/skill-matrix.md` — Maps problems to specialist skills.

### Stack-specific references (load only after detection)

- `references/rust-testing.md` — Rust unit/integration/property/bench.
- `references/js-ts-testing.md` — JS/TS with Jest/Vitest/Mocha/uv-vite.
- `references/python-testing.md` — Python with pytest/hypothesis.
- `references/go-testing.md` — Go table-driven/sub-tests.
- `references/e2e-playwright.md` — Playwright E2E (works for any UI stack).
- `references/e2e-cypress.md` — Cypress (alternative to Playwright).

## Specialist Skills to Load (not duplicate — load and follow)

| Situation | Load |
|---|---|
| Rust design, generics, errors, async, API shape | `rust-patterns` |
| Leptos API details | `leptos-guide` |
| Playwright deep reference (any activity) | `playwright-best-practices` |
| Quick browser automation, no test files | `playwright-cli` |
| Ad-hoc local browser checks | `webapp-testing` |
| Reproduce/measure/fix/codify a UI bug | `frontend-evidence-loop` |
| Full-page browser audit with screenshots | `ui-audit-protocol` |
| Layout / spacing / bounding-box audit | `layout-geometry-audit` |
| WCAG 2.2 deep dive | `accessibility` |
| LCP/INP/CLS budgets and measurement | `core-web-vitals` |
| Holistic web quality (perf + a11y + SEO) | `web-quality-audit` |
| Hard flaky test, perf regression, or runtime bug | `diagnose` |
| Splitting tests/docs across commits | `work-unit-commits` |
| Adversarial challenge of a test plan | `grill-me` |
| Deep automated Q&A on a topic (edge cases) | `auto-grill-loop` |
| One-shot adversarial report on a test plan | `auto-grill` |
| Code intelligence (CogniCode MCP) | `cognicode-sdd` |
| Time-travel debugging (Chronos MCP) | `chronos-sdd` |
| Entropy-based design quality (SOLID, connascence) | `entropy-sdd` |
| SDDK full pipeline (proposal → apply → verify) | `orchestrator` |
| Production code fix (complex refactor >50 LOC) | `sddk-apply` |

## Templates (load on demand from `assets/`)

- `assets/pyramid-checklist.md` — Executable audit checklist (stack-agnostic).
- `assets/grill-test-coverage.md` — Prompt template to grill for edge cases.
- `assets/test-strategy-doc.md` — One-page test strategy document template.
- `assets/cargo-test.toml` — Cargo `[dev-dependencies]` + `[profile.test]` baseline.
- `assets/vitest.config.ts` — Vitest config for a JS/TS + Vite project.
- `assets/jest.config.js` — Jest config baseline.
- `assets/pytest.ini` — pytest config baseline.
- `assets/playwright.config.ts` — Playwright config (any stack UI).
