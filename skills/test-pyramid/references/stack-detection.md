# Stack Auto-Detection

Before planning tests, the agent MUST detect the stack. This is the canonical detection protocol.

## Detection order (cheap signals first)

Run these in order. Stop at the first unambiguous answer; otherwise combine signals.

### 1. Repo root files (single signal → strong)

| File present | Implies |
|---|---|
| `Cargo.toml` (no `package.json`) | Pure Rust (may be workspace, single crate, or binary) |
| `package.json` (no `Cargo.toml`) | Pure JS/TS (or Node) |
| `Cargo.toml` + `package.json` | Full-stack: Rust backend + JS frontend (or vice versa) |
| `pyproject.toml` / `requirements.txt` | Python |
| `go.mod` | Go |
| `mix.exs` | Elixir |
| `Gemfile` | Ruby (Rails if `config/application.rb` exists) |
| `composer.json` | PHP |
| `*.csproj` / `*.sln` | .NET |
| `build.gradle*` / `pom.xml` | Java / Kotlin |

### 2. Framework signals (look inside the manifest)

**`Cargo.toml` dependencies:**
- `axum`, `actix-web`, `warp`, `rocket` → Rust HTTP API
- `leptos`, `dioxus`, `yew` → Rust WASM frontend
- `tauri` → Rust desktop / mobile
- `tokio`, `async-std` → async runtime (affects test patterns: `#[tokio::test]`)
- `sqlx`, `diesel`, `sea-orm` → Rust DB
- `rig`, `langchain-rust` → LLM orchestration

**`package.json` dependencies + devDependencies:**
- `react` (no `next`/`gatsby`/`remix`) → SPA React
- `next` → Next.js (App Router vs Pages Router — check `app/` vs `pages/`)
- `vue` (no `nuxt`) → SPA Vue
- `nuxt` → Nuxt
- `svelte` (no `sveltekit`) → Svelte SPA
- `@sveltejs/kit` → SvelteKit
- `@angular/core` → Angular
- `astro` → Astro
- `solid-js` → Solid
- `tailwindcss` → Tailwind (affects E2E selectors — use semantic, not class)
- `react-native` / `expo` → Mobile
- `electron` → Desktop
- `vite` → Vite-based (affects test runner preference: Vitest)
- `webpack` (no `vite`) → Webpack-based (Jest more common)
- `vitest` / `vite-plugin-vitest` → Vitest
- `jest`, `@testing-library/*` → Jest + Testing Library
- `cypress` → Cypress (E2E alternative to Playwright)
- `playwright`, `@playwright/test` → Playwright
- `msw` → MSW for network mocking
- `prisma` / `drizzle-orm` / `typeorm` / `sequelize` → JS/TS DB

**`pyproject.toml`:**
- `django` → Django
- `flask` / `fastapi` / `starlette` → Python HTTP API
- `sqlalchemy` / `alembic` → Python DB
- `pytest`, `hypothesis` → pytest + property-based

### 3. Test infrastructure files

- `playwright.config.*` → Playwright configured (any stack)
- `cypress.config.*` / `cypress.json` → Cypress
- `vitest.config.*` → Vitest
- `jest.config.*` → Jest
- `vitest.setup.*` / `jest.setup.*` → setup files (MSW, jest-dom, etc.)
- `conftest.py` → pytest fixtures
- `tests/`, `__tests__/`, `*.test.*`, `*.spec.*` → test files
- `e2e/`, `cypress/e2e/`, `playwright-tests/` → E2E test dirs
- `benches/`, `benchmarks/` → benchmark dirs
- `bench/` (Go) → Go benchmarks

### 4. Infrastructure signals

- `docker-compose.yml` / `compose.yaml` with `postgres` / `mysql` / `redis` / `nats` / `kafka` → real infra
- `testcontainers-*` deps → testcontainers used (good signal)
- `migrations/`, `db/migrate/`, `alembic/` → migrations present
- `Dockerfile`, `Dockerfile.*` → containerized
- `.github/workflows/`, `.gitlab-ci.yml`, `Jenkinsfile` → CI configured
- `codecov.yml`, `coveralls.yml` → coverage reporting configured

### 5. Workspace / monorepo signals

- `pnpm-workspace.yaml`, `lerna.json`, `nx.json`, `turbo.json` → JS monorepo
- `[workspace]` in `Cargo.toml` → Rust workspace (check `members`)
- Multiple top-level service dirs with own `package.json` → polyrepo

## Output: stack profile

After detection, the agent should be able to state in one paragraph:

> "Stack: **Rust workspace** (`crates/axum-server`, `crates/hodei-auth`, ...) with **Postgres (sqlx)** backend, **React 18 + Vite + Jest** frontend in `web-app/`, **Playwright** E2E (not yet installed), **docker-compose** for Postgres + NATS. Test runners: `cargo test` (Rust), `jest` (JS), no E2E runner yet. CI: GitHub Actions."

This profile drives every subsequent decision in the skill: which references to load, which templates to copy, which tool the agent uses to run tests.

## When detection is ambiguous

- **Mixed stack with no clear primary**: ask the user one question: "Is the focus the `crates/` (Rust) or the `web-app/` (React) for this work?"
- **Detected stack lacks tests entirely**: this is the normal starting point — propose a pyramid from scratch.
- **Detected stack has tests but no pyramid shape**: this is the audit case — run the checklist in `assets/pyramid-checklist.md`.
- **No manifest at all**: this is a one-off script or research repo. Default to plain `pytest` / `cargo test` / `node --test` patterns, suggest adding a manifest.
