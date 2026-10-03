# SCOPE-CONTRACT — c3m1-knowledge-merkle-tree

**Cycle:** `p-63676b11dc0ef88f/c3m1-knowledge-merkle-tree` (abierto, `OPEN/explore`)
**WorkItem:** C3m.1 — Knowledge Merkle Tree real (invalidación incremental por
fingerprints de unidad)
**Date:** 2026-10-03
**Predecesor:** C3m.0 cerrado en `a5722672` con **ADR-0154** `accepted`, que fija
`KMT` = Knowledge Merkle Tree y deja el nombre libre para la estructura.

> **Este SCOPE empieza por una medición que desmonta la premisa del encargo.**
> El roadmap describe C3m.1 como «Knowledge Merkle Tree mínimo real» con una
> jerarquía `project → package → module → file → symbol`. Antes de proponer esa
> jerarquía se midió qué existe, y el resultado está en §2. **La premisa del
> encargo —que el KMT es una estructura de fingerprints que hay que completar—
> no está escrita en ninguna spec**, y la superficie real es más pequeña y más
> rara de lo que el roadmap sugiere. Se dice aquí en vez de descubrirlo
> implementando.

---

## 1. Qué se afirma, y qué no

**Se afirma:** que `KMT` deja de ser un nombre sin estructura detrás y pasa a
designar un árbol de unidades con **fingerprints por nodo**, capaz de responder
`build`, `diff` y `affected_units` — que es lo que el roadmap promete y lo que
ninguna spec define hoy.

**No se afirma:** que este KMT vaya a tener consumidor mañana. La medición de §2
dice que el índice que existe **no lo construye nadie fuera de sus propios tests**
y que `run_reactive_verify` **no tiene ningún llamador de producto**. Construir
la estructura sin resolver eso produce exactamente lo que produjo INC-DEBT-064:
un mecanismo que funciona y al que nadie pregunta.

## 2. Medición previa, no supuesto

Sobre `HEAD` = `f0ee87a2`:

| Cuestión | Cómo se respondió | Resultado |
|---|---|---|
| ¿Existe `KmtNode`/`KmtTree`? | `grep -rn 'KmtNode\|KmtTree'` en `crates/` | **cero coincidencias** |
| ¿Existe `affected_units`? | ídem | **cero** |
| ¿Qué hay hoy con `fingerprint`? | `grep -ro 'fingerprint' crates/ --include=*.rs \| wc -l` | **118**, pero **ninguno es de árbol**: son idempotency keys de `dynamic_expansion.rs` |
| ¿Quién construye el `KmtUnitIndex`? | `grep -rn 'KmtUnitIndex'` | **nadie** fuera de su propio `fn kmt()` de test (`reactive_verify.rs:234`) |
| ¿Quién llama `run_reactive_verify`? | `grep -rn 'run_reactive_verify'` | **solo sus 4 tests** del mismo fichero |
| ¿Quién produce el `WorkspaceChangeSet` que consume? | `grep -rn 'WorkspaceChangeSet' crates/` | **nadie** fuera de `reactive_verify.rs` |
| ¿Qué promete la spec del módulo? | `arch-spec-037`, línea 20 | **una línea**: «KMT/freshness identifies affected units/contracts» |

**Las dos últimas filas son el hallazgo.** El módulo está gobernado por
`arch-spec-025` + `arch-spec-037` — es trabajo de spec, no un descuido — y esas
specs **nombran KMT sin definir su estructura**. El hueco de C3m.1 es real, pero
no es «completar una estructura empezada»: es **definir una que nunca se
escribió**, sobre una entrada que hoy nadie produce.

## 3. El riesgo que esto crea, y por qué el SCOPE lo pone delante

Un árbol de fingerprints con `build`/`diff`/`affected_units` es un objeto
determinista y verificable, y por eso mismo es **fácil de construir y fácil de
dar por bueno sin consumidor**. La precedente está a dos commits de aquí:
INC-DEBT-064: el mecanismo se cerró completo y no lo llama nadie — la misma
forma, con un detector que no tenía dientes hasta una release que nadie podía
publicar.

**Por eso STOP 1 de este SCOPE es una medición y no un tipo:**

> Antes de escribir `KmtNode`, se responde con evidencia **quién** va a
> alimentar el KMT en la primera superficie que lo use, y **de dónde salen los
> fingerprints**. Sin esa respuesta, este ciclo no pasa de `explore`.

No es un requisito de forma: es la diferencia entre un árbol que alguien
consulta y un árbol que nadie mira.

## 3bis. El resultado de la medición de STOP 1: **NEGATIVO**

Medido sobre `HEAD` = `f0ee87a2`, la cadena entera del pipeline, extremo por
extremo. La columna derecha es lo que decide:

