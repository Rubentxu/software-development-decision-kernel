# PRE-FLIGHT — VA15 `cl-canario-de-aislamiento-entre-proyectos`

## Intención

Falsar H1–H6 con un canario sintético, **sin** los datos del incidente real, que
no están en esta máquina. El canario no repara nada: intenta **reproducir** un
cross-project leak. Si no lo reproduce, la hipótesis queda falsificada con
evidencia; si lo reproduce, hay causa y regresión.

## Lo que ya está medido, y por qué este bloque existe

La lectura del código dice que el leak es imposible. **La lectura ya falló una vez
en este repositorio**: VA14 escribió «una infracción» donde había nueve, porque la
criba era más estrecha que el instrumento. Concluir «imposible» leyendo sería el
mismo error con el signo cambiado. Por eso esto se ejecuta, no se razona.

### Los tres ficheros que deciden

```
stable_project_id(remote, scope)      = hash("sddk.project.remote.v1",   [remote, scope])
stable_fallback_project_id(seed,scope) = hash("sddk.project.fallback.v1", [seed,     scope])
stable_fallback_seed(path)            = hash("sddk.project.fallback.seed.v1", [path])
```

`crates/sddk-domain/src/identity.rs:656,662,691`. El fallback hashea **la ruta
canónica completa**, no el basename. Y los dos dominios de derivación son
distintos a propósito, para que un id derivado de remote y otro derivado de
fallback no puedan coincidir nunca.

### El namespacing que protegería del incidente

`paths.rs:153` → `project_data = data_home/sddk/projects/<project_id>`.
Los tres `load_binding` de produccion (`context_cmd.rs:339,568,798`) leen de
`bindings_root(&paths.project_data)`, con ese `project_id` resuelto del checkout
actual (`resolve_via_canonical`, `context_cmd.rs:1001`).

Un binding de Fabric vive en `…/p-<fabric>/context/bindings/` y **no es
alcanzable** desde PipelineK, porque el camino se construye con la identidad del
proyecto desde el que se opera.

### Y el hueco que sí existe

`AgenticBinding::reattach(persisted) = persisted.clone()`
(`agentic_session_binding.rs:136`). Sin comparación, y `AgenticBindingError` tiene
tres variantes y **ninguna** es un mismatch. Además el binding **no lleva
`workspace_id`** (`session` + `target` + `semantic_refs` + `context_basis` +
`receipts`), luego mismo proyecto con worktree distinto reusa el binding en
silencio. Eso es X6 y es real.

## El diseño que hace que el canario pueda fallar

Un canario que no puede fallar no es un canario. Por eso los fixtures llevan
**basenames identicos en directorios distintos**:

```
canario/pipeline-kotlin/      remote github.com/example/pipeline-kotlin
canario/pipelinek-fabric/     remote github.com/example/pipelinek-fabric
canario/otro-a/proyecto/      remote github.com/example/pipeline-kotlin
canario/otro-b/proyecto/      remote github.com/example/pipelinek-fabric
```

La segunda pareja tiene las dos con basename `proyecto`. Es la que separa
«identidad por remote» de «identidad por nombre»: si la identidad usara el
basename, esas dos **colisionarian**, y con el código actual no.

Aislado con `SDDK_DATA_DIR` a un temporal, para no tocar los 265 proyectos
reales que hay en esta maquina.

## Mutaciones, y la expectativa honesta de cada una

El brief pide cinco. **No todas pueden dar rojo**, y decirlo antes es parte del
diseño: una mutacion que no puede fallar se reporta `SKIP` con su motivo, nunca
`PASS`.

| # | mutacion | expectativa declarada |
|---|---|---|
| M1 | `stable_project_id` hashea solo el scope y descarta el remote | **ROJO** — las dos del mismo basename colisionan |
| M2 | `bindings_root` ignora `project_data` y usa un directorio global | **ROJO** — el binding de B se lee desde A |
| M3 | `stable_fallback_seed` hashea el basename en vez de la ruta | **SKIP esperado** — el fallback no interviene con remote presente, luego no puede dar rojo. Se ejecuta y se declara lo que pase |
| M4 | `resolve_via_canonical` deja de leer el remote | **SKIP esperado** — cae al seed de ruta, que sigue distinguiendo. **El brief espera ROJO y medido no lo dara**: es un resultado, no un fallo |
| M5 | borrar la comparacion de `target.project_id` | **ROJO** si hay binding cruzado que leer |

**M4 es el hallazgo anticipado del bloque.** El brief lo declara mutacion
obligatoria en rojo porque da por hecho que quitar el remote rompe la identidad.
Medido: no la rompe, porque el fallback hashea la ruta completa y las rutas son
distintas. **Un test que se declara rojo y no puede estarlo no prueba nada**, y
esa es exactamente la clase de garantia que este bloque esta tratando.

## No-objetivos

- **No** repara nada. Ni `WorkspaceAttestation`, ni `ExecutionContextAdmission`,
  ni admission boundary. El brief dice no arreglar hasta saber cual ocurrio.
- **No** hardcodea `pipeline-kotlin` ni `pipelinek-fabric` en produccion.
- **No** introduce fuzzy matching ni compara display names.

## Superficie y riesgo

| | |
|---|---|
| Fichero nuevo | 1 script de canario + su mutacion |
| Ficheros de produccion | 0 salvo durante las mutaciones, y se restauran byte-identico |
| Riesgo | el canario toca el binario: cada mutacion recompila |
| Perfil | `sddk-cli` + el script. Nada mas |

## Stop conditions

1. Si el codigo actual **reproduce** el leak, se para y se reporta antes de
   tocar nada. El canario no parchea.
2. Si una mutacion deja el arbol sin restaurar, `exit 1` inmediato.
3. Si el canario no puede distinguir los dos casos, no se cuenta como
   falsador.

## Readiness

**READY.** Todo lo que el bloque afirma esta medido sobre `HEAD = e60dae75`. Lo
unico que queda por medir es su propia salida.