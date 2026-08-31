# Playwright E2E Reference (any UI stack)

Patterns for the **Playwright** layer of the pyramid. Works for any UI stack: React, Vue, Svelte, Angular, Leptos, plain HTML. For the comprehensive 300+ line Playwright reference, load `playwright-best-practices` — this file covers what we use day-to-day.

## When E2E is the right layer

- Critical user journey ("sign up", "create execution", "view dashboard").
- Cross-page flows that integration tests cannot cover cheaply.
- Visual regression of chrome (header, sidebar, layout).
- Accessibility audit of the full rendered page (`axe-playwright`).
- Mobile/responsive sanity.
- Cross-browser compatibility check.

**When E2E is wrong**:
- Pure business logic (use unit test in the host language).
- Single component behavior (use component test).
- Form validation rules (use component test).

## Configuration (start from `assets/playwright.config.ts`)

```ts
// playwright.config.ts
import { defineConfig, devices } from '@playwright/test'

export default defineConfig({
  testDir: './e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 2 : undefined,
  reporter: process.env.CI ? 'github' : 'list',

  use: {
    baseURL: process.env.BASE_URL || 'http://127.0.0.1:5173',
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
  },

  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
    { name: 'firefox',  use: { ...devices['Desktop Firefox'] } },
    { name: 'mobile',   use: { ...devices['iPhone 14'] } },
  ],

  // Spin up the stack. Multiple servers for full-stack apps.
  webServer: [
    { command: '<backend-start-cmd>', port: 8080, reuseExistingServer: !process.env.CI, timeout: 120_000 },
    { command: '<frontend-start-cmd>', port: 5173, reuseExistingServer: !process.env.CI, timeout: 60_000 },
  ],
})
```

`webServer` accepts any `command` per stack: `cargo run -p <crate>`, `npm run dev`, `python -m uvicorn app:app`, `go run .`, etc.

## Page Object Model (POM)

```ts
// e2e/pages/login.page.ts
import { type Page, expect } from '@playwright/test'

export class LoginPage {
  constructor(private page: Page) {}
  async goto() { await this.page.goto('/login') }
  async login(email: string, password: string) {
    await this.page.getByLabel(/email/i).fill(email)
    await this.page.getByLabel(/password/i).fill(password)
    await this.page.getByRole('button', { name: /sign in/i }).click()
  }
  async expectVisible() {
    await expect(this.page.getByRole('heading', { name: /sign in/i })).toBeVisible()
  }
}
```

Tests stay short and readable; locators and waits live in the page object.

## Locators (the most important rule)

**Prefer** (in order):
1. `getByRole('button', { name: /.../ })`
2. `getByLabel(/.../)`
3. `getByText(/.../)`
4. `getByTestId('...')` (only when no semantic alternative exists)

**Avoid**: CSS selectors (`div.foo > span:nth-child(2)`), XPath, tag names with indices.

## Auto-wait (no `waitForTimeout`)

Playwright auto-waits for actionability. For assertions, use:

```ts
await expect(page.getByRole('alert')).toBeVisible()         // waits
await expect(page).toHaveURL(/\/dashboard/)
await expect(page).toHaveTitle(/Dashboard/)
```

Never use `page.waitForTimeout(500)` — it masks races.

## Test tags

```ts
test('critical: user can create an execution @smoke @critical', async ({ page }) => { ... })
```

Run smoke first: `npx playwright test --grep @smoke`.

## Accessibility (axe-playwright)

```ts
import AxeBuilder from '@axe-core/playwright'

test('dashboard has no critical a11y violations', async ({ page }) => {
  await page.goto('/dashboard')
  const results = await new AxeBuilder({ page })
    .withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'])
    .analyze()
  const critical = results.violations.filter(v => v.impact === 'critical' || v.impact === 'serious')
  expect(critical, JSON.stringify(critical, null, 2)).toEqual([])
})
```

## Visual regression

```ts
await expect(page).toHaveScreenshot('dashboard.png', {
  maxDiffPixelRatio: 0.01,
  fullPage: true,
})
```

Update with `npx playwright test --update-snapshots` and review the diff in `__snapshots__/`.

## API mocking (Playwright-level)

```ts
await page.route('**/api/v1/executions', async (route) => {
  await route.fulfill({ json: { items: [] } })
})
```

Useful for testing UI without the backend, or for forcing error states.

## Flakiness triage

1. Re-run with `--retries=3 --reporter=line` to confirm intermittent.
2. Get the trace: `npx playwright test --trace on`.
3. Open `trace.zip` with `npx playwright show-trace`.
4. Look for: real timing dependency, missing `await`, shared global state, parallel tests clobbering fixtures.
5. Codify the fix as a regression test in the layer above.

## Stack-specific tips

- **React + Vite**: `npm run dev` starts on 5173. Use `npm run preview` against a built bundle for more realistic E2E.
- **Next.js**: `npm run dev` or `npm run start` (after build). For App Router, use the `app/` URL structure.
- **Vue + Vite**: same as React + Vite.
- **Leptos**: `trunk serve` or `cargo leptos serve`. The frontend is WASM, so test against the running dev server.
- **Static SPA**: `npx serve dist -l 5173` or `python -m http.server 5173` from the build dir.
- **Backend (Rust) + Frontend (any)**: `webServer` array with both commands.

## References to load

- `playwright-best-practices` — Authoritative Playwright reference (every activity mapped to files).
- `playwright-cli` — Quick browser automation without writing test files.
- `webapp-testing` — `with_server.py` helper for ad-hoc browser checks.
- `frontend-evidence-loop` — The loop we use for UI bug investigations.
- `ui-audit-protocol` — Evidence-driven browser auditing.
- `accessibility` — Deep WCAG 2.2.
- `core-web-vitals` — LCP/INP/CLS budgets and measurement.
- `diagnose` — Disciplined loop for hard flakiness.
