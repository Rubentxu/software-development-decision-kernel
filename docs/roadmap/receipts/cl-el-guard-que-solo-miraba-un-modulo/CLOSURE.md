# CLOSURE — VA14 `cl-el-guard-que-solo-miraba-un-modulo`

## Qué cambió, medido

La ley «el núcleo define las preguntas; los providers saben obtener la evidencia»
la vigilaba **un** fitness sobre **un** módulo de **cuarenta y seis**. Ahora la
vigila `crates/sddk-domain/tests/kernel_purity_fitness.rs` sobre los 46, con
**12 controles**, y `tests/test_kernel_purity_fitness_mutation.sh` le aplica
**11 mutaciones**, cada una con su control.

```
kernel_purity_fitness          12 passed; 0 failed
test_kernel_purity_fitness_mutation.sh   PASS=11 FAIL=0 SKIP=0
sddk-domain completo           1042 passed / 0 failed / 5 ignored
cargo clippy --workspace --all-targets -D warnings   0 errores
cargo fmt --all --check        limpio
shellcheck tests/test_kernel_purity_fitness_mutation.sh   limpio
```

## Las nueve infracciones que encontró al extender el alcance

Mi criba inicial decía «exactamente una». **Era una, y era mentira**, y la
correccion importa mas que el numero: cribi con un vocabulario de **seis**
entradas cuando el guard usa las **treinta y cuatro** reales. Al no incluir los
nombres de lenguaje no vi cinco de las nueve infracciones. **Medir «no hay nada»
con un instrumento más estrecho que el que se acabará usando es medir otra
cosa**, y el nombre que se le da —«una violación»— es el que hace que el resto
del bloque se escriba sobre una base falsa.

| fichero:línea | lo que decía |
|---|---|
| `macros.rs:20` | «Stable on all **Rust** editions ≥ 2018» |
| `test_adapters.rs:45` | «Ecosystem identifier e.g. "**rust**", "**typescript**", "**python**"» |
| `test_adapters.rs:47` | «e.g. "**Cargo.toml**"» |
| `test_model.rs:564` | «(e.g. "**rust**", "**typescript**", "" for neutral)» |
| `test_ports.rs:122` | «"tests/unit/foo.rs" for **Rust**, "test/Foo.test.ts" for **TypeScript**» |

**Las cinco eran doc-comments de producción que nombran la herramienta para
explicar el campo.** La de `test_adapters.rs:47` es la más reveladora porque el
campo ya se llama `manifest_path` y la frase ya dice «the ecosystem manifest
file»: el nombre de la herramienta **no añade nada** y solo ata el núcleo a un
ecosistema. Al retirarlo, dos de los cinco textos **ganan** información —«el
núcleo lo lee como una etiqueta opaca y no lo interpreta» y «qué fichero sea es
asunto del adapter, no del núcleo»—, y eso es lo que un ejemplo concreto
sustituía sin decirlo.

## El atajo que se descartó, y por qué

La primera versión del diseño detectaba zonas `#[cfg(test)]` **sin léxico**: «del
primer `#[cfg(test)]` al final del fichero». Se descartó por medición, en dos
pasos.

1. **Es ceguera, y en el corpus.** `test_apply.rs:33` es
   `#[cfg(test)] use crate::test_select::ImpactPlannerV1;` —que **no es un
   módulo**—. La regla habría eximido **592 líneas de producción**.
2. **El atajo alternativo tampoco.** «Hasta la próxima línea sin sangrar» falla
   al revés: `test_adapters.rs` tiene **30 líneas a columna 0** dentro de su
   zona de test, que son el cuerpo de un JSON en cadena cruda.

Ninguno de los dos atajos es «menos preciso»: uno es ceguera y el otro es ruido,
y son fallos de signo opuesto. Por eso hay un lexer, y por eso el corpus entero
pasa por él: **379 líneas con llaves dentro de cadenas**, raw strings que abren
con una llave en la misma línea, y **55 literales de carácter** con la ambigüedad
de los lifetimes.

La propiedad que hace aceptable el lexer está escrita en el fichero: **si el
lexer se desincroniza, tiene que fallar hacia el ruido, nunca hacia el
ciego**. Una zona que no cierra devuelve `Err`, y `Err` es un fallo del guard,
no una zona vacía.

## El falsador encontró cuatro controles míos que no tenían dientes

Esta es la parte que justificaría el bloque por sí sola. Los controles se
escribieron, todos verdes, y **cuatro no distinguían el defecto que decían
medir**. Lo unico que las encontro fue el falsador.

| control | por qué no discriminaba | qué lo arregla |
|---|---|---|
| el `#[cfg(test)] use` | el nombre prohibido estaba en una línea suelta después; con el detector roto y con el bien, se veía igual | meterlo **dentro** del cuerpo de la función de producción, y quitar la línea en blanco |
| cadena cruda | el fixture tenía `{` y `}` **compensados** | llave impar, y **número impar de comillas** |
| cadena normal / literal | igual, compensados | llave impar en cada uno |
| comentarios de bloque | sin cuerpo de producción detrás | añadirlo |

