---
id: INC-DEBT-050
title: "Normalizar el remote a minúsculas reasignó el project_id de 25 adopciones (24%) y dejó sus ledgers huérfanos, sin migración y sin golden pin que lo detectara"
status: open
severity: critical
priority: P1
partially_resolved_at: 2026-10-02
partially_resolved_in_session: session-66
resolved_part: "el apply deja de ser una operacion unica e inejecutable: escritura acotada por WHERE, renombrado de estado declarado y verificado, recuento y escritura comparten predicado, rechazo del storage traducido a problema legible, y test propio de la escritura (24 casos, 11 mutaciones detectadas). session-66"
open_part: "la migracion de los 25 receipts NO EXISTE: los 15 proyectos tienen 3477 filas en tablas append-only y el project_id esta horneado en el content_hash del fact log, asi que la identidad es inmutable desde el primer evento. El storage sigue intacto. Camino propuesto: alias de proyecto (tabla from_id->to_id que el CLI resuelve al derivar), que es trabajo de diseño con SCOPE + ADR."
detected_at: 2026-10-01
detected_in_session: session-63
component: identity
surface: crates/sddk-domain/src/identity.rs
cluster_id: CL-IDENTITY
related: [INC-DEBT-049, INC-DEBT-028]
references:
  - crates/sddk-domain/src/identity.rs
  - crates/sddk-cli/src/lib.rs
  - docs/debt/INC-DEBT-049-READOPTION-REASSIGNS-PROJECT-IDSILENTLY-ORPHANS-HISTORY.md
  - docs/debt/INC-DEBT-028-NONDETERMINISTIC-FALLBACK-IDENTITY.md
  - tests/cycle-artifacts/p-63676b11dc0ef88f/session63-pin-identity-authority/RECEIPT.md
fingerprint: "remote_case_normalization_reassigns_project_id_without_migration_or_golden_pin"
---

## Qué es

`project_id` es `hash(remote normalizado, scope)`. El commit `52182522`
(2026-09-30) añadió `normalize_remote_path` con minúsculas para que un cambio de
mayúsculas en la URL no bifurcara el ledger. **Pero cambiar el algoritmo de
normalización cambia el `project_id` de todo proyecto ya adoptado, y no hubo
migración.**

El propio subject del commit dice lo que el cambio comprobaba y lo que no:

> `fix(domain): normalizar case del path del remote — case-change ya no forkea el ledger (D2)`

Para un proyecto que adopta **después** del fix, es cierto. Para uno ya
adoptado **antes**, el mismo cambio **forKEA su ledger**: es exactamente la
propiedad que el commit afirma ofrecer.

## Reproducción (OBSERVED, session-63, esta máquina)

Mismo remote, misma scope, distinto `project_id` según la *caste* del owner:

```text
https://github.com/Rubentxu/software-development-decision-kernel   -> p-63676b11dc0ef88f
https://github.com/rubentxu/software-development-decision-kernel   -> p-995939af668a53d8
https://github.com/Rubentxu/software-development-decision-kernel.git -> p-83b01d24861af185
https://github.com/rubentxu/software-development-decision-kernel.git -> p-447c6f4fd496d161
```

Los dos primeros son **reales**, no sintéticos: son los dos `project_id` que
coexisten en esta máquina, con receipts que declaran **el mismo remote**:

| receipt | timestamp | remote declarado | project_id |
|---|---|---|---|
| workspaces/`w-2e7853…`/adoption.json | `2026-09-30T07:47:47Z` | `…/Rubentxu/software-development-decision-kernel` | `p-63676b11dc0ef88f` (**65 ciclos, 3.911.680 B de ledger**) |
| workspaces/`w-92344b…`/adoption.json | `2026-09-30T19:29:34Z` | `…/rubentxu/software-development-decision-kernel` | `p-995939af668a53d8` (**vacío**) |

**12 horas de distancia**, mismo día, mismo repo. El commit `52182522` entra
entre medias. El último receipt con la forma pre-normalización es de las
`09:07:15Z`, nueve horas antes del fix.

## Magnitud (medida sobre `~/.local/share/sddk/projects`, no estimada)

```text
receipts de adopcion revisados:                 104
ids que NO coinciden con la derivacion actual:    25   (24%)
```

