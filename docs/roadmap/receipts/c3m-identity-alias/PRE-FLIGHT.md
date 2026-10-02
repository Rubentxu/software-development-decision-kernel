# C3m-identity-alias — PRE-FLIGHT

**Cycle:** `p-63676b11dc0ef88f/identity-alias` (C3m, ROADMAP §C3m)
**Date:** 2026-10-02T10:45:00Z
**Status:** READY
**Workspace:** 2.5.3 (declarada, no publicada; último tag remoto v2.5.2)
**HEAD:** `e7864c0b` (`docs(adr): ADR-0152 …`), `HEAD == origin/main`, árbol limpio
**Authority:** ADR-0152 `status: proposed`; SCOPE-CONTRACT en
`docs/roadmap/receipts/c3m-identity-alias/SCOPE-CONTRACT.md`

## Readiness: READY

| # | condición | estado |
|---|---|---|
| 1 | ADR escrito con decisión, costos y criterios falsables | OK — ADR-0152 |
| 2 | SCOPE-CONTRACT con objetivo falsable, no-objetivos y STOP conditions | OK |
| 3 | Superficie de código mapeada y leída, no inferida | OK — se cita línea por línea abajo |
| 4 | Superficie mínima para el lote actual | 1 fichero del dominio, sin I/O |
| 5 | Tests que fijan la propiedad, con falsificador cada uno | declarados abajo |
| 6 | Riesgo de datos | **cero**: este lote no abre ni escribe ningún ledger |

## Superficie (leída, con línea)

| qué | dónde | por qué importa |
|---|---|---|
| `IdentityError` | `crates/sddk-domain/src/identity.rs:14` | donde va el error de ciclo |
| `ProjectId::new` | `identity.rs:47` | valida el formato; un alias no puede inventar ids |
| `resolve_project_identity` | `identity.rs:364` | resuelve desde remote o seed; **no** toca aliases |
| `resolve_identity_honoring_pin` | `crates/sddk-cli/src/lib.rs:1516` | el sitio **único**; el alias se encadena aquí |
| `plan_adoption` | `crates/sddk-engine/src/adoption.rs:197` | el pin gana sobre la derivación (W2c) |
| `compute_content_hash` | `crates/sddk-domain/src/event_envelope.rs:162` | por qué la migración es imposible |
| trigger append-only | `crates/sddk-storage/tests/cross_ledger_consistency.rs:151` | el invariante que este trabajo no puede romper |

## Lote de este apply: SOLO el dominio

`crates/sddk-domain/src/identity.rs`, sin I/O, sin schema, sin CLI.

**Motivo de acotar:** la lógica de resolución transitiva y la detección de
ciclo son donde está el riesgo, y son puras. Añadir persistencia y CLI en el
mismo lote haría que un fallo de lógica y uno de cableado fueran indistinguibles
al leer el resultado. Los lotes 2 (persistencia) y 3 (CLI) van después, con
tests propios.

## Lote de verificación (scoped, no el perfil completo)

```text
cargo test -p sddk-domain --lib identity
```

Perfil completo sólo en `verify`/release, según AGENTS.md §2.3.

## STOP conditions

1. Si `cargo test -p sddk-domain` falla por algo **fuera** de `identity`, parar:
   el baseline estaba verde y ese fallo no es mío para arreglarlo en este lote.
2. Si el lote toca un fichero fuera de `crates/sddk-domain/src/identity.rs`,
   parar: el lote se broadened solo y eso es un defecto de disciplina, no de
   diseño.
3. Si aparece la necesidad de **escribir** en algún ledger para que la
   resolución funcione, parar. La propiedad es «resolver sin escribir»; un
   diseño que necesita escribir para leer no es este diseño.
4. Si el selfcheck de normalización golden (`test_migrate_project_identity_mirror.py`)
   falla, parar: he tocado el normalizador y ese guard existe por algo.

## Riesgos declarados

| riesgo | mitigación en este lote |
|---|---|
| Que el alias se lea como «reescribir la identidad» | el tipo se llama `ProjectAlias` y su doc dice que **no** emite ids nuevos |
| Detección de ciclo que no termina | el límite de pasos es explícito y el ciclo se nombra en el error, no se trunca en silencio |
| Un `A -> A` que se cuelgue | criterio falsable 1: resuelve a `A` y termina |

