# Test-Tooling Evidence & Audit (verified history, inventory, boundaries, accretion, gaps, plan)

> Evidence document. NOT a policy ADR. Policy: ADR-0069. Sequencing: ADR-042. This file captures the verified state at 2026-08-28 as input to roadmap reprioritization.

## 1. Verified history

| Date | Event | Evidence |
|------|-------|----------|
| 2026-08-19 | ADR-0022 (sddk-testkit) proposed — mentions Bats/shellspec as candidate tooling | `docs/adr/ADR-0022-sddk-testkit.md` |
| 2026-08-20 | ADR-0045 (graph-store-contracts) accepted — test references with mixed tooling | `docs/adr/ADR-0045-graph-store-contracts.md` |
| 2026-08-22 | INC-CYCLE-11-PYTEST-CONTRACT-P1 filed — pytest boundary debt (Python used for shell-boundary test) | `docs/debt/INC-CYCLE-11-PYTEST-CONTRACT-P1.md` |
| 2026-08-22 | INC-CYCLE-14-APPLY-PUSH-VIOLATION filed — apply pushed 4 commits to origin/main (closed 2026-08-23) | `docs/debt/INC-CYCLE-14-APPLY-PUSH-VIOLATION.md`, cluster `CL-APPLY-PUSH-DISCIPLINE` |
| 2026-08-23 | INC-DEBT-006 filed — cycle-16 apply-push discipline violation, severity critical/P0, resolved via retag at `c1945dc`; forward debt for cycle-17 prompt hardening registered | `docs/debt/INC-DEBT-006-apply-push-discipline-cycle-16-violation.md`, cluster `CL-08` |
| 2026-08-26 | REPOSITORY-TARGET-LAYOUT.md mentions Bats for shell boundary tests | `docs/sddk-decision-kernel-architecture/09-implementation/REPOSITORY-TARGET-LAYOUT.md` |
| 2026-08-28 | Commit `643180a` introduces 19 shell contract tests in `tests/test_*.sh` | `git show 643180a --stat` (19 new shell scripts) |
| 2026-08-28 | ADR-0069 accepted — test-tooling ownership policy (Rust/Shell/Python/JS) | `docs/adr/ADR-0069-test-tooling-ownership.md` |
| 2026-08-28 | ADR-042 accepted — test-tooling sequencing and migration plan | `docs/sddk-decision-kernel-architecture/03-adrs/ADR-042-TEST-TOOLING-BOUNDARY.md` |
| 2026-08-28 | INC-KERNEL-CLI-AGENT-INFORMATION-FLOW-APPLY-PUSH-VIOLATION filed — third apply-push violation; cycle `p-52b95ef55999f9de/kernel-cli-agent-information-flow` | `docs/debt/INC-KERNEL-CLI-AGENT-INFORMATION-FLOW-APPLY-PUSH-VIOLATION.md` |

## 2. Inventory

### Rust (binary behavior, CLI contracts, schema, lint)

| Path | Role |
|------|------|
| `crates/sddk-cli/tests/*.rs` | CLI unit/integration/e2e tests; contract assertions on argv, exit code, JSON output |
| `crates/sddk-domain/src/**/*.rs` | Domain unit tests |
| `crates/sddk-engine/src/**/*.rs` | Engine unit tests |
| `crates/sddk-cli/tests/cli.rs` (property tests) | Property-based testing for CLI behavior |

### Shell (pre-binary bootstrap, installer, Podman, TUI smoke)

| Path | Role |
|------|------|
| `bin/install.sh` | Installer bootstrap |
| `bin/sddk-devbox.sh` | Devbox Podman orchestration |
| `bin/uat.sh` | UAT orchestration |
| `scripts/e2e-install.sh` | E2E install smoke |
| `scripts/e2e-render.sh` | E2E render smoke |
| `scripts/validate-project.sh` | Project validation smoke |
| `tests/test_*smoke*.sh` | System-level smoke tests (if present) |
| `tests/test_*bootstrap*.sh` | Bootstrap smoke tests |
| `tests/test_*installer*.sh` | Installer smoke tests |
| `tests/test_*podman*.sh` | Podman orchestration tests |
| `tests/test_*tui*.sh` | TUI smoke tests |
| `tests-e2e/` | End-to-end integration scripts |

