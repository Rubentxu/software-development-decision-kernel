# C3m-identity-alias — SCOPE-CONTRACT: alias de identidad de proyecto a nivel de storage

**Cycle:** `p-63676b11dc0ef88f/identity-alias` (C3m, Semantic & Boundary Convergence)
**Baseline:** `main@e5b03b7d` (workspace v2.5.3, declarada no publicada)
**Opened:** 2026-10-02T08:45:00Z
**Owner:** orchestrator (ejecución directa)
**Authority basis:** AGENTS.md §3 (gates preautorizados); **ADR-0152 `status: proposed` — la implementación NO arranca hasta que el operador lo acepte**.
**Closes:** INC-DEBT-050 (critical/P1), parte de INC-DEBT-049 (high/P1)

## 1. Objetivo (falsable)

Que `sddk` resuelva la identidad de un proyecto a través de un alias
almacenado, de modo que los **25 receipts huérfanos** de esta máquina se
resuelvan a la identidad que tiene su historial, **sin escribir una sola fila
en el fact log**.

**Criterio de salida:** `scripts/migrate_project_identity.py audit` reporta
**0** receipts huérfanos, y el recuento total de filas en las seis tablas
append-only de los 15 ledgers afectados es **exactamente el mismo** que antes
(3.477 más lo que se añada después). Si el número de filas cambia, el criterio
falla.

## 2. Lo que ya está medido (OBSERVED, esta máquina, 2026-10-02)

| magnitud | valor |
|---|---|
| receipts con `project_id` que no coincide con la derivación actual | 25 de 148 |
| proyectos afectados | 15 |
| filas en tablas append-only | 3.477 |
| proyectos migrables reescribiendo el fact log | **0** |
| proyectos con dos identidades vivas | 8 (6 con ciclos en ambos lados, 2 con el nuevo vacío) |
| ciclos que existen sólo del lado nuevo | 51 |
| colisiones de nombre corto al unificar | 1 (`r6-workers-probe-wiring`) |
| receipts de este repo con pin ya aplicado | 1 checkout, `pinned_at: 2026-10-01T11:48:55Z` |

## 3. No-objetivos

- **NO** tocar una sola fila de `events_v1` ni de las otras cinco tablas
  append-only. El trigger lo prohíbe y el hash lo detectaría.
- **NO** fusionar los dos ledgers de un proyecto con dos identidades. Exige
  reescribir `cycle_id`, y hay una colisión de nombre corto.
- **NO** borrar `.sddk/project-pin.json` de este checkout en este ciclo. El pin
  y el alias coexisten hasta que el alias esté desplegado y verificado; quitar
  el pin antes sería dejar el checkout sin la mitigación que hoy funciona.
- **NO** re-derivar ni «arreglar» ids antiguos. Un alias no cambia lo que se
  emitió; cambia a qué apunta lo que se resuelve hoy.

## 4. Superficie

| fichero | cambio |
|---|---|
| `crates/sddk-domain/src/identity.rs` | tabla `project_aliases`, resolución transitiva, detección de ciclo |
| `crates/sddk-cli/src/lib.rs` | encadenar el alias en `resolve_identity_honoring_pin` (**un solo punto**) |
| `crates/sddk-cli/src/lib.rs` | `sddk project alias --from --to --reason` (`--reason` obligatorio) |
| `crates/sddk-cli/src/lib.rs` | `sddk adopt status` y `project resolve` **declaran** que resolvieron por alias |
| `crates/sddk-storage/` | migración de esquema de la tabla, append-only |

## 5. Plan de test (scoped)

Anclado a los criterios 1–6 de **ADR-0152 § Verification**. Cada criterio trae
su falsificador; un criterio sin falsificador que se pueda ejecutar no cuenta.

1. Sin alias, la resolución no cambia. Falsificador: sembrar un alias que no
   aplica y exigir identidad idéntica.
2. Ciclo `A -> B -> A` ⇒ **error duro**, salida no cero, nombra el ciclo.
   Falsificador: un guard que sólo avisa es FAIL.
3. La declaración aparece en la salida de `adopt status` y `project resolve`.
   Falsificador: borrar la línea y exigir que el test falle.
4. Borrar un alias falla (append-only). Falsificador: ejecutar el borrado.
5. Los 25 aliases aplicados ⇒ `audit` en 0 y recuento de filas append-only
   idéntico. Falsificador: cambiar el número de filas ⇒ FAIL.
6. `verify_stream_chain` sigue OK sobre el stream canónico de al menos un
   proyecto con fact log. **Es el criterio que ningún otro cubre.**

## 6. STOP conditions

- Si el hash de algún evento cambia al aplicar un alias ⇒ **parada inmediata**:
  significa que se tocó el fact log.
- Si `resolve_identity_honoring_pin` deja de ser el único resolutor ⇒ parada.
  Ese fue el defecto que W2c corrigió (cinco resolutores desincronizados).
