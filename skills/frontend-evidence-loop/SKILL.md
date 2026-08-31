---
name: frontend-evidence-loop
description: Trigger: fix frontend, improve UI, responsive bug, visual regression, accessibility fix, browser-driven web development. Defines a detailed evidence-first frontend workflow: reproduce, measure, patch, verify, and codify tests.
license: MIT
metadata:
  author: OpenCode
  version: "1.0"
---

## Activation Contract

Use when implementing or fixing frontend behavior and the work should be driven by browser evidence and regression tests.

## Hard Rules

- Do not patch first and inspect later.
- Reproduce the issue in runtime before changing code.
- Prefer turning important bugs into failing or measurable checks before fixing.
- Re-verify in browser after every meaningful UI change.
- Leave behind regression coverage for significant UI bugs.

## Development Loop

### Phase 1 — Reproduce

1. Open the real route.
2. Record viewport and state.
3. Capture a screenshot or snapshot.
4. Check console and relevant requests.

### Phase 2 — Measure

1. Determine whether the problem is visual, behavioral, accessibility, or responsive.
2. Gather dimensions, accessible names, focus behavior, or network symptoms.
3. Express the failure as one of:
   - a Playwright assertion
   - a geometry condition
   - an accessibility scan result
   - a documented browser trace

### Phase 3 — Patch

1. Apply the minimum safe fix.
2. Avoid unrelated polish in the same pass.
3. Preserve user-facing semantics and accessible names.

### Phase 4 — Verify

1. Re-run the browser flow.
2. Re-check mobile/tablet/desktop when layout changed.
3. Confirm console cleanliness for the fixed scenario.
4. Capture updated evidence.

### Phase 5 — Codify

Add or update Playwright coverage for:

- overflow regressions
- menu/sidebar visibility issues
- clipped CTAs or filters
- accessible names/labels
- route smoke tests for critical flows

## What to Generate

- a concise browser-backed finding or fix summary
- exact verification commands run
- targeted Playwright tests for meaningful regressions
- screenshots/snapshots only when they add proof

## Forbid

- approving from source inspection only
- CSS-only guesswork without runtime verification
- using brittle CSS selectors when a user-facing locator exists
- adding snapshot baselines where a semantic assertion is enough

## References

- `references/web-development-workflow.md`
- `../ui-audit-protocol/references/tooling-matrix.md`
