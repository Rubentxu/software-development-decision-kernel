# Grill Topic Template — Edge-Case Discovery

Copy this template and customize the bracketed parts. The result is a topic you can hand to `auto-grill-loop-orchestrator` (or load the `auto-grill-loop` skill).

## Template (long form)

```
Edge cases, failure modes, and adversarial inputs for
[unit_or_module_or_path] in [stack/framework].

Focus on:
- [category_1: concurrency | trust boundary | persistence | auth | money | time | i18n | a11y | perf]
- [category_2]
- [category_3]

Constraints:
- [deterministic only | no real network | no real time | no real DB]
- [max iterations or runtime budget]
- [specific libraries or patterns to use or avoid]

Success criteria:
- [what "well-tested" means for this scope]
- [what behaviors must be explicitly asserted]
- [what behaviors must be explicitly rejected]

Out of scope:
- [things to NOT grill — e.g., "do not re-grill X, that was done in <previous-grill>"]
```

## Examples

### Example 1: Rust function

```
Edge cases, failure modes, and adversarial inputs for
`parse_execution_request` in `crates/execution-engine/src/parser.rs` (Rust).

Focus on:
- Malformed JSON: missing fields, duplicate keys, null bytes, deep nesting, very large payloads.
- Unicode edge cases: BOM, RTL markers, zero-width chars, surrogate pairs in strings.
- Numeric edge cases: i64::MIN, i64::MAX, floats with precision loss.
- Time edge cases: leap second, DST transition, far future, far past.

Constraints:
- Must run under `cargo test` with `#[cfg(test)]`.
- No real network, no real DB.
- Deterministic — no `SystemTime::now()`.

Success criteria:
- All known CVE-class parser bugs from JSON libraries (2017–2024) are explicitly tested.
- For every error branch in the parser, there is at least one test.
- Property-based test verifies roundtrip serialize/deserialize.

Out of scope:
- The HTTP handler wrapping this parser (covered in another module).
```

### Example 2: React component

```
Edge cases, failure modes, and accessibility gaps in
the login form on the web-app (React 18 + Vite + MUI).

Focus on:
- Empty / malformed / unicode email inputs.
- Server error states (400, 401, 500, network timeout).
- Keyboard navigation: tab order, focus trap, escape key.
- Screen reader: form labels, error announcements, focus management.
- a11y: contrast, ARIA, label associations, error message role.
- Loading state: button disabled, spinner visible, no double-submit.

Constraints:
- Tests use Testing Library + jest-axe + MSW for the network.
- No real backend.
- Must work on mobile viewport (iPhone 14).

Success criteria:
- Empty email shows a clear, accessible error.
- Submit button is disabled while in flight.
- Network error announces via `role="alert"`.
- axe finds zero critical or serious violations.
- Tab order is logical; focus is visible.

Out of scope:
- The actual API endpoint implementation.
```

### Example 3: Coverage audit

```
Coverage audit for the [project_name] test pyramid.

Focus on:
- Which layers are missing entirely (E2E? Property? Component?).
- Which modules have weak assertions (covered but not testing behavior).
- Which modules have high line coverage but low mutation score.
- Which flows are tested only manually.

Constraints:
- Use the checklist in `assets/pyramid-checklist.md` as the baseline.
- Do not propose 100% coverage; identify the highest-value gaps.

Success criteria:
- A prioritized list of 5–10 test additions that improve the pyramid shape.
- For each, the layer, the file(s), the rationale, and the estimated effort.

Out of scope:
- Production code changes (this is a test plan, not a refactor plan).
```

### Example 4: Bug-driven

```
Regression coverage for bug #142: "user sees stale data after refresh on
the dashboard page".

Focus on:
- All code paths that affect dashboard data freshness.
- Cache invalidation, SWR/React Query, server-pushed updates.
- Time-of-update vs time-of-render races.
- Network failure during refetch.

Constraints:
- Tests should reproduce the bug deterministically.
- Use Playwright with a mocked API to control timing.

Success criteria:
- A failing test that reproduces #142 is in the suite.
- The test passes after the fix.
- The test fails if the fix is reverted.
- Related code paths (e.g., other "stale data" flows) have analogous tests.

Out of scope:
- Performance testing (separate concern).
```

## Where the report goes

`auto-grill-loop-orchestrator` writes the final report to:

```
`{grill-reports-dir}/{YYYY-MM-DD}-auto-grill-{topic-slug}.report.md`
(under SDDK adoption: `$SDDK_DATA_DIR/projects/<id>/cycle-artifacts/{cycle_id}/grill/` — ADR-0011; standalone: `docs/grill/`)
```

Read the report. Map each "accepted decision" to one or more test cases. Surface "decisions requiring user validation" to the user before writing tests. Use "rejected alternatives" to write tests that assert the rejected behavior is rejected.

## Limits

- The grill is one LLM's structured view. Cross-check critical decisions.
- The grill cannot replace runtime evidence. For performance / load / race issues, use `diagnose` + Chronos.
- The grill is a complement to manual exploratory testing, not a replacement.
