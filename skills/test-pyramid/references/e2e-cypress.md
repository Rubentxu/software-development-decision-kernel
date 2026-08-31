# Cypress E2E Reference (alternative to Playwright)

Patterns for the **Cypress** layer of the pyramid. Use Cypress when the project is already Cypress-based or when the team prefers its test runner. For new projects, prefer Playwright (see `e2e-playwright.md`).

## When Cypress is the right choice

- Existing Cypress-based project (migrate only if there's a strong reason).
- The team values the Cypress UI for debugging.
- The app is a single browser, single OS (Cypress multi-browser support is improving but still weaker than Playwright's).
- Component testing is a strong need (Cypress's component testing is solid for React/Vue).

## When NOT to use Cypress

- Need for cross-browser + cross-OS in one tool → use Playwright.
- Need to test against many origins / iframes → use Playwright.
- Native mobile testing → use Playwright.
- Need for parallel test shards on a budget → use Playwright (Cypress Cloud is paid).

## Configuration (cypress.config.ts)

```ts
import { defineConfig } from 'cypress'

export default defineConfig({
  e2e: {
    baseUrl: process.env.BASE_URL || 'http://127.0.0.1:5173',
    specPattern: 'cypress/e2e/**/*.cy.{ts,js}',
    supportFile: 'cypress/support/e2e.ts',
    screenshotsFolder: 'cypress/screenshots',
    videosFolder: 'cypress/videos',
    video: false,  // enable in CI if you want recordings
    retries: { runMode: 2, openMode: 0 },
  },
  component: {
    devServer: { framework: 'react', bundler: 'vite' },
    specPattern: 'src/**/*.cy.{ts,tsx}',
  },
})
```

## Locators (the most important rule)

**Prefer** (in order):
1. `cy.findByRole('button', { name: /sign in/i })` (via `@testing-library/cypress`)
2. `cy.contains('button', 'Sign in')`
3. `cy.get('[data-testid="sign-in"]')` (last resort)

**Avoid**: CSS class selectors, XPath, complex descendant selectors.

## Component testing

```tsx
// src/LoginForm.cy.tsx
import { mount } from 'cypress/react'
import { LoginForm } from './LoginForm'

it('rejects empty email', () => {
  cy.mount(<LoginForm />)
  cy.findByRole('button', { name: /sign in/i }).click()
  cy.findByRole('alert').should('contain.text', /email required/i)
})
```

## Network stubbing (cy.intercept)

```ts
cy.intercept('POST', '/api/v1/auth/login', { statusCode: 200, body: { token: 'fake' } }).as('login')
cy.findByRole('button', { name: /sign in/i }).click()
cy.wait('@login')
```

## Anti-patterns

- `cy.wait(500)` — use `cy.intercept(...).as('x')` + `cy.wait('@x')` or assertions.
- Sharing a Cypress session across tests.
- CSS-class selectors in queries.
- Testing the same flow at component + E2E — pick one.
- Heavy `before` / `beforeEach` that hides which state you're in.

## References to load

- `playwright-best-practices` — The recommended alternative.
- `frontend-evidence-loop` — UI bug investigation.
- `accessibility` — a11y in component tests.
- `diagnose` — For hard flakiness.
