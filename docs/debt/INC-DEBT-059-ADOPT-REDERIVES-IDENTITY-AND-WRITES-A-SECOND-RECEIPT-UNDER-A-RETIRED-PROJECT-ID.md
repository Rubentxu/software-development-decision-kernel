---
id: INC-DEBT-059
title: "El store de alias se resuelve para `project resolve` y no para `adopt`: adopt re-deriva la identidad y `adopt apply` escribe un segundo recibo bajo el project_id retirado"
status: resolved
severity: high
priority: P1
resolved_at: 2026-10-02
resolved_in_session: session-69
resolution: "La identidad se resuelve UNA vez, en la CLI, y entra ya resuelta en el engine. `AdoptionPlanInput` deja de llevar los cuatro campos de derivacion y lleva `identity: ResolvedProjectIdentity`; `plan_adoption` deja de llamar a `resolve_project_identity` por completo, luego la posibilidad de derivar por dentro desaparece en vez de quedar prohibida por nota. Cuatro superficies resuelven por el resolver canonico: `adopt`, `context bootstrap` (que tenia DOS sitios de resolucion), `generate docs` (cuarta superficie, encontrada por el falsificador y no por la lectura) y el propio engine, que deja de decidir. 6 tests de costura, falsificador PASS=4 FAIL=0 SKIP=0 con las tres mutaciones detectadas, 5361 tests del workspace a 0 fallos."
open_part: "NINGUNO de esta deuda. Lo que queda abierto NO es parte de ella y esta escrito mas abajo: este arreglo impide que se creen mas receipts espurios, NO limpia los que ya existen, y el criterio 5 de ADR-0152 depende de esa limpieza."
detected_at: 2026-10-02
detected_in_session: session-69
component: identity
surface: [crates/sddk-cli/src/lib.rs, crates/sddk-cli/src/context_cmd.rs, crates/sddk-engine/src/adoption.rs]
cluster_id: CL-IDENTITY
related: [INC-DEBT-049, INC-DEBT-050]
references:
  - crates/sddk-cli/src/lib.rs
  - crates/sddk-cli/src/context_cmd.rs
  - crates/sddk-engine/src/adoption.rs
  - docs/architecture/adrs/ADR-0152-STORAGE-LEVEL-PROJECT-IDENTITY-ALIAS.md
  - docs/debt/INC-DEBT-050-REMOTE-CASE-NORMALIZATION-REASSIGNS-PROJECT-IDS-WITHOUT-MIGRATION.md
  - docs/debt/INC-DEBT-049-READOPTION-REASSIGNS-PROJECT-IDSILENTLY-ORPHANS-HISTORY.md
fingerprint: "adopt_rederives_identity_and_writes_a_second_receipt_under_a_retired_project_id"
---

## Qué es

El store de alias de ADR-0152 se resuelve en **un** sitio, y ese sitio no es el
que usa `adopt`. Hay tres puntos de llamada de
`resolve_identity_honoring_pin*`:

| Punto de llamada | Superficie | Usa alias |
|---|---|---|
| `lib.rs:1551` | el wrapper | — |
| `lib.rs:1636` `resolve_project_ids` | `context`, inferencia de ciclo | sí |
| `lib.rs:1903` `run_project_resolve` | `sddk project resolve` | sí |
| `lib.rs:2133` `prepare_adoption_plan` | **todo `sddk adopt`** | **no** |

`prepare_adoption_plan` llama a `plan_adoption` (`adoption.rs:195`), y
`plan_adoption` llama a `resolve_project_identity` **directamente**
(`adoption.rs:203`). La tabla de aliases no se carga en esa ruta: es el único
camino de identidad del CLI que no la consulta.

El pin sí se respeta, pero por un mecanismo **paralelo** dentro de
`plan_adoption`, no por el resolver canónico. El resultado son **dos**
resolutores de identidad, que es exactamente lo contrario de lo que ADR-0152
autorizó al fijar un único punto de decisión.

