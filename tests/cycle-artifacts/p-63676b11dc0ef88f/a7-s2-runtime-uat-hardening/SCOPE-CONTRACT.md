# SCOPE-CONTRACT — a7-s2-runtime-uat-hardening (A7 Chronos, slice 2)

## Goal

Hardening UAT R7, R10, R12 de `05-CHRONOS-HANDOFF.md` §8, fake-driven.

| Req | Test | Estado |
|---|---|---|
| R7 timeout/cancel → typed error, no éxito parcial | t_r7 | ✅ |
| R10 determinismo (digest estable en capturas repetidas) | t_r10 | ✅ |
| R12 resource bounds (budget 0 fail-closed, shape acotado) | t_r12 | ✅ |

## Out of scope (A7-S3)

- R8 restart/reconnect (requiere harness de proceso).
- R11 contradiction runtime↔static survives reconciliation.
- R13 no bypass AuthorityEngine.

## Evidence

- `cargo test -p sddk-engine --test a7_s2_runtime_uat_hardening` → 3 PASS / 0 FAIL.
- Test-only; sin cambios en src.
