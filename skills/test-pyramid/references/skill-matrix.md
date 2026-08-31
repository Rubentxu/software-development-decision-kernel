# Skill Matrix — which specialist skill to load

This is the index that maps a concrete problem to the skill you should load next. The main `SKILL.md` is the entry point; these are the specialists.

## By problem

| Problem | Load this skill | Why |
|---|---|---|
| Choosing Rust error type (`thiserror` vs `anyhow`) | `rust-patterns` (Ch. 9) | Decision guide + examples |
| Async Rust / Tokio test hangs | `rust-patterns` (Ch. 16) | Pin, Send, cancellation safety |
| SQLx migration test fails on CI | `rust-patterns` (Ch. 14) + `diagnose` | Test isolation, runtime per test |
| Leptos signal not updating in test | `leptos-guide` | Signals, ownership, `create_runtime` |
| React form re-renders too much | `frontend-design` | Render optimization patterns |
| Flaky Playwright test | `playwright-best-practices` (flaky-tests) + `diagnose` | Trace, race, isolation |
| Visual regression introduced | `playwright-best-practices` (visual-regression) + `frontend-evidence-loop` | Snapshot strategy |
| Axe finds a serious violation | `accessibility` | WCAG 2.2 remediation |
| LCP regression in production | `core-web-vitals` + `playwright-best-practices` (performance) | Measure, optimize, codify |
| Test review: weak assertions | `grill-me` | Adversarial challenge of test quality |
| Need a deep Q&A on edge cases | `auto-grill-loop` | Multi-agent autonomous interview |
| Need a one-shot adversarial report | `auto-grill` | Single-pass grilling |
| Need to ship a new test infra PR | `work-unit-commits` + `chained-pr` | Commit slicing for review |
| Frontend bug found, need to debug live | `frontend-evidence-loop` + `ui-audit-protocol` | Reproduce → measure → fix |
| Layout / spacing issue | `layout-geometry-audit` | Bounding-box audit |
| Performance regression on a Rust hot path | `rust-patterns` + `diagnose` + `criterion` | Profile, fix, regression-test |
| New teammate onboarded to test pyramid | `onboard` agent (orchestrator-level) | Walk the real codebase |
| Design-level testability analysis | `entropy-sdd` | SOLID-entropy, connascence |
| Code-aware refactor across many files | `cognicode-sdd` (CogniCode MCP) | Safe refactor, impact analysis |
| Time-travel debugging a flaky test | `chronos-sdd` (Chronos MCP) | Capture, replay, find race |
| Need to change production code to make it testable | `sddk-apply` (via `orchestrator`) | SDDK flow |

## By stack layer

| Layer | Primary skills | Supporting skills |
|---|---|---|
| Rust unit | `rust-patterns` | — |
| Rust integration / DB | `rust-patterns` | `diagnose` |
| JS/TS unit + component | (this skill) | `accessibility` |
| Leptos unit + component | `leptos-guide` | (this skill) |
| Playwright E2E | `playwright-best-practices` | `frontend-evidence-loop`, `accessibility` |
| Cypress E2E | (this skill) | `frontend-evidence-loop` |
| Python pytest | (this skill) | `diagnose` |
| Go testing | (this skill) | `diagnose` |
| Performance | `core-web-vitals`, `criterion` | `diagnose` |
| A11y | `accessibility` | `web-quality-audit` |
| Process / commit slicing | `work-unit-commits` | `chained-pr`, `branch-pr` |

## By phase of the SDD cycle

| Phase | Load |
|---|---|
| Spec / propose | (orchestrator's choice) |
| Apply (write code + tests) | `test-pyramid` + the relevant layer skill |
| Verify | `playwright-best-practices` (or appropriate E2E skill) + `diagnose` if a failure needs investigation |
| Archive | (orchestrator's choice) |

## Decision rule

If a problem fits **exactly** in a specialist skill, load only that skill — do not also load the main one. Load the main `test-pyramid` skill when:
- Designing or auditing a whole layer of the pyramid.
- Unsure which layer a test belongs in.
- Onboarding someone to a project's testing approach.
- You need the **edge-case grill** flow (use this skill + `auto-grill-loop` together).
- The project is **multi-stack** and you need stack-aware detection.

## Skill loading order (cheapest first)

1. `test-pyramid` (this skill) — the entry point and stack detector.
2. The stack-specific reference (e.g., `references/rust-testing.md`) — only after detection.
3. The specialist skill for the layer (e.g., `playwright-best-practices` for E2E).
4. The diagnostic skill if something breaks (e.g., `diagnose` for flakiness).