25 receipts repartidos en **16 `project_id` distintos** sobre **13 repos
remotos**, todos con el owner `Rubentxu` en mayúscula:

```text
p-c1fac1fea05615c6  CogniCode                                p-f58d41952fdf56c1  arch-skillkit
p-52b95ef55999f9de  sddk-framework                           p-f4d8f28cd78d443e  sddk-hardness
p-38e02210a9f14317  arch-stack                               p-033dccc0fef1a91f  pipelinek-release-harness
p-07dc4abdd11cc9be  golem                                    p-74299cf88f51dab9  skillgraph
p-28fce7028ac3c497  bevy-2d-editor                           p-3416cfb8288f8964  chronos
p-4c8272c9e7dcdfa2  pipelattice                              p-8ccd53f319722656  asdf-pipelinek
p-7c4aff45199a2069  reactive-narrative-studio-specs          p-733fb505b5a6bd2d  pipeline-kotlin
p-63676b11dc0ef88f  software-development-decision-kernel
```

Un cuarto de las adopciones de esta máquina quedaron con su ledger fuera del
alcance del CLI. **Los datos no se pierden** — los storages siguen íntegros —
pero `sddk` resuelve la identidad nueva y opera sobre un storage vacío.

### Addendum session-65 — remedición con herramienta propia

La medición se ha repetido con `scripts/migrate_project_identity.py audit`
—que deriva los ids con un espejo fijado por test contra el Rust real— y
corrige dos cosas. **No se reescribe lo anterior**: se consigna lo que se
midió entonces y lo que se mide ahora.

```text
                                    session-63        session-65
receipts de adopcion revisados:         104              117
ids que NO coinciden:                    25  (24%)        25  (21%)
project_id viejos afectados:             16               15
remotos con casse distinta:              13               15
```

- **Los 25 receipts huérfanos se confirman y son estables.** El total de
  receipts crece (104 → 117) porque los propios gates del repo crean adopts
  de prueba; los huérfanos no. El porcentaje baja por eso, no porque se haya
  arreglado nada.
- **El "16 `project_id` / 13 repos" de la frase de resumen era incorrecto
  desde que se escribió**: la lista de detalle de este mismo documento ya
  enumera **15 ids sobre 15 remotos**, que es lo que reproduce la remedición.
  La lista era correcta; la frase que la resumía, no. Se corrige aquí para
  que nadie cite `16`/`13` como si fueran medidos.
- **El espejo por sí mismo era un riesgo adicional.** La primera versión de
  ese script reimplementaba `normalize_remote_url` con reglas *parecidas pero
  distintas* al Rust: rechazaba `git@host:owner/repo` (la forma de remote más
  común), trataba el puerto por defecto como global en vez de por esquema y
  aceptaba authority y puerto vacíos que el Rust rechaza. Con ese espejo, un
  `apply` habría escrito ids equivocados en ledgers reales. Se corrigió antes
  de ejecutar nada y se fijó con `tests/test_migrate_project_identity_mirror.py`
  (10 tests) más 7 falsificadores observados.

## Por qué no lo detectó nadie

INC-DEBT-028 ya establecía el principio para el camino del *fallback seed*:

> El string de dominio `sddk.project.fallback.seed.v1` está **pinado por golden
> value** en `fallback_seed_is_pinned_to_known_value`: cambiar el dominio
> reasignaría silenciosamente el `project_id` de todo proyecto sin remote, y
> **ningún test estructural lo detectaría**.

**Ese mismo razonamiento se aplica al camino del remote, y ahí no hay pin.**
`stable_project_id` usa el dominio `sddk.project.remote.v1` y **ningún test
fija su salida**: `grep 'p-[0-9a-f]\{16\}'` en `crates/sddk-domain` sólo
encuentra ids escritos a mano en fixtures y comentarios, ninguno como golden
value del derivador. Por eso una reasignación del 24% pasó sin ruido: no había
nada que la detectara.

Es la misma clase de defecto que INC-DEBT-028, un nivel más arriba y con más
alcance, y la misma lección aplicada a un caso que nadie cubrió.

## Relación con INC-DEBT-049

Session-63 registró INC-DEBT-049 con la hipótesis de que *"el remote había
cambiado"*. **Era incorrecta, y queda corregido aquí:** el remote es el mismo.
Lo que cambió fue el **código que lo normaliza**. INC-DEBT-049 describe el
síntoma en este repo; esta incidencia describe su causa y su alcance.

