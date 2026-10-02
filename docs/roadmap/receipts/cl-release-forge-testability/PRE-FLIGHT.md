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

## Mapa objetivo → guard

Este mapa va **aquí**, en el PRE-FLIGHT, y no en un fichero aparte: un gate de
requisitos que no puede señalar dónde falta su objetivo es insatisfacible, y eso
ya pasó en esta serie con `04-req-testable.py`.

| Objetivo | Requisito | Guard | Dónde se mira |
|---|---|---|---|
| **O1** — la rama es alcanzable sin red | R1 | `T1` + `T3` | `T1` comportamiento: la función existe y la CLI delega en ella. `T3` estructural: lee el fuente |
| **O2** — el test ejercita el cuerpo real | R2 | `T2` | `T2` invoca la función con `MockForge` y comprueba el **resultado** |
| **O3** — cero cambio de comportamiento | R3, STOP 1 | `T4` | `T4` compara capacidades, orden de pasos y presencia del ticket antes/después |
| **O4** — la afirmación queda sustituida | R4 | `T5` | `T5` el guard del comentario: el doc ya no afirma «no tiene test» |

## Riesgos

- **R-a** — Que `MockForge` sea demasiado permisivo y el test pase con la rama
  rota. **Mitigación:** STOP 2, y el falsificador del lote 3 muta la rama para
  comprobar que el test cae por la razón correcta.
- **R-b** — Que la extracción cambie el comportamiento sin que nadie lo note.
  **Mitigación:** `T4` structural, no de conteo.
- **R-c** — Que el guard `T3` se cumpla con la rama inlined. **Mitigación:**
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