| Símbolo | Ocurrencias | Ficheros **fuera** de `reactive_verify.rs` |
|---|---|---|
| `HostEvent` | 13 | **0** |
| `HostEventKind` | 17 | **0** |
| `ChangeSetCoalescer` | 5 | **0** |
| `WorkspaceChangeSet` | 7 | **0** |
| `KmtUnitIndex` | 4 | **0** |
| `run_reactive_verify` | 5 | **0** |
| `ArchitectureConformanceDelta` | 23 | **9** |

**Seis de los siete no aparecen en un solo fichero fuera de su propio módulo.**
La entrada del pipeline no la produce nadie: no hay adaptador que normalice un
evento del host, luego no hay `ChangeSet`, luego no hay índice que construir.
Y de los 9 ficheros que mencionan la salida, varios lo hacen **para declarar que
no hay conversión** — `architecture_debverify/types.rs:120` lo dice literalmente
—; el consumidor real es `architecture_receipt/compose.rs`, que compone un
recibo de un `ArchitectureConformanceDelta` que **nadie produce**.

**El resultado es negativo y por eso este ciclo NO construye el árbol.** El STOP
condition del §6 es explícito: sin fuente real de fingerprints el ciclo se
detiene en `explore` y escribe por qué. Un árbol de fingerprints construido
sobre una entrada que nadie produce sería un segundo mecanismo sin consumidor,
que es la forma exacta de INC-DEBT-064 y la que este SCOPE existe para evitar.

**Lo que hay que decidir, y no es de este ciclo:** quién produce `HostEvent`.
Mientras no haya un adaptador que normalice un cambio del host, todo el módulo
`reactive_verify.rs` —y el KMT que se le quiere encima— es un contrato escrito
sin implementación detrás. Esa es una pregunta de **superficie de producto**, y
la respuesta no se deduce leyendo código.

## 3ter. ¿Es un caso o un patrón? **Medido, y el instrumento falló cuatro veces**

La pregunta que deja abierta el hallazgo de §3bis es si «módulo público sin
entrada» es un hueco aislado o la misma forma en muchos sitios. Se mide, con
un instrumento que **se autocomprueba contra cuatro casos conocidos** —verificados
a mano, no por el propio script— antes de publicar nada:

| Control | Esperado | Por qué se sabe a mano |
|---|---|---|
| `cycle_pause` | **sí** consumido | `sddk-cli/src/cycle.rs:1967` llama `engine.cycle_pause(...)` |
| `reactive_verify` | **no** consumido | STOP 1 de este ciclo, §3bis |
| `event_bus` | **sí** consumido | `sddk-cli/src/cycle.rs:21` lo importa y llama `emit_outcome_event` en `:1443` |
| `architecture_receipt` | **sí** consumido | `sddk-cli/src/architecture_cmd.rs:28` lo importa |

Los dos últimos son **módulos directorio**, y son el control del camino nuevo:
sin ellos, la corrección siguiente no habría tenido con qué autocomprobarse.
Si alguno de los cuatro no sale como se espera, el script **no publica ningún
número** y sale con código 2.

**El instrumento dio cuatro números falsos antes de dar uno con base, y por qué
falló importa más que el número:**

| Versión | Criterio | Dio | Por qué estaba mal |
|---|---|---|---|
| v1 | primer `pub struct` del módulo, buscado en `sddk-engine/src` | 52 | **el alcance**: la CLI vive en otro crate, así que todo lo que consume salía como «sin entrada» |
| v2 | lo mismo, buscado en todo `crates/` | 41 | **la unidad**: medía *mención del símbolo*, no *consumo*; la CLI llama un **método** (`engine.cycle_pause`) sin construir el struct de entrada |
| v3 | quién **usa el módulo**, `lib.rs` excluido | 38 | **la vía y el denominador**: la CLI consume por el camino corto (`use sddk_engine::algo`) sin que el nombre del módulo aparezca, y por método; además el criterio de «fichero base» sólo aceptaba `src/<m>.rs` |
| v4 | v3 + vía del método + símbolos reexportados + `mod.rs` + categoría «solo tests» + 4 controles | **el de abajo** | — |

**El fallo de v3 que casi se lleva por delante la conclusión.** Al buscar
`src/<m>.rs`, el script se saltaba **23 de los 131 módulos declarados**:
los que son directorio con `mod.rs`. No eran módulos menores — entre los
saltados estaban `architecture_receipt`, `architecture_debverify`,
`verify_kernel`, `debverify_kernel`, `context_compiler`, `event_bus`,
`intelligence_loop` y `observation`. Y **`architecture_receipt` es exactamente
el consumidor real de la salida de `reactive_verify`**, según el §3bis. Un
instrumento que se salta al consumidor de lo que investiga no puede después
declarar «sin consumidor»: se dice de lo que se mira, no de lo que no se miró.

