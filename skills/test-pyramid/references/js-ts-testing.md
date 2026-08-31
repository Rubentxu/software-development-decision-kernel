# JS/TS Testing Reference

Patterns specifically for the **JS/TS** half of the pyramid. Covers Jest, Vitest, and lightweight test runners. For framework-specific (React, Vue, Svelte), see the corresponding reference in this skill or load the framework's specialist skill.

## Choosing a test runner

| Signal | Pick |
|---|---|
| Vite project | **Vitest** (fast, native ESM, Vite-powered) |
| Webpack / Next.js (no Vite) | **Jest** (mature, ubiquitous) |
| Node-only library | **Node test runner** (`node --test`) or **Vitest** |
| Deno project | `deno test` |
| Bun project | `bun test` |

Both Jest and Vitest support the same Testing Library / user-event / jest-dom APIs. Migrating is mostly a config change.

## Unit tests (the foundation)

```ts
// src/utils/format.ts
export function formatCurrency(cents: number, currency = 'USD'): string {
  return new Intl.NumberFormat('en-US', { style: 'currency', currency }).format(cents / 100)
}

// src/utils/format.test.ts
import { describe, it, expect } from 'vitest'
import { formatCurrency } from './format'

describe('formatCurrency', () => {
  it('formats zero', () => {
    expect(formatCurrency(0)).toBe('$0.00')
  })

  it('formats negative as minus', () => {
    expect(formatCurrency(-199)).toBe('-$1.99')
  })

  it('rounds half to even (banker rounding)', () => {
    expect(formatCurrency(125)).toBe('$1.25')
  })

  it('respects currency override', () => {
    expect(formatCurrency(1000, 'EUR')).toMatch(/€|EUR/)
  })
})
```

**Rules**:
- `describe` per unit, `it` per scenario.
- Name scenarios by behavior, not by input mechanics.
- Pure-function tests are the fastest layer — bulk of the suite.

## Property-based tests

`fast-check` for parsers, serializers, and invariant-checking code.

```ts
import { fc, test } from 'fast-check'
import { formatCurrency } from './format'

test.prop([fc.integer({ min: -1_000_000, max: 1_000_000 })])('always includes currency symbol', (cents) => {
  const out = formatCurrency(cents)
  expect(out).toMatch(/[\$€£¥]/)  // adjust for your currencies
})
```

## Component tests (the workhorse for UI)

```tsx
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { LoginForm } from './LoginForm'

it('rejects empty email', async () => {
  const onSubmit = vi.fn()
  render(<LoginForm onSubmit={onSubmit} />)
  await userEvent.click(screen.getByRole('button', { name: /sign in/i }))
  expect(screen.getByRole('alert')).toHaveTextContent(/email required/i)
  expect(onSubmit).not.toHaveBeenCalled()
})
```

**Rules**:
- Query by `getByRole`, `getByLabelText`, `getByText` — **never** by CSS class or test-id unless nothing else is reachable.
- `userEvent` is async: `await userEvent.click(...)`. `fireEvent` is sync and bypasses real user interaction.
- Use `findBy*` for elements that appear after async work.
- `waitFor` only when no `findBy*` works; never `setTimeout` / `waitForTimeout` to mask a race.

## Hooks

```tsx
import { renderHook, act } from '@testing-library/react'

it('updates counter', () => {
  const { result } = renderHook(() => useCounter())
  act(() => result.current.inc())
  expect(result.current.value).toBe(1)
})
```

For hooks that need a context (Redux, MUI Theme, etc.), wrap with a provider using `wrapper`.

## Async + MSW (network)

Use MSW v2 — handlers define the API contract; tests don't.

```ts
// src/mocks/handlers.ts
import { http, HttpResponse } from 'msw'
export const handlers = [
  http.post('/api/v1/auth/login', async ({ request }) => {
    const body = await request.json()
    if (!body.email) return HttpResponse.json({ error: 'email required' }, { status: 400 })
    return HttpResponse.json({ token: 'fake' })
  }),
]
```

In setup, start a server before all tests; reset handlers between tests.

## Snapshot tests (use sparingly)

- One snapshot per stable visual contract (a logo, a chrome bar).
- Never snapshot entire pages — diff becomes noise.
- Review the diff like code; regenerate explicitly with `--updateSnapshot` / `-u`.

## Common anti-patterns

- `fireEvent` instead of `userEvent` (bypasses real user interaction).
- CSS class selectors in queries.
- `waitForTimeout` instead of `findBy*` / `waitFor`.
- Mocking the store / provider instead of using the real one.
- Snapshotting rendered JSX without a stable visual contract.
- Sharing one server / DB across tests (use `beforeEach` / `afterEach`).

## References to load

- `playwright-best-practices` — For E2E.
- `frontend-evidence-loop` — For UI bug investigation.
- `accessibility` — For a11y in components.
- `work-unit-commits` — Tests in the same commit as the behavior.