## Remedios

**1. El pin funciona y ya está aplicado en este repo.** Con el fix de
session-63 (`5b5e2d58`, que hace que las cinco vías del CLI honren el pin),
`sddk adopt status` y `sddk cycle status` ya reportan `p-63676b11dc0ef88f`.
Verificado ejecutando el binario compilado con el pin aplicado.

**2. Migración de los 25 receipts.** Requiere reescribir `project_id`,
`workspace_id` y las rutas derivadas de cada receipt, y mover los ledgers.
Es **destructivo y necesita al operador**: no se ejecuta desde un agente.

**3. El arreglo de fondo — HECHO en session-64.** `stable_project_id` y
`normalize_remote_url` tienen ahora valores absolutos fijados por test (tres
tests nuevos, falsificadores F53–F55 OBSERVED abajo). Cualquier cambio futuro
en el normalizador, en el dominio `sddk.project.remote.v1` o en el framing del
hash rompe un test en vez de reasignar silenciosamente el `project_id` de cada
proyecto del mundo. Es el mismo mecanismo que ya protegía el seed de fallback,
aplicado donde faltaba.

El comentario del golden pin dice explícitamente lo que **no** hay que hacer si
falla: copiar el valor nuevo. Eso reasignaría la identidad de todos los
proyectos con ese remote; lo correcto es decidir antes si procede migración.

**4. Regla de cambio:** cualquier modificación de `normalize_remote_url` o del
dominio `sddk.project.remote.v1` es **breaking change** y requiere migración o
pin, aunque parezca inocua. Sin esa regla, el mismo defecto se repite en el
próximo refactor del normalizador.

### Corrección de session-64: el "gate automático de CI" que propuse NO era deuda real

Esta incidencia, tal como se escribió en session-63, dejaba un cuarto remedy
pendiente: *"convertir la regla BREAKING CHANGE en un gate automático de CI"*.
**Se reevaluó y se retira: no era deuda real.**

Se había escrito sin verificar cómo funciona el pipeline. Verificado en
`.github/workflows/ci.yml`:

- `cargo test --workspace` **sí** se ejecuta en el workflow, así que el golden
  pin **ya corre automáticamente** con cada perfil completo. No hacía falta
  ningún mecanismo nuevo.
- Y el propio encabezado del workflow dice: *"MANUAL-ONLY (2026-08-10): SDDK
  never depends on CI/CD. Validation runs locally; cloud CI is an optional
  on-demand check via workflow_dispatch, **never a gate**"*.

Añadir un gate en la cloud habría sido redundante **y** contrario a la política
del repo, que por AGENTS.md §2.5 trata la cloud como evidencia asíncrona, nunca
como bloqueo. El gate autoritativo es el perfil local, y el golden pin ya está
dentro de él.

**Lección aplicada a sí misma:** es el mismo patrón que la regla del operador
enuncia — *alerta de deuda sin verificar si sus criterios siguen vigentes no es
deuda real*. Esta vez la alerta la escribí yo treinta minutos antes, en el
documento que estaba redactando. Lo que la deja cerrada no es una discusión de
gusto sino un `grep` en el pipeline.

**Queda una sola parte abierta, y es la que de verdad importa:** la migración de
los 25 receipts.

## Falsificadores OBSERVED (session-64) — el pin dorado está puesto y muerde

Tres tests nuevos en `crates/sddk-domain/src/identity.rs`:

| Test | Qué fija |
|---|---|
| `project_id_is_pinned_to_known_values` | valor absoluto de `stable_project_id` en 3 formas: scope raíz `"."`, owner en GitHub, subpath en GitLab |
| `remote_normalization_is_pinned_to_known_values` | salida exacta del normalizador: casse mixta, `.git`, puerto por defecto, puerto no-por-defecto, scp/ssh, credenciales + query + fragment |
| `case_normalization_reassigned_real_project_ids_without_migration` | los **dos ids reales** que convivieron en esta máquina, con sus valores exactos |