## OBSERVED: los dos comandos discrepan sobre qué proyecto es el checkout

Reproducción fuera de este repositorio (regla de cero intrusión, AGENTS.md §1),
en `/var/home/rubentxu/repro-c3`, con `HOME`/`XDG_*` aislados. Secuencia:
checkout nuevo con remote; `adopt apply` con el pin puesto en `X`; pin retirado;
alias `Y -> X` declarado. Al final, el mismo checkout, sin pin, en el mismo
instante:

```
$ sddk project resolve --root ws --scope .
project_id: p-0000000000000aaa
identity_alias: p-c4319c598bc98be8 -> p-0000000000000aaa        # exit 0

$ sddk adopt status --root ws --scope .
status: absent
project_id: p-c4319c598bc98be8
receipt: .../projects/p-c4319c598bc98be8/workspaces/w-464b.../adoption.json
ledger: .../projects/p-c4319c598bc98be8/ledger.sqlite
detail: receipt and ledger registration are absent                # exit 1
```

`project resolve` resuelve `Y -> X` y lo declara. `adopt status` no ve el alias,
se queda en `Y`, **mira un ledger que no existe** y reporta `absent`. El alias
está declarado, el store lo tiene, el resolver lo aplica — y `adopt` responde
que ese proyecto no está adoptado. La mitad de la redirección es invisible
desde la superficie de adopción.

## OBSERVED: `adopt apply` recrea el huérfano que ADR-0152 existe para cerrar

Sobre ese mismo checkout, con el alias declarado:

```
$ sddk adopt apply --root ws --scope .
status: complete
project_id: p-c4319c598bc98be8

$ find .../data/sddk/projects -name adoption.json
.../p-0000000000000aaa/workspaces/w-4c77.../adoption.json
.../p-c4319c598bc98be8/workspaces/w-464b.../adoption.json   <-- NUEVO
```

Aparece un **segundo recibo de adopción bajo el id retirado**. `project resolve`
sigue diciendo `p-0000000000000aaa` inmediatamente después, así que el proyecto
queda con dos adopciones y la redirección declarada no evita ninguna de las dos.

Esto es la enfermedad que motivó ADR-0152. El ADR dice, textual:

> los 25 receipts huérfanos quedan resolubles sin tocar ninguno

y el store se construyó para que **no volvieran a aparecer**. El mecanismo los
vuelve a crear, de forma determinista, en cada `adopt apply` sobre un checkout
con alias, y **sin avisar**: el comando sale `complete`. Con 14 aliases
declarados en el storage real de esta máquina, la condición es alcanzable en 14
proyectos.

## Por qué ningún test lo cazó

```
$ grep -c alias crates/sddk-cli/tests/adoption_contract.rs
0
$ grep -c alias crates/sddk-cli/tests/project_pin_e2e.rs
0
```

**Cero** ocurrencias en los dos ficheros que cubren la superficie de `adopt` y la
del pin. Los tests viven a ambos lados de la costura y ninguno la cruza. Es la
**misma** forma que la mutación `resolve_bypasses_the_wiring` del lote 3 de
ADR-0152, que escapó por idéntica razón y que ya costó partir
`run_project_resolve_with` para poder cruzarla: aquí la costura existe y nadie la
cruza porque **nunca se cruzó**.

## Un doc afirma la convergencia que el código no tiene

`lib.rs:1699-1703`, doc de `ProjectPin`:

