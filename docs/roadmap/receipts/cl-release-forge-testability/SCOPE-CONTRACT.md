# SCOPE-CONTRACT — cl-release-forge-testability

**Cycle:** `p-63676b11dc0ef88f/cl-release-forge-testability`
**Date:** 2026-10-03

---

## Qué entra

- **O1** — La rama `ReleaseRoute::Forge` de `release apply` queda **alcanzable por
  la suite sin red**, extrayendo su cuerpo a una función que reciba
  `&mut dyn Forge`.
- **O2** — El test ejercita **el cuerpo real de la rama** —ticket, `plan_release`,
  `apply_release` y el mapeo de errores— y no una reimplementación.
- **O3** — **Cero cambio de comportamiento**: mismas capacidades, mismo orden
  `CreatePr → MergePr → CreateRelease`, mismo `AdmissionTicket`, mismo
  `version_authority`, mismos mensajes de error.
- **O4** — La afirmación de `release_cmd.rs:2064` y `:2110` («la ruta forge no
  tiene test», «no es alcanzable sin red») queda **sustituida por un test que la
  alcanza**, y las dos se corrigen en el sitio.

## Qué NO entra

- **No** se ejecuta la ruta forge contra un GitHub real. Serían tres escrituras
  privilegiadas sobre un repositorio ajeno (AGENTS.md §1, cero intrusión).
- **No** se toca la ruta **local**, que está cubierta y es la que usa
  `scripts/release.sh`.
- **No** se cambia `GitHubForge`, ni `apply_release`, ni `plan_release`: ya son
  polimórficos y correctos. El defecto está **solo** en el call site.
- **No** se ensancha la visibilidad de nada a `pub`.
- **No** se ejecuta la release 2.5.3, que sigue bloqueada por la clave KMS.

## Requisitos

- **R1** — El cuerpo de la rama forge vive en una función que recibe
  `&mut dyn Forge`.
- **R2** — Un test con `MockForge` llega a `apply_release` y comprueba el
  **resultado**, no sólo que no entre en pánico.
- **R3** — Un guard **estructural**: un test lee el fuente y exige que el call
  site de la CLI **delegate** en esa función. Sin él, alguien puede volver a
  inlinear `GitHubForge::new(repo)` y dejar la rama inalcanzable **con todos los
  tests en verde** — que es exactamente el defecto que se viene a cerrar.
- **R4** — El mapa **objetivo → guard** está escrito **en el PRE-FLIGHT**, antes
  de implementar. Un gate de requisitos que no puede señalar dónde falta su
  objetivo es insatisfacible, y eso ya pasó una vez en esta serie
  (`04-req-testable.py`).
- **R5** — Ningún test verde se reescribe.

## STOP

- **STOP 1** — Si hacer la rama alcanzable exige **debilitar una comprobación de
  capacidad**, cambiar el orden de los pasos, o mover el `AdmissionTicket`, el
  arreglo se **descarta** aunque los tests pasen. La testabilidad no compra
  permiso para relajar un control.
- **STOP 2** — Si `MockForge` resulta tan permisivo que la rama passes también
  con el `version_authority` equivocado o con los pasos en orden incorrecto, el
  test **no cuenta** y hay que decirlo antes de escribirlo.
- **STOP 3** — Si el test solo puede vivir fuera del crate, y eso obliga a
  exponer `pub` algo que hoy es `pub(crate)`, se para: la alternativa sería
  ensanchar la superficie pública por testabilidad, que es un coste que este
  ciclo no acepta.
- **STOP 4** — Si el guard estructural R3 resulta cumplible con la rama
  inlined, se corrige el guard antes que el producto.

## Lo que este ciclo NO promete

Que la ruta forge **funcione** contra GitHub real. Eso no se sabrá hasta que se
ejecute contra un GitHub real, y ese sigue siendo un `NOT_RUN` declarado. Lo que
este ciclo entrega es que **deje de ser imposible comprobarla**, que es una
condición necesaria y no suficiente.
