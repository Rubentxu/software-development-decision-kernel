# UI Auditor Tooling Matrix

## Primary browser evidence tools

### `playwright-cli`

Use first for:
- opening a local or remote page quickly
- changing viewport sizes
- capturing snapshots with refs/boxes
- reading console output
- checking requests
- collecting traces for hard repros

Best for reconnaissance and interactive smoke checks.

### Repo Playwright tests

Use when the behavior should become a permanent regression guard.

Best for:
- viewport assertions
- overflow checks
- accessible name checks
- route-level smoke tests
- repeatable CI coverage

### `webapp-testing`

Use only when a custom script is genuinely needed.

Best for:
- bespoke scripted measurements
- exploratory automation that does not yet deserve a real test file
- cases where CLI snapshots are not expressive enough

## Supporting tools

### Browser extensions / agent-browser / DevTools helpers

Use as supporting evidence only.

Good for:
- manual corroboration
- ad hoc visual inspection
- device-mode exploration

Not sufficient on their own for final approval.

## Rule of thumb

- recon first: `playwright-cli`
- codify important checks: Playwright tests
- custom script only if needed: `webapp-testing`
- extensions/devtools: corroborate, never replace evidence
