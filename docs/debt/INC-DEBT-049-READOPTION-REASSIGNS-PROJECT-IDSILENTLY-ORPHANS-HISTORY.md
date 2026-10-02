---
id: INC-DEBT-049
title: "La re-adopción reasignó el project_id del repo y `adopt status` sigue reportando `complete` sobre un storage vacío, sin declarar que 65 ciclos y 3.9 MB de ledger viven bajo la identidad anterior"
status: open
severity: high
priority: P1
partially_resolved_at: 2026-10-01
partially_resolved_in_session: session-63
resolved_part: "el pin pasa a ser la autoridad de identidad en las CINCO vias del CLI (session-63)"
open_part: "adopt status / cycle status no declaran aun la existencia de historial bajo otra identidad; requiere SCOPE + ADR de contrato de estado"
detected_at: 2026-10-01
detected_in_session: session-62
component: identity
surface: crates/sddk-engine/src/adoption.rs
cluster_id: CL-IDENTITY
related: [INC-DEBT-028, INC-DEBT-037]
references:
  - crates/sddk-engine/src/adoption.rs
  - crates/sddk-cli/src/context_cmd.rs
  - crates/sddk-cli/src/lib.rs
  - crates/sddk-cli/src/cycle.rs
  - crates/sddk-domain/src/identity.rs
  - docs/debt/INC-DEBT-028-NONDETERMINISTIC-FALLBACK-IDENTITY.md
  - tests/cycle-artifacts/p-63676b11dc0ef88f/session62-c3m2-knowledge-basis-revise-identity/RECEIPT.md
fingerprint: "readoption_reassigns_project_id_and_adopt_status_reports_complete_over_empty_storage"
---

## Qué es

Este repo tiene **dos identidades de proyecto vivas**, y la que resuelve el CLI
es la que **no tiene historia**.

`sddk adopt status` responde **`status: complete`**. Ese `complete` es un **PASS
falso**: describe un receipt de adopción válido sobre un storage que no ve nada
de los 65 ciclos del proyecto.

## Evidencia (OBSERVED, session-62, sobre esta máquina)

**La identidad que el CLI resuelve hoy:**

```text
$ sddk adopt status --root . --scope .
status: complete
project_id: p-995939af668a53d8
ledger: /home/rubentxu/.local/state/sddk/projects/p-995939af668a53d8/ledger.sqlite
```

**Y aun así el ciclo no existe para el CLI:**

```text
$ sddk cycle status
error: no active cycle found for project p-995939af668a53d8
```

**El historial real, bajo la identidad anterior:**

```text
$ ls -1d .sddk/cycles/p-63676b11dc0ef88f* | wc -l
65
$ stat -c%s ~/.local/state/sddk/projects/p-63676b11dc0ef88f/ledger.sqlite
3911680
```

**Volumen comparado (medido, no estimado):**

| | `p-63676b11dc0ef88f` (histórico) | `p-995939af668a53d8` (resuelto hoy) |
|---|---|---|
| ledger | 3.911.680 B · 173 refs a eventos | 380.928 B · 6 refs a eventos |
| ciclos en `.sddk/cycles/` | **65** | 0 |
| `cycle-artifacts` en storage | 8 | 0 |

**La causa, con fecha y actor, está en el propio receipt de adopción:**

```json
{
  "project_id": "p-995939af668a53d8",
  "identity_source": "remote",
  "remote_url": "https://github.com/rubentxu/software-development-decision-kernel",
  "scope": ".",
  "canonical_workspace_path": "/var/mnt/DiscoChino2-fast/Proyectos/agentesIA/sddk-framework",
  "timestamp": "2026-09-30T19:29:34Z",
  "actor": "rubentxu"
}
```

