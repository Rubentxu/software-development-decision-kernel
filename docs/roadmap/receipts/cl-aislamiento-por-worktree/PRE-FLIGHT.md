# PRE-FLIGHT — VA16 `cl-aislamiento-por-worktree`

## Intención

Medir, no razonar, si el estado de contexto de sesión de un **worktree** es
alcanzable desde otro worktree del **mismo proyecto**. Es el caso X6 del brief, y
existe porque VA15 demostro que el caso cross-project del brief **no es
alcanzable**: el namespacing en disco por `project_id` lo hace moot.

## Lo que ya esta medido, con el binario publicado

Sondeo ejecutado contra `sddk 2.12.0` con `SDDK_DATA_DIR` temporal y dos checkouts
del **mismo** remote `github.com/example/obra`:

```
bootstrap --root wt-a --session S   -> project: p-f1bf640403f38968
                                       workspace: w-b8cf194cbc514d2501fdb054
                                       binding: .../S-probe.json (written: true)
bootstrap --root wt-b --session S   -> project: p-f1bf640403f38968
                                       workspace: w-1740fecf4d8470a57a701e84
                                       binding: .../S-probe.json (written: false)
```

Cuatro hechos, todos medidos:

1. **El `workspace_id` distingue.** Distinto en A y en B. El sistema calcula bien
   la identidad de worktree.
2. **El binding es UNO.** Un solo `S-probe.json`, en
   `p-f1bf.../context/bindings/`. A y B escriben en el mismo fichero.
3. **B acepta la sesion de A sin escribir nada** (`written: false`). El binding
   heredado era identico, asi que `next_basis` devolvio `previous.clone()`
   (`context_cmd.rs:387`).
4. **El binding no lleva ningun discriminante de worktree.** Su contenido entero
   es `{session, target{kind,project_id}, semantic_refs, context_basis, receipts}`.

Y el control que mas importa: los **unicos** ficheros separados por workspace
fueron los dos `workspaces/<w-id>/adoption.json`. El mecanismo por workspace
**existe y funciona** — luego que bindings y capsules no lo usen no es una
limitacion del sistema, es una omision.

## Por que este caso es peor que el que el brief prioriza

El brief pone el admision boundary sobre `project_id`. Aqui el `project_id`
**coincide legitimamente**, porque es el mismo proyecto. **Ninguna ley C1-C5
puede detectar este leak: todas comparan project_id, y aqui el project_id
correcto.** Solo el `workspace_id` lo distingue, y el `workspace_id` no
participa en ningun almacen de contexto.

Un guard por `project_id` PASSARIA en verde mientras sirve contexto de otro
worktree. Eso es lo que este bloque tiene que dejar escrito.

## Donde vive el namespacing, medido

`crates/sddk-engine/src/paths.rs:123-168`, `resolve_xdg_paths`:

| ruta | namespaced por |
|---|---|
| `project_data` (:153) | **project** |
| `artifacts`, `cycle_artifacts`, `generated` (:159-161) | **project** |
| `ledger` (:161) | **project** |
| `receipt` (:163-166) | **workspace** |

Y en `context_cmd.rs`, los cuatro almacenes de sesion:

| almacen | fn | namespaced por |
|---|---|---|
| capsules | `capsule_root` :541 | **project** |
| bindings | `bindings_root` :1057 | **project** |
| deltas | `deltas_root` :1053 | **project** |
| reads | `reads_root` :953 | **project** |

`workspace_id` se calcula en `bootstrap` (`context_cmd.rs:208`) y se **expone**
en el resultado (`:370`) sin participar en ninguna de esas rutas.

## El diseno que hace que el canario pueda fallar

El fixture son **dos checkouts del mismo remote**, que es materialmente el caso
`same remote / same scope / same project_id / different workspace_id` del brief:
lo que SDDK observa es `canonical_workspace_path`, no el mecanismo de git.

Cuatro comprobaciones y **un control positivo**. El control positivo es W4 y es
el que mas valor tiene: demuestra que el namespacing por workspace se sabe hacer,
y por tanto la ausencia en bindings/capsules es una omision y no un limite.

| # | que afirma |
|---|---|
| W0 | precondicion: mismo project_id, **distinto** workspace_id. Sin esto el fixture no reproduce el caso |
| W1 | el binding de sesion NO debe ser alcanzable desde el otro worktree |
| W2 | el binding debe llevar un discriminante de worktree |
| W3 | el bootstrap en B no debe absorber en silencio el binding de A |
| W4 | **control positivo**: el namespacing por workspace funciona cuando se usa (`adoption.json`) |

## Mutaciones, y que se espera de cada una

La forma es distinta de VA15, y a proposito: **aqui no hay defensa que romper**.
Las mutaciones **la construyen**, y el canario tiene que notar el cambio. Una
mutacion que no cambia el veredicto significa que el canario no esta midiendo lo
que dice medir.

| # | mutacion | expectativa declarada |
|---|---|---|
| F1 | `resolve_xdg_paths` cuelga `project_data` del `workspace_id` (una linea) | **el veredicto cambia**: A y B dejan de compartir. Es el fix real, y cabe en una linea |
| F2 | `next_basis` deja de heredar: siempre `seq: 0` | **el veredicto cambia**: B deja de heredar el basis de A |
| F3 | comentario inocuo en `bindings_root` | **el veredicto NO cambia**. Si cambia, el canario reacciona a ruido, no a la propiedad |

F3 es el control que hace que F1 y F2 signifiquen algo: sin el, «el veredicto
cambio» podria significar «el canario detecta cualquier cambio».

## No-objetivos

- **No repara nada.** No anade `workspace_id` al binding, ni crea
  `ExecutionContextAdmission`, ni toca las leyes del brief. Este bloque mide y
  deja el veredicto con su falsificador.
- **No decide la politica** de si un reattach cross-worktree se permite. X6 pide
  esa decision; aqui solo se mide si el sistema puede siquiera notar el caso.
- **No toca produccion fuera de las mutaciones**, que se restauran
  byte-identicas.

## Superficie y riesgo

| | |
|---|---|
| Fichero nuevo | 1 script de canario |
| Ficheros de produccion | 0 salvo durante las mutaciones, restaurados byte-identico |
| Riesgo | cada mutacion recompila; el target se mantiene FUERA del temporal para que las rebuild sean incrementales |
| Perfil | `sddk-cli` + el script. Nada mas |

## Stop conditions

1. Si la base sale verde, el aislamiento por worktree ya existe: se para y se
   reporta antes de tocar nada.
2. Si una mutacion deja el arbol sin restaurar, `exit 1` inmediato.
3. Si W0 falla, el fixture no reproduce el caso y no cuenta como medicion.

## Readiness

**READY.** Todo lo que el bloque afirma esta medido contra el binario publicado y
por lectura con linea. Lo unico que queda por medir es su propia salida.