# EXPLORATION-REPORT — cl-release-forge-testability

**Cycle:** `p-63676b11dc0ef88f/cl-release-forge-testability` (`OPEN/explore`)
**Date:** 2026-10-03
**Origen:** el objetivo pide comprobar que lo que se afirma siga vigente, y
`CURRENT.md` dejó escrito —session-69e— que `vault graph` y `vault show` estaban
**NO MEDIDOS**, explícitamente *no* «correctos». Al medir `vault show` resultó que
no trunca y ya declara; el candidato que quedaba con más peso era otro.

---

## La afirmación que hay que verificar

Dos comentarios del propio código, en `crates/sddk-cli/src/release_cmd.rs`:

- `release_cmd.rs:2064` — «**la ruta forge no tiene test**, y su call site podia
  informar un lockstep que no se habia comprobado sin que la suite se enterara».
- `release_cmd.rs:2110` — «`release apply --route forge` **no es alcanzable sin
  red**, y quitar el bloque entero no rompia nada».

La segunda es una afirmación fuerte: no es que falte un test, es que **la rama es
inalcanzable para la suite**. Si es cierta, el producto tiene una ruta de
publicación completa —`pr.create` → `pr.merge` → `release.create`, tres
capacidades privilegiadas— que **nunca se ha ejecutado ni un solo vez bajo
verificación**.

## Medición

Búsqueda de todos los usos de la ruta forge en el workspace:

| Lugar | Qué es |
|---|---|
| `sddk-gateway/src/forge.rs:133` | `pub fn with_runner(repo, runner)` — el seam de inyección **ya existe** |
| `sddk-gateway/src/forge.rs:520`, `:569` | dos tests **del gateway** que usan `with_runner` |
| `sddk-cli/tests/cli.rs:4657` | test de la CLI que comprueba que `--repo` **sin** `--route forge` **falla** |
| `sddk-cli/src/release_cmd.rs:882` | el call site: `GitHubForge::new(repo)` — **runner real, fijo** |

**Ningún test alcanza la rama forge de la CLI.** El único test de la CLI que la
menciona comprueba precisamente que **no** se entra en ella sin `--route forge`.

### Por qué no es alcanzable: no es una limitación de la red

Es lo que hace el hallazgo interesante. La limitación **no está en el adaptador**:

- `GitHubForge` guarda `runner: Box<Runner>`, donde `Runner` es un
  `dyn Fn(&RunSpec) -> Result<RunOutcome, RunnerError>`.
- `GitHubForge::with_runner` construye el adaptador con un runner inyectado, y lo
  usan dos tests.
- `plan_release(input, forge: &dyn Forge)` y
  `apply_release(..., forge: &mut dyn Forge, ...)` **ya son polimórficos**.
- `MockForge` es `pub` y está re-exportado en `sddk-gateway/src/lib.rs:40-41`.

**Todo el mecanismo de inyección existe y funciona. Lo que falta es el seam en el
call site**, que construye `GitHubForge::new(repo)` con el runner real y no deja
forma de sustituirlo. Por eso la rama es «inalcanzable sin red»: **no porque la
red sea necesaria, sino porque el call site no ofrece la alternativa.**

Es la diferencia entre «esto no se puede probar» y «esto no se ha conectado para
poder probarse». La segunda es un defecto de cableado, y la segunda se arregla.

## Qué NO se afirma

- **No se afirma** que la ruta forge tenga un defecto. No hay evidencia de que
  falle; hay evidencia de que **nunca se ha ejecutado bajo prueba**.
- **No se propone** ejecutarla contra un GitHub real. Serían tres escrituras
  privilegiadas sobre un repositorio ajeno —`pr.create`, `pr.merge`,
  `release.create`—, y la regla de cero intrusión de AGENTS.md §1 prohíbe
  precisamente eso.
- **No se toca** la ruta local, que sí está cubierta y es la que usa
  `scripts/release.sh`.

## La forma del remedio

Extraer el cuerpo de la rama forge a una función que reciba `&mut dyn Forge`:

```
fn apply_release_forge(gateway, forge: &mut dyn Forge, …) -> Result<ReleaseOutcome>
```

La CLI la llama con `&mut GitHubForge::new(repo)`; el test la llama con
`&mut MockForge::…`. Sin globales, sin `#[cfg(test)]` sobre el estado, sin
ensanchar visibilidad: los tests de este módulo ya viven dentro del crate.

Es la misma forma que ya se aplicó con éxito en `ledger export` —cablear la
declaración en vez de añadir una segunda— y la inversa de lo que hizo el
antiguo falsificador: no añadir una segunda ruta, sino hacer alcanzable la que
hay.

## Coste y riesgo

Bajo. Una función extraída y un test nuevo. **No cambia ninguna comprobación de
capacidad, ni el orden `CreatePr → MergePr → CreateRelease`, ni el mapeo de
errores**, y eso es exactamente lo que hay que comprobar con STOP 1.

## El precedente que obliga a mirar

Este ciclo viene de una serie en la que **instrumento y guard han fallado más que
el producto**, y donde el patrón que más rinde es *medir antes de reparar*. Aquí
la afirmación que se mide es del propio autor del código, y por eso toca
verificarla en vez de darla por buena: resultará **cierta**, y la medición lo dice
con nombres de línea.