Los 23 no cambiaron el número final —resultaron ser 21 con consumidor y 2 solo
desde tests— pero el número de v3 estaba mal aunque el resultado coincidiera, y
por un motivo que esta vez no cambió la respuesta. Eso no es suerte: es la razón
por la que el denominador se declara explícito.

**El resultado, con los cuatro controles en `OK` y sin módulos sin clasificar:**

| | v3 (108 medidos) | **v4 (131 de 131)** |
|---|---|---|
| Declarados en `lib.rs` | 131 | **131** |
| Medidos | 108 | **131** |
| Sin fichero base (sin clasificar) | 23 | **0** |
| Con consumidor de producto | 76 | **97** |
| **Consumidos solo por pruebas** | 8 | **10** |
| **Sin consumidor, ni de producto ni de pruebas** | 24 | **24** |

**Las 24 son las mismas 24**, y todas ellas son módulos de fichero plano: **ningún
módulo directorio está sin consumir**. De las 24 está `reactive_verify` —el
módulo que motivó esta medición—, así que el hallazgo se confirma sobre su
propio caso, y ahora sobre la superficie completa y no sobre cinco sextos.

Entre las 10 «solo tests» están `dynamic_expansion` y `ext_outcome`, que **sí
funcionan** pero sólo desde pruebas, y también `tasks` e
`intelligence_advisory`, que son módulos directorio enteros. Es una categoría
distinta: se usa, ningún comando lo llama. Reportarlos igual que «nadie los
mira» sería el mismo error de medir mal.

**Un nombre que se parece a una cosa y es otra.** `gate_evaluator` está en la
lista de sin consumidor, y el comando `sddk cycle evaluate-gate` existe y
funciona —lo he usado en esta misma sesión—. No es el mismo código: el módulo
evalúa dos gates declarados en el contrato de cycle-7b sobre `DebtReport`
(`debt-severity-assigned` y `debt-priority-assigned`), mientras el comando lo
resuelve por `GateEvaluationInput` en otro camino. **Dos cosas que comparten
nombre sin relación**: el mismo patrón que llevó a KMT en ADR-0154, y la
razón por la que un nombre compartido no es una autoridad compartida.

**Lo que esta medición NO concluye.** Que un módulo no tenga consumidor no lo
hace código muerto: puede ser superficie pública para adopters externos, puede
estar esperando a que un ciclo lo conecte, y el propio `gate_evaluator` es la
prueba de que un nombre puede no tener consumidor y aun así su *propósito*
estar cubierto por otro camino. Esto es un **inventario de riesgo con números
medidos**, no una lista de borrables.

El instrumento queda en `medir-modulos-consumidor.py`, junto a este SCOPE, para
que el número sea reproducible y no una afirmación de sesión.

## 4. Alcance de este ciclo, y lo que queda fuera

**Dentro:**

1. La medición de §3: qué alimenta el KMT y de dónde salen los fingerprints,
   con el resultado escrito aunque sea negativo.
2. `KmtNode`/`KmtTree` con fingerprints por nodo y las tres operaciones que el
   roadmap nombra (`build`, `diff`, `affected_units`), **si y solo si** §3 dio
   una fuente real.
3. Un guard que fije el comportamiento, falsificado con mutaciones al source
   real.

**Fuera, y no por pereza sino porque son otros WorkItems:**

- `as_of` y `analyzer-change`: el roadmap los nombra, pero dependen de una
  fuente de análisis que §3 tiene que medir primero.
- **C3m.3** (provenance provider-neutral), **C3m.4** (evidencia y confianza) y
  **C3m.5** (mapa de contextos): son WorkItems distintos con contratos propios.
- `run_reactive_verify` no se reescribe ni se conecta a un producto en este
  ciclo. Conectarlo es una decisión que necesita saber quién lo invoca.

## 5. Criterios de aceptación

El ciclo no se cierra por «el código existe». Se cierra con:

1. La medición de §3 escrita, **con resultado**, no con la intención de medirla.
2. Si hubo árbol: fingerprints que **cambian cuando cambia el contenido** y
   **no cambian cuando no cambia**, que es la propiedad que lo hace útil y la
   que un árbol sin fingerprints no tiene.
3. `diff` que detecta un cambio real y **no** inventa uno donde no lo hay.
4. Guard falsificado con mutaciones aplicadas al **source real**, con el recuento
   de detectadas / sobrevividas / no medibles declarado.
5. Perfil completo del workspace en verde, y el recuento **medido**, no heredado.

## 6. Stop conditions

- Si §3 no encuentra una fuente real de fingerprints, el ciclo **se detiene en
  `explore`** y escribe por qué. No se construye un árbol speculative.
- Si construir el árbol obliga a tocar `run_reactive_verify` o su contrato, el
  ciclo se detiene y se escribe qué haría falta.
- Si el guard necesita una excepción para pasar, **el guard se corrige**, no la
  excepción. Y una exclusión que tape un caso real se cuenta como
  sobrevida, no como detalle de forma.
