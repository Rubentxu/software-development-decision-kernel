# SCOPE-CONTRACT — C3m.4 Estados evidenciales en lugar de confidence mágica

**Cycle:** `p-63676b11dc0ef88f/c3m4-evidence-states`
**Estado:** `explore` · `Readiness: NOT_READY`
**Baseline / HEAD al abrir:** `0f8e30b4`
**Abierto por:** miniMax Code (mvs_b98f2520808543c8bfd72b7d38e01c34), session-69o

---

## 1. Qué se afirma, y qué no

**Se afirma:** que el `confidence` numérico que el roadmap quiere eliminar
(C3m.4, «eliminar confidence mágica de Snapshot L1») **no es un número mágico
sino un número con una genealogía inventada**, y que la primera pregunta
—«¿qué exige el criterio que lo respalda?»— tiene una respuesta que cambia la
naturaleza del trabajo.

**No se afirma:** que G01 no deba fulfilled, ni que el consumidor de snapshot
esté mal, ni que 0.95 y 0.5 sean números sin relación con nada. Se afirma algo
más estrecho y verificable, y está en §2.

---

## 2. La medición, y su resultado

### 2bis. El número que C3m.4 nombra

`crates/sddk-gateway/src/storage_snapshot_l1_consumer.rs:124`:

```rust
// A non-zero ledger head means the snapshot actually observed
// durable events; an empty fact log is reported at half confidence.
let confidence = if snapshot.log_head > 0 { 0.95 } else { 0.5 };
```

Medido sobre ese campo, `SecretaryProposal::confidence`:

| Pregunta | Respuesta medida |
|---|---|
| ¿Quién lo **escribe** en producción? | **un** sitio: `storage_snapshot_l1_consumer.rs:124` |
| ¿Quién lo **lee** en producción? | **nadie.** Hay **9** lecturas de un campo `confidence` en código de producto en todo el repo, y **ninguna es de un `SecretaryProposal`**: son de `ContinuationCandidate`, del trigger de `dynamic_expansion`, de `AgentContributionEnvelope`, de `UatOracleAssessment` y de `TestSelectionPlanV1` — cuatro tipos más, cada uno con su propio contrato |
| ¿Lo lee el mismo tipo `confidence` del dominio UAT? | `UatOracleAssessment.confidence` **sí se discrimina** (`LowAiConfidence: mejor confidence < 0.7`, `uat.rs:540`) y **el CLI lo muestra** (`sddk-cli/src/uat.rs:3896`). Es un tipo distinto con consumidores reales, y **queda fuera de este ciclo** |

**El número no gobierna nada.** La suite sí lo afirma, en dos tests cuyo objeto
es exactamente ese número: `empty_log_head_reduces_confidence`
(`storage_snapshot_l1_consumer.rs:189`) y `empty_log_head_e2e`
(`crates/sddk-gateway/tests/aiw_s7b_snapshot_l1_consumer.rs:57`). Es decir: **los
únicos consumidores de la confianza de un proposal son los tests que comprueban
que vale 0.95 o 0.5**, lo que es un círculo cerrado — el número existe porque un
test lo afirma, y el test lo afirma porque el número existe.

> **Corrección de un número propio, registrada porque el error va en la
> dirección que no avisa.** El primer recuento dio **8** lecturas en producción.
> Era un **8 falso por defecto del instrumento**: el corte de
> `#[cfg(test)]` acababa en el **fin del fichero**, así que no veía
> `sddk-cli/src/uat.rs:3896`, que está **después** del módulo de test de ese
> fichero y **es código de producción que muestra una confianza**. Un corte que
> se salta producción produce un número **más pequeño**, y un número más pequeño
> parece más tranquilizador. El corte ahora cierra por conteo de llaves.

### 2ter. La genealogía del número, y por qué es el hallazgo

El comentario del test e2e ancla el 0.95/0.5 a **G01**, y el `//!` del módulo
(`:3`) dice «Closes G01». Se siguió la cita hasta el final:

| Referencia | Qué dice | ¿Existe? |
|---|---|---|
| `storage_snapshot_l1_consumer.rs:6` — `//! Spec: …/aiw-s7b-…/SCOPE-CONTRACT.md` | la spec del módulo | **NO EXISTE.** El directorio existe y tiene `RECEIPT.md` y `UAT-EVIDENCE.yaml`; el `SCOPE-CONTRACT.md` no está |
| `UAT-EVIDENCE.yaml:3` | «scope: `aiw-s7-secretary-attention/SCOPE-CONTRACT.md` §S7-STOP-2 (G01+G03)» | **sí existe** — el scope real de G01 es el de **otro** ciclo |
| `aiw-s7-secretary-attention/SCOPE-CONTRACT.md:43` — texto de G01 | «snapshot Planning reconciliado, A bloquea B» | **0 menciones de `confidence` en todo el fichero** |
| `docs/history/proposals/all-proposals/2026-09-19-adaptive-inputs-workflows/uat/UAT-MATRIX.md:42` — **fila canónica de G01** | criterio: «snapshot Planning reconciliado, A bloquea B»; aceptación: «**Agenda indica candidato/causa y refs; NO autorización de ejecución por `project_next`**» | **0 menciones de `confidence`** |

