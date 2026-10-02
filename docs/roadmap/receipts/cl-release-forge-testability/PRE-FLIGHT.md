# PRE-FLIGHT — cl-release-forge-testability

**Cycle:** `p-63676b11dc0ef88f/cl-release-forge-testability`
**Date:** 2026-10-03
**Readiness: READY**

---

## Premisas, comprobadas antes de decidir

| # | Premisa | Cómo se comprobó |
|---|---|---|
| **P1** | La rama forge de la CLI no tiene test | `with_runner` aparece solo en `forge.rs:520` y `:569` (tests del gateway) y en la propia definición. El único test de la CLI que nombra forge es `cli.rs:4657`, que comprueba que `--repo` **sin** `--route forge` falla. **Ninguno alcanza la rama.** |
| **P2** | El runner se puede inyectar | `GitHubForge { repo, runner: Box<Runner> }` con `pub fn with_runner` — **ya existe** (`forge.rs:133`). |
| **P3** | El motor acepta cualquier forge | `plan_release(input, &dyn Forge)` y `apply_release(…, &mut dyn Forge, …)` — **ya son polimórficos** (`release.rs:193`, `:422`). |
| **P4** | Hay un doble de prueba utilizable | `pub struct MockForge` (`forge.rs:336`), re-exportado en `sddk-gateway/src/lib.rs:40-41`. |
| **P5** | El test puede vivir sin ensanchar visibilidad | Los tests de `release_cmd.rs` viven en el `mod tests` del propio módulo y reached `super::`; una `fn` privada es alcanzable desde ahí. **STOP 3 no dispara.** |
| **P6** | El defecto es de cableado, no de la red | El runner real se fija en `release_cmd.rs:882` y no hay forma de sustituirlo. Todo lo demás ya está preparado. |

Las seis se comprobaron **leyendo y buscando**, no suponiendo. P1 en particular
contradicía la comfy assumption de que «no hay test porque necesita red», y lo
que muestra es que no hay test porque **el call site no ofrece la alternativa**.

## Los guards, en la tabla que el gate lee

Un guard no es un requisito: es un requisito **convertido en algo que puede
salir falso**. El gate `requirements-testable` lee esta tabla, y por eso tiene
que estar aquí y no en un fichero aparte — un gate que no puede señalar dónde
falta su objetivo es insatisfacible, que es lo que pasó con `04-req-testable.py`
en el ciclo de `ledger export`.

| Guard | Qué exige | Qué pasa si no se cumple |
|---|---|---|
| R1 | La rama delega en una función que recibe `&mut dyn Forge` | La rama vuelve a construir el runner real en línea y queda inalcanzable |
| R2 | Con `MockForge`, la función llega a `apply_release` y devuelve los pasos **aplicados** | Devuelve un `Err`, o un outcome vacío, y el test no mira nada |
| R3 | **Estructural**: el fuente muestra que el call site **delega** y no construye `GitHubForge::new` en la rama | Re-inlinear deja la rama inalcanzable **con todos los tests en verde** |
| R4 | **Estructural**: mismas capacidades (`pr.create`, `pr.merge`, `release.create`), mismo orden `CreatePr → MergePr → CreateRelease`, y el `AdmissionTicket` sigue envolviendo la cadena | Una extracción reordena o relaja un control sin que nada lo note |
| R5 | El doc de `release_cmd.rs` **ya no** afirma que la ruta forge «no tiene test» ni que «no es alcanzable sin red» | La afirmación falsa sobrevive a un test que la contradice |

## Mapa objetivo → guard

| Objetivo | Guards | Por qué ese |
|---|---|---|
| **O1** la rama es alcanzable sin red | R1, R3 | hoy `release_cmd.rs:882` construye el runner real en línea, sin alternativa |
| **O2** el test ejercita el cuerpo real | R2 | una reimplementación daría verde sin ejecutar la rama |
| **O3** cero cambio de comportamiento | R4 | una extracción puede reordenar o relajar sin que nadie lo note |
| **O4** la afirmación queda sustituida | R5 | el comentario sobrevive a un test que lo contradice |

## Riesgos

- **RA** — Que `MockForge` sea demasiado permisivo y el test pase con la rama
  rota. **Mitigación:** STOP 2, y el falsificador del lote 3 muta la rama para
  comprobar que el test cae por la razón correcta.
- **RB** — Que la extracción cambie el comportamiento sin que nadie lo note.
  **Mitigación:** `R4` structural, no de conteo.
- **RC** — Que el guard `R3` se cumpla con la rama inlined. **Mitigación:**
  STOP 4, y el falsificador incluye precisamente esa mutación.

## Decisión

**READY.** El defecto está **medido y localizado**, el mecanismo de inyección
**existe** y no hay que inventarlo, y el cambio es una extracción de función más
un test. El riesgo mayor —relajar un control para poder probar— está cubierto por
STOP 1, que descarta el arreglo aunque los tests pasen.

## Veredicto sobre el alcance temporal

Este ciclo **no** desbloquea la release 2.5.3, que sigue bloqueada por la clave
KMS del operador. Entrega algo distinto y previo: que la ruta de publicación por
forge **deje de ser imposible de verificar**, que es condición necesaria —no
suficiente— para poder afirmar nada sobre ella.
