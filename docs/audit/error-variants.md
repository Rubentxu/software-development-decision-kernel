# Cycle 3 — Error variant audit

**Date:** 2026-08-19
**Cycle:** `kernel-cycle-3-carries-over` (carry-over from cycle 2 debt-verify)
**Audit method:** `grep -rn "<EnumName>::" crates/ --include='*.rs' | grep -v "pub enum"` per enum, manual identification of variants that only appear in their own definition.

## Result: 15 unused variants trimmed, 0 `#[allow(dead_code)]` added

| Enum | Total variants | Used | Unused (trimmed) |
|------|---------------:|-----:|-----------------:|
| `CompileError` (`workflow_ir.rs`) | 10 → 8 | 8 | 2 |
| `WorkflowError` (`workflow.rs`) | 7 → 3 | 3 | 4 |
| `AttemptError` (`workflow_run.rs`) | 4 → 1 | 1 | 3 |
| `NodeRunError` (`workflow_run.rs`) | 5 → 1 | 1 | 4 |
| `WorkflowRunError` (`workflow_run.rs`) | 4 → 2 | 2 | 2 |
| **TOTAL** | **30 → 15** | **15** | **15** |

### Detail

**`CompileError` (10 → 8, −2):**
- Trimmed: `YamlSerde`, `InvariantSubsumed`
- Kept: `EmptyCapabilityAllowlist`, `ExpansionNotAllowed`, `UnsupportedSchemaVersion`, `BudgetExceedsLimit`, `OperatorNotAllowed`, `CapabilityNotInAllowlist`, `CycleDetected`, `HashCollision`

**`WorkflowError` (7 → 3, −4):**
- Trimmed: `InvalidTransition`, `InvalidStateRef`, `ManifestNotFound`, `PolicyViolation`
- Kept: `MissingArtifact`, `MissingGate`, `TransitionNotFound`

**`AttemptError` (4 → 1, −3):**
- Trimmed: `StillInFlight`, `IdempotencyCollision`, `CapsuleMissing`
- Kept: `AlreadyTerminal`

**`NodeRunError` (5 → 1, −4):**
- Trimmed: `DepsUnsatisfied`, `MaxRetriesExceeded`, `AlreadyRunning`, `CascadeRequired`
- Kept: `InvalidStateTransition`

**`WorkflowRunError` (4 → 2, −2):**
- Trimmed: `BudgetExhausted`, `IrHashMismatch`
- Kept: `InvalidTransition`, `AlreadyTerminal`

## Why no `#[allow(dead_code)]` annotations

The cycle 2 `tasks.md` T-017/T-018 estimated "5 unused + 5 kept with `#[allow(dead_code)]`" based on a placeholder — the actual audit found 15 unused and 0 kept-with-allow. Every kept variant is genuinely emitted by some code path, so no `#[allow(dead_code)]` is needed.

## Correctness verification

- `cargo build --workspace` ✅ green
- `cargo test --workspace` ✅ green (run after apply-phase; 0 regressions)
- `cargo clippy --workspace -- -D warnings` ✅ green
- 0 unreferenced variant uses detected by grep

## Cycle-4 carry-over

If a cycle-4 task needs a previously-trimmed variant back, the git history (`git log -- crates/sddk-domain/src/{workflow_ir,workflow,workflow_run}.rs`) shows the pre-cycle-3 form. Re-introduction should be paired with the live code path that emits it.