Y entonces, la cláusula que sí menciona el número:

| Artefacto | Texto |
|---|---|
| `aiw-s7b-…/RECEIPT.md:42` | «G01 \| **PASS** \| evidence ref = `storage:{adapter_id}:{log_head}` (reconciled pair, A blocks B); **confidence 0.95/0.5 by head**, asserted in unit + integration» |
| `aiw-s7b-…/UAT-EVIDENCE.yaml:11` | «…; **confidence tracks durability (0.95 with head > 0, 0.5 otherwise)** via the real SecretaryL1Engine» |

**El resultado de la medición, y es el que reordena el ciclo:** la fila canónica
de G01 exige que la agenda indique **candidato, causa y refs** — y el `evidence_ref`
que el código deriva (`storage:{adapter_id}:{log_head}`) cumple eso exactamente.
**La cláusula de la confianza no está en la fila canónica, ni en el SCOPE que la
define, ni en la línea que la cita.** Está en el `RECEIPT.md` del propio ciclo
que se certificó, que la añadió a la fila y se marcó `PASS` contra ella.

**Traducción:** *un ciclo escribió una cláusula normativa, se certificó contra
su propia cláusula, y dejó dos tests que ahora defienden el número como si
fuera un requisito.* El 0.95/0.5 no es un número mágico con una base discutible:
es un número con **pedigree falso**.

### 3. La segunda instancia, medida por separado

`crates/sddk-domain/src/test_select.rs:709`:

```rust
// REQ-3: confidence = 1.0 when fully mapped
let confidence = if prop.has_unmapped { 0.0 } else { 1.0 };
```

**La rama `0.0` es código muerto.** `plan()` retorna con
`Err(AdapterError::InvalidInput)` en `:687` si `prop.has_unmapped`, y
`has_unmapped` se fija una sola vez en `:247` y no se muta (las únicas cuatro
apariciones del fichero son la definición, la construcción, y estas dos
lecturas). **La línea 709 sólo se alcanza con `has_unmapped == false`, luego
`confidence` es una constante `1.0`.** Y ese campo tampoco lo lee nadie.

Misma clase que la anterior: **un `confidence` que no puede contener información
y cuyos únicos consumidores son los tests que lo comprueban.**

---

## 4. Por qué el ciclo NO sigue a un arreglo

Porque **no se puede borrar 0.95/0.5 sin declarar antes qué exige G01**, y
actualmente **dos artefactos commiteados afirman que lo exige** (`RECEIPT.md:42`
y `UAT-EVIDENCE.yaml:11`), mientras la fila que lo define **no lo menciona**
(`UAT-MATRIX.md:42`). Borrar el número dejando esos dos artefactos como están
produce un repo donde la certificación y el código se contradicen **y nadie
sabe cuál dice la verdad** — que es exactamente el defecto que C3n existe para
erradicar, y que INC-DEBT-063 ya registrou una vez para recibos de ciclo.

Es la misma lección de C3m.0, y sale del mismo sitio: **una spec que nombra un
símbolo que ya no existe declara canónica la nada; un recibo que declara un
criterio que la spec no contiene declara canónica una invención.** Por eso el
siguiente paso no es `apply` sino una decisión normativa escrita.

---

## 5. Alcance, y lo que queda fuera

**Dentro, si la decisión se toma:**

1. Una **ADR** que declare qué exige G01 y qué es la confianza en una propuesta:
   número con `ConfidenceBasis`, o estado evidencial (`Observed` / `Empty` /
   `Stale` / `Conflicted` / `Missing`), o retirada del campo.
2. Reconciliar **los tres artefactos** que hoy se contradicen — la fila canónica,
   el `RECEIPT.md` y el `UAT-EVIDENCE.yaml` — en **el mismo movimiento** que
   cambia el código. Dejar uno de los tres desalineado repite el defecto.
3. Corregir la cita rota de `storage_snapshot_l1_consumer.rs:6`.
4. Retirar la rama muerta de `test_select.rs:709` y su `REQ-3` si ese REQ no
   existe fuera de ese comentario.
5. Guard que fije el comportamiento, falsificado con mutaciones al source.

**Fuera:**

- Tocar `UatOracleAssessment.confidence`. **Ese sí se discrimina** (`< 0.7`) y
  es un tipo distinto con consumidores reales; C3m.4 habla de Snapshot L1.
