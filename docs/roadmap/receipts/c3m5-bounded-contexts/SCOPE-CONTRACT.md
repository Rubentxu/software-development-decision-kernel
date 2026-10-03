# SCOPE-CONTRACT — C3m.5 R0 bounded-context decision

**Ciclo:** `c3m5-bounded-contexts`
**Estado:** MEDIDO, decisión-taking cerrada en `explore` con la decisión **NO** tomada,
y la razón medida.
**Fecha:** 2026-10-03
**Instrumento:** [`medir-contextos.py`](medir-contextos.py) · **Falsación:**
[`falsificar-medicion.sh`](falsificar-medicion.sh)

---

## 1. Qué pide C3m.5, literalmente

`docs/roadmap/ROADMAP.md:126` dice, íntegro:

> C3m.5 R0 bounded-context decision.

Una línea. No dice cuántos contextos, ni cuáles, ni cómo se decide. **Se comprobó
que no existe taxonomía declarada en ninguna parte**: `grep -rn "bounded context"`
sobre `docs/` devuelve usos de la palabra en prosa (`docs/architecture/README.md:63`,
`arch-spec-046`, `arch-spec-A3-S3`) y ningún catálogo de contextos que PuEDE
aplicarse al código.

**Consecuencia que condiciona todo lo demás:** la decisión R0 no se puede
contrastar contra un criterio declarado, porque no existe. Cualquier
taxonomía que se escriba es una propuesta, y una propuesta no se mide: se
mide **cuánto del motor deja fuera**, que es lo que este ciclo mide.

## 2. La unidad de módulo, que no es una

Antes de contar nada hay que decir qué es un módulo, porque tiene **tres
layouts** y contarlos mal desplaza todas las cifras:

| Layout | Forma | Nº en `sddk-engine` |
|---|---|---|
| raíz-fichero | `src/foo.rs` | 109 |
| raíz-directorio | `src/foo/mod.rs` | 23 |
| **raíz-partida** | `src/foo.rs` **+** `src/foo/` | **1** |

El tercero es `authority_engine`: `authority_engine.rs` (1.235 líneas) declara
`pub mod bridge;` y `pub mod runner;` (líneas 1232-1233) y sus fuentes viven en
el directorio homónimo (569 líneas). **1.804 líneas en un módulo, partidas en
dos estilos de fichero.** Un conteo que solo mire el `.rs` dice 1.235 y parece
un módulo mediano.

Total: **133 raíces, 96.649 líneas.**

## 3. La superficie pública tiene dos formas, y una no tiene nombre

| Forma | Nº raíces | Líneas |
|---|---|---|
| `pub mod X;` — publica el **nombre** | 131 | 95.006 |
| `mod X;` + `pub use X::*;` — publica **solo los símbolos** | **2** | **1.643** |
| `mod X;` sin reexportar nada | **0** | 0 |

Los dos módulos de la segunda fila son `adoption` (1.229 líneas,
`lib.rs:16` y `:155`) y `paths` (414 líneas, `lib.rs:109` y `:222`).
**Las 133 raíces exponen símbolos públicos: no hay una sola raíz privada de
verdad.** Y 1.643 líneas llegan al usuario por un camino donde
`sddk_engine::<módulo>` **no existe** — un mapa de nombres no puede ni
encontrarlas ni dibujarlas.

## 4. El consumo no va por rutas: va por una fachada

`sddk-engine` define `pub struct Engine<L: Ledger>` en `lib.rs:1144`, y **cinco
ficheros distintos le extienden el tipo con `impl Engine`**, 40 `pub fn` en
total:

```
lib.rs 27 · dynamic_expansion.rs 7 · cycle_replan.rs 2 · cycle_pause.rs 2 · cycle_supersede.rs 2
```

El CLI llama **16 métodos distintos** de esa fachada
(`apply_cycle_start`, `apply_transition`, `cycle_pause`, `cycle_replan`,
`cycle_resume`, `cycle_supersede`, `diff`, `evaluate_gate`, `ledger`,
`plan_cycle_start`, `query`, `reconstruct`, `register_policy`, `strict`,
`workflow`).

**Consecuencia directa y verificable:** `crates/sddk-cli/src/cycle.rs:1967`
llama `context.engine.cycle_pause(...)` y el método está definido en
`cycle_pause.rs:68`. Un grafo de dependencias que solo mire rutas `use` declara
ese módulo **sin consumidor**, y se equivoca. Es el mismo error que ya'avais
publicado y corregido: el instrumento de C3m.1 lo documentaba en su cabecera
y tenía un control explícito (`cycle_pause -> True`).

