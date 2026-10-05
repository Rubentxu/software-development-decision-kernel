# Cierre de `cl-project-declares-its-version` (VA11)

**Fecha:** 2026-10-05
**Ciclo:** `p-63676b11dc0ef88f/version-coherence-071` · ADR-0165
**Estado:** cerrado y publicado

---

## Qué mide esto

VA10 cerró **preguntar** al build tool y dejó escrito un hueco en su propio cierre:
la cobertura de ecosistemas es una tabla de trece filas, y quien no aparezca ahí no
publica.

Este bloque cierra el hueco de la otra manera. La tabla no se amplía —**ampliarla
es conocimiento de una herramienta más, y hay que repetirlo por cada ecosistema
que aparezca**—. Lo que se hace es que un proyecto pueda **declarar su versión** en
un fichero de seis líneas y publicar, sin que SDDK sepa qué tecnología usa.

---

## Las dos mediciones previas

| Medición | Resultado | Qué obligó |
|---|---|---|
| `/tmp/agnos-puro`: `build.sbt` (Scala, fuera de la tabla) + `{"authority":"version","version":"4.2.0"}` | `VERSION TARGET ERROR: no release target found` | La declaración del proyecto **no era** una vía: era decorativa |
| `/tmp/agnos-rust`: `Cargo.toml` en 4.2.0 **y** el mismo JSON | `authority: Invalid`, `productVersion: none` | Una forma no entendida **tapaba** la declaración buena |

La segunda es la grave, y es la que este bloque nombra: **un fichero que el proyecto
escribe para ser más explícito no puede dejar de ser legible por no caber en lo que
el build entendía.** La vía que existe para no depender de la tabla era la que tapaba
la tabla.

---

## El defecto que encontró la implementación, y era el tercero

Al escribir el falsador se midió que los doce sitios que construyen
`ProviderError::Unavailable` dicen:

```
el provider sddk.gateway/.sddk/version-source.json no se pudo ejecutar: ...
```

El fichero **sí se leyó, sí se interpretó y sí respondió**. Cuatro de esos doce
sitios —JSON roto, `schema_version` a futuro, `authority` inexistente, `version` que
no parsea— son el **caso contrario** del texto que emiten.

Es la misma clase que el banner de Gradle que VA9 corrigió. Un mensaje cierto para
todos los fallos habidos no distingue ninguno, y aquí los dos hechos que confunde
piden reparaciones **opuestas**: a comprobar el binario cuando el problema está en el
fichero, o al revés.

**Decisión, con su criterio:** `ProviderError::Malformed`, y la frontera es una
pregunta que se responde sin ambigüedad — *¿pudimos leer lo que el provider leyó?*
`no` → `Unavailable`; `sí, y no lo entendimos` → `Malformed`. `Unavailable` no se
toca: su semántica es cierta para el binario ausente, y ese caso ya está verificado.

---

## El cuarto defecto, y era del código publicado

`release_targets` recibía el `Invalid` de la raíz **con sus motivos**, y los miraba
y los tiraba para decidir solo si había versión. Con `schema_version: 99` contestaba
`la raiz no declara version`, y eso es **falso**: la raíz sí declara, declara una que
este build no sabe leer, y el mensaje mandaba a buscar un fichero que no faltaba.

Un hecho que se calcula y se descarta es un hecho que el lector no puede comprobar.
`TargetSet` gana `root_refusal`, y `None` queda siendo el silencio de verdad.

---

## Estado final medido

| Proyecto | Antes | Después |
|---|---|---|
| Scala, `build.sbt`, declara 4.2.0 | `no release target found` | **publica**, con evidencia `declaracion-del-proyecto/.sddk/version-source.json` |
| Rust, `Cargo.toml` 4.2.0 + declara 4.2.0 | `Invalid`, no publica | **`CrossValidated`**, publica |
| Rust, `Cargo.toml` 4.2.0 + declara 4.3.0 | (no había vía) | **`Conflict`**, puerta exit 1 nombrando **las dos** declaraciones |
| Rust, `Cargo.toml` 4.2.0, `schema_version: 99` | `Invalid` + `no se pudo ejecutar` | **`Malformed`**, y el mensaje dice que la raíz sí declara |
| `authority: "tag"` | `ReleaseRefIsAuthority` | **intacto** — no se rompió |
| Tabla sola, sin declarar | `Declared` de la fila | **intacto** — declararse no es obligatorio, y eso cuesta |
| Maven, `pom.xml`, sin declarar | `declared at ./pom.xml` con `help:evaluate` | **intacto** — el dialecto de VA10 sigue siendo el que se ejecuta |

