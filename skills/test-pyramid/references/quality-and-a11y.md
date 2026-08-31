# Quality, Coverage, Accessibility, Performance

Cross-cutting concerns for the pyramid. These apply to **any stack** and **any layer**.

## Coverage

| Layer | What to track | What NOT to track |
|---|---|---|
| Unit | Line + branch per module | Generated code, `tests/`, doc tests |
| Integration | Scenario coverage (named seams) | Line coverage (you're testing boundaries) |
| E2E | User-journey coverage (named flows) | Line coverage (not what E2E is for) |
| Component | Line + branch per component | Snapshot files, mocks |

**Thresholds** (configurable; defaults are sane starting points):

- **Unit / Component**: 80% lines, 75% branches.
- **Integration**: scenario coverage, not percentage.
- **E2E**: journey coverage, not percentage.
- **Overall pyramid**: 70% unit, 20% integration, 10% E2E by count and runtime.

**Rules**:
- Coverage is a gate, not a goal. A test that bumps coverage by 0.1% on dead code is waste.
- Branch coverage > line coverage for parsing, validation, error handling.
- Mutation testing (`cargo-mutants`, `stryker-js`, `mutmut` for Python) on critical modules reveals weak tests.

## Mutation testing

| Stack | Tool | Command |
|---|---|---|
| Rust | `cargo-mutants` | `cargo mutants -p crate_name` |
| JS/TS | `stryker-js` | `npx stryker run` |
| Python | `mutmut` / `cosmic-ray` | `mutmut run` |
| Go | `go-mutesting` / `mutagen` | `mutagen test` |

Run on a schedule (weekly), not every PR. Start with critical modules: auth, payments, validators, state machines.

## Accessibility (WCAG 2.2 AA minimum)

Layers of defense:

1. **Component tests**: `jest-axe` / `axe-core` per interactive component.
2. **E2E**: `axe-playwright` per critical page with `wcag2aa` tags.
3. **Manual**: keyboard-only walkthrough of key flows; screen reader (VoiceOver/NVDA) on top 3 journeys.
4. **CI gate**: fail build on any `critical` or `serious` axe violation.

Load the `accessibility` skill for the WCAG deep dive.

## Performance budgets

**Web Vitals (any UI):**
- **LCP** (Largest Contentful Paint): ≤ 2.5s on 4G/Moto G4.
- **INP** (Interaction to Next Paint): ≤ 200ms.
- **CLS** (Cumulative Layout Shift): ≤ 0.1.

**Backend (any stack):**
- p50, p95, p99 latency budgets per endpoint.
- Error rate ≤ 0.1% on happy path.
- Saturation: CPU < 70% under target load.

**Measure at the test layer with:**
- Web: Playwright + Lighthouse, or `web-vitals` library in production.
- Rust: `criterion` + `tokio-console` for scheduling.
- Go: `go test -bench` + `pprof`.
- Python: `pytest-benchmark` + `py-spy` / `memray`.
- Node: `clinic.js`, `0x`, autocannon.

Load the `core-web-vitals` skill for the Web deep dive. Load `diagnose` if a budget is being missed and you need the loop.

## Security smoke tests

- **All stacks**: `cargo audit` / `npm audit` / `pip-audit` / `govulncheck` in CI, fail on high/critical.
- **OWASP top 10 quick checks**:
  - Auth: token expiry, refresh, revocation, MFA bypass.
  - Input: parser fuzzed (`cargo-fuzz`, `atheris` for Python, `go-fuzz`).
  - Output: HTML escape, no `innerHTML` for user data, `dompurify` for markdown.
  - Headers: CORS, CSP, HSTS, X-Content-Type-Options.
  - Storage: no secrets in local storage; httpOnly + secure cookies; encrypted at rest.
  - Crypto: argon2/bcrypt for passwords, never MD5/SHA1 for auth, unique nonces.

## Documentation tests (often forgotten)

- **Rust**: doc tests in `///` examples — they run as tests and serve as the user-facing API contract.
- **JS/TS**: a `.stories.tsx` (Storybook) is a documentation test that double-checks render.
- **Python**: doctest in docstrings (`>>>` blocks) for runnable examples.
- **Go**: `ExampleXxx()` functions in `_test.go` files get rendered as docs and run as tests.

## References to load

- `accessibility` — WCAG 2.2 implementation.
- `core-web-vitals` — LCP/INP/CLS optimization.
- `web-quality-audit` — Holistic web quality scoring.
- `best-practices` — Cross-cutting web best practices.
- `diagnose` — When a metric is off and you need the loop.
- `entropy-sdd` — SOLID-entropy compliance for testability.
