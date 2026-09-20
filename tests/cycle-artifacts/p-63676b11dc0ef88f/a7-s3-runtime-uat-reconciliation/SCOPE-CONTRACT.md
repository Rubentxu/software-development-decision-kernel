# SCOPE-CONTRACT — a7-s3-runtime-uat-reconciliation (A7 Chronos, slice 3)

## Goal

UAT R8, R11, R13 de `05-CHRONOS-HANDOFF.md` §8 — cierre del bloque A7 UAT.

| Req | Test | Estado |
|---|---|---|
| R8 restart/reconnect → sesión nueva, independiente y completa | t_r8 | ✅ |
| R11 contradicción runtime↔static sobrevive como `Conflicted` | t_r11 | ✅ |
| R13 no bypass: AuthorityEngine deniega actor sin capability | t_r13 | ✅ |

## Notas

- R8 se valida a nivel port (sesión nueva por capture, sin estado
  residual). El harness de proceso real del adapter MCP queda fuera
  del scope fake-driven de A7; el invariante (ids distintos,
  resultado completo tipado) queda pinado.
- R11 usa `resolve_subject` (contradiction-preserving resolution,
  REQ-A4S0-013): Affirms(RuntimeProvider) + Denies(StaticProvider)
  → Conflicted con ambas listas preservadas.
- R13 usa `DefaultAuthorityEngine::admit` con política
  `default_low_risk`: actor sin "cycle.lifecycle" → Deny; actor
  privilegiado → Allow (prueba de que es decisión del engine).

## Evidence

- `cargo test -p sddk-engine --test a7_s3_runtime_uat_reconciliation` → 3 PASS / 0 FAIL.
- Test-only; sin cambios en src.
