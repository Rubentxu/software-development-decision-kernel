# SCOPE-CONTRACT — cl-build-identity

**Cycle:** `p-63676b11dc0ef88f/cl-build-identity` (por abrir)
**WorkItem:** remedio de **INC-DEBT-064** (high/P1, open)
**Date:** 2026-10-03

> **Este documento se reescribió después de medir.** La primera versión proponía
> «un `build.rs` embebe el SHA del checkout». La medición desmontó esa
> propuesta en cuatro puntos (§3), y el diseño que sale es distinto. Se conserva
> el texto original en §10, sin borrar, porque explica por qué se escribió así.

---

## 1. Qué se afirma, y qué no

**Se afirma:** que el binario de `sddk` declara **qué commit es**, con una
procedencia declarada, de modo que dos binarios con el mismo número de versión
dejen de ser indistinguibles.

**No se afirma:** que el binario sepa si el checkout está al día. Eso requiere el
checkout, y un binario instalado en la máquina de un usuario no lo tiene.

## 2. Medición previa, no supuesto

| Cuestión | Cómo se respondió | Resultado |
|---|---|---|
| ¿Hay identidad de build embebida? | `ls build.rs`, `grep GIT_SHA\|VERGEN\|BUILD_SHA` | **no existe ninguna** |
| ¿Cómo se imprime `--version`? | `main.rs` delega en `run_from`; clap usa `CARGO_PKG_VERSION` | `sddk 2.5.3`, sin SHA |
| ¿Quién consume `--version`? | `grep --version` en `scripts/`, `tests/`, `.github/` | **dos** consumidores reales |
| ¿Se puede añadir texto a `--version`? | lectura de `install.sh:399-417` | **no**: `awk '{print $NF}'` toma el último campo |
| ¿`dev doctor` ya avisa? | ejecución + `doctor.rs:425-468` | no; solo entorno y coherencia de bundle |

**El hallazgo de `install.sh` fija el diseño de la superficie.** La línea 416 es
`printf '%s' "$out" | awk '{print $NF}'`, y hoy el último campo de `sddk 2.5.3`
es `2.5.3`. Si el SHA se añadiera **al final**, el último campo pasaría a ser `)`
y el instalador almacenaría un paréntesis como versión. El comentario de la
línea 390 documenta que esa línea ya dio un fallo parecido en session-16.

**Por eso `--version` no se toca.**

## 3. La medición que desmontó la propuesta original

Se construyó un crate mínimo con el `build.rs` que el ciclo iba a usar, y se
midió en cuatro escenarios. **Los cuatro son fallos reales:**

| Escenario | Resultado medido |
|---|---|
| **Sin `.git`** | build **exit 0**, declara `unknown` — degrada bien |
| **Con `.git`, sin `rerun-if-changed`** | el script se **cachea**: la identidad queda congelada en el primer build |
| **Con `rerun-if-changed` solo sobre `.git/HEAD`** | **congelada**: `.git/HEAD` no cambia de contenido al commitear, sigue siendo `ref: refs/heads/<rama>` |
| **Con el ref resuelto también, y `packed-refs` presente** | **congelada y además INCORRECTA**: declaraba `9b3f0076` cuando el HEAD era `247e808d` |

**Conclusión, y es el resultado del ciclo:** un `build.rs` que deriva la
identidad de `.git` **no puede, por sí solo, garantizar que el valor que declara
corresponda al código que se compiló**. Hay una condición —refs empaquetados— en
que emite un SHA con toda la apariencia de ser verdad y no lo es.

**Un detector que declara un valor obsoleto sin señal es peor que no tener
detector**, porque su salida es indistinguible de la correcta. Eso es exactamente
lo que pasó en el cuarto escenario, y es el motivo de que el diseño cambie.

## 4. Diseño, después de la medición

**La identidad la fija quien lanza el build, no el script de build.**

1. `SDDK_GIT_SHA` es la fuente de verdad, puesta por quien construye: el paso 3
   de `release.sh`, el job de CI, o quien compila a mano.
2. El `build.rs` es un **respaldo degradado**: si nadie fija la variable,
   declara `unknown` — **nunca un SHA adivinado**.
3. La identidad embebida **lleva su procedencia** —`env`, `git` o `absent`— de
   modo que un `unknown` no se pueda confundir con un commit.
4. El estado **sucio** se declara como campo propio, no se deduce: un
   `f7313bef` puede tener cambios sin commitear encima, y el SHA solo no lo dice.

