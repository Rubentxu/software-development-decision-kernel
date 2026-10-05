# PRE-FLIGHT — VA11 `cl-project-declares-its-version`

**Cycle:** `p-63676b11dc0ef88f/version-coherence-071`
**Fecha:** 2026-10-05
**Predecesor:** VA10 `2387c579` (ADR-0164), cerrado en `97dd9293`

---

## Readiness: READY

---

## Qué falta, medido

VA10 cerró la capacidad de preguntar al build tool y dejó escrito un hueco que
este bloque recoge: la cobertura de ecosistemas es una **tabla de 13 filas** en
`DEFAULT_DECLARATIONS` (`version_provider.rs:129-243`), y quien no aparezca ahí
no publica.

Hay una vía que debería librar de esa tabla —la declaración del propio proyecto
en `.sddk/version-source.json`— y **está a medias**. MEDIDO, su contrato entero
(`version_provider.rs:550-574`) acepta exactamente dos cosas:

| `authority` | respuesta |
|---|---|
| `"tag"` | `ReleaseRefIsAuthority` |
| cualquier otra cosa | `Err` — *«este build entiende "tag"»* |

**No hay forma de que un proyecto declare un valor de versión.** Solo declarar
que su versión va en el tag.

### MEDIDO 1 — un proyecto fuera de la tabla no publica

`/tmp/agnos-puro`: un `build.sbt` (Scala, no está en la tabla) y
`.sddk/version-source.json` diciendo `{"authority":"version","version":"4.2.0"}`.

```
$ sddk release version inspect --root .
error: VERSION TARGET ERROR: no release target found under ..
```

La declaración del proyecto es **ignorada**, no rechazada: `authority: "version"`
cae en la rama `Err`, que el reducer trata como `Invalid`… salvo que el fichero
no llegue a leerse porque no hay target.

### MEDIDO 2 — y una declaración que sí es buena la tapa

`/tmp/agnos-rust`: `Cargo.toml` con `version = "4.2.0"` **y** el mismo JSON.

```
providers_answering: sddk.gateway/.sddk/version-source.json,
                     sddk.gateway.declaration-file/Cargo.toml
  - sddk.gateway/.sddk/version-source.json [Invalid] el provider no se pudo
    ejecutar: .sddk/version-source.json declara authority "version"; este build
    entiende "tag"
authority: Invalid
productVersion: none
```

`Invalid` manda sobre `Declared` —correcto, es la ley del reducer: un fichero que
existe y no se entiende no se ignora—. Pero aquí el fichero es **legible y
válido**: lo que pasa es que declara una forma que el build no conoce. El
resultado es que un proyecto con la versión perfectamente declarada en
`Cargo.toml` **no publica**, y la culpa la lleva un fichero que el proyecto
escribió para ser más explícito.

Eso es un defecto de la capacidad, no una política: la vía que existe para no
depender de la tabla es la que tapa la tabla.

---

## Decisión

`.sddk/version-source.json` acepta una **segunda** forma, y solo una más:

```json
{ "schema_version": 1, "authority": "version", "version": "4.2.0" }
```

→ `VersionProbe::Declared`, con evidencia
`declaracion-del-proyecto/version-source.json`, `location` la ruta y `digest:
None` — **la misma convención que las trece filas de la tabla**, porque el
contrato de `SingleDeclarationProvider` ya decidió eso y una declaración en
fichero no es una excepción.

**Y nada más.** Las tres cosas que este bloque NO hace, escritas para que no se
expandan por inercia:

- **No gana a la tabla.** Es una fuente más, y el reducer decide. Un proyecto
  con `Cargo.toml` en 4.2.0 y el JSON en 4.2.0 queda `CrossValidated`; con 4.3.0
  queda `Ambiguous` y falla cerrado. **La declaración del proyecto no es una
  autoridad sobre las demás: es otra forma de decir lo mismo, y si dice otra
  cosa es una contradicción.**
- **No se toca `DEFAULT_DECLARATIONS`.** Añadir filas sigue siendo el
  mecanismo para los ficheros que SDDK ya sabe leer. Este bloque no amplía esa
  lista; hace que la lista deje de ser la única puerta.
- **No hay `default`.** Sin `authority` se falla, como hoy; con `authority:
  "version"` sin `version` se falla, y el motivo dice cuál de las dos falta.

## Por qué esto y no arreglar la tabla

Porque son dos arreglos distintos y este es el que no depende de que SDDK sepa
qué tecnología usa nadie.

Una fila más en la tabla es conocimiento de una herramienta más: funciona, y
hay que repetirlo por cada ecosistema que aparezca. Una declaración en
`.sddk/version-source.json` funciona para **cualquier** proyecto, incluido uno
escrito esta semana, y no requiere que SDDK sepa nada de él.

Lo que queda después de este bloque —la tabla sigue decidiendo quién publica
**si el proyecto no se declara**— es un defecto real, y queda escrito en el
CLOSURE como el siguiente. No se esconde, porque arreglarlo exige una decisión de
política que este bloque no tiene por qué tomar solo: ¿qué pasa con un repo que
tiene `build.sbt`, no declara nada, y SDDK no conoce Scala? Hoy no publica. Hay
argumentos para que no publique y argumentos para que publique como
`Undeclared`, y **elegir es política**.

