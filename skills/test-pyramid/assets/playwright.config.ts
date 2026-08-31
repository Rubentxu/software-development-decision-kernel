// Playwright config baseline — full-stack (backend + frontend).
// Drop into playwright.config.ts at the repo root, alongside e2e/ tests.
// Replace the <...> placeholders with your stack's start commands.

import { defineConfig, devices } from '@playwright/test'

const PORT_BACKEND = 8080
const PORT_FRONTEND = 5173

export default defineConfig({
  testDir: './e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 2 : undefined,
  reporter: process.env.CI ? [['github'], ['html', { open: 'never' }]] : 'list',

  timeout: 30_000,
  expect: { timeout: 5_000 },

  use: {
    baseURL: process.env.BASE_URL || `http://127.0.0.1:${PORT_FRONTEND}`,
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
  },

  projects: [
    { name: 'chromium-desktop', use: { ...devices['Desktop Chrome'] } },
    { name: 'firefox-desktop',  use: { ...devices['Desktop Firefox'] } },
    { name: 'mobile-safari',    use: { ...devices['iPhone 14'] } },
  ],

  // Run order: smoke (fast, critical) → full suite.
  // Tag tests with @smoke / @critical to opt in.
  grep: process.env.GREP || undefined,
  grepInvert: process.env.GREP_INVERT || undefined,

  // Spin up the stack. `reuseExistingServer: true` locally for fast dev loops.
  // Replace the commands below with your stack:
  //   Rust: 'cargo run -p <crate>'
  //   Node: 'npm run dev'
  //   Python: 'uvicorn app.main:app --port 8080'
  //   Go: 'go run .'
  //   Static: 'npx serve dist -l 5173'
  webServer: [
    {
      command: '<backend-start-command>',
      port: PORT_BACKEND,
      reuseExistingServer: !process.env.CI,
      timeout: 120_000,
      stdout: 'pipe',
      stderr: 'pipe',
    },
    {
      command: '<frontend-start-command>',
      port: PORT_FRONTEND,
      reuseExistingServer: !process.env.CI,
      timeout: 60_000,
      stdout: 'pipe',
      stderr: 'pipe',
    },
  ],
})
