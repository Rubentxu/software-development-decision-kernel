# PRE-FLIGHT — VA14 `cl-el-guard-que-solo-miraba-un-modulo`

## Intención

La ley «el núcleo define las preguntas; los providers saben obtener la evidencia»
la vigila **un** fitness sobre **un** módulo de **cuarenta y seis**. Este bloque
extiende la vigilancia a todo el dominio y, de paso, repara la única violación
real que aparece al extenderla.

## CORRECCION A LOS PUNTOS 2 Y 3 DE ABAJO — MEDIDA, NO OPINADA

Este `PRE-FLIGHT` se escribió diciendo «escaneé los 46 con el mismo vocabulario
del guard». **Es falso, y la frase es la que importa.** La criba real fue un
`grep` de **seis** entradas (`cargo`, `gradle`, `npm`, `go.mod`,
`package.json`, `bazel`) cuando el guard usa **treinta y cuatro**, que incluyen
los nombres de lenguaje.

El efecto fue medir «no hay nada» con un instrumento más estrecho que el que se
acabaría usando, y **declarar una infraccion donde hay cinco**:

| lo que este PRE-FLIGHT afirma | lo medido al escribir el guard |
|---|---|
| 3 módulos nombran tecnología | **4**: `macros.rs`, `test_adapters.rs`, `test_model.rs`, `test_ports.rs` |
| 42 ocurrencias | **51** |
| «de esas 42, exactamente UNA viola la ley» | **nueve, en cinco líneas, todas doc-comments de producción** |

Las ocho que faltaban son nombres de lenguaje (`rust`, `typescript`, `python`)
escritos con mayúscula inicial, y **`Cargo.toml`** con mayúscula inicial. Ninguna
estaba en la criba de seis.

La lección no es «usa el `grep` bueno», que es lo de Perogrullo: es que **un
recuento que sale suspiciously limpio con un instrumento parcial no es un
recuento, es la ausencia de una medición**. Cuando el número que sale es «1» y
el dominio tiene cuarenta y seis módulos, la respuesta honesta es «todavía no lo
he mirado bien», y por eso este bloque no empezó a escribir el guard hasta
después de hacerlo bien.

El resto del `PRE-FLIGHT` —el atajo descartado, la estructura de zonas, el
riesgo de la exclusión— se sostiene: se comprobó contra el guard escrito, y el
riesgo del `#[cfg(test)] use` resultó **peor** de lo previsto, porque la segunda
defensa que tiene el detector lo hace no observable en el corpus real.

---

## Lo que estaba medido cuando se escribio esto

**1. El guard cubre 1 de 46.**

```
crates/sddk-domain/src/*.rs            → 46 módulos
crates/sddk-domain/tests/version_authority_fitness.rs:110
    let source = include_str!("../src/version_authority.rs")...
```

Un `include_str!` con una ruta literal. Los otros 45 módulos no los mira nadie.

**2. Escané los 46 con el mismo vocabulario del guard.** Tres módulos nombran
tecnología concreta, y los tres son módulos de producción —`test_adapters`,
`test_apply`, `test_model` son sobre *selección de tests*, declarados en
`lib.rs`, no dobles de `#[cfg(test)]`—:

| módulo | ocurrencias | dónde |
|--------|-------------|-------|
| `test_adapters.rs` | 30 | 1 antes del módulo de test, 29 después |
| `test_apply.rs` | 6 | todas después |
| `test_model.rs` | 6 | todas después |

**3. De esas 42, exactamente UNA viola la ley** en el sentido que VA13 estableció:

```rust
// crates/sddk-domain/src/test_adapters.rs:47
/// Path to the ecosystem manifest file (non-empty, e.g. "Cargo.toml").
```

Es **el mismo defecto** que VA13 corrigió en `version_authority.rs`: un
doc-comment que nombra la herramienta concreta para explicar el campo. Aquí
además es másographedebilitante, porque el campo se llama `manifest_path` y la
frase ya dice «the ecosystem manifest file» — el nombre de la herramienta **no
añade nada** y solo ata el núcleo a un ecosistema.

**4. Las otras 41 son legítimas, y la razón es estructural.** Están dentro de
`#[cfg(test)]` y construyen el input rechazado o el fixture:

```rust
// test_apply.rs:1276 — el test prueba que el comando NO entra en la sesión
"cargo test", "npm test", "gradle test", "cargo build", "npm run",
```

Nombrar lo prohibido **en el test que lo prohíbe** es lo mismo que citar la
contaminación en el changelog: el nombre está puesto por una razón, y quitarlo
deja el test sin nada que afirmar.

**5. La zona se puede medir sin parsear Rust.** La estructura es limpia:

```
test_adapters.rs   #[cfg(test)] mod tests { en 397-398   hits en 47 (FUERA), 513..875 (DENTRO)
test_apply.rs      #[cfg(test)] en 33 y 626            hits en 1276..1286 (DENTRO)
test_model.rs      #[cfg(test)] mod tests { en 1371    hits en 1711..2182 (DENTRO)
```

**Y el dato que da la ley, no solo la cuenta:** el propio `test_apply.rs` declara
en su cabecera que *«the session holds NO runner commands or ecosystem
strings»*. Es una invariante documentada por el propio módulo, y este bloque la
convierte en algo comprobado en vez de en algo prometido.