El `project_id` se deriva de `hash(remote normalizado, scope)`
(`stable_project_id`, `crates/sddk-domain/src/identity.rs:404`). Cambió el
remote —o se adoptó con el remote canónico por primera vez— y con él cambió la
identidad. Reproducido: `stable_project_id("https://github.com/rubentxu/software-development-decision-kernel", ".")`
devuelve exactamente `p-995939af668a53d8`; ninguna combinación de remote ni de
scope razonable reproduce `p-63676b11dc0ef88f` (probado también contra el
fallback seed por ruta antigua y actual).

**El CLI no tiene detección de identidad previa distinta.** No hay una sola
coincidencia de "otra `project_id` con el mismo proyecto" en `crates/sddk-cli/src/`.

## Por qué es un PASS falso y no una molestia

`status: complete` es exactamente la clase de afirmación que este repo ya
trató como severa dos veces: **un artefacto que declara algo que no es**. La
misma forma que C3l.7 (un gate que certificaba conformidad sin ejecutarse) y
que INC-DEBT-047 (un CHANGELOG que describía otra cosa).

Consecuencia operativa sobre la premisa del proyecto —*SDDK es la autoridad
exclusiva del estado operativo*—: **la autoridad no ve su propia historia**.
Cualquier reanudación que confíe en `sddk cycle status` arranca desde un storage
vacío con un `complete` que la invite a no mirar atrás. Las 65 filas de
`.sddk/cycles/` y los 3.9 MB de ledger siguen en disco; lo que se perdió es el
**acceso**, no los datos.

## Agravante: dos identidades, un vault

Los dos proyectos declaran el mismo nombre y **el mismo vault**:

```text
p-63676b11dc0ef88f → vault_path: /home/rubentxu/.sddk-knowledge/sddk-framework
p-995939af668a53d8 → vault_path: /home/rubentxu/.sddk-knowledge/sddk-framework
```

Dos identidades escribiendo sobre el mismo vault viola el principio de una
autoridad canónica por concepto (AGENTS.md §2.7): ahora el vault es un destino
sin dueño declarado.

## Lo que NO es esta incidencia (matices honestos)

- **El código no cambió la identidad.** El `project_id` sigue siendo una
  derivación pura y determinista de remote + scope. No hay regresión de
  determinismo: el fix de INC-DEBT-028 sigue vigente.
- **La causa inmediata fue una acción humana explícita** (`actor: rubentxu`,
  timestamp propio). Re-adoptar un repo puede ser correcto; lo que no es
  correcto es que hacerlo **cargue en silencio** la historia anterior.
- **No se ha perdido nada.** Los dos storages están íntegros en disco.
- **No bloquea el trabajo activo**, que usa `tests/cycle-artifacts/` +
  `docs/roadmap/`, no el CLI de cycle. El impacto es latente, no inmediato.

## Qué es defecto del repo, y qué es reméd local

**Remedio local (operativo, sin tocar código).** El CLI ya trae la primitiva
exacta para esto — `IdentitySource::Pinned` y `sddk project pin`, cuyo `--reason`
documenta literalmente *"remote renamed, case drift, monorepo split"*:

```bash
sddk project pin --root . --project-id p-63676b11dc0ef88f \
  --reason "remote renombrado: la identidad historica conserva 65 ciclos y 3.9 MB de ledger"
```

Esto **no se ejecuta sin que el operador lo autorice**: cambiar la identidad
autoritativa del repo es una decisión de gobernanza, no una corrección de una
slice. Se deja escrito y sin aplicar.

**Defecto del repo (esto sí es código).** Ni `adopt status` ni `cycle status`
declaran que existe historial bajo otra identidad. Un repo re-adoptado debería,
como mínimo, **advertir**:

- que existe otro `project_id` con el mismo `project_name`/`vault_path`;
- cuántos ciclos y qué tamaño de ledger quedan fuera de la vista;
- y `sddk project pin` como salida.

## Por qué no se corrige en una slice de código sin decisión previa

