---
id: INC-DEBT-066
title: "G01 se certifico PASS contra una clausula que el ciclo mismo escribio y que ni la fila canonica ni el SCOPE que define el criterio contienen, y el numero que la clausula invento (confidence 0.95/0.5) no lo lee nadie"
status: open
severity: high
priority: P1
fingerprint: "uat_row_certified_against_cycle_authored_clause"
fingerprint_aliases: []
cluster_id: CL-VERIFICATION
created: 2026-10-03
created_by: miniMax Code (mvs_b98f2520808543c8bfd72b7d38e01c34)
owner: miniMax Code (mvs_b98f2520808543c8bfd72b7d38e01c34)
detected_at: 2026-10-03
detected_in_session: session-69o
component: sddk-gateway / AIW-S7b
surface: G01 (UAT-MATRIX) vs el RECEIPT.md y UAT-EVIDENCE.yaml del ciclo aiw-s7b
related: [INC-DEBT-063, INC-DEBT-065]
references:
  - docs/roadmap/receipts/c3m4-evidence-states/SCOPE-CONTRACT.md
  - docs/roadmap/receipts/c3m4-evidence-states/verificar-medicion.py
  - docs/roadmap/receipts/c3m4-evidence-states/falsificar-medicion.sh
  - crates/sddk-gateway/src/storage_snapshot_l1_consumer.rs
  - docs/history/proposals/all-proposals/2026-09-19-adaptive-inputs-workflows/uat/UAT-MATRIX.md
---

## Qué es

**G01 está marcado `PASS` en el recibo de su ciclo, y el `PASS` incluye una
cláusula que no está en la fila que define G01.** La cláusula dice
«confidence 0.95/0.5 by head». **La fila canónica de G01 no la menciona, y el
SCOPE que transcribe esa fila tampoco.**

La cadena, medida fichero a fichero:

| Referencia | Qué dice | ¿Existe? |
|---|---|---|
| `storage_snapshot_l1_consumer.rs:6` — `//! Spec: …/aiw-s7b-…/SCOPE-CONTRACT.md` | la spec del módulo | **NO EXISTE.** El directorio existe con `RECEIPT.md` y `UAT-EVIDENCE.yaml`; el `SCOPE-CONTRACT.md` no está |
| `UAT-EVIDENCE.yaml:3` | «scope: `aiw-s7-secretary-attention/SCOPE-CONTRACT.md` §S7-STOP-2 (G01+G03)» | **sí existe** — el scope real de G01 está en el ciclo de **otro nombre** |
| `aiw-s7-secretary-attention/SCOPE-CONTRACT.md:43` | G01 = «snapshot Planning reconciliado, A bloquea B» | **0 menciones de `confidence` en todo el fichero** |
| `UAT-MATRIX.md:42` — **fila canónica de G01** | criterio: «snapshot Planning reconciliado, A bloquea B»; aceptación: «Agenda indica candidato/causa y refs; NO autorización de ejecución por `project_next`» | **0 menciones de `confidence`** |
| `aiw-s7b-…/RECEIPT.md:42` | «G01 \| **PASS** \| evidence ref = …; **confidence 0.95/0.5 by head**, asserted in unit + integration» | la cláusula **aparece aquí por primera vez**, junto al `PASS` |
| `aiw-s7b-…/UAT-EVIDENCE.yaml:11` | «…; **confidence tracks durability (0.95 with head > 0, 0.5 otherwise)**» | la repite |

**La fila canónica exige que la agenda indique candidato, causa y refs — y el
`evidence_ref` que el código deriva (`storage:{adapter_id}:{log_head}`) cumple
eso exactamente.** Lo que la fila no dice es nada sobre confianza.

## El daño concreto

**Un ciclo escribió una cláusula normativa, se certificó `PASS` contra su propia
cláusula, y dejó dos tests que ahora defienden el número como si fuera
requisito.** El número en cuestión
(`storage_snapshot_l1_consumer.rs:124`):

```rust
let confidence = if snapshot.log_head > 0 { 0.95 } else { 0.5 };
```

**Medido: ese campo no gobierna nada.** Hay **9** lecturas de un campo
`confidence` en código de producto en todo el repo y **ninguna es de un
`SecretaryProposal`** — son de `ContinuationCandidate`, del trigger de
`dynamic_expansion`, de `AgentContributionEnvelope`, de `UatOracleAssessment` y
de `TestSelectionPlanV1`. **Sus únicos consumidores son los dos tests que
comprueban que vale 0.95 o 0.5** (`empty_log_head_reduces_confidence`,
`storage_snapshot_l1_consumer.rs:189`; `empty_log_head_e2e`,
`aiw_s7b_snapshot_l1_consumer.rs:57`): el número existe porque un test lo afirma,
y el test lo afirma porque el número existe.