**Consecuencia asumida:** un binario construido a mano sin `SDDK_GIT_SHA`
declara `unknown`, y eso es menos útil que un SHA equivocado pero es cierto. La
alternativa —derivarlo de `.git`— es lo que §3 midió como incapaz de decirlo.

## 5. Objetivos

1. **O1.** El binario declara commit **y procedencia**, en una superficie propia,
   **sin alterar la forma de `--version`**.
2. **O2.** Sin `SDDK_GIT_SHA` ni repositorio, el binario **compila** y declara
   `unknown` con procedencia `absent`.
3. **O3.** Un checkout con cambios sin commitear se declara sucio, en campo
   propio, y **no** se presenta como el mismo estado que uno limpio.
4. **O4.** Existe una comparación ejecutable entre la identidad del binario y el
   checkout que diga `al día` / `retrasado` / `sin checkout` / `desconocido`, y
   **ninguna** de esas respuestas es «bien» por omisión.

## 6. Lo que NO entra de alcance

- **No** se cambia `sddk --version`, por el motivo medido de §2.
- **No** se deriva de `.git` en el `build.rs`. Es la propuesta que §3-medió
  incapaz, y dejarla sería repetir un defecto ya caracterizado.
- **No** se toca `BUNDLE.toml` ni el contrato del bundle: propagar el SHA al
  artefacto distribuido es concernia propia.
- **No** se arregla la ventana declarada-pero-no-publicada: la resuelve publicar,
  y publicar está bloqueado por la clave KMS.
- **No** se inventa versionado nuevo. El SHA es identidad de build, no versión:
  no alimenta `Cargo.toml` ni el cálculo de SemVer.

## 7. Guards

| Guard | Qué fija | Por qué |
|---|---|---|
| **R1** | la identidad se expone sin que `--version` cambie de forma | STOP 2 |
| **R2** | sin fuente, el binario compila y declara `unknown`/`absent` | STOP 1 |
| **R3** | un checkout sucio no se presenta como limpio | O3 |
| **R4** | la comparación devuelve `desconocido` —no «al día»— cuando la identidad no se puede establecer | STOP 3 |
| **R5** | la comparación dice `retrasado` con dos binarios reales, medido | propiedad del ciclo |
| **R6** | la procedencia acompaña siempre al valor, y `unknown` nunca se emite con procedencia `git` | §3, escenario 4 |

## 8. STOP conditions

- **STOP 1 — la construcción no puede depender de nada externo.** Sin
  `SDDK_GIT_SHA` y sin repositorio, el build pasa y declara `unknown`. Un
  `build.rs` que falla deja de poder construirse desde un tarball, que es una
  forma real de distribución.
- **STOP 2 — `--version` no cambia de forma.** `install.sh:416` y
  `release.sh:1360` dependen del último campo. Comprobación literal, no opinión.
- **STOP 3 — la comparación no puede responder «al día» por omisión.** Si no
  sabe, dice que no sabe.
- **STOP 4 — si el remedio exige tocar el bundle, se corta.**
- **STOP 5 — si la identidad se declara sin procedencia, se descarta.** Un SHA
  sin de dónde vino es el escenario 4 de §3 con otro nombre.
- **STOP 6 — si la única forma de obtener la identidad es derivarla de `.git`
  con `rerun-if-changed`, se descarta**, por §3. La procedencia `git` existe solo
  como **dato de diagnóstico**, no como fuente de verdad.

## 9. Riesgos declarados antes de empezar

- **R-a. Alguien compila a mano y obtiene `unknown`.** Es el comportamiento
  correcto y declarado, no un fallo: la alternativa es un valor que miente.
- **R-b. La comparación depende de cómo se construyó el binario.** Un binario de
  un tarball no tiene con qué compararse y debe decirlo.
- **R-c. `SDDK_GIT_SHA` puede mentir si quien la pone miente.** Se acota
  declarando la procedencia y validando el formato del SHA.
- **R-d. Coste de recompilación.** Un `build.rs` que se re-ejecute en cada build
  alarga el ciclo; se acota con `rerun-if-changed` acotado.

## 10. La propuesta original, conservada

La primera versión de este SCOPE proponía, literal, «un `build.rs` embebe el
commit con el que se construyó». **§3 la midió y la descartó**: con
`packed-refs` presente emitía un commit obsoleto sin ninguna señal. Se conserva
el texto porque explica por qué el documento se reescribió, y reescribirlo sin
más no explicaría nada.

## 11. Fuera de alcance del ciclo, y declarado

- **La ruta forge contra un GitHub real** sigue `NOT_RUN`.
- **INC-DEBT-050, 060, 061, 063 y 049** son decisiones del operador y no se tocan
  aquí.