Toca un **contrato de estado**: si `adopt status` pasa a reportar algo distinto
de `complete`, hay que decidir si es un estado nuevo (`complete_with_orphaned_history`),
si `complete` pasa a ser un warning, o si la advertencia va sólo a `cycle status`.
Eso cambia lo que otros consumidores y skills (`sddk-cycle-resume`,
`sddk-debt-verify`) pueden asumir, así que necesita SCOPE y ADRs, no una slice.

## Falsificadores exigidos cuando se implemente

1. **F49** — storage con el mismo `project_name` bajo otro `project_id` y `adopt
   status` **no** lo menciona ⇒ falla.
2. **F50** — el mismo escenario y `sddk cycle status` no cuenta los ciclos
   huérfanos ⇒ falla.
3. **F51** — dos proyectos con distinto `vault_path` ⇒ la advertencia **no**
   dispara (evita el falso positivo de "mismo nombre, distinto proyecto").
4. **F52** — proyecto recién adoptado sin historia previa ⇒ la salida **no**
   cambia respecto a la actual (sin regresión de ruido para el caso normal).

## Resolución parcial (session-63): el pin era la autoridad declarada y no lo era

El operador autorizó el remedio local. **Aplicado**:
`sddk project pin --root . --project-id p-63676b11dc0ef88f`. Y al aplicarlo
salió el hallazgo que convertía esta incidencia de "deuda" en **defecto de
producto**:

```text
$ sddk project pin …            -> project pinned: p-63676b11dc0ef88f  (OK)
$ sddk project resolve …        -> project_id: p-63676b11dc0ef88f
                                   identity_source: pinned              (OK)
$ sddk adopt status             -> project_id: p-995939af668a53d8      (NO)
$ sddk cycle status             -> no active cycle found for
                                   project p-995939af668a53d8          (NO)
```

El pin se escribía y **no surtía efecto en dos de las tres vías**. Concretamente
había **cinco resolvers de identidad independientes** en el crate y sólo dos
leían el pin:

| # | Resolver | Leía el pin | Superficie |
|---|---|---|---|
| 1 | `RuntimeContext::open` (`cycle.rs`) | ✅ | `cycle` con `--cycle` explícito |
| 2 | `run_project_resolve` (`lib.rs`) | ✅ | `project resolve` |
| 3 | `resolve_project_ids` (`lib.rs`) | ❌ | `config set` |
| 4 | inferencia de ciclo (`cycle.rs`) | ❌ | `cycle status`, `cycle next` |
| 5 | `plan_adoption` (`sddk-engine`) | ❌ | `adopt status/plan/apply/…` |

**La causa raíz era una afirmación falsa en el propio código.** El doc de
`ProjectPin` decía *"When present, `project resolve` and every runtime context
honor it over remote/seed derivation"*. Dos de cinco contexts no lo honraban.
Y un segundo comentario, en la inferencia de ciclo, decía *"using the same
logic as RuntimeContext::open (remote OR fallback_seed OR generate)"* — la
enumeración omitía precisamente el pin, que era la diferencia que rompía.

**Por qué nadie lo cazó:** los dos tests e2e del pin (`pin_overrides_remote_drift`,
`unpin_restores_remote_derivation`) ejercitan **sólo `project resolve`**, el
único resolver que ya funcionaba. El contrato que el doc declaraba no tenía
ninguna prueba.

### Corrección

Una sola función canónica decide la identidad de todo el CLI:
`resolve_identity_honoring_pin` (`lib.rs`). El pin gana sobre remote y sobre
seed. `plan_adoption` es una función pura sin acceso a disco, así que recibe el
pin como dato (`AdoptionPlanInput.pinned_project_id`) en vez de leer el fichero:
el CLI lo lee, el engine lo razona. Un pin malformado **falla cerrado** en
`validate_plan_input` — no puede caerse al remote en silencio, que es como se
produjo el `status: complete` sobre un storage vacío.

Además `.sddk/project-pin.json` se añadió a `.gitignore`: es configuración de
identidad **por máquina**, y versionarlo forzaría a todo otro checkout del repo
al `project_id` de quien lo commitea — exactamente la bifurcación que el pin
existe para evitar. `.sddk/followups/` sigue trackeado a propósito.