## 5. La regresión que casi se publica

Este ciclo **reconstruyó el instrumento desde cero** y la primera versión dio
**37** módulos sin consumidor, contra los **24** de `INC-DEBT-065`. No se
publicó ninguno de los dos: se contrastaron.

La cifra de 37 era **falsa en 13 módulos**, todos consumidos por vías que un
grafo de rutas no ve. Al añadir las cuatro vías reales —ruta `use`, grupo
anidado, símbolo reexportado y llamada por fachada— y un control para cada una,
la cifra bajó.

**Y entonces apareció el defecto que el contraste con el otro instrumento
delató, que es un bug propio y no una diferencia de criterio:** para un módulo
**fichero**, `relative_to(SRC).parts[0]` devuelve el nombre **con extensión**
(`human_resume_view.rs`), así que la autoexclusión nunca casaba y el módulo se
contaba a sí mismo como su propio consumidor. Dos módulos
(`human_resume_view`, `secretary_l2_replan`) salían con consumidor fantasma.

**Dos instrumentos escritos por separado, sobre el mismo repo, midiendo la
misma pregunta, discreparon; y el defecto estaba en el código de uno de ellos,
no en la noción de consumo.** Un número contrastado con otro número del mismo
autor no está contrastado: es el mismo error con otro multiplicador. La
primera versión de la "cuenta independiente" de este script compartía
exactamente el punto ciego del parser —ninguno de los dos sabía contar
llaves— y por eso los dos decían 2 y los dos estaban mal.

## 6. Convergencia final, con las seis diferencias una a una

| | Nº |
|---|---|
| Este script | **30** |
| `INC-DEBT-065` (publicado, `status: open`) | **24** |

El conjunto de este script es un **superconjunto estricto**: ninguno de los 24
publicados se pierde. Las 6 diferencias, cada una comprobada abriendo el fichero:

| Módulo | Causa |
|---|---|
| `adoption` | `mod adoption;` es privado (`lib.rs:16`) con `pub use adoption::*;` (`:155`). No está en el denominador de 131 `pub mod` de INC-065, que mide la superficie **por nombre**. |
| `paths` | Igual: `lib.rs:109` + `:222`. 414 líneas de superficie pública sin ruta nominal. |
| `durable_map_fanout` | INC-065 lo cuenta consumido por un **enlace en un doc-comment**: `typed_reduce_aggregator.rs:8` cita `[DurableMapFanOut](crate::durable_map_fanout::…)` dentro de un `//!`. **Citar no es consumir.** |
| `gate_signing` | `pub use gate_signing::*;` (`lib.rs:202`). El asterisco no prueba consumo; la única referencia externa es una cadena en un test. |
| `inc_generator` | `pub use inc_generator::*;` (`lib.rs:211`). Igual, y la única referencia externa también es una cadena en un test. |
| `up_to_date` | INC-065 lo cuenta consumido por **coincidencia de subcadena**: `sddk-domain/src/goal.rs:168` declara `pub fn is_up_to_date`, que contiene el texto `up_to_date`. No es el módulo. |

**Ninguna de las 6 corrige a INC-DEBT-065 hacia arriba y ninguna lo corrige
hacia abajo por criterio: cuatro son correcciones suyas y dos no están en su
universo.** El número publicado no se reescribe aquí: se corrige en su fichero,
que es donde vive.

## 7. Lo que la convención propuesta no alcanza

Los `DOMINIOS` del instrumento son **una propuesta de este script**, escrita a
mano, y por eso la tabla imprime `SIN CAJA` en vez de forzar el encaje. El
resultado es el dato útil:

| Dominio | Raíces | Líneas | Sin consumidor |
|---|---|---|---|
| **`SIN CAJA`** | **68** | **36.007** | 27 |
| decision | 12 | 11.508 | 3 |
| verification | 9 | 11.320 | 3 |
| architecture | 8 | 8.219 | 1 |
| governance | 9 | 7.152 | 2 |
| alignment | 5 | 7.562 | 0 |
| knowledge | 5 | 4.498 | 0 |
| agentia | 10 | 4.735 | 1 |
| intelligence | 5 | 2.721 | 1 |
| eventing | 2 | 2.927 | 0 |