**Por qué `high` y no `critical`.** No hay pérdida de datos ni ruptura de
seguridad. Lo que se rompe es **la veracidad de la certificación**, que es la
única moneda del framework: un `PASS` que certifica algo que la fila no exige
hace que **`PASS` deje de significar «cumple el criterio»** y pase a significar
«cumple lo que el ciclo decidió exigir». Es la misma clase que
INC-DEBT-063 —un recibo que declara algo que la autoridad no respalda— y la
misma que llevó a C3n a existir.

**Y tiene un coste corriente:** el roadmap quiere eliminar la confidence
mágica de Snapshot L1 (C3m.4), y **no se puede** sin resolver antes qué exige
G01, porque hoy **dos artefactos commiteados afirman que la exige** y la fila
que la define **no la menciona**. Borrar el número dejando esos dos como están
produce un repo donde la certificación y el código se contradicen y nadie sabe
cuál dice la verdad.

## La segunda instancia, misma clase, medida aparte

`crates/sddk-domain/src/test_select.rs:709`:

```rust
// REQ-3: confidence = 1.0 when fully mapped
let confidence = if prop.has_unmapped { 0.0 } else { 1.0 };
```

**La rama `0.0` es código muerto**: `plan()` retorna con
`Err(AdapterError::InvalidInput)` en `:687` ante la misma condición, y
`has_unmapped` se fija una vez en `:247` y no se muta. La línea 709 sólo se
alcanza con `has_unmapped == false`, luego **`confidence` es una constante
`1.0`**. Y ese campo tampoco lo lee nadie.

Un `confidence` que **no puede contener información**, y cuyos únicos
consumidores son los tests que lo comprueban. Misma clase que el anterior.

## Cómo se mide

```bash
python3 docs/roadmap/receipts/c3m4-evidence-states/verificar-medicion.py
bash    docs/roadmap/receipts/c3m4-evidence-states/falsificar-medicion.sh
```

`verificar-medicion.py` comprueba **una a una** las afirmaciones de esta
incidencia y sale con código 1 si alguna no se sostiene. Estado: **todas
verificadas, 0 fallos**.

`falsificar-medicion.sh` aplica **12 mutaciones al source real** sobre una copia
del repo y exige que el verificador caiga en cada una: **12 detectadas, 0
sovrevividas, 0 no medibles**, `shellcheck` limpio.

**La primera pasada dio 8 de 12 y las dos supervivientes fueron defectos del
verificador, no del repo** — y son la parte que conviene leer, porque un
verificador que sobrevive a su propia mutación es un documento que se certifica
solo:

| Supervivida | Por qué sobrevivió | Corrección |
|---|---|---|
| el `RECEIPT.md` deja de declarar `PASS` en G01 | el check buscaba `PASS` en el **fichero entero**, y hay filas ajenas que también lo tienen | se fija **la fila de G01** |
| aparece un noveno consumidor en producción | el corte de `#[cfg(test)]` acababa en el **fin del fichero**, y no veía el código añadido **después** del módulo de test | el corte cierra por **conteo de llaves** |

**La segunda corrección cambió un número: el recuento de lecturas en producción
subió de 8 a 9**, porque el verificador arreglado vio
`sddk-cli/src/uat.rs:3896`, que el anterior no veía. *Un corte que se salta
código de producción da un número más pequeño, y un número más pequeño parece
más tranquilizador.*

## Salidas

Son del **operador**, y ninguna es un arreglo mecánico:

1. **Declarar qué exige G01** (ADR), y reconciliar **en el mismo movimiento** la
   fila canónica, el `RECEIPT.md` y el `UAT-EVIDENCE.yaml` con el código. Dejar
   uno de los tres desalineado repite el defecto.
2. **Decidir qué es `confidence` en una propuesta**: número con
   `ConfidenceBasis`, estado evidencial (`Observed` / `Empty` / `Stale` /
   `Conflicted` / `Missing`), o retirada del campo.
3. **Corregir la cita rota** de `storage_snapshot_l1_consumer.rs:6`.
4. **Retirar la rama muerta** de `test_select.rs:709`, o escribir el `REQ-3` que
   la justifica en un sitio normativo en vez de en un comentario.

## Lo que esta incidencia NO dice

- **No dice que G01 no se cumpla.** El `evidence_ref` derivado cumple la fila
  canónica. Lo que se impugna es la **cláusula añadida** y el `PASS` que la
  cubre.
- **No dice que la confianza sea inútil en general.**
  `UatOracleAssessment.confidence` **sí se discrimina** (`< 0.7`) y el CLI la
  muestra. El defecto es de `SecretaryProposal.confidence`, no del dominio.
- **No es un problema de seguridad ni de pérdida de datos**, y por eso no es
  `critical`.

## Cierre

Cierra cuando los **tres** artefactos sean coherentes entre sí y con el código,
verificado leyendo los tres — no leyendo un `status:`. **No cierra por antigüedad
ni porque el número desaparezca**: cerrar cuando el número ya no está, sin
declarar qué exigía G01, deja el mismo hueco con menos código.