| # | Mutación | Resultado |
|---|---|---|
| **F53** | quitar el `.to_lowercase()` del path en `normalize_remote_path` | **OBSERVED** — `remote_normalization_is_pinned_to_known_values` FAILED: `left: "https://github.com/Acme/Widgets"` frente a `right: "https://github.com/acme/widgets"` |
| **F54** | dominio del hash `sddk.project.remote.v1` → `.v2` | **OBSERVED** — FALLAN `project_id_is_pinned_to_known_values` **y** `case_normalization_reassigned_real_project_ids_without_migration` |
| **F55** | invertir el framing (remote y scope intercambiados) | **OBSERVED** — FALLAN los mismos dos |

**El primer F53 diseñado estaba mal y se sustituyó antes de ejecutarlo.**
Proponía mutar `to_lowercase` → `to_ascii_lowercase`, que para entradas ASCII
produce **exactamente el mismo resultado**, así que el pin no habría fallado y
la prueba habría sido inútil. Un falsificador que no puede fallar no es un
falsificador. Las tres mutaciones finales cambian cada una el valor derivado, que
es la condición para que el pin signifique algo.

**Un valor de tener las capas separadas:** con F53 (el normalizador), el golden
del **hash** sigue verde, porque `stable_project_id` recibe el remote ya
normalizado. Con F54 y F55 (el hash), el golden del **normalizador** sigue
verde. Cada pin protege **su** capa y ninguno depende del otro. Ese reparto es
justo lo que hacía falta: el defecto original cruzaba las dos.

**El test histórico fija más de lo previsto:** con F54 y F55 falla también,
porque contiene los ids reales. Avisa de que un cambio en el dominio o en el
framing no es "otro hash": es exactamente la misma reasignación silenciosa de la
vez anterior.

## Límites declarados

- La medición es **de esta máquina**. No se afirma nada sobre otras; el
  mecanismo, eso sí, es independiente del entorno: depende sólo del código.
- **No se ejecutó ninguna migración.** El remedio 2 está escrito y no aplicado.
- `normalize_remote_path` normaliza el **path**, no el host: `github.com` ya
  venía en minúsculas del input, pero un host con mayúsculas seguiría
  reasignando. No verificado si algún remoto de esta máquina tiene esa forma.
### Addendum session-65i — remedición y precondiciones de `apply`, sin migrar

Auditoría reejecutada tras el push de `25955f43`. **Los 25 huérfanos se
confirman exactamente, y no crecen**:

```text
selfcheck: OK (21 normalizaciones + 8 rechazos + 2 ids heredados, contra el Rust real)
receipts revisados: 132          (era 117 en session-65)
ids que NO coinciden con la derivacion actual: 25
```

El total de receipts sube porque los propios gates del repo crean adopts de
prueba; los huérfanos no. El `selfcheck` se auto-verifica contra el Rust real
antes de decir nada, que es lo que hace la medición creíble.

**Lo que cambia respecto a lo que se vio antes:** el alcance de la escritura
está medido, y son 20 tablas de un storage que el CLI ya no puede leer. El
plan cuantifica 206 `gate_receipts.project_id`, 206 `cycle_id`, 61 filas de
`events_v1`, 21 ciclos, 57 `cycle_leases`, y una fila de `projects` — es decir,
**la fila de registro del proyecto**, no solo artefactos derivados.

### Precondiciones de `apply`: las dos, satisfechas

Sin ejecutar la migración, se dejaron listas las dos cosas que `apply` exige:

1. **Plan** — `.migration-plan.json` (18 990 bytes),
   `sha256 = 7f695037dca06842395f9e09f979c2ed0f8d3c8c89d96dd004ec67bb30c2958a`.
2. **Backup verificado** —
   `/var/home/rubentxu/.sddk-migration-backup/20261001T234240Z`,
   **7035 ficheros copiados y verificados byte a byte**, con
   `BACKUP-COMPLETE` y su manifiesto.

El storage real queda **intacto**: la auditoría posterior sigue reportando 25.

El comando queda a un paso, y no se ejecuta:

```bash
scripts/migrate_project_identity.py apply \
  --plan .migration-plan.json \
  --confirm 7f695037dca06842395f9e09f979c2ed0f8d3c8c89d96dd004ec67bb30c2958a \
  --backup /var/home/rubentxu/.sddk-migration-backup/20261001T234240Z
```