### Falsificadores OBSERVED

| # | Mutación | Resultado |
|---|---|---|
| **F49** | `prepare_adoption_plan` deja de pasar el pin | **OBSERVED** — e2e FAILED: *"adopt status debe honourar el pin (esperaba `p-pinnedauthoritative01`, derivado `p-c3d5cbc69b93a9bb`)"* |
| **F50** | la inferencia de ciclo deja de leer el pin | **OBSERVED** — e2e FAILED en la aserción de `cycle status` |
| **F51** | `resolve_identity_honoring_pin` ignora el pin | **OBSERVED** — unit test FAILED: *"resolve_project_ids debe honourar el pin"* |
| **F52** | `plan_adoption` ignora el pin | **OBSERVED** — `pinned_project_id_wins_over_remote_derivation` FAILED |

Un valor que se conserva: con la mutación de F52, el test de pin malformado
**sigue pasando**, porque la validación vive en `validate_plan_input`, que es
independiente del `match` de derivación. Son dos comportamientos distintos y
están cubiertos por separado; un solo test no habría detectado esa separación.

**El primer intento de F49 fue una falsación y se descartó:** la mutación se
aplicó con 8 espacios de indentación sobre una línea que tenía 4, así que no
tocó nada y el e2e pasó "sin romper". Sólo se cuenta como OBSERVED después de
verificar que el fichero mutado había cambiado de verdad.

## Lo que sigue ABIERTO

La mitad de esta incidencia **no** está resuelta y no se arregla en una slice:

- `adopt status` y `cycle status` siguen **sin declarar** que existe historial
  bajo otra identidad con el mismo `vault_path`. Ahora reportan correctamente
  el proyecto pinneado, pero un checkout sin pin que se re-adopte seguirá
  reportando `complete` sobre un storage vacío sin avisar.
- Eso exige decidir el contrato: ¿estado nuevo (`complete_with_orphaned_history`),
  `complete` pasa a warning, o la advertencia va sólo en `cycle status`? Cambia
  lo que `sddk-cycle-resume` y `sddk-debt-verify` pueden asumir. **SCOPE + ADR.**

## Corrección de session-63: la hipótesis de la causa era ERRÓNEA

Este documentoDice que *"`p-63676b11dc0ef88f` no se reproduce desde ningún
remote ni scope probado"* y lo interpretó como **remote cambiado**. **Eso es
falso, y queda corregido sin reescribir la observación original**, que sigue
siendo cierto lo que midió: el id no se reproduce desde el remote *tal como lo
normaliza el código hoy*.

La verdad es más simple y más grave. Los dos receipts **declaran el mismo
remote**, y sólo se diferencian en la *caste* del owner:

| receipt | remote declarado | project_id |
|---|---|---|
| `2026-09-30T07:47:47Z` | `…/Rubentxu/software-development-decision-kernel` | `p-63676b11dc0ef88f` (65 ciclos) |
| `2026-09-30T19:29:34Z` | `…/rubentxu/software-development-decision-kernel` | `p-995939af668a53d8` (vacío) |

```text
https://github.com/Rubentxu/software-development-decision-kernel  -> p-63676b11dc0ef88f
https://github.com/rubentxu/software-development-decision-kernel  -> p-995939af668a53d8
```

Entre medias entró el commit `52182522` *"normalizar case del path del remote —
case-change ya no forkea el ledger (D2)"*. **El remote no cambió: cambió el
código que lo normaliza**, y con él el `project_id` de todo proyecto ya
adoptado, sin migración.

La causa y su alcance están en **[INC-DEBT-050](INC-DEBT-050-REMOTE-CASE-NORMALIZATION-REASSIGNS-PROJECT-IDS-WITHOUT-MIGRATION.md)**:
**25 receipts de 104 (24%)**, 16 `project_id` y 13 repos remotos quedaron
huérfanos en esta máquina.

