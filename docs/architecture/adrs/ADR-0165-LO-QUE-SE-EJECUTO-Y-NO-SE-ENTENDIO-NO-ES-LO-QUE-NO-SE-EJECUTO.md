---
id: ADR-0165-LO-QUE-SE-EJECUTO-Y-NO-SE-ENTENDIO-NO-ES-LO-QUE-NO-SE-EJECUTO
title: Lo que se ejecutó y no se entendió no es lo que no se pudo ejecutar, y una declaración no es un cálculo
status: accepted
proposed_at: 2026-10-05
accepted_at: 2026-10-05
cycle: p-63676b11dc0ef88f/version-coherence-071
accepted_by_cycle: p-63676b11dc0ef88f/version-coherence-071
supersedes: null
superseded_by: null
extends: [ADR-0164, ADR-0163, ADR-0157]
component: release
surface: crates/sddk-gateway/src/version_provider.rs, crates/sddk-domain/src/version_authority.rs
closes: []
---

# ADR-0165 — Lo que se ejecutó y no se entendió no es lo que no se pudo ejecutar

- **Status:** `accepted` (2026-10-05)
- **Extiende:** ADR-0164 (el dialecto ata las cinco caras), ADR-0163 (preguntar al
  build tool), ADR-0157 (la autoridad de versión)
- **Superficie:**
  - `crates/sddk-gateway/src/version_provider.rs` — `declared_version`,
    `DeclaredAuthorityProvider::observe`, `TargetSet::root_refusal`, `refusal_legible`
  - `crates/sddk-domain/src/version_authority.rs` — `ProviderError::Malformed`
- **Ciclo:** `cl-project-declares-its-version`

---

## Contexto

Dos decisiones en un bloque, porque la primera **produjo** la segunda.

### La primera: la cobertura de ecosistemas es una tabla de trece filas

`DEFAULT_DECLARATIONS` (`version_provider.rs:129-243`) dice qué ficheros sabe leer
SDDK. Quien no aparezca ahí no publica, y eso es conocimiento de una herramienta por
cada ecosistema que aparezca: una fila más es trabajo que hay que repetir, y trabajo
que hay que repetir se queda viejo.

Existe una vía que debería librar de la tabla: que el proyecto declare su versión
en `.sddk/version-source.json`. MEDIDO, su contrato entero aceptaba **exactamente
dos cosas**:

| `authority` | respuesta |
|---|---|
| `"tag"` | `ReleaseRefIsAuthority` |
| cualquier otra | `Err` — *«este build entiende "tag"»* |

No había forma de declarar un valor de versión. Solo declarar que la versión va en
el tag.

### MEDIDO 1 — un proyecto fuera de la tabla no publica

`/tmp/agnos-puro`: un `build.sbt` (Scala, no está en la tabla) y
`.sddk/version-source.json` con `{"authority":"version","version":"4.2.0"}`.

```
$ sddk release version inspect --root .
error: VERSION TARGET ERROR: no release target found under ..
```

### MEDIDO 2 — y una declaración buena la tapa

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

`Invalid` manda sobre `Declared` — correcto, es la ley del reducer: un fichero que
existe y no se entiende no se ignora en silencio. Pero aquí el fichero es legible y
válido: declara una forma que el build no conoce. El resultado es que un proyecto
con la versión perfectamente declarada en `Cargo.toml` **no publica**, y la culpa la
lleva un fichero que el proyecto escribió para ser más explícito.

**La vía que existe para no depender de la tabla es la que tapa la tabla.**

### La segunda: el mensaje dice el hecho contrario

Al escribir el falsador apareció esto. Los doce sitios que construyen
`ProviderError::Unavailable` producen este texto:

```
el provider sddk.gateway/.sddk/version-source.json no se pudo ejecutar:
.sddk/version-source.json declara authority "bogus"; este build entiende "tag"
```

El fichero **sí se leyó, sí se interpretó y sí respondió**. Lo que pasó es que su
respuesta no cabe en lo que este build entiende. Y el texto dice que no se pudo
**ejecutar**, que es el hecho contrario.

Cuatro de esos doce sitios son el caso invertido: JSON con sintaxis rota,
`schema_version` a futuro, `authority` inexistente, `version` que no parsea.

---

## Decisión

### 1. El proyecto declara su versión

`.sddk/version-source.json` acepta una **segunda** forma, y solo una más:

```json
{ "schema_version": 1, "authority": "version", "version": "4.2.0" }
```

→ `VersionProbe::Declared`, con evidencia
`declaracion-del-proyecto/.sddk/version-source.json`, `location` la ruta y
`digest: None`.

**Y nada más.** Escrito para que no se expanda por inercia:

