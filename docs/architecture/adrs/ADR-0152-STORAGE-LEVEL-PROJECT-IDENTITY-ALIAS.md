---
id: ADR-0152-STORAGE-LEVEL-PROJECT-IDENTITY-ALIAS
title: Resolve project identity through a storage-level alias, because the identity is immutable once the fact log exists
status: proposed
proposed_at: 2026-10-02
cycle: p-63676b11dc0ef88f/identity-alias
supersedes: null
superseded_by: null
component: identity
surface: crates/sddk-domain/src/identity.rs
closes: [INC-DEBT-050, INC-DEBT-049]
---

# ADR-0152 — Storage-level project identity alias

> Status: **proposed**. La decisión normativa corresponde al operador; lo que
> este documento hace es dejar la pregunta con sus tres costos escritos, porque
> hasta ahora nadie la había escrito y las tres partes la asumieron
> distinto.
> Cycle: `p-63676b11dc0ef88f/identity-alias`
>
> **Implementation progress (session-66, lote 1 de 3):** la capa de dominio
> está implementada y falsificada — `ProjectAlias`, `AliasTable`,
> `AliasResolution` y las dos variantes de error en
> `crates/sddk-domain/src/identity.rs`, con 11 tests y **6/6 mutaciones
> detectadas**. Faltan la persistencia (lote 2) y el cableado en
> `resolve_identity_honoring_pin` más el comando (lote 3). El ADR **sigue
> `proposed`**: su decisión no está implementada entera y promoverla a
> `accepted` con un tercio sería exactamente el tipo de `accepted` sin
> evidencia que este repo lleva tres incidencias persiguiendo.

## Context

`project_id` se deriva de `(remote normalizado, scope)`. Derivarlo significa que
**cualquier cambio en el normalizador reasigna identidades ya emitidas**, y esa
cadena de hechos ya está medidas en esta máquina.

### 1. La identidad es inmutable desde el primer evento

Éste es el hecho que cierra el problema, y se descubrió **ejecutando** el
`apply` de INC-DEBT-050, no leyendo el plan:

- `events_v1`, `attempts_v1`, `workflow_runs_v1`, `node_runs_v1`,
  `workflow_run_events_v1` y `backlog_item_events_v1` llevan triggers
  `BEFORE UPDATE` / `BEFORE DELETE` que hacen `RAISE(ABORT, '... are
  append-only')`. Hay un test que exige ese invariante:
  `crates/sddk-storage/tests/cross_ledger_consistency.rs:151`.
- Y por encima del trigger, `EventEnvelopeV1::compute_content_hash`
  (`crates/sddk-domain/src/event_envelope.rs:162`) anula **únicamente**
  `content_hash`, `sequence` y `recorded_at`. `project_id`, `stream_id` y
  `cycle_id` **entran en el hash**.

Reescribir esas columnas no es «relajar un trigger»: es dejar
`verify_stream_chain` fallando con `hash_drift` de forma permanente.

**Un `project_id` está horneado en un fact log encadenado por hash. La identidad
de un proyecto es inmutable en cuanto tiene un solo evento.**

### 2. Por tanto la migración no existe

Medido sobre los 15 proyectos con receipts huérfanos de esta máquina:

| magnitud | valor |
|---|---|
| receipts cuyo `project_id` almacenado no coincide con la derivación actual | **25 de 148** |
| proyectos afectados | **15** |
| filas en tablas append-only | **3.477** |
| proyectos **migrables** | **0** |

No es que la migración sea difícil. Es que no hay operación que preserve el
invariante del fact log.

### 3. Y la re-adopción, que era el remedio anterior, parte el historial

El mismo proyecto lógico se volvió a adoptar cuando el normalizador ya bajaba
la caja del remoto (`Rubentxu/…` contra `rubentxu/…`). Eso produjo **8
proyectos con dos identidades vivas**:

- **6** tienen ciclos en los dos lados. Los ciclos del lado nuevo **no existen
  en el viejo**: 51 ciclos que sólo viven a un lado, y una única colisión de
  nombre corto (`r6-workers-probe-wiring`) que colisionaría al unificar.