**Por qué no se ejecuta aquí.** Es destructivo y reescribe la fila de
`projects` — la que hace que el proyecto exista. El operador dejó la
migración en espera en la sesión anterior, y eso no se ha des-hecho: preparar
las precondiciones no es autorizarlas. Lo que sí se ha hecho es que la
decisión, cuando se tome, no necesite preparación previa: el plan está
generado, el backup está verificado y el hash está anotado.

`.migration-plan.json` **no se commitea**: contiene rutas absolutas de esta
máquina y es un artefacto de una ejecución concreta, no del repositorio.

---

## Addendum session-66 — el `apply` no era solo destructivo: era inejecutable

El operador autorizó el `apply`. Antes de ejecutarlo se leyeron los ledgers
reales en vez de confiar en el plan, y el plan mentía en tres puntos.

### 1. El `UPDATE` iba sin `WHERE`

`sqlite_impact` contaba las filas que **casaban** con el id viejo. La
escritura hacía `UPDATE "t" SET "c" = ?` sobre la **tabla entera**. En dos
ledgers de esta máquina hay filas que no son del proyecto que se migra:

| ledger | filas ajenas | qué son |
|---|---|---|
| `p-63676b11dc0ef88f` | 79 ciclos + 1 proyecto + 1 workspace | centinela `__spine_import__` (import legacy) |
| `p-7c4aff45199a2069` | 10 ciclos + eventos + gate receipts | **otro proyecto**, `p-490921be0aac9b69` |

Un `UPDATE` sin `WHERE` habría reasignado la identidad de los dos, y el plan
—que es lo que el operador revisa— no lo mencionaba, porque para el plan esas
filas no existen. Ninguno de los tres guardas que ya había (selfcheck, digest
del plan, backup verificado) lo detectaba: los tres corren **antes** de
escribir, y el defecto está **dentro** de la escritura.

### 2. El directorio de estado no se renombraba nunca

El código hacía `dst = src.replace(old, new)` y abría `sqlite3.connect(dst)`.
Dos cosas a la vez:

- `Path.replace` es *renombrado de fichero* con un destino, no sustitución de
  cadena. `Path(d).replace(old, new)` lanza `TypeError`. El `apply` habría
  abortado en el primer renombrado, sin migrar nada.
- Aunque se hubiera escrito `str(d).replace(old, new)`, el directorio de estado
  no estaba en el plan: solo `share_dirs_to_rename`. Sobre un directorio
  inexistente, `sqlite3.connect` falla; si el destino existiera, creaba una
  base **vacía** y el ledger real se quedaba atrás, con su historia y sin
  copia.

### 3. Ocho de los quince destinos YA EXISTEN

El plan asumía «renombrar a un hueco libre». No es el caso. Por cada proyecto
lógico hay **dos** ids — el de antes del fix y el de después — porque el mismo
repositorio se volvió a adoptar cuando el normalizador ya bajaba la caja
(`Rubentxu/…` contra `rubentxu/…`). Los dos lados guardan historia real:

| nivel | proyectos | qué pasa |
|---|---|---|
| **A** | 7 de 15 | el id nuevo no tiene storage. Renombrado limpio y sin ambigüedad. |
| **B** | 2 de 15 | el id nuevo tiene directorio y ledger, pero **0 ciclos**. Es una cáscara de la re-adopción; el grueso sigue en el viejo. |
| **C** | 6 de 15 | **ciclos en los dos lados** (p.ej. 125 viejo / 11 nuevo). Exige una unión real de dos ledgers. |

La separación importa porque no es la misma operación. El nivel A es un
renombrado. El B es un movimiento con una cáscara que tirar. El C es una fusión, y
una fusión no está escrita, no está probada y no está autorizada.

**El `apply` original abortaba en el primero de los ocho**, antes de migrar
nada: ya comprobaba `(share/projects/<nuevo>).exists()`. Es decir, el comando
llevaba dos sesiones «a un paso» y no podía llegar a completarse.

### Estado tras session-66

`scripts/migrate_project_identity.py` corregido y **sin ejecutar**:
predicado único (`owned_predicate`) compartido por recuento y escritura;
`WHERE` en todas las escrituras, con sufijo conservado; renombrado de estado
declarado en el plan y ejecutado **al final**, tras verificar; comparación
`filas escritas == filas contadas` con reversión; postcondición que comprueba
que las filas ajenas siguen intactas; y `foreign_values` que **declara** en el
plan lo que no se va a tocar, para que el plan no vuelva a ocultarlo.