**Lección sobre el método:** busqué la causa en el remoto durante dos
rondas de sondeo (30 remotes × 4 scopes, más el fallback seed por ruta) y
concluí *"remote distinto"*. El certeza de que un receipt declara un remote es
lo que estaba delante desde el principio y no se contrastó con el receipt
**histórico**. Un solo `cat` de los dos `adoption.json` habría dado la
respuesta. **Medir el contraejemplo antes de extender la hipótesis.**

## Conocimiento negativo útil

No se pudo reproducir `p-63676b11dc0ef88f` desde ningún remote ni scope probado,
ni desde el fallback seed de las rutas antigua y actual. La afirmación de
INC-DEBT-028 línea 108 —*"un proyecto remoto (como `sddk-framework`,
`p-63676b11dc0ef88f`) no se ve afectado: su identidad viene del remote"*— es
**inverificable hoy**: ese id ya no se deriva del remote actual. No se corrige
aquí porque el documento es historia verificada de session-56 y esta observación
es de session-62; queda consignada, no reescrita.

---

## Addendum session-65i: `adopt status` resolvia `conflict` en este repo

### Qué se observó

Con el pin activo (`.sddk/project-pin.json` -> `p-63676b11dc0ef88f`) y el
storage de este repo ya convergido, `adopt status` respondia `conflict` en las
cuatro consultas reales. No era un artefacto de tests: era el binario release
2.5.3 hablando sobre el almacenamiento vivo.

La causa esta en la **cadena de comparacion de identidad**, y son **tres**
sitios, no uno:

1. **`plan_adoption` (`crates/sddk-engine/src/adoption.rs`)** — la rama
   `Some(pinned)` del `match input.pinned_project_id` construia la identidad
   dejando `remote_url: None`, mientras el camino no pinneado lo resuelve. El
   pin sobreescribia `project_id` y tiraba el resto de la identidad. Como
   `remote_url` **es** identidad, `same_identity` comparaba `None` contra
   `Some(...)` y declaraba conflicto contra un recibo que coincidia en las
   otras siete comparaciones. Corregido: se resuelve la identidad con los
   mismos inputs del camino no pinneado y se sustituye solo `project_id`.
   `identity_source` se conserva en `Pinned` a proposito —
   `context_cmd.rs:1066-1069` lo lee para reenviar el pin.

2. **`same_identity`** — comparaba `remote_url` como cadena CRUDA.

3. **`inspect_ledger`** — comparaba `existing.remote_url !=
   plan.identity.remote_url` tambien en crudo, sobre la fila de la tabla
   `projects`.

Los sitios 2 y 3 son **independientes**: arreglar el 2 sin el 3 solo traslada
el conflicto. Por eso el arreglo es una unica funcion,
`remote_urls_match`, usada por los dos.

### Por qué la comparación cruda era incorrecta

El dominio ya habia decidido que la identidad del remoto es **insensible al
case**: `normalize_remote_path` (`crates/sddk-domain/src/identity.rs`) baja
cada segmento a minuscula **antes** de hashear el `project_id`, y eso lo fija
el test golden `case_change_in_owner_or_repo_resolves_to_same_project_id`.
Dos remotos que difieren solo en el case acuñan el **mismo `project_id`**, luego
son la misma identidad por definicion. Compararlos en crudo contradecia esa
decision ya tomada.

La fila real lo confirma: el recibo y la fila `projects` de este repo guardan
`https://github.com/Rubentxu/...` en mayusculas porque se acunaron el
2026-09-30, antes del commit `52182522`. El `project_id` derivado hoy es
identico (`p-63676b11dc0ef88f` bajo el pin) y aun asi se reportaba conflicto.

### Evidencia

RED, con el detalle propio de cada sitio (no uno solo):