- **2** tienen el id nuevo con directorio y ledger pero **0 ciclos**: una
  cáscara de la re-adopción.

Es decir: la re-adopción no es una reparación, es la razón del split. Partió el
historial en dos en lugar de unirlo, porque **unirlo era imposible** por el
punto 1.

### 4. El pin por checkout ya existe, y no converge

`resolve_identity_honoring_pin` (`crates/sddk-cli/src/lib.rs:1496`) es **el
único** sitio donde se decide la identidad del proyecto, y el pin
(`.sddk/project-pin.json`) gana sobre la derivación. Eso ya está implementado y
centralizado; es la reparación correcta del lado del checkout, y este repo ya
la tiene aplicada (`pinned_at: 2026-10-01T11:48:55Z`).

**Lo que el pin no hace es converger.** Vive en el checkout, así que:

- Los 24 proyectos huérfanos que no son este repo siguen sin pin, y sus
  checkouts nuevos resolverán al id vacío.
- Un checkout nuevo de este mismo repo también resolvería a
  `p-995939af668a53d8` (0 ciclos) si el pin no viaja con él.
- El pin es una respuesta por checkout a un problema de storage.

## Decision

**Añadir un alias de identidad a nivel de storage, resuelto en el mismo sitio
único donde ya se decide la identidad.**

```text
project_aliases(from_id TEXT PRIMARY KEY, to_id TEXT NOT NULL,
                reason TEXT, created_at TEXT)
```

`from_id` es **lo que el código deriva hoy**. `to_id` es **la identidad
canónica, la que tiene el historial**. Para esta máquina:

```text
p-995939af668a53d8  ->  p-63676b11dc0ef88f
```

Cuatro reglas, y las cuatro son fail-closed:

1. **La resolución ocurre en `resolve_identity_honoring_pin`**, después del pin
   y antes de devolver. No en un segundo sitio: el defecto que W2c ya corrigió
   fue exactamente que cinco resolutores independientes nadie los sincronizó.
2. **Las cadenas se resuelven transitivamente.** `A -> B -> C` resuelve a `C`.
   Un ciclo (`A -> B -> A`) es un **error duro**, no un aviso: un ciclo hace la
   resolución no terminante, y la no terminación silenciosa es la clase de
   fallo que este repo lleva tres incidencias persiguiendo.
3. **Los aliases son append-only.** No se borran. Retirar un alias exige
   re-apuntar al revés, y el registro de por qué existió sobrevive al
   proyecto.
4. **Toda resolución a través de un alias se declara en la salida.** Es lo que
   convierte el `status: complete` de INC-DEBT-049 en un hecho honesto: si el
   `project_id` que se reporta no es el que se derivó, el operador tiene que
   verlo sin tener que ir a buscarlo.

### Por qué un alias y no las otras dos cosas que se consideran

| opción | por qué no |
|---|---|
| **migrar el fact log** | Imposible. Punto 1. Reescribir `project_id` rompe la cadena de hash de forma permanente. |
| **fusionar los dos ledgers** | También imposible sin reescribir: los `cycle_id` de los dos lados llevan prefijos distintos y hay una colisión de nombre corto. Fusionar «a mano» es reescribir con más pasos. |
| **fijar el normalizador para siempre** | Evita el próximo fork, no el actual. Y no puede: normalizar más (p.ej. Unicode) volvería a cambiar el resultado. Un pin por checkout no converge, y eso es lo que hace la fila 2. |
| **alias a nivel de storage** | No toca el fact log, no reescribe historia, no pierde receipts, y **converge**: un checkout nuevo sin pin resuelve al id con historial. |

## The property this trades away

Un alias es **una redirección global y permanente sin caducidad**. El coste
explícito:

- **Un alias mal escrito manda la identidad de un proyecto entero a otro sitio,
  y nada en el sistema lo detecta por comparación.** No hay forma de que el
  código sepa que `p-9959… -> p-6367…` es una corrección y no un robo de
  identidad. Se mitiga con la regla 4 (se declara) y con el hecho de que el
  alias requiere una acción explícita y auditada, no deriva sola.