> **This doc used to be a false claim.** It said "every runtime context honors
> it" while only two of five independent resolvers did; `adopt status`, `config
> resolve` and cycle inference silently re-derived from the remote. Nothing
> tested the contract it asserted. All resolvers now go through
> [`resolve_identity_honoring_pin`]. INC-DEBT-049.

La última frase es **falsa**, y lo es por la misma función que el propio doc
nombra: `adopt status` sigue siendo un resolutor independiente. Se corrigió para
el pin y se dejó el mismo agujero para el alias, y al cerrarlo declara una
afirmación de convergencia que ningún test mide — que es el defecto que el doc
dice haber cerrado. Un doc no se ejecuta, así que el que lo vigila tendría que
ser un test **estructural** sobre los puntos de llamada.

## Impacto sobre ADR-0152

El criterio 3 («`sddk adopt status` y `sddk project resolve` dicen que
resolvieron a través de un alias, con el `from` y el `to`») es **ROJO**. No por
una línea que falte en el render: `AdoptionStatus` no tiene forma de declarar un
alias porque la identidad que recibe nunca pasó por la tabla.

Añadir el campo `alias_origin: Option<ProjectId>` a `AdoptionStatus` y su línea
de render **no cierra el criterio**: se midió que `plan.identity.alias_origin()`
es `None` **siempre**, porque `plan_adoption` no resuelve alias. El campo
serializaría `none` en el 100% de los casos — una declaración que nunca se
dispara, que es peor que no declararlo porque hace el criterio *parecer*
satisfacido a quien lea la estructura. Ese cambio se midió y se revirtió en vez
de commitearse.

**El ADR no se promueve.** C3 en rojo significa que los 6 criterios no se
suman.

El criterio 5 («audit reporta 0 receipts huérfanos») queda en la misma línea: hoy
reporta 1 divergencia por el par de skillgraph retirado, y aunque se reconciliara,
el mecanismo que lo produce seguiría vigente.

## AMPLIADO tras el primer commit: `context bootstrap` también se salta el store, y es peor

La primera versión de esta INC declaraba `surface: crates/sddk-cli/src/lib.rs` y
medía solo `adopt`. Al mapear el segundo punto de llamada de producción de `plan_adoption`
la superficie resultó **más ancha**, y el defecto **peor**. Se amplía aquí en vez
de reescribir el commit ya publicado: el diario es append-only y también la
evidencia de una corrección de alcance.

`converge_adoption` (`context_cmd.rs:1061`) es el otro llamador de `plan_adoption`
en producción, y **el que lo llama tampoco resuelve el alias**:

- `context_cmd.rs:1023-1024` — `resolve_identity` llama a
  `sddk_domain::resolve_project_identity` **directamente**, no al resolver
  canónico.
- `context_cmd.rs:1066-1069` — reenvía el id a `plan_adoption` **solo** si
  `identity_source == Pinned`. Un checkout con alias y **sin** pin cae en
  `pinned_project_id: None`, y el engine vuelve a derivar. Es exactamente el
  atajo que más abajo se declara incorrecto.

**OBSERVED** sobre el mismo sandbox (`repro-c3c.sh`), mismo alias, mismo instante:

```
$ sddk project resolve --root ws --scope .
project_id: p-0000000000000aaa
identity_alias: p-c4319c598bc98be8 -> p-0000000000000aaa