## Por qué no es «añadir un fichero a una lista»

Porque **no funciona**. Aplicar el escáner plano de hoy a los 46 módulos daría
**41 rojos falsos**, y un guard que produce rojos falsos entrena a su lector a
ignorarlo — que es la razón por la que el propio guard lleva una nota sobre
ello. El trabajo del bloque no es alcance: es **la distinción que hace que la
ley signifique algo**.

Hay dos zonas y la ley es distinta en cada una:

- **Producción y doc-comments**: nombrar una herramienta **enseña**. El lector
  del núcleo aprende que hay un ecosistema. Viola.
- **`#[cfg(test)]`**: nombrar una herramienta **prueba el rechazo**. No entra en
  el artefacto, no decide nada. No viola.

Ambas son «la palabra aparece». Solo una rompe la ley. Un guard que no
distingue—no puede distinguir— solo puede callarse o gritar.

## Objetivo falsable

1. El fitness escanea **los 46 módulos**, no uno.
2. Distingue la zona de código de test de la de producción **por estructura**
   (`#[cfg(test)]`), no por una lista curada a mano.
3. Reporta `PASS` solo si **cero** ocurrencias fuera de zona de test.
4. Tiene **controles** que demuestren las dos potencias: que ve un nombre fuera
   de zona, y que la exclusión de zona no lo ha vuelto ciego.
5. La única violación real encontrada queda reparada, y el arreglo explica por qué
   ese doc-comment ahora dice por qué no nombra nada.

## El fallo que este bloque puede cometer, y con nombre

**Una exclusión por zona es un agujero con forma de guard.** Si el criterio de
«esto es zona de test» admite un `#[cfg(test)]` pegado a un item de producción,
entonces el guard se puede desconectar **añadiendo una línea**, y quedaría verde
con el veto desenchufado — el mismo fallo que ya se encontró una vez en
`tests/test_uat_naming_boundary_policy.sh`, donde por eso existe un control que
comprueba que el guard puede fallar.

Por eso el punto 4 no es opcional: **un control que el guard puede fallar es lo
que distingue un guard de una exculpación.**

## No-objetivos

- **No** extender a `sddk-gateway`, `sddk-engine` ni `sddk-cli`. Son otra ley con
  otros límites: el gateway **debe** conocer nombres de fichero, porque su
  trabajo es preguntar al build tool. Este bloque es sobre el núcleo.
- **No** ampliar el vocabulario. 41 entradas ya funcionan; añadir más sería
  trabajo sin defecto que lo justifique.
- **No** tocar la deuda que no es esta ley (`AdapterFact`, targets sin cuerpo,
  observabilidad, CI).

## Superficie y riesgo

| | |
|---|---|
| Fichero de código | 1 (el fitness) |
| Fichero de dominio | 1 (`test_adapters.rs`, un doc-comment) |
| ADR | no procede: no hay decisión de diseño nueva, se ejecuta una ley ya escrita |
| Riesgo | **alto en la falsificación, bajo en el código**: un guard mal construido que grita es peor que no tener guard |
| Perfil de prueba | `cargo test -p sddk-domain --test version_authority_fitness` + el resto del crate |

## Riesgo de entorno

El fallo fantasma ya se midió **cinco veces** en los últimos bloques, y la última
fue un instrumento mío corriendo en paralelo. Este bloque **no lanza nada en
paralelo** con ningún release, y su perfil acotado es `sddk-domain` únicamente —
el SUT es el fitness y el doc-comment, no el workspace entero. Si la medición
exige más, se para y se declara.

## Stop conditions

1. Si la detección de zona no puede hacerse de forma **estructural y robusta** —
   si exige un parser de Rust o heurísticas que den falsos positivos — **se para
   y se reporta**. No se publica un guard que adivine.
2. Si aparecen **más** violaciones fuera de zona de test que la única medida, se
   miden todas antes de tocar ninguna; el bloque no se convierte en unamudanza
   de alcance.
3. Si algún control no cae cuando debe, el guard no entra. Un fitness que no
   puede fallar no es un fitness.

## Readiness

**READY.** Todo lo que el bloque afirma está medido sobre el árbol actual
(`HEAD = 78dcaca0`, workspace `2.12.0`, post-release de `v2.12.0`), y lo único
que queda por medir durante el bloque es la salida del propio guard nuevo.

## Hallazgo lateral, que no pertenece a este bloque

`version_authority_fitness.rs:80` dice:

```
/// de compilación. El defecto estaba en el instrumento, no en el código, y se
///icidal aquí antes de que apareciera por azar.
```

«icidal» no es una palabra. Es basura ASCII, introducida por `74c13916`, y **el
escáner de contaminación no la ve** porque ese escáner busca cirílico, CJK, kana,
hangul, fullwidth y emoji — todos no-ASCII. Un guard que cubre una categoría y
deja al lado un hueco contiguo que es su gemelo exacto: ASCII corrupto donde
buscabas no-ASCII corrupto.

Se repara aquí porque es una línea y está en el fichero que este bloque toca. Se
**declara** el hueco —nadie vigila basura ASCII— y abrirlo es otro bloque.