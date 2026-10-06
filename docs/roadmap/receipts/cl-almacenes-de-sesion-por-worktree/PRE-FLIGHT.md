# PRE-FLIGHT — VA17 `cl-almacenes-de-sesion-por-worktree`

## Decision del operador, que este bloque ejecuta

Dos decisiones tomadas sobre VA16, que measured que el aislamiento por worktree no
existe:

1. **Solo los cuatro almacenes de sesion** (`bindings`, `capsules`, `deltas`,
   `reads`) pasan a ser **por worktree**. `knowledge-profile.json`, `artifacts`,
   `cycle_artifacts` y `generated` **se quedan por proyecto**.
2. **Cross-worktree es fail-closed**: mismo proyecto con worktree distinto exige
   **rebind explicito**, genera receipt, invalida el `ContextBasis` y **nunca
   reasigna en silencio**.

## Lo que esta medido, no supuesto

### El layout y quien lo sostiene

`resolve_xdg_paths` (`paths.rs:123`) ya recibe `workspace_id` y **solo lo usa
para `receipt`**. `AdoptionPaths` tiene 8 campos, **sin constructor asociado**: se
construye en un unico sitio, `paths.rs:155-167`. Anadir un campo es un punto.

Las cuatro raices son `fn` privadas de `sddk-cli`, todas en `context_cmd.rs`, y
**el engine no conoce el layout**: `load_binding(root: &Path, …)` y
`save_binding(root: &Path, …)` reciben la raiz inyectada. Cambiar el layout es
cambiar la raiz que se pasa.

### No hay lectura transversal

Medido: **ningun sitio enumera bindings de todos los worktrees o de todos los
proyectos**. En produccion solo se usan `load_binding` (puntual, por sesion) y
`save_binding`. `list_sessions`/`load_all` existen pero sus unicos llamadores son
un test unitario y un test de integracion. Esto hace el cambio mucho mas seguro
de lo que el titular sugiere: no hay superficie que actualizar.

### El coste, en cifras

| Superficie | Coste |
|---|---|
| Definiciones raiz | 4 (`context_cmd.rs:541/764/1053/1057`) |
| Llamadas de produccion | 10 |
| Usos de test en `context_cmd.rs` | 16 |
| Construccion/deserializacion de `AgenticBinding` | 3 sitios |
| Tests Rust dependientes del layout | **0** |
| Tests shell dependientes del layout | **7 ficheros / 12 lineas**, 3 con patch de Rust embebido |
| Consumidores fuera de `crates/` y `tests/` | **0** |

## Los tres datos que deciden el diseno

**1. `AgenticBinding` no tiene ni un `#[serde(default)]`** — medido:
`grep -c 'serde(default)' == 0`. Anadir `workspace_id` **sin** default deja
ilegibles los bindings ya persistidos: serde exige el campo. Hay 91 en disco,
reales.

Por eso el campo nuevo lleva `#[serde(default)]`, y **`None` significa «worktree
desconocido»**, que es exactamente el caso que la politica del operador manda
rechazar. El default no es una comodidad de compilacion: es la codificacion de
la tercera clase de la tripartition (proyecto / worktree / desconocido).

**2. Los 212 ficheros existentes no dicen de que worktree son.** Medido en
`~/.local/share/sddk/projects/`: 91 bindings en 15 proyectos, 94 deltas en 5,
26 capsules en 6, 1 reads en 1. Ninguno lleva `workspace_id`.

Migrarlos exigiria **inventar** un `workspace_id`. Este bloque **no migra**. Los
ficheros quedan donde estan, inertes y recuperables; el layout nuevo empieza
limpio. Se declara, no se disimula. Es la unica opcion que no inventa datos ni
duplica la autoridad del layout.

**3. Un `drain` de deltas que no encuentra nada **no es lo mismo** que un `drain`
que no tiene nada.** Si el layout nuevo esta vacio y el antiguo tiene 94 ficheros
pendientes, un `drain` que responde «nada que drenar» es **perdida de datos
disfrazada de exito**. Eso es peor que el leak que VA16 midio, y por eso el drain
avisa cuando puede haber estado en el layout antiguo. **Nunca descartar en
silencio** es la misma regla que **nunca reasignar en silencio**, aplicada al otro
lado.

## Diseno

```
project_data/workspaces/<workspace_id>/context/
    bindings/  capsules/  deltas/<session>/  reads/
```

`AdoptionPaths` gana `session_root`, y las cuatro raices cuelgan de ahi. Los
cuatro almacenes quedan en un unico sitio por worktree, junto al `adoption.json`
que ya vivia ahi.

`AgenticBinding` gana `workspace_id: Option<String>` con `#[serde(default)]`, y
tres registros de receipt (ASB-005) en el camino de reattach explicito.

## Mutaciones que tendran que morir por su propio control

La misma disciplina de VA15 y VA16, y **el canario de VA16 cambia de rol**: de
«midiendo que el leak existe» a **guard estricto** (`EXPECT_LEAK="no"`). Si
apenas el guard queda verde sin cambiar el codigo, el canario no vigila.

Ademas: volver a poner las cuatro raices por proyecto debe tirar el canario;
quitar el `#[serde(default)]` debe romper la lectura de los bindings antiguos;
y la comprobacion del drain debe morir si se le quita el aviso.

## No-objetivos

- **No migra datos.** No inventa `workspace_id`.
- **No toca** `knowledge-profile`, `artifacts`, `cycle_artifacts`, `generated` ni
  `ledger.sqlite`. Quedan por proyecto, por decision.
- **No introduce** lectura transversal de bindings: no existe y no se crea.
- **No decide** el formato del receipt de reattach mas alla de lo que ASB-005 ya
  define.

## Stop conditions

1. Si los 212 ficheros existentes no pueden quedar inertes sin riesgo de
   perdida silenciosa, se para antes de tocar nada.
2. Si un solo binding antiguo se vuelve ilegible, `exit 1`: es una perdida de
   datos disfrazada de exito, y el mecanismo que la causo esta aqui mismo.
3. Si el canario de VA16 pasa a verde sin que el codigo cambie, el guard no
   vigila: se para.

## Readiness

**READY**, con una condicion: el cambio de layout es **rompente por
naturaleza** y esta bloque lo hace explicito, no silencioso. Lo unico que queda
por medir es si los 7 tests shell que Asumen el layout viejo sobreviven a que
todos a la vez, y eso lo decide la ejecucion, no la lectura.