$ sddk context bootstrap --root ws --scope . --session probe-c3
status: no_capsule_source
project: p-c4319c598bc98be8            <-- el id RETIRADO
adoption: complete                     <-- y afirma que esta completo
binding: sddk/context/bindings/probe-c3.json (written: true)
```

**Tres comandos, tres respuestas sobre el mismo estado:** `project resolve` da el
`to` y declara el salto; `adopt status` da el `from` con `absent`;
`context bootstrap` da el `from` con **`complete`**.

Es peor que `adopt status` en dos cosas que importan:

1. **Afirma algo falso en positivo.** `adopt status` decía `absent`, que es
   «no encuentro nada» — un error ruidoso. `context bootstrap` dice
   **`adoption: complete`** sobre una adopción bajo un id retirado, y además
   **escribe un binding de sesión durable** en el data dir de ese id
   (`written: true`). Tras el arreglo, nada leerá ese binding: `context
   bootstrap` abrirá el data dir del `to` y no lo encontrará. Queda **atrapado
   en el sitio al que el alias ya no lleva a nadie**.
2. **Es el segundo escritor del mismo huérfano.** Tras el bootstrap, el `find`
   de `adoption.json` muestra los **dos** recibos, sin que nadie los pidiera dos
   veces.

**Y un tercer doc falso, el más literal de los tres.**
`context_cmd.rs:1001`, el doc de `resolve_identity`:

> /// Resolve project/workspace identity with the SAME resolver as `adopt`.

Es literalmente cierto y exactamente lo contrario de lo que importa: ambos
llaman a `sddk_domain::resolve_project_identity`, la función a la que **los dos**
se saltan. El doc documenta la **concordancia entre los dos bypass** como si
fuera concordancia con la autoridad. Es la tercera afirmación de convergencia
que este trabajo produce y ninguna se cumple — las otras dos en
`lib.rs:1699-1703` y en el propio criterio 3 del ADR, que por eso es rojo.

**Consecuencia sobre la gravedad.** Sigue siendo `high` y no `critical`, y el
razonamiento se amplía en vez de cambiar: no hay pérdida de datos (el binding es
nuevo, no sobrescribe nada del `to`), ni corrupción de `content_hash`, ni
brecha de seguridad, y el recibo duplicado sigue siendo resoluble por el alias.
Lo que se **agrega** es que ahora hay un segundo escritor y una afirmación
positiva falsa. La condición de escalada no cambia: `critical` en cuanto el
mecanismo toque un proyecto cuyo id retirado **no** tenga alias declarado,
porque entonces ni el recibo ni el binding los resuelve nadie.

**Lo que esto NO cambia:** la causa sigue siendo una sola y el arreglo sigue
siendo uno. Las dos superficies comparten la entrada —`plan_adoption`— y el
movimiento correcto es el mismo: que ninguna de las dos derive identidad, y que
la derive **una sola vez** el resolver canónico. Lo que cambia es el **recuento
de puntos de llamada que hay que mover**: dos, no uno, y una de ellos reenvía el
pin con una condición que hay que eliminar en vez de especificarla.

## Gravedad

`high`, no `critical`. La razón de esa elección, porque la diferencia importa:
esta INC crea un recibo duplicado bajo un id retirado, y ese recibo **es
resoluble** por el mismo alias que el mecanismo del store ya resuelve — que es
justo lo que `critical` significa para INC-DEBT-050 y lo que aquí **no** se
cumple. No hay pérdida de datos, no hay corrupción de `content_hash`, y el
`verify_stream_chain` de los ids canónicos no se toca: el criterio 6 de ADR-0152
sigue en pie. Lo que hay es un contrato roto y dos comandos que dan respuestas
distintas sobre el mismo estado, sin aviso. Eso es `high` en la taxonomía
(«broken contract»), y sube a `critical` en cuanto el mecanismo se aplique a un
proyecto cuyo id retirado **no** tenga alias declarado, porque entonces el
recibo no es resoluble por nadie.

## Reproduction

```bash
bash /var/home/rubentxu/repro-c3.sh    # alias + pin + adopt status
bash /var/home/rubentxu/repro-c3b.sh   # adopt apply -> segundo recibo
bash /var/home/rubentxu/repro-c3c.sh   # context bootstrap -> segundo recibo + binding
```

## Resolution (abierta)

Precondición de la corrección: **`SCOPE-CONTRACT` + `PRE-FLIGHT` propios**, y
test **RED antes** del arreglo, no después.

El arreglo tiene que **mover** la decisión, no añadirla. `plan_adoption` re-deriva
porque el engine es filesystem-free por diseño y la tabla la carga la CLI; eso es
correcto y no se toca. Lo que no puede seguir siendo cierto es que **el engine
derive identidad**: no puede, porque no tiene la tabla, luego cualquier
resolución que produzca es sistemáticamente la pre-alias.

La forma correcta, y la que hace verdadera la afirmación de «un solo punto» **por
construcción** en vez de por nota: `AdoptionPlanInput` deja de llevar
`remote_url` + `pinned_project_id` + `scope` + `fallback_seed` y lleva la
**identidad ya resuelta**. `plan_adoption` deja de llamar a
`resolve_project_identity` por completo, y con ello desaparece la posibilidad de
que alguien vuelva a derivar por ahí. Los dos puntos de llamada de producción
(`prepare_adoption_plan` y `converge_adoption`) resuelven **una vez** por
`resolve_identity_honoring_pin_with` y pasan el resultado.

**Descartada la forma corta, y el motivo es el que ya está medido en el campo:**
pasar el id resuelto como `pinned_project_id` no exige tocar nada y **no
funciona**. Haría que el engine lo volviera a tratar como pin, así que
`identity_source` no viajaría y el `alias_origin` tampoco — la información se
perdería igual, un nivel más adentro y con una forma que parece correcta. Y
`context_cmd.rs:1066-1069` ya hace exactamente eso hoy, condicionado a
`identity_source == Pinned`, y por eso un checkout con alias y **sin** pin es el
caso que se rompe. Esa condición hay que **eliminarla**, no propagarla: es el
mismo error en la superficie hermana.

**Coste asumido y declarado:** `plan_adoption` tiene 11 construcciones de
`AdoptionPlanInput` — 2 en producción, 9 en tests — y los tests que cubren el pin
**dentro del engine** (`pinned_project_id_wins_over_remote_derivation`,
`malformed_pin_fails_closed_instead_of_falling_back_to_the_remote`) migran a la
CLI, que es donde el pin vive y donde estaba desde que el pin es un asunto del
checkout. Es el mismo criterio con el que se corrigió INC-DEBT-049 una vez, y
perder cobertura no se acepta: se cambia de sitio, no se deja de medir.

Criterios de cierre, todos falsables:

1. Un test **estructural** sobre los puntos de llamada: ni
   `prepare_adoption_plan` ni `resolve_identity` de `context_cmd.rs` llaman a
   `resolve_project_identity`. Falsificador: reintroducir la derivación en
   cualquiera de los dos y exigir fallo.
2. Un test e2e que cruza la costura: alias declarado + `adopt status` ⇒ el
   `project_id` reportado es el `to`, `identity_alias` nombra `from -> to`, y el
   ledger leído es el del `to`. Falsificador: borrar la línea de declaración y
   exigir fallo (es el falsificador que el criterio 3 ya exige).
3. Un test e2e de no-regresión: `adopt apply` sobre un checkout con alias
   **no** crea un recibo bajo el id retirado. Falsificador: contar los ficheros
   `adoption.json` antes y después.
4. **Un test e2e para `context bootstrap`**, que es la segunda superficie y la
   que afirma `complete`: con alias declarado ⇒ el `project` reportado es el
   `to`, y el binding se escribe bajo el data dir del `to`. Falsificadores: dos,
   y ambos importan — sustituir el proyecto por el `from` (falla por
   identificación) y contar los `adoption.json` y los ficheros de binding antes
   y después (falla por escritura).
5. El doc de `ProjectPin` **y** el de `resolve_identity` en `context_cmd.rs`
   corregidos, y un test estructural que verifique que la afirmación de
   convergencia se sostiene — o que el doc deja de afirmarla. Los tres docs que
   este trabajo produjo afirman una convergencia que no existe; un doc que la
   afirma sin test es la clase que INC-DEBT-049 ya pagó una vez.
6. `scripts/migrate_project_identity.py audit` re-ejecutado sobre el storage real
   **después** del arreglo, para el criterio 5 de ADR-0152.

## Nota sobre el estado del árbol

El cambio que sí compila —`alias_origin` en `AdoptionStatus` más su línea de
render— **fue revertido sin commitear** en vez de dejar un campo que declara un
alias que nunca ocurre. No hay nada que recuperar: está en el historial de esta
sesión, no en un commit.

---

## Resolution: cerrada en session-69, con dos lotes

**Lote 1** (`07c3fd5c`): tests y nada más, con los cuatro RED midiendo la
propiedad y no el andamiaje.

**Lote 2**: el arreglo. Los seis criterios de cierre, medidos:

| # | criterio | cómo se midió |
|---|---|---|
| 1 | test estructural sobre los puntos de llamada | dos guards: el engine no nombra `resolve_project_identity` ni `pinned_project_id` en producción, y la CLI tiene **exactamente una** llamada |
| 2 | e2e que cruza la costura | `adopt status` reporta el `to` y nombra `from -> to`; falsificado por **M1** |
| 3 | e2e de no-escritura | `adopt apply` no crea recibo bajo el `from`; se cuentan los ficheros antes y después |
| 4 | e2e de `context bootstrap` | reporta el `to` y no escribe recibo ni binding bajo el `from`; **dos** falsificadores, por identificación y por escritura |
| 5 | los docs corregidos | los **tres** docs falsos reescritos, y el de la CLI sostenido por el conteo de llamadas |
| 6 | audit sobre el storage real | **NO se puede cerrar con este trabajo** — ver abajo |

Y una **cuarta superficie** que no estaba en el enunciado: `sddk generate docs`
escribía la documentación generada bajo el data dir del id **retirado**
(`repro-c3d.sh`). No salió de leer el SCOPE ni de medir el arranque: la encontró
**el falsificador**, buscando un segundo resolutor, con las otras tres ya
arregladas. Es la cuarta afirmación de convergencia que este trabajo producía y
que era falsa, y la cuarta vez que contar puntos de llamada encuentra lo que la
inspección no.

**Gates:** 5361 tests del workspace a 0 fallos, `cargo exit=0` · 6/6 de la
costura · fmt limpio · clippy `-D warnings` exit 0 · falsificador
**PASS=4 FAIL=0 SKIP=0** con las tres mutaciones detectadas.

**El arreglo demasiado amplio, y por qué importa que un test lo cazara.** El
defecto preexistente que se encontró de paso —`find_persisted_fallback_seed` no
encontraba recibos **pinneados**, porque el pin sobrescribe `identity_source`—
se corrigió primero derivando la semilla de la ruta canónica, copiando lo que
hace `resolve_project_ids`. Eso es demasiado: convierte cualquier directorio en
un proyecto y deja muerto el fallback in-repo de los repos no adoptados. Lo
cazó `real_cli_exit_status_tracks_lint_errors_and_stale_checks` con
`SDDK009`. El arreglo correcto era **una cláusula en el predicado**, no una
fuente nueva de semillas. Es la clase de arreglo que funciona en el caso que
estabas mirando y rompe el que no estabas mirando.

### Lo que queda abierto, y NO es parte de esta deuda

1. **El storage real no se limpia.** Este arreglo impide que se creen más
   receipts espurios; los que ya existen **siguen ahí**, y los bindings
   atrapados por `context bootstrap` **siguen atrapados**.
2. **El criterio 5 de ADR-0152 no se puede cerrar con este trabajo**, porque
   exige que el audit reporte 0 huérfanos sobre el storage real. Depende de (1).
   Qué receipts espurios se retiran es **decisión del operador**.
3. **ADR-0152 no se promueve.** Su **criterio 3 pasa de ROJO a medido** —con su
   propio falsificador, M1— pero los criterios **5** y **6** siguen sin medir, y
   seis criterios no se suman.
4. **La ruta forge de `release apply` contra un GitHub real** sigue sin medir.
   Pendiente propio, declarado, ajeno a esta deuda.

### Lo que el arreglo NO limpia por diseño

`plan_adoption` ya no puede derivar, pero el fallo por defecto sigue siendo
**silencioso**: si alguien reintroduce un resolutor, la costura lo ve porque
`the_cli_has_no_second_identity_resolver` cuenta llamadas. Ese es el guard que
faltaba y que este trabajo añade, y es la razón por la que M3 se detecta.