```text
fossilized_capitalized_receipt_is_still_the_same_identity ... FAILED
  detalle: Some("receipt identity differs from plan; refresh only accepts
            runtime metadata drift")   left: Conflict  right: Complete

fossilized_capitalized_ledger_row_is_still_the_same_identity ... FAILED
  detalle: Some("ledger project identity differs from plan")
                                                    left: Conflict  right: Complete
```

GREEN: 6/6 en `adoption::tests`, y 1375/1375 en la suite del engine.

**MUTACION (session-65i).** Sustituir la comparacion por `right == *right`
—devolviendo `true` siempre que ambos lados normalicen— dejo los dos tests
positivos **EN VERDE**. Fijaban «el mismo repo con otro case ya no es
conflicto» pero NO «un repo distinto sigue siendo conflicto»: un guard que
declara siempre coincidencia era aceptable. Se anadieron dos tests negativos
(`different_remote_under_the_same_pin_is_still_a_different_identity_{receipt,ledger}`)
y la mutacion paso a ser detectada (`left: Complete`, `right: Conflict`).

Los negativos usan **pin** a proposito. Sin pin, un remoto distinto acuna otro
`project_id`, luego apunta a rutas inexistentes y el veredicto es `Absent`: el
test discriminaba, pero por el guard equivocado (las rutas), no por la
comparacion de identidad. El pin es la unica forma de que dos remotos
genuinamente distintos compartan `project_id` y rutas.

Cuarta vez que un test falsador encuentra en si mismo lo que la inspeccion no.
Aqui encontro algo que la inspeccion no habia considerado: **la mitad negativa
del contrato**.

**Verificacion end-to-end sobre el storage vivo**, mismo repo, mismo pin, solo
cambia el binario:

```text
/home/rubentxu/.local/bin/sddk (release 2.5.3, sin el arreglo)
  -> status: conflict
     detail: receipt identity differs from plan

/var/home/rubentxu/cargo-targets/debug/sddk (con el arreglo)
  -> status: complete
```

### Lo que esto NO resuelve

**INC-DEBT-050 sigue abierta y no se ve afectada.** El arreglo cambia como se
COMPARA la identidad; no reubica 25 recibos que viven bajo un `project_id`
distinto al que deriva el remoto actual. Este repo no necesita migracion porque
tiene pin; los otros 24 recibos huerfanos sin pin siguen huerfanos.

La comparacion normalizada es la contraparte **no destructiva** de la
migracion: acepta el estado fosilizado en lugar de reescribirlo. Ambas cosas
pueden convivir — y esta es la lectura correcta: el pin es la via barata y
local, la migracion sigue siendo la unica que unifica los `project_id`.

### Correccion de superficie

`surface:` y `references:` declaraban `crates/sddk-cli/src/adopt.rs`, un
fichero **que no existe**. Las superficies reales de este defecto son
`crates/sddk-engine/src/adoption.rs` (los tres sitios) y
`crates/sddk-cli/src/lib.rs` + `context_cmd.rs` (lectura del pin).

---

## Addendum session-66b — la decision normativa que faltaba ya esta escrita

**ADR-0152** (`docs/architecture/adrs/ADR-0152-STORAGE-LEVEL-PROJECT-IDENTITY-ALIAS.md`, `status: proposed`) decide la pregunta que este documento llevaba dos sesiones plantando: como se resuelve una identidad de proyecto cuando la derivacion y el storage no coinciden.

Su decision: un alias a nivel de storage (`project_aliases(from_id, to_id)`), resuelto **en el mismo sitio unico** donde ya se decide la identidad (`resolve_identity_honoring_pin`), fail-closed en las cuatro reglas que importan (cadenas transitivas, ciclo = error duro, aliases append-only, y declaracion visible cuando se resuelve por alias).

La SCOPE-CONTRACT del ciclo es `docs/roadmap/receipts/c3m-identity-alias/SCOPE-CONTRACT.md`. **La implementacion no arranca hasta que el operador acepte el ADR**: hasta entonces esto es una propuesta con sus costos escritos, no un trabajo en curso.
