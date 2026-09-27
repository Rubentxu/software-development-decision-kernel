---
id: INC-AUDIT-S14-TEST-PORTS-UNCONSUMED
title: "test_ports.rs declara 9 traits sin ningun consumidor fuera de sddk-domain"
status: open
severity: high
priority: P1
created: 2026-09-27
discovered_by: session-14 audit (code-based, no documentado previamente)
cluster_id: CL-SPECULATIVE-GENERALITY
fingerprint: "sddk_test_ports_unconsumed_v1"
---

## Qué es

`crates/sddk-domain/src/test_ports.rs` (1.033 LOC) declara 9 traits de
arquitectura de plugins para el subsistema de verificación con testing
scoped. Además, `crates/sddk-domain/src/test_apply.rs` (1.358 LOC) es una
librería genérica construida sobre esos ports.

## Evidencia (verificada, no documental)

Los 9 traits tienen **cero referencias fuera de `sddk-domain`**:

| Trait | Ocurrencias repo | Fuera de `sddk-domain` | Implementadores |
|---|---|---|---|
| `ActiveChangeSetPort` | 6 | **0** | solo fakes de test |
| `ProjectTopologyPort` | 4 | **0** | 0 |
| `SutGraphPort` | 1 | **0** | **0** |
| `VerificationCapabilityRegistry` | 1 | **0** | 0 |
| `TestCatalogPort` | 1 | **0** | 0 |
| `TestImpactPlannerPort` | 12 | **0** | solo `MockPlanner` de test |
| `VerificationExecutorPort` | 4 | **0** | solo `FakeExecutor` de test |
| `TestEvidenceRepository` | 7 | **0** | 0 |
| `VerificationPolicyPort` | 1 | **0** | 0 |

`TestApplySession` (`test_apply.rs:197`) tiene **0 consumidores** fuera de
`sddk-domain`.

Comando de reproducción:

```
for t in ActiveChangeSetPort ProjectTopologyPort SutGraphPort \
         VerificationCapabilityRegistry TestCatalogPort \
         TestImpactPlannerPort VerificationExecutorPort \
         TestEvidenceRepository VerificationPolicyPort; do
  echo "$t: $(grep -rn "\b$t\b" crates/ --include='*.rs' \
    | grep -v 'crates/sddk-domain/' | wc -l)"
done
```

## Por qué importa

~2.400 LOC de arquitectura de plugins compilada, testeada y mantenida sin
un solo consumidor de producción. Los únicos implementadores son fakes
dentro del propio crate de los tests que los exercise. Es generalidad
especulativa en el sentido de Ousterhout: se paga el coste de
mantenimiento sin cobrar el beneficio de la abstracción.

## Regla propuesta (para que no se repita)

**Un port no entra al repo sin su primer consumidor externo en el mismo
commit.** Si el consumidor es el subsistema de `prompts/sddk/change-scoped-testing.md`
(C5), entonces entra con él. Si no, se borra.

## Opciones

- **(a) Conectar** con el consumidor real cuando C5 lo requiera. Coste: el
  trabajo de C5. Beneficio: la abstracidad se justifica sola.
- **(b) Borrar** `test_ports.rs` + `test_apply.rs`. Ahorro neto ~2.400 LOC.
  Riesgo: bajo, verificado 0 consumidores.

## Estado

Abierto. No se ejecuta en este ciclo — es un WorkItem propio, no un
quick win. La decisión requiere saber si C5 va a necesitar el
impact-planner.
