# Evidence-Driven Web Development Workflow

## Recommended tool order

1. `playwright-cli` for repro and snapshots
2. repo Playwright tests for codified assertions
3. `webapp-testing` for exceptional custom scripting
4. accessibility scans with `@axe-core/playwright`

## Best-practice defaults

- Test user-visible behavior, not implementation details.
- Use web-first assertions (`toBeVisible`, `toHaveText`, `toHaveAccessibleName`, `toHaveScreenshot` only when visual comparison is truly needed).
- Keep tests isolated.
- Prefer user-facing locators (`getByRole`, `getByLabel`, `getByTestId` as explicit contract).
- Use traces for flaky or multi-step failures instead of relying only on screenshots.
- Use automated accessibility checks plus manual keyboard review.

## Suggested artifact set by task type

### Responsive/layout bug
- before snapshot
- measured overflow or bounding boxes
- after snapshot
- Playwright viewport assertion

### Accessibility fix
- failing or reported a11y finding
- accessible-name/focus assertion
- optional axe attachment for broad scans

### Visual drift / UX polish
- screenshot pair or snapshot pair
- geometry evidence if spacing/alignment is involved
- targeted smoke assertion if the issue is structural

## Notes on extensions and browser tooling

- Chrome DevTools device mode is useful for manual corroboration.
- Accessibility Insights or axe DevTools can help exploratory review.
- These tools should enrich investigation, not replace automated runtime evidence.
- If an extension-oriented tool is unavailable in the environment, the workflow must degrade cleanly to Playwright-based evidence.
