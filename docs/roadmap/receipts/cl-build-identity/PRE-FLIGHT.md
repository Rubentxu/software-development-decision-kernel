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

## 2. Mapa objetivo → guard

| Objetivo | Guard | Cómo se ejercita |
|---|---|---|
| O1 la identidad se expone sin tocar `--version` | R1 | inspección de la forma de `--version` antes y después |
| O2 sin fuente, compila y declara `unknown`/`absent` | R2 | build en un directorio sin repo y sin la variable |
| O3 un checkout sucio no se presenta como limpio | R3 | árbol con un cambio sin commitear |
| O4 la comparación no dice «bien» por omisión | R4 | sin checkout, y con identidad `unknown` |
| propiedad del ciclo | R5 | dos binarios reales, uno construido antes del último commit |
| §3, escenario 4 | R6 | una identidad con procedencia `git` nunca declara `unknown` |

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