Lo que se conserva de VA10 también: leer la declaración **no escribe en el árbol**.

---

## Los falsadores, y lo que encontró cada uno

**Gateway — `crates/sddk-gateway/tests/version_project_declaration.rs`, 12/12.**

`e1`–`e12` están escritos para caer por su propia comprobación. La regla que se
siguió: una mutación que no muta es `SKIP`, nunca `PASS`.

| Falsador | Qué mide | Resultado |
|---|---|---|
| **M1** | versión ilegible → `Undeclared` | cae `e4` + `e12` |
| **M2** | `Malformed` vuelve a decir «no se pudo ejecutar» | cae `e12` + `e3` |
| **M3** | declarar pone `digest` | cae `e2` |
| control | el resto | **12/12** |

**CLI — `crates/sddk-cli/tests/release_version_project_declares.rs`, 8/8.**
End-to-end sobre el binario, midiendo **la puerta** (`release plan`, exit ≠ 0) y no
el informe (`matches`, `inspect`, exit 0).

---

## El riesgo que el propio `PRE-FLIGHT` dejó escrito antes de empezar

El modo de fallo anunciado no era que un proyecto nuevo no publicara. Era **que uno
que hoy publica dejara de hacerlo**, porque una rama nueva del parser cambia el
resultado de una lectura que ya funcionaba.

Se vigiló en las dos direcciones, y ahí es donde está la parte incómoda: el riesgo
resultó ser **el contrario** del escrito, y aun así lo cubre la misma tabla. Un
proyecto con tabla **y** declaración que coinciden tiene que quedar `CrossValidated`
—una redundancia no es una validación— y uno que discrepa tiene que **fallar
cerrado**, no elegir la que le da la gana al fichero de texto.

Un test que solo mira «publica / no publica» habría dado verde a las dos Diseases.

---

## Una lección del propio falsador

El primer `e5` llamaba a `declared_version`, que **no lee `authority`** — esa decisión
vive en el `match` de `observe`, que necesita disco. Una rama no puede medir el
dispatcher que la elige, igual que `d5`/`d6` de VA10 no podían medir que el dialecto
usara su parser.

**Es la tercera vez en cuatro bloques que un test falsador encuentra en sí mismo lo
que la inspección no.** El patrón: cuando un test pasa a la primera, hay que mirar
qué está midiendo antes de celebrate.

---

## Deuda declarada, no abierta

- **La tabla sigue decidiendo quién publica si el proyecto no se declara.** Un repo
  con `build.sbt`, sin declarar nada, y Scala que SDDK no conoce, **hoy no publica**.
  Es un defecto real: arreglarlo es una decisión de **política** —¿publica como
  `Undeclared`?— y hay argumentos en las dos direcciones. No se toma sola aquí.
- **`release version matches` y `release handoff` no preguntan al build tool.** Sus
  `Args` no declaran `--evaluate-build`.
- **Seis de los siete targets built-in no tienen cuerpo** (`target_cmd.rs:112-114`
  solo cablea `change`).
- **`AdapterFact` / `EvidenceResolver`**: 532 líneas de producción sin consumidor, con
  `#[allow(unused)]` y un `exit=` sin cumplir.
- **Cero observabilidad estructurada** (`INC-AUDIT-S14-NO-STRUCTURED-LOGGING`).
- **CI**: 5 de 5 workflows son `workflow_dispatch`, y `release.yml` publica sin tests.

Nada de eso se resuelve en este bloque, y ninguno lo bloquea.

---

## Un texto que se corrigió de paso

`suiteFallaron` estaba escrito así — dos palabras pegadas — en un doc-comment de
**VA10 ya publicado** (`2387c579`). El guard de contaminación del repo no lo ve:
mira CJK y cirílico real, y esto era latino. Corregido a «una suite fallo asi».

El guard de StuckTogether que falta está declarado en el mismo sitio que el resto de
la deuda de calidad.