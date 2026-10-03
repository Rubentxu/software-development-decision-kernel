# PRE-FLIGHT — cl-build-identity

**Cycle:** `p-63676b11dc0ef88f/cl-build-identity` (por abrir)
**WorkItem:** remedio de **INC-DEBT-064** (high/P1, open)
**Date:** 2026-10-03

---

## 1. Premisas, y cómo se comprobó cada una

Ninguna se da por buena. Las que se comprobaron ejecutando llevan el comando.

| # | Premisa | Comprobación | Resultado |
|---|---|---|---|
| P1 | No hay identidad de build embebida | `ls crates/sddk-cli/build.rs`; `grep -r GIT_SHA\|VERGEN\|BUILD_SHA crates/` | **confirmada** |
| P2 | `cargo` lee el paquete sin `.git` | copia del repo sin `.git`, `cargo metadata --no-deps` | **exit 0**, 8 paquetes, 2.5.3 |
| P3 | Un `build.rs` que busca el commit **degrada** sin repo | crate mínimo con el `build.rs` real, sin `.git` | **exit 0**, `sha=unknown source=absent` |
| P4 | Derivarla de `.git` **no basta** | mismo crate, cuatro escenarios | **congelada** en 3 de 4; con `packed-refs` **declara un commit obsoleto** |
| P5 | `--version` no puede cambiar de forma | `install.sh:399-417` | `awk '{print $NF}'` toma el último campo → añadir SHA al final devuelve `)` |
| P6 | `dev doctor` no detecta esta condición | ejecución + `doctor.rs:425-468` | solo entorno y coherencia de bundle |
| P7 | Un SHA **sí** discrimina dos binarios | `22-medir-binario-al-dia.py` con los dos binarios | `cycle list`: `False` en el viejo, `True` en el del código |
| P8 | `git` está disponible en el entorno de build | `git --version` + los cuatro escenarios | sí, y su ausencia es el caso degradado de P3 |

**P4 es la premisa que cambia el trabajo**, y por eso está al principio de todo:
no es un detalle de implementación, es lo que hace que la propuesta original
sea insuficiente.

## 2. Guards

Cada guard dice qué fija y **cómo se rompe**. Un guard sin modo de fallo escrito
es una aserción que no puede caer, y un objetivo sin guard es una nota.

| Guard | Qué fija | Cómo se rompe |
|---|---|---|
| R1 | la identidad de build se expone en superficie propia, y `--version` conserva su forma exacta | el SHA se añade al final de `--version` y `install.sh:416` guarda un paréntesis como versión |
| R2 | sin `SDDK_GIT_SHA` ni repositorio, el binario **compila** y declara `unknown` con procedencia `absent` | el `build.rs` devuelve error y la construcción falla en un directorio sin `.git` |
| R3 | un checkout con cambios sin commitear se declara sucio, en campo propio | el valor solo lleva el SHA, y un binario construido sobre un árbol sucio se presenta como limpio |
| R4 | la comparación responde `desconocido` —nunca «al día»— cuando la identidad no se puede establecer | la comparación sale verde por omisión cuando no hay checkout o cuando la identidad es `unknown` |
| R5 | la comparación dice `retrasado` con dos binarios reales, medido | comparar la fecha del binario en vez del SHA, y dos binarios del mismo día salen «al día» |
| R6 | la procedencia acompaña siempre al valor, y `unknown` nunca se emite con procedencia `git` | el `build.rs` deriva de `.git` y emite el commit del **primer** build con aspecto de verdad |

### Mapa objetivo → guard

| Objetivo | Guards | Evidencia medida hoy |
|---|---|---|
| **O1** la identidad se expone sin alterar `--version` | R1, R6 | `install.sh:416` hace `awk '{print $NF}'`; `sddk --version` es `sddk 2.5.3` |
| **O2** sin fuente, compila y declara `unknown`/`absent` | R2 | build medido: exit 0, `sha=unknown source=absent` |
| **O3** un checkout sucio no se presenta como limpio | R3 | hoy el SHA es el del último commit y no dice nada del cambio sin commitear |
| **O4** la comparación no dice «bien» por omisión | R4, R5 | `cycle list` separa los dos binarios: `False` en el viejo, `True` en el del código |

**R6 no es un guard de conveniencia: es el cuarto escenario del §3 del
SCOPE**, medido, donde el build declaraba `9b3f0076` con el HEAD en `247e808d`.

## 3. Qué se necesita del entorno

- `cargo`, `git`: presentes (P8).
- **No** hace falta red, ni credenciales, ni un repositorio remoto.
- **No** hace falta la clave KMS: el ciclo no publica.

## 4. Qué NO se necesita, y por qué

- **No** hace falta tocar el bundle. Por eso STOP 4 no corre riesgo.
- **No** hace falta publicar 2.5.3. El ciclo construye en local; publicar sigue
  bloqueado por la clave KMS y es del operador.
- **No** hace falta ejecutar contra un repositorio ajeno. La comparación es entre
  el binario y el checkout, ambos locales.

## 5. Riesgos que siguen abiertos tras la medición

- **R-c. `SDDK_GIT_SHA` puede mentir si quien la pone miente.** Se acota
  validando el formato y declarando la procedencia. **No** se puede cerrar por
  construcción: una variable de entorno es una entrada, no una prueba.
- **R-a. Un binario construido a mano queda en `unknown`.** Es correcto y
  declarado; la consecuencia es que `dev doctor` no puede afirmar nada sobre él.
  Se acepta: la alternativa era un valor que miente, que es lo que §3 midió.
- **R-d. Refs empaquetados.** La procedencia `git` queda como **dato de
  diagnóstico** y no como fuente de verdad (STOP 6). Un `build.rs` que la use
  para decidir queda prohibido por diseño, no por convención.

## 6. Veredicto

**Readiness: READY**

Todas las premisas que sostienen el diseño están medidas, y la que no se
sostuvo está escrita en P4 con su modo de fallo. El trabajo puede empezar.

Lo que **no** se afirma aquí: que el arreglo vaya a funcionar. Eso lo dirá la
falsificación, y el falsificador de este ciclo tendrá que detectar cada uno de
los cuatro escenarios de §3 del SCOPE.