- **La indirección es permanente.** Cada resolución de este proyecto henceforth
  pasa por una tabla. Es el mismo coste que cualquier rename en git, y la misma
  razón por la que git lo acepta.
- **Dos proyectos que convergieron a un id siguen teniendo dos directorios en
  disco.** El alias unifica la *resolución*, no el *storage*. Los 3.477
  eventos siguen viviendo bajo el id viejo, y eso es correcto: ocurrieron
  cuando ese era el id del proyecto.

## Consequences

- `crates/sddk-domain/src/identity.rs`: la tabla y su resolución, puras y sin
  filesystem, como el resto del dominio.
- `crates/sddk-cli/src/lib.rs`: `resolve_identity_honoring_pin` encadena el
  alias. **Un solo punto**, que es la razón por la que W2c funcionó.
- Un comando nuevo para declarar un alias, con `--reason` obligatorio y el
  destino que se puede comprobar (`sddk project alias --from … --to …`).
- Los 25 receipts huérfanos quedan resolubles sin tocar ninguno.
- INC-DEBT-049 y INC-DEBT-050 comparten causa raíz y quedan cerrables por la
  misma vía. Son una deuda, no dos.
- **Lo que NO arregla:** el `.sddk/project-pin.json` de este checkout se puede
  retirar cuando el alias esté desplegado y verificado; hasta entonces los dos
  mecanismos coexisten y el pin manda.

## Verification

Criterios falsables. Un ADR sin ellos es una opinion con formato.

1. **La propiedad, con su falsificador.** Resolver con un alias da el mismo
   `project_id` que resolver sin él **cuando no hay alias** (el caso normal no
   cambia), y da el `to_id` cuando lo hay. Falsificador: sembrar un alias
   `A -> A` y exigir que el sistema **no** entre en bucle.
2. **Un ciclo es un error duro, no un aviso.** `A -> B -> A` falla con código de
   salida no cero y nombra el ciclo. Falsificador: construir el ciclo y exigir
   fallo; un guard que solo avisa es un FAIL.
3. **La declaración es visible.** `sddk adopt status` y `sddk project resolve`
   dicen que resolvieron a través de un alias, con el `from` y el `to`.
   Falsificador: borrar la línea de declaración y exigir que el test falle.
4. **Los aliases son append-only.** Intentar borrar uno falla. Falsificador:
   ejecutar el borrado y exigir error.
5. **Sobre el storage real de esta máquina**, después de aplicar los 15
   aliases: `scripts/migrate_project_identity.py audit` reporta **0** receipts
   huérfanos, y `ledger.sqlite` de cada id canónico conserva su recuento de
   filas exactamente igual (3.477 en total, más las que se añadan después).
   Falsificador: el número de filas cambia ⇒ FAIL.
6. **La cadena de hash sigue verificada después.** Para al menos un proyecto con
   fact log, `verify_stream_chain` sobre su stream canónico devuelve OK. Este
   es el criterio que ningún otro cubre y el que más importa: si el alias
   tocara una fila, esto falla.

## See also

- `crates/sddk-domain/src/identity.rs` — derivación y pin
- `crates/sddk-cli/src/lib.rs:1496` — `resolve_identity_honoring_pin`
- `crates/sddk-domain/src/event_envelope.rs:162` — `compute_content_hash`
- `crates/sddk-storage/tests/cross_ledger_consistency.rs:151` — el invariante
  append-only
- `docs/debt/INC-DEBT-050-REMOTE-CASE-NORMALIZATION-REASSIGNS-PROJECT-IDS-WITHOUT-MIGRATION.md`
- `docs/debt/INC-DEBT-049-READOPTION-REASSIGNS-PROJECT-IDSILENTLY-ORPHANS-HISTORY.md`
- `scripts/migrate_project_identity.py` — auditoría y clasificación
  (`blocked_append_only`)
