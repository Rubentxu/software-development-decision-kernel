// Vitest config baseline for a Vite-based project (any framework: React, Vue, Svelte, etc.)
// Drop into vitest.config.ts at the project root and run with `npx vitest`.
import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'  // swap for vue() / svelte() / etc.

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/test/setup.ts'],
    css: false,
    coverage: {
      provider: 'v8',
      reporter: ['text', 'html', 'json'],
      include: ['src/**/*.{ts,tsx}'],
      exclude: ['src/**/*.test.{ts,tsx}', 'src/test/**', 'src/mocks/**'],
      thresholds: {
        lines: 80,
        functions: 80,
        branches: 70,
        statements: 80,
      },
    },
  },
})