- Rediseñar `SecretaryL1Engine::propose` más allá de lo que la ADR decida.
- Recertificar AIW-S7b entero: eso es C3n.2, y este ciclo solo deja la
   medición y la decisión.

---

## 6. Criterios de aceptación

1. Existe una ADR con `status` explícito que declara qué exige G01 y qué
   significa `confidence` en una propuesta de Secretary L1.
2. Los **tres** artefactos que hoy se contradicen (fila canónica, `RECEIPT.md`,
   `UAT-EVIDENCE.yaml`) son coherentes entre sí y con el código, **verificado
   leyendo los tres**, no por el estado de un `status:`.
3. `storage_snapshot_l1_consumer.rs:6` cita un fichero que existe, o se retira la
   cita — **verificado con `test -f`**, no leyendo la línea.
4. `test_select.rs:709` ya no tiene rama inalcanzable, o el REQ que la justifica
   está escrito en un sitio normativo.
5. Un guard verde **y falsificado con mutaciones al source**, declarando
   detectadas / sobrevividas / no medibles. `NO MEDIBLE` no es `DETECTADA`.
6. Verificación con el perfil completo: `cargo test --workspace`, `fmt`, `clippy
   -D warnings`.

---

## 7. Stop conditions

1. **Si la ADR no se acepta, el ciclo se detiene en `explore`** y escribe por
   qué. No se toca el código antes de la decisión: cambiar la semántica de un
   campo público sin decir qué exige el criterio que lo respalda es la forma
   de INC-DEBT-063.
2. **Si al reconciliar aparece un tercer artefacto que contradiga a los dos
   medidos**, la medición se repite antes de tocar nada. §2 son tres
   referencias **leídas una a una**, no una jurisprudencia que se pueda citar
   de memoria.
3. **Si quitar el número rompe un consumidor real no medido en §2bis**, la
   medición estaba incompleta y se corrige la medición, no el criterio.
4. **Prohibido** declarar G01 `PASS` o `FAIL` en este ciclo: la fila canónica no
   se reescribe aquí, se reconcilia la evidencia que la sostiene.

---

## 8. Qué se entrega si el ciclo se detiene

El hallazgo, con su cadena de citas completa y verificada, más la entrada de
deuda que lo registra. **C3m.4 no construye nada sin la decisión**, del mismo
modo que C3m.1 no construyó el KMT sin entrada — y la razón es la misma en los
dos casos: *construir sobre una base que nadie ha declarado es la forma exacta de
INC-DEBT-064.*

---

## 9. La medición está autocomprobada, y falsificada

Este documento afirma rutas que existen, líneas que dicen lo que dice y conteos.
Un documento de medición que no se autocomprueba tiene el mismo problema que un
guard que no se falsifica: **afirma más de lo que ha medido**.

- `verificar-medicion.py` — comprueba **una a una** las afirmaciones de §2 y
  §3 contra el disco. Sale con código 1 y el nombre del fallo si alguna no se
  sostiene, y en ese caso **el documento no se publica**. Estado actual:
  **todas verificadas, 0 fallos**, con el recuento de lecturas **fijado** a 9
  para que una décima obligue a mirarla.
- `falsificar-medicion.sh` — aplica **12 mutaciones al source real** (sobre una
  copia del repo, nunca sobre el real) y exige que el verificador **caiga** en
  cada una. `shellcheck` limpio.

**Resultado: 12 detectadas, 0 sobrevividas, 0 no medibles.**

**La primera pasada dio 8 de 12, y las dos supervivientes fueron sobre el
verificador, no sobre el repo:**

| Supervivida | Por qué sobrevivió | Corrección, en el check |
|---|---|---|
| **M5** — el `RECEIPT.md` deja de declarar `PASS` en la fila de G01 | el check buscaba la subcadena `PASS` en el **fichero entero**, y hay filas ajenas que también la tienen | se fija **la fila de G01** y se comprueba `\| G01 \| PASS \|` en ella |
| **M7** — aparece un noveno consumidor de `confidence` en producción | el corte de `#[cfg(test)]` acababa en el **fin del fichero**, así que **no veía** el código añadido después del módulo de test | el corte cierra por **conteo de llaves** hasta devolver la profundidad a cero |

**Y la corrección de M7 cambió un número del §2 de 8 a 9**, y para eso importa: el check arreglado vio `sddk-cli/src/uat.rs:3896`, que el check
anterior no veía. *Un corte que se salta código de producción da un número más
pequeño, y un número más pequeño parece más tranquilizador.* La corrección sube
el listón; no lo baja.

Las dos correcciones tienen **mutaciones propias** (M11 y M12) precisamente para
que un arreglo hecho «para dar verde» no pueda sobrevivir sin que se note.
