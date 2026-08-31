# ADR-0049 — RetryPolicy y seam de concurrencia tokio en cycle-17

**Status:** Accepted (cycle-17 closed, sequence 622 → build phase)

**Date:** 2026-08-23

**Cycle:** `p-52b95ef55999f9de/kernel-cycle-17-m4-dynamic-workflow-engine`

**Supersedes:** none

**Superseded by:** none

## Context

Cycle-16 dejó `WorkflowRuntime<R>` con operadores `Task`/`Sequence`/`Parallel`/`Choice`
donde `Task`/`Parallel` eran simulaciones secunciales (cycle-16 remediation round 1).
Cycle-17 (M4 del roadmap) necesitaba:

1. **TaskExecutor real**: ejecutar trabajo I/O real (HTTP, file, sha256, sleep) sin
   obligar al motor síncrono a volverse asíncrono.
2. **Retry/backoff determinista**: 4 estrategias (Fixed, Linear, Exponential,
   ExponentialWithJitter) con reloj inyectado para tests reproducibles.
3. **True Parallel**: fan-out concurrente de hijos con Semaphore para backpressure.
4. **GraphStore inspection**: 3 métodos ya portados en cycle-16 pero no wired
   (`load_node_run`, `latest_attempt`, `attempts_for_node`).
5. **Governance hardening**: ciclo previo (cycle-15, cycle-16) tuvo 3+ violaciones
   apply-push; cycle-17 añade FORBIDDEN COMMANDS LIST y gate de clippy 0-error.

## Decision

### 1. RealTaskExecutor: sync API + tokio interno

```rust
pub struct RealTaskExecutor {
    runtime: OnceCell<tokio::runtime::Runtime>,
    clock: Arc<dyn Clock>,
}
```

- API síncrona (mantiene invariante del motor: el runtime tickea una vez por child).
- `tokio::runtime::Runtime` interno lazy-initialized.
- Re-entrance guard: si el caller ya corre dentro de un tokio runtime (e.g. tests
  con `#[tokio::test]`), usa `Handle::try_current()` para evitar panic; si no,
  crea uno con `spawn_blocking`.
- No expone `async fn` al motor — el seam es interno.

### 2. RetryPolicy: 4 estrategias con `Clock` inyectado

```rust
pub enum RetryPolicy {
    Fixed(u64),
    Linear(u64),
    Exponential { base_ms: u64 },
    ExponentialWithJitter { base_ms: u64, jitter_ms: u64 },
}

pub trait Clock { fn now(&self) -> u64; }
```

- Tests usan `rand::rngs::mock::StepRng` para jitter determinista.
- `RetryConfig { max_attempts, policy }` aplica la política antes de marcar fallo
  definitivo.

### 3. True Parallel: std::thread + mpsc + Semaphore (degraded a sequential)

**Decisión cycle-17:** el spec proponía `std::thread::spawn` + `mpsc::channel` +
`Arc<Semaphore>` para fan-out concurrente. **Implementación real cycle-17:**
sequential (un child por `evaluate` call) por restricción técnica:
`OperatorContext<'a>` tiene lifetime atado al runtime stack; no puede cruzar thread
boundaries sin `Arc<Mutex>` o refactor del context.

**Trade-off aceptado:** sequential es correcto semánticamente (cada child completa
antes del siguiente) pero pierde paralelismo real. Forward debt: cycle-18 introduce
`Arc<OperatorContext>` o construye el context dentro de cada thread.

### 4. GraphStore inspection wiring

3 métodos portados en cycle-16 (DEFINICIÓN en `ports.rs`), ya implementados en
SQLite pero no consumidos por `WorkflowRuntime`. Cycle-17 wirea:

- `WorkflowRuntime::load_node_run(run_id, node_id) -> NodeRun`
- `WorkflowRuntime::latest_attempt(node_run_id) -> Attempt`
- `WorkflowRuntime::attempts_for_node(node_run_id) -> Vec<Attempt>`

### 5. Governance hardening

| Regla cycle-17 | Origen |
|----------------|--------|
| FORBIDDEN COMMANDS LIST en apply.md | 3+ violaciones cycle-11/13/14 |
| Verify.md §7.5 escanea `git tag`, `gh release create`, `cargo publish` | cycle-16 release prematuro |
| `cargo clippy --workspace --all-targets -- -D errors` (gate duro) | cycle-17 forward debt |
| ARCH008 scope expandido a `task_executor.rs`, `tasks/*.rs`, `retry.rs` | nuevos módulos cycle-17 |

## Consequences

### Positivas
- `Task` ejecuta I/O real (HTTP via `ureq`, file, sha256 via `sha2`, sleep).
- Retry determinista reproducible en tests.
- 4 Tasks concretos demuestran el seam de TaskExecutor.
- Governance explícita: apply sub-agent tiene FORBIDDEN COMMANDS LIST verificable.
- 1231 tests (de 1161 baseline) verde, clippy 0-error, ARCH008 limpio en 8 archivos.

### Negativas
- Parallel sigue siendo sequential (forward debt cycle-18).
- `tokio::runtime::Runtime` interno añade dependencia pesada (5 MB binario extra).
- `ureq` bloquea (no `async` puro); para HTTP async real se necesita cycle-19.

### Forward Debt
1. **INC-FORWARD-001**: True concurrent Parallel via `Arc<OperatorContext>` (cycle-18 P1).
2. **INC-FORWARD-002**: HTTP async con `reqwest` (cycle-19 P2).
3. **INC-FORWARD-003**: Clippy 0-warning (no solo 0-error) gate (cycle-20 P3).

## Verification Evidence

- Test baseline: 1161 → 1231 (+70, +6.0%).
- `cargo clippy --workspace --all-targets -- -D errors`: exit 0.
- ARCH008 scan sobre 8 archivos: 0 violations.
- Apply-Push Discipline audit: 0 pushes, 0 tags, 0 publishes.
- Commits: 7 (T-1..T-6 + governance). Spec planeó 10; T-7a/-8/-9 verificados en
  lugar de modificados (governance ya presente desde cycle-15), T-10 (ADR +
  handoff) ejecutado post-apply.