Y un quinto, que era del **falsador** y no del guard: cuando un parche rompía la
compilación, el control desaparecía del informe y mi código lo contaba como
«SIGUE VERDE» — declarando sana una mutación degenerada. Y un sexto: el contador
`total_ok` usaba `grep -c ... || echo 0`, que devuelve **dos ceros** cuando
cuenta cero, luego `[ "0\n0" -lt 12 ]` no es una comparación entera y **el
aborto de la base rota nunca se disparó**. Con la base a cero controles en pie
corrieron las once mutaciones y las declaró todas supervivientes.

**Un instrumento que no cuenta es peor que no tener instrumento**, porque
devuelve una respuesta.

## Los controles, y la matriz de mutaciones

Las once mutaciones, cada una muerta por la suya:

| # | mutación | control que cae |
|---|---|---|
| M1 | sin normalizar la caja | `un_nombre_con_mayuscula_tambien_infringe` |
| M2 | ninguna zona se exime | `la_zona_de_test_no_infringe_y_prueba_el_rechazo` |
| M3 | zona sin mirar el terminador del item | `un_cfg_test_sobre_un_use_no_exime_lo_que_viene_despues` |
| M4 | cadenas crudas sin enmascarar | `una_cadena_cruda_con_llaves_no_cierra_la_zona` |
| M5 | cadenas normales sin enmascarar | `una_cadena_normal_y_un_literal_de_caracter_no_cuentan_como_llaves` |
| M6 | literales de carácter sin enmascarar | el mismo |
| M7 | comentarios de bloque sin enmascarar | `un_comentario_con_llaves_no_cierra_la_zona` |
| M8 | zona sin cerrar se presume limpia | `el_guard_falla_cerrado_ante_una_zona_indeterminable` |
| M9 | límite de palabra → subcadena | `el_escaner_de_palabras_no_es_una_subcadena` |
| M10 | vocabulario encogido | `el_vocabulario_no_se_ha_vaciado` |
| M11 | el guard vuelve a mirar un módulo | `el_guard_cubre_todos_los_modulos_del_dominio` |

**M4 tuvo que quedarse viva tres rondas antes de morir**, y lo que la mataba no
era la llave impar sino el **número impar de comillas**. Una cadena cruda con
comillas pares es indistinguible de una cadena normal —el camino de cadena
normal las empareja dos a dos y cierra en el mismo sitio—, luego desactivar su
tratamiento no producía ceguera sino ruido. Con comillas impares el camino
normal **deja una cadena abierta que se come el resto del fichero**. La ley que
gobierna el caso no es «cuenta las llaves»: es «cuenta las llaves **que están
donde el código dice que están**».

## Una autoridad, y no dos

El vocabulario vivía en dos ficheros. **Dos copias de una ley son dos
autoridades**, y dos autoridades significan que alguien actualiza una y la otra
protege de menos sin que nada se entere. Se retiró la vieja, y con ella
`find_word`, `is_word_byte` y los dos tests de fitness; `version_authority_fitness.rs`
conserva solo la tabla de verdad del reducer, que es lo que siempre fue lo suyo.

Las **dos referencias colgantes** que quedaron —`version_authority.rs:29` y `:616`
apuntaban a `the_decision_module_names_no_concrete_technology`, que ya no existe—
se actualizaron. Una referencia a un test borrado es un hecho falso escrito en
el código, y es la misma clase que las otras ocho: el núcleo diciendo una cosa y
la fuente haciendo otra.

## El `idal`

`version_authority_fitness.rs:80` decía «**icidal** aquí antes de que apareciera
por azar». No es una palabra: es basura ASCII, introducida por `74c13916`, y **el
escáner de contaminación no la ve** porque busca cirílico, CJK, kana, hangul,
fullwidth y emoji — todos no-ASCII. Se retiró con la sección, y el repo ya no lo
tiene.

Un guard que cubre una categoría y deja al lado un hueco contiguo que es su
gemelo exacto —ASCII corrupto donde buscabas no-ASCII corrupto— es el mismo
patrón del campo de `STATE.yaml` con la prosa en falso: **la cobertura es
categórica y el hueco está justo al lado**.

## Lo que este bloque NO hizo, y por qué

- **No extendió la ley a `sddk-gateway`, `sddk-engine` ni `sddk-cli`.** Son otra
  cosa: el gateway **debe** conocer nombres de fichero, porque su trabajo es
  preguntar al build tool. Aplicar el mismo vocabulario ahí produciría decenas
  de infracciones que no son infracciones.
- **No amplió el vocabulario.** Treinta y cuatro entradas funcionan; añadir más
  es trabajo sin defecto que lo justifique.
- **No abrió el hueco de la basura ASCII.** Declarado arriba.
- **No tocó** `AdapterFact`/`EvidenceResolver`, los targets sin cuerpo, la
  observabilidad ni el CI. Siguen donde estaban.

## Verificación

El SUT del bloque es `sddk-domain`, y es lo que se ha corrido: el perfil del
crate completo, clippy del workspace, y el falsador. **El perfil completo del
workspace no se ha ejecutado**, porque `apply` no lo exige y porque los cambios
en el dominio son doc-comments y dos ficheros de test: no cambian ninguna
firma. Eso queda declarado en vez de dado por supuesto — es exactamente la
razon por la que VA13 tardó dos bloques en ver un defecto de doc-comment.