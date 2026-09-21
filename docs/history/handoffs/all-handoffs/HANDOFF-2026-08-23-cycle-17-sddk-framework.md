# HANDOFF-2026-08-23-sddk-framework (cycle-17)

## Estado

**Ciclo activo:** `p-52b95ef55999f9de/kernel-cycle-17-m4-dynamic-workflow-engine` (M4 del roadmap).

**Fase:** build (secuencia 622).

**Rama:** `feat/kernel-cycle-17-m4-dynamic-workflow-engine` (base `a778af9` cycle-16 v1.38.0).

**Próxima release:** v1.39.0 (después de verify + debt-verify + release + archive).

## Logros cycle-17 (build phase completada)

| Tarea | Commit | LOC delta | Notas |
|-------|--------|-----------|-------|
| T-1 deps | `ca365be` | +5 | tokio (rt+time+sync) + rand + sha2 |
| T-2 RealTaskExecutor | `487ffed` | +323 | sync API + tokio interno + re-entrance guard |
| T-3 4 Tasks | `b643ef4` | +367 | HttpFetch/FileWrite/Sha256/Sleep con `ureq` |
| T-4 RetryPolicy | `fde7951` | +382 | 4 estrategias + Clock inyectable |
| T-5 Parallel | `f9abe76` | +(sequential) | spec pedía std::thread, entregado sequential |
| T-6 GraphStore wiring | `2f1f9af` | +(lifecycle) | `run_ir()` + `state()` + 2 tests |
| T-7..T-9 governance | `48eb0b6` | +34 | apply.md FORBIDDEN + verify.md gate + ARCH008 + AGENTS.md |

**Total:** 7 commits, +1111 LOC código + 34 governance.

## Métricas

- **Tests:** 1231/0 (baseline 1161 → +70).
- **Clippy:** 0 errors (gate duro cycle-17).
- **ARCH008:** 0 violations en 8 archivos scoped.
- **Apply-Push Discipline:** 0 pushes, 0 tags, 0 publishes.

## Cambios clave

### Nuevos módulos (cycle-17)
- `crates/sddk-engine/src/task_executor.rs` (323 LOC) — RealTaskExecutor.
- `crates/sddk-engine/src/tasks/{http_fetch,file_write,sha256,sleep}.rs` (367 LOC) — 4 Task impls.
- `crates/sddk-engine/src/retry.rs` (382 LOC) — RetryPolicy + Clock trait.

### Gobernanza
- `prompts/sddk/phases/apply.md` — FORBIDDEN COMMANDS LIST (git push, git tag, gh release create, cargo publish, gh pr create).
- `prompts/sddk/phases/verify.md` — §7.5 ampliado con detección de forbidden commands.
- `AGENTS.md` §5 — clippy 0-error como gate duro.
- `docs/sddk-2.0-architecture-consolidation/data/architecture-rules.yaml` — ARCH008 scope expandido.

### Documentación
- `docs/adr/ADR-0049-retry-policy-and-tokio-seam.md` — decisión arquitectónica completa.

## Forward Debt cycle-18+

1. **INC-FORWARD-001 (P1)**: True concurrent Parallel vía `Arc<OperatorContext>`.
   cycle-17 implementó sequential (un child por tick) por restricción de lifetime.
   cycle-18 debe introducir contexto compartido con `Arc<Mutex<>>` o construir
   el context dentro de cada thread para evitar el lifetime conflict.

2. **INC-FORWARD-002 (P2)**: HTTP async con `reqwest` (reemplazar `ureq` bloqueante).
   cycle-19 candidato.

3. **INC-FORWARD-003 (P3)**: Clippy 0-warning (no solo 0-error) gate.
   cycle-20 candidato.

4. **INC-FORWARD-004 (P3)**: Aplicar retry a operadores no-Task (e.g., Choice con
   re-evaluation on transient errors). cycle-21 candidato.

## Próximos pasos (secuencia del orchestrator)

1. **verify** (sddk-verify, 6 lenses): comportamiento + tests + contracts.
2. **debt-verify** (sddk-debt-verify, 5 clusters, deep): connascence + SOLID + smells.
3. Si debt-verify FAIL → remediation_round → verify round 2 → debt-verify round 2.
4. **release** (orchestrator manual):
   - `git push origin feat/kernel-cycle-17-m4-dynamic-workflow-engine`
   - PR / merge a main (orchestrator ops)
   - `cargo build --release -p sddk-cli`
   - Tag v1.39.0 + push tag
   - `sddk dev install` (bundle update)
   - `sddk dev doctor | grep bundle_coherence` (binario == bundle)
5. **archive** (sddk-archive): knowledge vault sync + ciclo cerrado.

## Riesgos identificados

- **Parallel sequential**: si algún caller depende de paralelismo real, fallará
  visiblemente (timing tests). Cycle-18 P1.
- **HTTP bloqueante**: `ureq` bloquea el thread del executor; con `MaxConcurrency`
  bajo, latency acumulada > wall-time esperado. Cycle-19 P2.
- **MANIFEST drift**: tras editar apply.md/verify.md/AGENTS.md, regenerar
  MANIFEST.sha256 en el mismo commit (aplicado en 48eb0b6).