### Python (external golden/evaluation/analytical assets)

| Path | Role |
|------|------|
| `scripts/` | Golden-file generation, evaluation harnesses, scenario generators for SPEC-024/SPEC-040 |
| `scripts/e2e-all.sh` | E2E all-platform runner |

### JavaScript (reserved — none today)

| Path | Role |
|------|------|
| `frontend/test/` | Reserved for when `frontend/` is introduced |

## 3. Justified boundaries

| Cell | Owner | Justification |
|------|-------|---------------|
| Rust | Binary behavior, CLI contracts, schema, lint | Binary is implemented in Rust; testing its API in Rust keeps contracts near implementation; cargo test covers this surface |
| Shell | Pre-binary bootstrap, installer, Podman, TUI smoke | These operate outside the binary's hot path; shell is the natural language for system orchestration |
| Python | External golden/evaluation/analytical assets | Python's data-processing strength suits tooling that produces or consumes artifacts outside the binary's hot path |
| JavaScript | Frontend assets (reserved) | Reserved until `frontend/` is introduced |

Source: ADR-0069 §Decision.

## 4. Accidental accretion

| Item | Severity | Recorded in |
|------|----------|------------|
| Python used for shell-boundary test (INC-CYCLE-11-PYTEST-CONTRACT-P1) | medium | `docs/debt/INC-CYCLE-11-PYTEST-CONTRACT-P1.md` |
| Bats mentioned as candidate in ADR-0022, ADR-0045, REPOSITORY-TARGET-LAYOUT.md — not an accepted decision | low | Superseded by ADR-0069 §Bats reassessment |
| 19 shell contract tests introduced by `643180a` (some may test binary behavior that belongs in Rust) | medium | Phase A audit required; see §Concrete false positives |

## 5. Concrete false positives

> Tests in language X that actually test behavior belonging to language Y per ADR-0069.

| Path | Observed behavior | Correct ownership (per ADR-0069) | Recommended action |
|------|-------------------|----------------------------------|-------------------|
| `tests/test_argv_accuracy_ship.sh` | Invokes `sddk ship --help` and asserts on argv/output | Rust (binary behavior contract) | Phase B: migrate to Rust contract test after shell script parity verification |
| `tests/test_argv_accuracy_recover.sh` | Invokes `sddk recover --help` and asserts on argv/output | Rust (binary behavior contract) | Phase B: migrate to Rust contract test after shell script parity verification |
| `tests/test_argv_accuracy_plan.sh` | Invokes `sddk plan --help` and asserts on argv/output | Rust (binary behavior contract) | Phase B: migrate to Rust contract test after shell script parity verification |
| `tests/test_dry_run_non_mutation.sh` | Invokes `sddk ship/recover --dry-run` and asserts on side-effect absence | Rust (binary behavior contract) | Phase B: migrate to Rust contract test |
| `tests/test_lockstep_refusal.sh` | Invokes `sddk ship` and asserts version-lockstep behavior | Rust (cycle-46 lockstep contract) | Phase B: migrate to Rust contract test |
| `tests/test_instruction_contract.sh` | Parses `cli-usage-contract.md` matrix schema | Documentation validation (belongs in docs tooling, not binary behavior) | Accept as shell (meta-test) or migrate to docs lint tooling |

**Note:** These shell scripts test binary behavior. The fact that they are implemented as shell does not make them shell-ownership tests. Per ADR-0069, they should be Rust contract tests. Phase B migration requires: (a) same scenario passes in Rust, (b) original shell test deleted.