- Si el recuento de filas append-only no coincide exactamente ⇒ parada, antes
  de intentar «ajustar» nada.
- Si el operador no acepta ADR-0152, este ciclo no arranca. El documento queda
  como propuesta.

## 7. Entregables

- `ADR-0152` aceptado (o rechazado con motivo).
- Tabla, resolución, comando y declaración, con los 6 criterios verdes.
- Los 15 aliases declarados sobre el storage real, cada uno con su `reason`.
- `migrate_project_identity.py audit` en 0, con el apéndice del criterio 5.
- INC-DEBT-050 cerrada; INC-DEBT-049 reducida a su parte no resuelta por esto.

## 8. Riesgos

| riesgo | por qué importa | mitigación |
|---|---|---|
| Alias mal escrito manda una identidad a otro sitio | no hay comparación posible: el código no sabe qué alias es «correcto» | acción explícita y auditada, `--reason` obligatorio, y **declaración visible** en la salida |
| La indirección se vuelve permanente | cada resolución futura pasa por la tabla | es el mismo coste que un rename en git, y la misma razón por la que se acepta |
| Un alias tapa una separación real | dos proyectos distintos podrían converger sin querer | `--to` debe existir ya en el storage: no se crea un destino nuevo |
| Ciclo de aliases | resolución no terminante | error duro, con el ciclo nombrado |
| Que el criterio 5 «pase» sin migrar nada | un verde vacío es peor que un gate ausente | los criterios 1–4 y 6 tienen que pasar también; y 5 compara un número, no un bool |

## 9. Fuera de alcance

- INC-DEBT-051 (contrato de versión por adapter) — otro ciclo.
- INC-DEBT-057 (corrupción de script en docs/) — requiere que quien escribió
  cada frase diga qué quiso decir.
- La clave del KMS, que bloquea v2.5.3 y es del operador.
- Publicar el alias en una release: primero verde local, después release.

---

## Addendum — lote 1 (dominio) ejecutado, y un defecto que encontró el falsificador

**PRE-FLIGHT:** `PRE-FLIGHT.md`, `Readiness: READY`.
**Superficie tocada:** `crates/sddk-domain/src/identity.rs` y nada más, como
el pre-flight acotaba. Ni schema, ni CLI, ni storage.

### Lo implementado

`ProjectAlias` (from/to/reason/created_at), `AliasTable` (append-only por
construcción: no hay `remove`), `AliasResolution` (id final más los saltos,
para que la declaración del ADR regla 4 tenga de dónde servirse), y las dos
variantes de error.

### Verificación

```text
cargo test -p sddk-domain --lib identity                      57 passed; 0 failed
cargo test -p sddk-domain --lib                               574 passed; 0 failed
cargo fmt --check                                             limpio
cargo clippy -p sddk-domain --all-targets -- -D warnings      limpio
cargo check --workspace --all-targets                         exit 0
```

El `check --workspace` no es opcional aquí: `IdentityError` es un enum
**público** y añadir una variante rompe cualquier `match` exhaustivo de otro
crate. Los usos que hay son todos construcciones, pero eso se verificó
compilando, no leyendo.

### El falsificador encontró un defecto real en MI test

**6 mutaciones, 5 detectadas. La que no:** `no_cycle_detection`. Al quitar la
detección de ciclo, la suite seguía **verde**.

La causa: el test afirmaba sobre el **texto** del error (`contains("cycle")`).
Sin detección, un ciclo de dos saltos no se colgaba — corría hasta el tope de
16 saltos y devolvía `AliasCycle` igualmente, con un mensaje que casaba con las
aserciones. El test pasaba **por el motivo equivocado**.

Arreglo: `AliasChainTooLong` es ahora una variante **distinta** de `AliasCycle`,
y los tests afirman sobre la variante con `matches!`, no sobre el texto. Un
ciclo y una tabla malformada son defectos distintos con arreglos distintos — re-
apuntar un alias frente a reparar la tabla — y un test que no los distingue no
puede certificar ninguno.

Re-falsificado: **6/6 detectadas**, con `no_cycle_detection` rompiendo 2 tests.

Es la cuarta vez en esta sesión que un falsador encuentra un defecto que leer
no habría encontrado, y la primera que encuentra uno **en el trabajo de esta
sesión** en vez de en algo heredado.

### Lotes que quedan

| lote | superficie | por qué separado |
|---|---|---|
| 2 | persistencia de la tabla | un fallo de lógica y uno de cableado tienen que ser distinguibles por el resultado |
| 3 | `resolve_identity_honoring_pin` + `sddk project alias` + declaración en la salida | el cableado es donde se re-introduce el defecto de los cinco resolutores |

Ninguno de los dos toca el storage real de la máquina. Los 15 aliases se
aplican después, con su propio criterio de recuento de filas.
