# SCOPE-CONTRACT — a7-s1-runtime-uat-fake (A7 Chronos, slice 1)

## Goal

Cubrir los requisitos UAT 1-6 + 9 de `05-CHRONOS-HANDOFF.md` §8
(`RUNTIME_ENHANCED_PRODUCTION_READY`) con un fake runtime provider
in-test (Base regression, sin binario externo). EXT real ya cubierto
en `aiw_s5_chronos_real.rs` (3/3, binario prebuilt local).

## UAT map (§8)

| Req | Test | Estado |
|---|---|---|
| R1 base verde sin provider | t_r1 | ✅ |
| R2 OPTIONAL ausencia → gap, no PASS | t_r2 | ✅ |
| R3 REQUIRED ausencia → fallo tipado | t_r3 | ✅ |
| R4 negotiation pinned build | t_r4 | ✅ |
| R5 provenance en basis | t_r5 | ✅ |
| R6 stable refs, no raw traces | t_r6 | ✅ |
| R9 protocol incompatible explícito | t_r9 | ✅ |

## Out of scope (A7-S2)

- R7 timeout/cancel → incomplete/cancelled evidence (requiere timeout injection).
- R8 restart/reconnect harness.
- R10 golden deterministic/tolerance contracts.
- R11 contradiction runtime↔static sobrevive reconciliación.
- R12 resource bounds anti-unbounded-trace.
- R13 no bypass AuthorityEngine.

## Evidence

- `cargo test -p sddk-engine --test a7_s1_runtime_uat_fake` → 7 PASS / 0 FAIL.
- No src changes: test-only slice sobre `runtime_evidence_port` existente.