`tests/test_migrate_project_identity_write.py` fija el contrato de la escritura
(11 casos, incluidos centinela, proyecto ajeno, ledger perdido, destino
ocupado y reversión por recuento). Falsificado con 5 mutaciones: las 5 se
detectan.

El storage real sigue **intacto**: la auditoría posterior sigue reportando 25
receipts huérfanos. Lo que cambia es que ahora se sabe que migrarlos no es un
`apply`, sino tres operaciones con riesgo distinto, y que la de nivel C es una
decisión del operador, no un paso pendiente.

---

## Addendum session-66 (segunda parte) — la migración no existe: la identidad está sellada

El operador autorizó migrar los 7 renombrados limpios. Se ejecutó el `apply` y
abortó en el primero de ellos:

```
sqlite3.IntegrityError: events_v1 are append-only
```

Nada se escribió — la transacción no llegó a commitear y los renombrados, que
ocurren al final, no corrieron. La auditoría posterior sigue en 25.

### Lo que se encontró leyendo el storage

Cinco tablas del ledger son **append-only por diseño**, con triggers
`BEFORE UPDATE` / `BEFORE DELETE` que hacen `RAISE(ABORT)`:

`events_v1`, `attempts_v1`, `workflow_runs_v1`, `node_runs_v1`,
`workflow_run_events_v1`, `backlog_item_events_v1`

Y hay un test que exige ese invariante:
`crates/sddk-storage/tests/cross_ledger_consistency.rs:151`
("UPDATE must be rejected by trigger").

Peor: no es solo el trigger. `EventEnvelopeV1::compute_content_hash`
(`crates/sddk-domain/src/event_envelope.rs:162`) anula **únicamente**
`content_hash`, `sequence` y `recorded_at`. `project_id`, `stream_id` y
`cycle_id` **entran en el hash**. Reescribirlos no sería "relajar un trigger":
dejaría `verify_stream_chain` fallando con `hash_drift` para siempre.

**El `project_id` está horneado en un fact log encadenado por hash. La
identidad de un proyecto es inmutable en cuanto tiene un solo evento.**

### Consecuencia: la premisa de INC-DEBT-050 es falsa

Los 25 receipts no se pueden migrar. Ninguno. Los 15 proyectos tienen fact log:

| proyecto | filas append-only | receipts |
|---|---:|---:|
| `p-52b95ef55999f9de` | 171 | 2 |
| `p-63676b11dc0ef88f` | 590 | 1 |
| `p-733fb505b5a6bd2d` | 582 | 6 |
| `p-c1fac1fea05615c6` | 698 | 2 |
| `p-7c4aff45199a2069` | 172 | 2 |
| `p-f4d8f28cd78d443e` | 348 | 1 |
| `p-3416cfb8288f8964` | 472 | 1 |
| (los otros 8) | 26 – 120 | 1 – 2 |

**3.477 filas. 0 migrables.** Da igual que el destino esté libre, que no haya
colisión de nombres, que el `WHERE` sea correcto y que el backup esté
verificado: no hay operación.

Esto también explica el nivel C del addendum anterior. Los 8 proyectos con dos
ids no son un efecto secundario de la normalización: son la **re-adopción**,
que es la única vía que existe cuando la identidad ya no se puede cambiar. Y la
re-adopción parte el historial en dos en lugar de unirlo. Los 51 ciclos que
existen solo en el lado nuevo y la única colisión de nombre
(`r6-workers-probe-wiring`) son consecuencia de eso.

### Lo que sí se corrigió

`migrate_one` convierte ahora el rechazo del storage en un problema legible en
vez de un traceback crudo. Antes, un fallo en el quinto de siete habría
dejado los cuatro anteriores migrados y el único informe habría sido una
excepción de Python a mitad de un bucle.

`classify` declara una cuarta clase, `blocked_append_only`, que se evalúa
**antes** que las demás: si hay historia en una tabla append-only, el proyecto
se declara bloqueado sin intentar nada. El `apply` informa
`migrados: 0   intactos por clase: 15` y sale limpio.

Test: 24 casos, y falsificado con 11 mutaciones — incluidas "degradar el
bloqueo append-only" y "quitar la traducción de la excepción SQLite", que son
las dos queRDEN a copiar sin querer.