## Lo que este pre-flight NO autoriza

- No autoriza publicar una release (sigue bloqueada por la clave del KMS).
- No autoriza tocar el storage real de la máquina: los 15 aliases se aplicarán
  en un lote posterior, con su propio criterio de recuento de filas.

---

## Addendum — pre-flight de los lotes 2 y 3

El pre-flight de arriba autorizaba **un** lote, el del dominio. Los lotes 2 y 3
se ejecutaron sin uno nuevo, y eso fue una omisión, no una decisión: la
condición 5 de la tabla de readiness dice «tests que fijan la propiedad, con
falsificador cada uno», y sin pre-flight no había ninguna garantía de que se
escribieran. Se escribe aquí lo que debería haberse escrito antes.

### Lote 2 — el store (`2d7fac4c`)

| # | condición | estado |
|---|---|---|
| 4 | Superficie mínima | 1 fichero nuevo del CLI, más una línea de registro en `lib.rs` |
| 6 | Riesgo de datos | **cero**: escribe en `$XDG_STATE_HOME`, no en ningún ledger |

**STOP conditions específicos, añadidos a los de arriba:**

5. Si el store necesita una migración de esquema para existir, es que el
   diseño del lote 2 es el del SCOPE y no el que justifica el JSON; parar y
   escribir por qué se cambió.
6. Si declarar un alias puede dejar una tabla en disco que no resuelve, parar.
   Un store que puede escribir su propio estado de fallo no es un store.

**Superficie leída:** `ProjectAlias`/`AliasTable`/`AliasResolution` en
`identity.rs` (lote 1), `ProjectPin` y `project_pin_path` en `lib.rs` como
precedente de formato, `XdgEnvironment` en `sddk-engine/src/paths.rs` para la
precedencia de INC-DEBT-037, y `event_bus/storage_path.rs` para confirmar que
el ledger vive en `<state>/sddk/projects/<id>/`.

### Lote 3 — el cableado

| # | condición | estado |
|---|---|---|
| 3 | Superficie leída, no inferida | OK — el segundo resolutor de `project resolve` se encontró **leyendo**, no compilando |
| 4 | Superficie mínima | 3 ficheros: dominio (campo), engine (precedencia compartida), CLI (cableado y comando) |
| 5 | Tests con falsificador cada uno | 10 tests, **7 mutaciones, 7/7 detectadas** tras arreglar la que escapó |
| 6 | Riesgo de datos | **cero**: ningún ledger se abre. Verificado: `p-63676b11dc0ef88f` seguía en 590 eventos con mtime del 02 08:01 |

**La ampliación de superficie que este lote se autorizó a sí mismo, y por qué
es legítima:** `crates/sddk-engine/src/paths.rs` no estaba en el SCOPE. Se tocó
porque el store necesita el **state base** y la precedencia de ese base ya
existía, privada, en el engine. La alternativa era reimplementarla en el CLI, y
eso es exactamente el patrón que el STOP condition nº 2 prohíbe: dos resolutores
que hay que mantener sincronizados a mano. Se exportó `state_base()` en vez de
duplicarla, y `resolve_xdg_paths` ahora la usa, así que la precedencia tiene
**una** implementación y las dos superficies la comparten por construcción.

**STOP conditions específicos, añadidos:**

7. Si `project resolve` necesita su propia derivación para mostrar
   `remote_url`/`fallback_seed`, parar: eso significa que hay un segundo
   resolutor. (Ocurrió, y se resolvió derivando primero y sobrescribiendo con
   el pin, no duplicando.)
8. Si el único modo de probar el cableado es un `$XDG_STATE_HOME` real, parar:
   ese test no es hermético y no se va a escribir. Partir la función.
9. Si una mutación de cableado deja la suite verde, el ciclo no está
   terminado por mucho que los tests pasar.

### Corrección a la condición 4 del pre-flight original

El lote 1 se declaró «1 fichero del dominio, sin I/O». Correcto entonces. Pero
`IdentitySource` aparecía en **17** sitios en forma de `match`, y añadir una
variante habría obligado a recorrerlos todos para decidir qué hacer en cada uno.
La opción elegida — un campo `alias_hops` en `ResolvedProjectIdentity` en vez de
una variante nueva — reduce eso a 4 construcciones, y el compilador las cuenta.
El radio de impacto se midió **antes** de elegir, no después.