---

## Riesgo de datos

**Cero.** El fichero se lee igual; lo que cambia es qué se acepta de él y qué
se responde. Ningún fichero del proyecto se escribe.

## STOP conditions

1. Que hiciera falta tocar el reducer. No: `Declared` y `Ambiguous` ya existen.
2. Que hubiera que tocar el motor. No: es un `VersionProbe` más.
3. Que un proyecto que hoy publica dejara de publicar. Eso sí se mide, en las dos
   direcciones: la tabla intacta y `authority: "tag"` intacto son la condición.

## El riesgo de este bloque, escrito antes de empezar

El modo de fallo no es que un proyecto nuevo no publique. Es **que uno que hoy
publica deje de hacerlo**, porque una rama nueva del parser cambia el resultado
de una lectura que ya funcionaba.

La forma de vigilarlo es en las dos direcciones: una tabla de verdad donde cada fila sea
un repo real con lo que tiene hoy, y el mismo repo con la declaración añadida.
Si algo que antes publicaba deja de publicar, eso es un rojo aunque todas las
filas nuevas sean verdes.

## El otro riesgo, que es el de este bloque entero

Un fichero que el proyecto escribe para **ser más explícito** no puede dejar de
ser legible por no caber en lo que el build entendía. MEDIDO 2 es exactamente
eso. Y su generalization es incómoda: cada forma que el proyecto pueda escribir
y el build no entienda produce un `Invalid` que tapa lo que sí funcionaba, luego
el coste de añadir una forma no aceptada es **tapar la declaración buena**.

Ese es el motivo de que el error de `authority` desconocido siga siendo error
—eso es un error de verdad del proyecto— y de que `authority: "version"` **de
declare**. Un Parser que acepta cualquier cosa no evita el problema: lo
esconde cambiando qué se responde.

---

## Checkpoint — cambio de premisa, 2026-10-05, durante la implementación

Al escribir el falsador aparecieron dos fallos y **los dos eran
descubrimientos**, no aserciones mal escritas.

### El primero, mío: probaba la función equivocada

`e5` pedía que un `authority` desconocido dijera cuáles formas sí existen, y
llamó a `declared_version`, que **no lee `authority`** — esa decisión vive en el
`match` de `observe`, que necesita disco. Una rama no puede medir el dispatcher
que la elige, igual que `d5`/`d6` en VA10 no podían medir que el dialecto usara
su parser.

**Corrección:** `e5` va por el provider real, con `TempDir`. La lección es la
misma de entonces: dos llamadas que parecen iguales miden cosas distintas, y un
test que exercise la interior creyendo que exercise la puerta.

### El segundo, real: `ProviderError::Unavailable` miente

MEDIDO. Los doce sitios que construyen `Unavailable` producen este texto:

```
el provider sddk.gateway/.sddk/version-source.json no se pudo ejecutar:
.sddk/version-source.json declara authority "bogus"; este build entiende "tag" o "version"
```

El fichero **sí se leyó, sí se interpretó y sí respondió**. Lo que pasó es que su
respuesta no cabe en lo que este build entiende. Y el texto dice que no se pudo
**ejecutar**, que es el hecho contrario.

Es la misma clase que el banner de Gradle que VA9 corrigió:
`FAILURE: Build failed with an exception.` era cierto para todos los fallos de
Gradle que habían ocurrido jamás y por eso no distinguía ninguno. Aquí
`no se pudo ejecutar` es cierto para todos los `Unavailable` que han ocurrido
jamás, y **cuatro de ellos son "se ejecutó y su respuesta no se entiende"**, que
es lo contrario. Un mensaje que no distingue dos hechos opuestos manda al
operador a la reparación equivocada: a comprobar el binario cuando el problema
está en su fichero.

**Decisión, con su criterio:** añadir una variante `Malformed` a `ProviderError`.
La frontera es una pregunta y se responde sin ambigüedad:

> ¿Pudimos leer lo que el provider leyó?

| respuesta | variante |
|---|---|
| no — binario ausente, fichero ilegible por I/O | `Unavailable` (la que ya dice la verdad) |
| sí, y no lo entendimos | `Malformed` (nueva) |

`Unavailable` **no se toca**: su semántica es real y la usa el caso del binario
ausente, que es el que `b4` ya verifica.

**Alcance MEDIDO antes de decidir:** 12 sitios que construyen `Unavailable`, 6
textos citados en 4 ficheros, **ningún match exhaustivo** sobre el enum. El
cambio de superficie es acotado y no obliga a mover la frontera del reducer.

**STOP condition revisada:** el bloque decía «que hiciera falta tocar el reducer,
no». Esto **no** toca el reducer — `reduce()` no cambia y ninguna ley suya cambia
— pero sí añade una variante de error, que no estaba prevista. Se declara aquí en
vez de descubrirlo en el commit.