### Camino de cierre que sí existe

Un **alias de proyecto**: una tabla `project_aliases(from_id, to_id)` que el
CLI resuelve al derivar el id. Es lo que hace git con un rename y lo que hace
cualquier base de datos con una indirección. No toca el fact log, no reescribe
historia y no pierde los 25 receipts: solo deja de calcular mal.

Es trabajo de diseño (SCOPE + ADR), no un parche de script, y por eso queda
como el camino propuesto — no como algo ejecutado aquí.

---

## Addendum session-66b — la decision normativa que faltaba ya esta escrita

**ADR-0152** (`docs/architecture/adrs/ADR-0152-STORAGE-LEVEL-PROJECT-IDENTITY-ALIAS.md`, `status: proposed`) decide la pregunta que este documento llevaba dos sesiones plantando: como se resuelve una identidad de proyecto cuando la derivacion y el storage no coinciden.

Su decision: un alias a nivel de storage (`project_aliases(from_id, to_id)`), resuelto **en el mismo sitio unico** donde ya se decide la identidad (`resolve_identity_honoring_pin`), fail-closed en las cuatro reglas que importan (cadenas transitivas, ciclo = error duro, aliases append-only, y declaracion visible cuando se resuelve por alias).

La SCOPE-CONTRACT del ciclo es `docs/roadmap/receipts/c3m-identity-alias/SCOPE-CONTRACT.md`. **La implementacion no arranca hasta que el operador acepte el ADR**: hasta entonces esto es una propuesta con sus costos escritos, no un trabajo en curso.

---

## Addendum session-69c — el ADR fue aceptado, y que cambia y que no

**ADR-0152 pasa a `accepted`** (2026-10-02, ciclo
`p-63676b11dc0ef88f/identity-alias`). Los seis criterios estan medidos: cinco en
verde y falsificados, y el cuarto —la CLI es append-only— **reescrito**, porque
su redaccion anterior exigia un falsificador inejecutable.

El addendum session-66b de este documento dice «la implementacion no arranca hasta
que el operador acepte el ADR». Eso ya paso: hay 15 aliases declarados en el
storage real, el audit da **0** ids divergentes sobre **161** receipts, y el
operador eligio explicitamente el mecanismo del alias sobre el de retirar el
recibo cuando se le poids las dos opciones con su coste.

**Lo que esto aporta a este documento:**

- La causa ya esta aislada con nombre y fecha: el normalizador de remote cambio
  y el par `p-74299cf88f51dab9` / `p-b7740b96d79ec013` de `skillgraph` son el
  mismo proyecto con dos derivaciones, cinco dias y dos runtimes de distancia
  (1.171.2 el 26-sep, 2.5.3 el 1-oct). Se midio antes de reparar, porque
  «declarar el alias» y «retirar el recibo» son reparaciones opuestas y una es
  irreversible.
- Los 25 receipts del alcance original estan **todos** resueltos: el audit no
  reporta ninguno divergente. El remedio 2 de esta incidencia, el que se declaro
  «destructivo, requiere al operador, no ejecutada», ya no es necesario para el
  efecto observable.
- El remedio 3, el golden pin de `stable_project_id` y `normalize_remote_url`,
  sigue en pie e independiente: protege contra el *proximo* cambio de
  normalizador. El alias protege contra el *actual*.
- **Knowledge negativo que sigue valiendo:** el alias protege la lectura por la
  CLI. Un artefacto que no pase por el resolver canonico —un script suelto, una
  consulta directa al receipt— seguira viendo el id viejo. El remedy 3 es el que
  evita que el problema reaparezca, y el alias solo lo racciona.

**Lo que NO aporta, y por que la incidencia sigue `open`:** el alias produce el
mismo efecto observable que la migracion —ningun receipt queda huerfano— pero no
reescribe los receipts ni reescribe la historia. Si el criterio de cierre de esta
incidencia es «los ids ya no divergen», esta metida lo cumple; si es «los
receipts dicen el id que hoy se deriva», no, y eso solo lo logra una migracion
destructiva. **Que cierre o no es del operador**, porque el criterio esta escrito
aqui y no en el ADR. Lo que el ADR cambia es el frente `closes:` de su
frontmatter: pasa a `[]`, y esta incidencia pasa a `addresses:`.