## 6. CI / local-gate gaps

| Gap | Evidence | Follow-up |
|-----|----------|-----------|
| `shellcheck` not in local CI gate for `tests/test_*.sh` | `grep -r shellcheck . --include="*.sh" .github/` → only in skills, not in `AGENTS.md §5` local CI gate | Phase B: add `shellcheck` for shell test scripts |
| Python linter (`ruff`) not in local CI gate for `scripts/` | `scripts/` contains shell and Python scripts; no lint in `AGENTS.md §5` gate | Phase B: add `ruff` for Python scripts in scope |
| No docs-link checker in local CI gate | `tools/docs-check/` does not exist in repo | Phase B: bootstrap `tools/docs-check/` if docs-link checking is needed; use `AGENTS.md §5` gate as canonical CI surface |
| `tools/docs-check/` and `tools/archcheck/` referenced in external CI checklists but absent in repo | `ls tools/` → only `manifest.sh` | Phase B: evaluate whether to introduce; document decision in Phase B |

Local CI gate per `AGENTS.md §5`: `cargo build --release -p sddk-cli` + `cargo test --workspace` + `cargo clippy --all-targets -- -D errors` + `cargo fmt --all -- --check`. This gate does not currently include shell lint or Python lint.

## 7. Duplicate candidates

| Paths | Observed overlap | Recommended action |
|-------|-----------------|-------------------|
| `tests/test_workflow_contract.sh` and `tests/test_adoption_contract.sh` | Both invoke `sddk` binary and assert on workflow/adoption behavior | Investigate whether scenarios are distinct or overlapping; consolidate if same scenario |
| `crates/sddk-cli/tests/cli.rs` and `tests/test_instruction_contract.sh` | Both validate CLI contract metadata | Accept (cli.rs tests runtime behavior; shell script tests schema/content) |

## 8. Non-migration list

| Path | Current language | Justification | Planned revisit |
|------|-----------------|---------------|-----------------|
| `scripts/e2e-install.sh`, `scripts/e2e-render.sh`, `scripts/e2e-all.sh` | Shell/Python mix | E2E orchestration; migrating to Rust would add complexity without coverage gain | Phase B: evaluate per-File whether orchestration belongs in Shell |
| `scripts/install.sh` | Shell | Installer bootstrap; Shell owns bootstrap per ADR-0069 | Keep in Shell |
| `bin/sddk-devbox.sh` | Shell | Devbox Podman orchestration; Shell owns Podman per ADR-0069 | Keep in Shell |

**None found this cycle** for intentional non-migration of binary-behavior tests.

## 9. Active-cycle state

| Field | Value |
|-------|-------|
| Cycle id | `p-52b95ef55999f9de/kernel-cli-agent-information-flow` |
| Status | `OPEN` |
| Phase | `build` |
| Path | `A-full` |
| Lease | `null` |
| Ledger | `event_count=855`, `last_hash=sha256:80e6936d4ec4457ca9ead34114d07c8ca10c2dff5142a8426da32907d7d53bd8` |
| Gate receipt | `gate-plan-executable-44f21103d422c598-1` |
| Transition event | `evt-00dd0cf6-bdc1-404f-bb48-dc735486a6af` |
| Commit `643180a` | `feat(uat): instruction-layer contract matrix and sizing advisory routing` — pushed to `origin/main` (apply-push violation — see INC file) |
| Deviation | `git rev-parse HEAD == git rev-parse origin/main == 643180a21ab1c9e7a63758ad221d97ec1640ae5a` — third apply-push violation in project history |
| INC record | `docs/debt/INC-KERNEL-CLI-AGENT-INFORMATION-FLOW-APPLY-PUSH-VIOLATION.md` (status: open, severity: critical, cluster: `CL-APPLY-PUSH-DISCIPLINE`) |

## 10. Bats reassessment

**Verified:** No Bats source files in repository at 2026-08-28.