**68 de 133 raíces (51%) no caen en ninguna caja, y ese cajón sin nombre es el
mayor de todos**, más grande que `decision` y que `verification` juntos. No es
que falten prefijos en la lista: es que la lista agrupa por nombre y el motor
no está organizado por nombre.

## 8. Decisión

**No se puede tomar la decisión R0 como una taxonomía de módulos, y la razón
está medida, no es una preferencia de estilo.** Un mapa de contextos dibujado
sobre los módulos de `sddk-engine` sería falso en tres puntos a la vez:

1. **El consumo real no va por módulos.** Va por la fachada `Engine`, que es un
   tipo repartido en 5 ficheros. Un grafo de módulos describe como fronteras
   unas costuras que el código no tiene.
2. **La superficie pública tiene dos formas** y 1.643 líneas no tienen nombre de
   módulo. Cualquier frontera dibujada sobre nombres deja esas líneas sin dueño
   o las asigna al cajón equivocado.
3. **El 51% del motor no cae en ninguna convención de nombres**, y el resto
   tampoco está limpio: 38 raíces (14.852 líneas) están **anunciadas en la
   superficie pública y no las mueve ningún comando**.

**Lo que la medición sí sostiene como decisión, y es la de menor alcance que la
evidencia permite:** *la frontera de contexto de `sddk-engine` es el tipo
`Engine`, no el directorio de módulos.* El R0 se aplica al **conjunto de
métodos de la fachada** —16 los ve el CLI, 40 define el engine— y no a las 133
raíces. Nombrar contextos antes de decidir dónde está la frontera produce un
mapa que parece responder a la pregunta y no la responde.

**Lo que queda bloqueado y por qué:** los 38 módulos anunciados sin consumidor
(14.852 líneas) son trabajo de otro ciclo. La decisión de qué se queda y qué se
retira es **`R2` + `INC-DEBT-065`**, no C3m.5, y C3m.5 no puede adelantarla
porque un mapa de contextos que incluya código que nadie llama presenta como
habitado un contexto que no lo está.

## 9. Verificación del instrumento

- **Autoprueba del parser: 16/16** sobre código sintético con la respuesta
  escrita a mano, incluidos 6 casos que deben salir **vacíos** (cadena,
  comentario, doc-comment, consumo interno, alias, `::*`).
- **Controles contra el grafo real: 3**, todos contra listas escritas a mano, no
  contra `> 0`:
  - `reactive_verify` → 0 consumidores de producto (caso publicado en C3m.1).
  - `cycle_pause` → consumido **solo por la fachada**. Sin este control el
    instrumento era verde y falso en 13 módulos.
  - `event_bus` → 4 ficheros, contrastados uno a uno con la lista escrita a mano.
- **Convergencia con un instrumento ajeno**: declarada arriba, con las 6
  diferencias y su causa. El script **no publica** si el conjunto no queda
  explicado.
- **Falsación con mutaciones al source real: 5 detectadas, 0 sobrevividas**
  (`falsificar-medicion.sh`). Las cinco tocan `crates/` de verdad y se revierten
  con `git checkout --`, comprobando al final que la medición vuelve al baseline.
  - M1 consumo por ruta `use` → el conjunto encoge en 1.
  - M2 llamada por fachada `.<mod>()` → el conjunto encoge en 1. **La primera
    versión de M2 sólo mencionaba `Engine` y no ejercitaba la vía; daba el
    mismo resultado que una mutación de ruido, y por eso era un test que no
    probaba lo que decía. Corregido en el guard, no en el producto.**
  - M3 cita en doc-comment → no se mueve.
  - M4 subcadena `is_up_to_date` → no se mueve.
  - M5 `pub use X::*` añadido a `lib.rs` → no se mueve (`::*` no es evidencia).

## 10. Trabajo siguiente

- [ ] Corregir el recuento de `INC-DEBT-065` (24 → 30, con las 4 causas) **en su
      propio fichero**, sin reescribir la evidencia que ya había.
- [ ] Registrar el layout raíz-partida de `authority_engine` en la deuda de
      legibilidad: es el único módulo con dos estilos de fichero y ningún
      guard lo vigila.
- [ ] `C3m.5` queda en `explore` con `Readiness: NOT_READY`: la decisión de qué
      se hace con los 38 módulos anunciados sin consumidor pertenece a `R2`.
