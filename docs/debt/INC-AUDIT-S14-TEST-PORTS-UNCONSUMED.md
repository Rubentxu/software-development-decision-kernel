---
id: INC-AUDIT-S14-TEST-PORTS-UNCONSUMED
title: "test_ports.rs: 9 traits del SPI de SPEC-043, implementados dentro del crate pero sin consumidor externo aún"
status: open
severity: medium
priority: P2
created: 2026-09-27
revised: 2026-09-27
discovered_by: session-14 audit (code-based)
cluster_id: CL-SPECULATIVE-GENERALITY
fingerprint: "sddk_test_ports_unconsumed_v2"
fingerprint_aliases: ["sddk_test_ports_unconsumed_v1"]
severity_history: "high/P1 en la v1 de este INC; revisado a medium/P2 tras verificar los consumidores internos — ver 'Corrección de la auditoria inicial'"
---

## Corrección de la auditoría inicial

La primera versión de este INC severizó esto **high/P1** y recomendó
borrarlo como generalidad especulativa. **Era una conclusión
equivocada**, causada por medir el criterio equivocado: se contaron
consumidores *fuera* de `sddk-domain` (0) y se dedujo de ahí la
conclusión de que nada lo usaba.

Verificación posterior: hay **consumidores e implementadores reales
dentro del propio crate**:

| Fichero | Relación |
|---|---|
| `test_adapters.rs:206` | `impl ActiveChangeSetPort for ProfileAdapterV1` |
| `test_adapters.rs:239` | `impl ProjectTopologyPort for ProfileAdapterV1` |
| `test_evidence.rs:512` | `impl TestEvidenceRepository for EvidenceStoreV1` |
| `test_select.rs:33` | consume `test_ports::` en código de producción |
| `lib.rs:44,47,119,124` | `pub mod` + re-exports públicos |

Y lo decisivo: los 9 traits están **nombrados como requirement en
`SPEC-043-CHANGE-SCOPED-VERIFICATION-SERVICE.md` §4 "Ports and adapter
SPI"** — la spec que gobierna C5 (change-scoped verification).

## Qué es realmente

El SPI de ports que SPEC-043 §4 exige, con 4 de sus implementadores ya
escritos (`ProfileAdapterV1`, `EvidenceStoreV1`) y el resto pendiente.
El trabajo está **a medio camino por diseño**: los ports y la primera
implementación existen; el consumidor externo (el planner de
change-scoped verification) todavía no.

No es generalidad especulativa. Es arquitectura hexagonal con el
producto a medio construir, que es un estado legítimo y a menudo
correcto cuando la spec que la exige está aceptada y commitment existe.

## Por qué sigue siendo deuda (severity medium, no low)

- ~2.391 LOC (`test_ports.rs` 1.033 + `test_apply.rs` 1.358) que se
  compilan, testean y mantienen a cambio de un único consumidor
  interno.
- `SutGraphPort`, `TestCatalogPort`, `VerificationCapabilityRegistry` y
  `VerificationPolicyPort` **no tienen implementador** (0 referencias
  fuera de su propia declaración), ni siquiera interno.
- El riesgo no es que sobre código: es que C5 se retrase y esta capa
  quede pagándose indefinidamente sin el consumidor que la justifica.

## Regla propuesta (la que sigue valiendo)

**Un port sin implementador ni consumidor externo no entra al repo sin
su primer consumidor real en el mismo commit.** Si el consumidor es el
planner de C5, entonces entra con él. `test_ports.rs` es la excepción
que prueba la regla: tiene el implementador parcial, así que la regla se
aplaza al resto.

## Opciones

- **(a) Conectar con C5** — implementar el planner de change-scoped
  verification como primer consumidor real de los 9 ports. Es el
  camino que la spec ya tiene decidido. Coste: el trabajo de C5.
  Beneficio: la capa se justifica sola.
- **(b) Recortar a lo que tiene consumidor** — retirar los 4 ports sin
  implementador (`SutGraphPort`, `TestCatalogPort`,
  `VerificationCapabilityRegistry`, `VerificationPolicyPort`) y
  mantener los 5 con implementador. Ahorra ~fracción y reduce la
  superficie sin collo de C5. Riesgo: bajo, pero contradice SPEC-043 §4.
- **(c) Nada ahora** — el estado es legítimo si C5 arranca pronto.

## Decisión recomendada

(a), pero **con C5 como gate explícito**: si C5 no arranca, esta capa se
replantéa con (b). Lo que no conviene es el limbo actual: 2.391 LOC
pagándose sin fecha de consumidor.

## Estado

Abierto. **No se borra** — la recomendación original (borrar) queda
anulada por esta revisión. El trabajo corresponde a C5, no a una
limpieza de higiene.