- **No gana a la tabla.** Es una fuente más y el reducer decide. Coincidir →
  `CrossValidated`; discrepar → `Conflict` y puerta cerrada nombrando **las dos**
  declaraciones. La declaración del proyecto no es una autoridad sobre las demás:
  es otra forma de decir lo mismo, y si dice otra cosa es una contradicción.
- **No se toca `DEFAULT_DECLARATIONS`.** Añadir filas sigue siendo el mecanismo
  para los ficheros que SDDK ya sabe leer. Este bloque hace que la lista deje de
  ser la única puerta, no que desaparezca.
- **No hay `default`.** Sin `authority` se falla, como hoy; con `authority:
  "version"` sin `version` se falla, y el motivo dice cuál de las dos falta.

### 2. `Malformed` es un hecho distinto de `Unavailable`

La frontera es una pregunta y se responde sin ambigüedad:

> **¿Pudimos leer lo que el provider leyó?**

| respuesta | variante |
|---|---|
| no — binario ausente, fichero ilegible por I/O | `Unavailable` (la que ya dice la verdad) |
| sí, y no lo entendimos | `Malformed` (nueva) |

`Unavailable` **no se toca ni se deprecia**: su semántica es cierta y es la del
binario que no se pudo lanzar, que es el caso que el binario ausente verifica.

**Por qué esto no es cosmético.** Es la misma clase que el banner de Gradle que VA9
corrigió (`FAILURE: Build failed with an exception.` era cierto para todos los
fallos de Gradle que habían ocurrido jamás, y por eso no distinguía ninguno). Un
mensaje cierto para todos los fallos habidos no distingue ninguno, y cuando los dos
hechos que confunde piden reparaciones **opuestas** —comprobar el binario cuando el
problema está en el fichero, o al revés— manda al operador al sitio equivocado.

### 3. El motivo que se calcula y se tira no puede comprobarlo el lector

`release_targets` recibía el `Invalid` de la raíz con sus motivos y **los miraba y
los tiraba** para decidir solo si había versión. Con `schema_version: 99` la
respuesta era `la raiz no declara version`, y eso es **falso**: la raíz sí declara
versión, declara una que este build no sabe leer, y el mensaje mandaba a buscar un
fichero que no faltaba.

`TargetSet` gana `root_refusal: Option<String>`. `None` es el silencio de verdad
—nadie declaró— y ahí el mensaje antiguo era cierto.

---

## Consecuencias

**Lo que cambia para el usuario.** Un proyecto de cualquier tecnología publica con
un fichero de seis líneas. Un repo con tabla **y** declaración que coincidan queda
`CrossValidated`. Uno que discrepe falla cerrado nombrando las dos.

**Lo que no cambia.** `reduce()` no se toca y ninguna ley suya cambia. La puerta de
`ensure_release_ref_lockstep` no se toca. `DEFAULT_DECLARATIONS` sigue decidiendo
quién publica **si el proyecto no se declara**.

**Coste aceptado.** Un `Invalid` sigue tapando una `Declared` buena, y eso es
correcto: es la ley del reducer y este provider no la sortea. Lo que cambia es que
ahora un `Invalid` **por forma no entendida** es un `Malformed` con su mensaje
cierto, y el operador sabe si debe mirar el binario o el fichero.

---

## Lo que queda declarado y no abierto

**Un repo con `build.sbt`, sin declarar nada, y Scala que SDDK no conoce, hoy no
publica.** Es un defecto real y queda escrito aquí a propósito: arreglarlo exige una
decisión de **política** —¿debe publicar como `Undeclared`?— y hay argumentos en las
dos direcciones. Un provider que acepta cualquier cosa no evita el problema: lo
esconde cambiando qué se responde.

---

## Alternativas descartadas

**Ampliar `DEFAULT_DECLARATIONS` con `build.sbt`, `composer.json`, `mix.exs`,
`pubspec.yaml`, `Gemfile`, `*.csproj`, `meson.build`.** Funciona, y hay que
repetirlo por cada ecosistema que aparezca. Una declaración en
`.sddk/version-source.json` funciona para cualquier proyecto, incluido uno escrito
esta semana, y no requiere que SDDK sepa nada de él.

**Un parser que acepte cualquier `authority`.** No evita el problema de tapar la
declaración buena: lo esconde cambiando qué se responde, y el `Invalid` sigue
dominando.

**Que la declaración gane a la tabla.** Haría que un fichero de texto en la raíz
—anexo, sin firmar, editable— tenga más peso que el fichero de build que el
proyecto compila. Y una contradicción entre los dos dejaría de ser visible.

**Arreglar solo el mensaje sin partir `Unavailable`.** Habría que distinguir los
cuatro casos invertidos dentro de un texto que dice lo contrario en los otros ocho.
Un mensaje con dos significados opuestos es peor que dos mensajes.