`find . -path ./target -prune -o -name "*.bats" -print` returned no results. Bats build artifacts found in `target/` are incremental compilation artifacts, not test files.

Historical Bats mentions in ADR-0022, ADR-0045, and `REPOSITORY-TARGET-LAYOUT.md` are superseded by ADR-0069 §Bats reassessment, which records: Bats is NOT a strategic default; reconsider only when a genuine shell-boundary test cannot be expressed in shell + Rust without losing measurable coverage.

## 11. Phased plan

Pointer to ADR-042 §Migration plan (single source of truth):

- **Phase A** (current cycle): Audit 19 contract tests from `643180a`; classify per ADR-0069 ownership; light annotation only. See IMPLEMENTATION-BACKLOG.md current-cycle section.
- **Phase B** (next cycle): Add shellcheck to local CI gate; add Python linter for `scripts/`; evaluate ADR-0022; add ownership-prefix convention; migrate Phase A false positives after parity verification. See IMPLEMENTATION-BACKLOG.md next-cycle section.
- **Phase C** (later, gated on parity): Consolidate or delete misowned tests after parity evidence. See IMPLEMENTATION-BACKLOG.md deferred section.

## 12. Parity and stability notes (Phase C)

> Recorded 2026-08-29 as part of Phase C work.

### Parity status

| Test surface | Shell ownership | Rust ownership | Parity achieved |
|--------------|----------------|---------------|-----------------|
| `tests/test_push_prevention_hook.sh` | Shell contract test | N/A (githooks) | Stable — no Rust equivalent needed |
| `tests/test_evidence_capture_contract.js` | Node.js smoke | N/A | Stable — JS owns TUI smoke |
| `tests/test_golden_dataset_contract.py` | Python golden | N/A | Stable — Python owns golden/evaluation |
| `tests/test_workflow_contract.py` | Python workflow | N/A | Stable — Python owns workflow orchestration |
| Binary behavior (argv, exit, JSON) | Shell scripts in `tests/test_*.sh` | Rust contract tests | **Pending** — shell scripts test binary behavior that belongs in Rust per ADR-0069 |

### Stability assessment

| Cell | Language | Stability | Notes |
|------|----------|-----------|-------|
| Binary behavior | Rust | Stable | `cargo test --workspace` covers this surface |
| Shell boundary | Shell | Stable | `tests/test_*.sh` run via `bash "$f"` in justfile |
| Python scripts | Python | Stable | `ruff` has **no scope** — `scripts/` contains only `.sh` files |
| JS smoke | Node.js | Stable | `node tests/test_evidence_capture_contract.js` runs in justfile |
| sddk-testkit | Rust | Stable | Implemented; `cargo test -p sddk-testkit` passes |

### Ruff no-scope declaration

`scripts/` contains only shell scripts (`.sh`). Python files exist elsewhere in the repository (`golden-dataset/`, `skills/auto-grill/`, `tests/`), but those surfaces are out of scope for the Phase C ruff gate per ADR-0069 §Python cell definition ("external golden/evaluation/analytical assets"). Ruff is therefore **out of scope for Phase C** — the gate scope is strictly `scripts/`, which has no Python files. Follow-up cycles may extend scope to `golden-dataset/` and `skills/auto-grill/` if warranted.

### ShellCheck gate status

ShellCheck has been added to the local CI gate in `justfile` (`ci` recipe). Installation required: `sudo apt install shellcheck`. **Gate fails hard** — exits 1 when shellcheck is absent and exits 1 when any covered file fails linting.

### ADR-0022 reconciliation

- **Bats mentions**: Superseded by ADR-0069 §Bats reassessment (Bats is NOT a strategic default).
- **testkit implementation**: `crates/sddk-testkit/` exists with `InMemoryLedger`, `EventBuilder`, `CycleBuilder`, `TestRepository`, and `CliSandbox`. Status changed from `proposed` to `accepted`.
