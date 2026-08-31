# ADR-0069 — Test-Tooling Ownership (Rust / Shell / Python / JS)

**Status:** accepted (user-approved 2026-08-28)

---

## Approval provenance

- **Date:** 2026-08-28
- **Decision:** User binding approval of test-tooling ownership policy and Bats deprecation.
- **Authority:** Amendment-002 (sha `0f711b3aa9e4ea551870260f3009dc58c2c25b99f5bd7c858fbd06cb103b22e9`) and Amendment-003 (sha `35b82978…`) as the binding decisions that produced this ADR.
- **Acceptance:** At creation, per user decision. No future-cycle acceptance step required.

## Context

The current repository exhibits accidental accretion in test-tooling ownership. Over multiple cycles, tests have been added in languages that do not reflect the strategic ownership of the behavior under test. This creates:

- Boundary drift at PR review (unclear which language owns a given test surface).
- Debt-verify flagging the same boundary repeatedly without a policy anchor.
- Recurring incidents of pytest (Python) being used for shell-boundary tests, and Bats being reconsidered as a strategic default despite evidence it does not scale.

The recurring pattern is visible in:
- `docs/debt/INC-CYCLE-11-PYTEST-CONTRACT-P1.md` — pytest boundary debt.
- `docs/adr/ADR-0022-sddk-testkit.md` (proposed) — Bats/shellspec mentions.
- `docs/adr/ADR-0045-graph-store-contracts.md` — test references with mixed tooling.
- `docs/sddk-decision-kernel-architecture/09-implementation/REPOSITORY-TARGET-LAYOUT.md` — Bats mentions.

Without an explicit ownership policy, each boundary decision is ad-hoc, and debt-verify has no anchor against which to flag drift.

## Decision

The canonical test-tooling ownership for this repository is declared in four cells:

### Rust

**Owns:** Binary behavior, CLI contracts, schema, lint, cycle-46 lockstep, dry-run invariance, recover ledger invariance.

**Maps to:**
- `crates/` test suites (unit, integration, property).
- Contract tests in Rust where the contract surface is the binary's argv, exit code, JSON output, or schema.
- Tests asserting on `cargo` output, `sddk` CLI behavior, or any binary artifact.

**Rationale:** Rust is the implementation language of the binary. Testing binary behavior in Rust keeps the contract near the implementation, enables property-based testing, and ensures the local CI gate (cargo test) covers the contract surface.

### Shell

**Owns:** Pre-binary bootstrap, installer, Podman/system orchestration, TUI smoke orchestration.

**Maps to:**
- `tests-e2e/`
- `bin/install.sh`, `bin/sddk-devbox.sh`, `bin/uat.sh`
- `tests/test_*smoke*.sh` (system-level smoke)
- `tests/test_*bootstrap*.sh`, `tests/test_*installer*.sh`, `tests/test_*podman*.sh`, `tests/test_*tui*.sh`

**Rationale:** Shell is the natural language for system-level orchestration that operates outside the binary's hot path. These tests verify the environment around the binary, not the binary's API.

**Not a strategic default for binary behavior tests.** A test that calls the binary and asserts on its argv/JSON output belongs in Rust (contract test), not shell.

### Python

**Owns:** External golden/evaluation/analytical assets.

**Maps to:**
- `scripts/` — golden-file generation, evaluation harnesses, scenario generators.
- Analytical tooling for SPEC-024 / SPEC-040 outputs.
- Fixtures and corpus management that lives outside the binary's hot path.

**Rationale:** Python's strength in data processing and scripting makes it suitable for tooling that produces or consumes artifacts, but not for testing binary behavior directly.

**Not a strategic default for binary behavior.** A Python script that invokes the binary and asserts on stdout is a Rust contract test written in Python — it should be migrated.

### JavaScript

**Owns:** Tests for JavaScript assets (reserved; none today).

**Maps to:**
- `frontend/test/` if/when `frontend/` is introduced.

**Rationale:** Frontend assets require JavaScript testing infrastructure. Until `frontend/` is present, this cell is reserved.

## Bats reassessment

Bats is **not** a strategic default for this repository. The historical mentions in ADR-0022 (proposed), ADR-0045, and `REPOSITORY-TARGET-LAYOUT.md` reflect early-stage exploration, not an accepted decision.

**Reconsider Bats only when:**
- A genuine shell-boundary test cannot be expressed in shell + Rust without losing measurable coverage.
- The coverage loss is documented and weighed against Bats's maintenance cost.

**Supersedes:** Any claim in ADR-0022, ADR-0045, or `REPOSITORY-TARGET-LAYOUT.md` that Bats is a default or recommended tool for this repository.

## Consequences

### Easier

- PR review has a clear ownership anchor: "is this testing binary behavior? → Rust. Is this testing the system around the binary? → Shell."
- Debt-verify has a policy basis for flagging boundary drift.
- The 16 new shell contract tests introduced by commit `643180a` can be audited against this policy (Phase A current-cycle work).

### Harder

- Existing tests that violate this policy must be flagged and migrated (Phase B next-cycle ownership/lint foundation, after parity verification).
- PRs that mix ownership without justification must be rejected or annotated.

## Alternatives considered

| Alternative | Rationale for rejection |
|---|---|
| pytest-as-default for shell | Python is not the natural language for shell-boundary orchestration; adds a runtime dependency for what shell handles natively. |
| shellspec-as-default | Same as Bats — not a strategic default; maintenance cost outweighs coverage gain until a specific gap is demonstrated. |
| Go for everything | Out of stack; Go is not used in this repository. |
| Bats-as-default | Not a strategic default per user binding decision 2026-08-28; Bats is appropriate only for specific shell-boundary gaps. |

## Cross-references

- `docs/adr/ADR-0022-sddk-testkit.md` (proposed) — superseded by this decision for Bats; testkit proposal pending supersession or acceptance separately.
- `docs/adr/ADR-0060-prompt-layer-evidence-contracts.md` (accepted) — prompt-layer evidence contracts; related to CLI contract testing.
- `docs/adr/ADR-0068-bounded-execution.md` (cycle-44 foundation) — bounded execution; cycle-46 lockstep behavior owned by Rust.
- `docs/sddk-decision-kernel-architecture/03-adrs/ADR-042-TEST-TOOLING-BOUNDARY.md` (Accepted) — sequencing and migration plan; references this ADR for ownership policy.
- `docs/debt/INC-CYCLE-11-PYTEST-CONTRACT-P1.md` — pytest boundary debt; evidence of historical accretion.
- `docs/sddk-decision-kernel-architecture/09-implementation/TEST-TOOLING-EVIDENCE-AUDIT.md` — verified evidence, inventory, and audit of current test-tooling state.
