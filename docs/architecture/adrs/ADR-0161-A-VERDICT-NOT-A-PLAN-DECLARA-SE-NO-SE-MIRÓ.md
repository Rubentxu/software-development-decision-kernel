---
id: ADR-0161-A-VERDICT-NOT-A-PLAN-DECLARA-SE-NO-SE-MIRÓ
title: Una versión que resuelve no dice por qué, y el informe que explica el porqué tiene que ser tan completo como el peor caso
status: accepted
proposed_at: 2026-10-05
accepted_at: 2026-10-05
cycle: p-63676b11dc0ef88f/version-coherence-068
accepted_by_cycle: p-63676b11dc0ef88f/version-coherence-068
supersedes: null
superseded_by: null
extends: [ADR-0157, ADR-0158, ADR-0159, ADR-0160]
component: release
surface: crates/sddk-domain/src/version_inspection.rs
closes: []
---

# ADR-0161 — Un veredicto no explica su propio porqué, y un informe no puede ser menos completo cuando la noticia es buena

- **Status:** `accepted` (2026-10-05)
- **Extiende:** ADR-0157 (la versión se resuelve preguntando a providers),
  ADR-0158 (varios productos por repositorio), ADR-0159 (un tag no es una
  versión), ADR-0160 (roles de release)
- **Superficie:**
  - `crates/sddk-domain/src/version_inspection.rs` — `AssuranceLevel`,
    `VersionInspection`, `ProviderFinding`, `SkippedProvider`, `NotChecked`
  - `crates/sddk-domain/src/version_authority.rs` — `observe_all`,
    `resolve_inspecting`
  - `crates/sddk-cli/src/release_cmd.rs` — `sddk release version inspect`
  - `crates/sddk-gateway/tests/version_provider_contract.rs` — C1..C10

---

## Contexto

ADR-0157 dejó la versión como una **pregunta** que se responde preguntando a
providers, y no como un registro de manifiestos conocidos. Eso resolvió el
problema de fondo: el núcleo ya no nombra Cargo, ni npm, ni Gradle.

Pero dejó otro, más pequeño y más persistente: **resolver no es explicar.**
Cuando `release plan` dice `2.1.0`, hay tres preguntas distintas detrás de esa
línea y ninguna está contestada.

> ¿Quién declaró eso? · ¿A quién no se preguntó? · ¿Qué **no** se comprobó?

Responderlas exige hoy abrir `version_provider.rs`, y un agente que abre un
fichero para entender un veredicto es un agente cuya respuesta depende de un
refactor. Peor: las tres preguntas tienen la misma respuesta *falsa* disponible,
y las tres son verosímiles. Un informe verosímil que omite que no se
cross-validó nada es peor que no tener informe, porque **se cita**.

El daño de fondo que este ADR evita no es la falta de información. Es la
información **falsa con naturalidad**, la que se rellena sin pensar y que nadie
contrasta porque suena bien.

### Lo que este informe deliberadamente **no** hace

- **No decide.** El veredicto viene de `reduce`, y el informe solo lo describe.
  Reducir dos veces serían dos autoridades, y la que se imprime sería la que
  cuenta por estar la última.
- **No certifica.** Encontrar un `ProductVersion` no es certificar un release.
  El informe lo dice **en su propia salida**, no en un docstring que nadie lee
  mientras falla un release.
- **No recomienda.** Nunca dice «añade `version=` a X» ni «renombra tu
  fichero». Describir qué se observó es trabajo del informe; decidir cuál
  debería ser el layout del proyecto es trabajo del operador. Una herramienta
  que decide qué fichero debería tener un repositorio es una herramienta que ha
  adoptado el layout de un ecosistema como si fuera la forma de todos.

---

## Decisión

### 1. El informe se construye DESPUÉS de la reducción, y la reducción es una sola

`resolve_inspecting` observa, llama a `reduce` **una vez**, y construye el
informe sobre ese veredicto. `resolve` sigue existiendo, intacto, con su
contrato. Es un punto de entrada **adicional**, no un reemplazo: un método que
«mejora» `resolve` cambiaría lo que significa para todos los que ya lo llaman,
y eso es otro cambio.

### 2. `NOT_CHECKED` es un campo **calculado**

Cuatro hechos son siempre ciertos de este informe y uno depende del resultado.
No están escritos: se derivan de lo que pasó. Una lista escrita a mano se
desactualiza el día que el reducer cambie, y se desactualiza en silencio —que
es la forma que no se nota.

Entre ellos, dos que son la razón de ser del módulo:

- `release_reference_not_compared` — comparar la referencia con la versión es la
  pregunta de la regla de lockstep, y este informe no la hace. Un agente que lee
  «resuelto» aquí y concluye que el tag está verificado ha leído de más.
- `provider_set_is_sddks` — el conjunto de providers es el que SDDK distribuyó. Una
  herramienta que nadie registró es una fuente que **nunca se consultó**, y eso
  no es lo mismo que una fuente que se consultó y no encontró nada.

### 3. Un provider no puede ser **rechazado** por no tener evidencia re-checkeable

El núcleo no tiene autoridad para llamar deshonesto a un provider. Lo que puede
hacer es **decirlo**: `recheckable` es un hecho de cada hallazgo, y un provider
que no puede ser re-verificado aparece en `NOT_CHECKED` con su nombre. Un
`NOT_CHECKED` es una **declaración de límites**, no un veredicto contra nadie.

---

## Los dos defectos que este bloque encontró, y por qué están aquí

Ambos los encontró la **medición sobre repos de prueba**, no la inspección del
código. Se documentan porque son la razón de dos decisiones de este ADR.

### Defecto 1 — el informe era más completo cuanto peor salía la resolución

MEDIDO, comparando dos repos con el mismo conjunto de ficheros y distinto
resultado:

| repo | resultado | hallazgos en el informe |
|------|-----------|------------------------|
| dos declaraciones de `1.4.0` que coinciden | `CrossValidated` | **2** |
| `1.4.0` y `2.0.0` | `Conflict` | **14** |

La causa: `reduce` **poda** en `CrossValidated` — deja solo las observaciones que
declararon el valor acordado. Para el *veredicto* ese contrato es correcto: sus
observaciones contestan «quién estuvo de acuerdo con este valor». Para un
*informe* es desastroso, y produce un informe **más detallado en el fallo que en
el éxito**.

Eso es un defecto y no un detalle, porque un informe informativo no puede
depender de que la noticia sea buena: es exactamente en el caso de éxito donde
nadie va a sospechar que le falta algo.

**Arreglo:** `observe_all` separa «preguntar» de «reducir». El registro
completo se conserva, y `resolve_inspecting` reduce **ese mismo juego** — una
sola reducción, el mismo veredicto, ninguna autoridad nueva.

### Defecto 2 — el informe era correcto y no se podía leer

En el caso de conflicto, las catorce entradas salían en **una sola lista**, de
modo que las dos únicas líneas que contestaban quedaban debajo de doce
«CMakeLists.txt no está en este target».

**Arreglo:** tres secciones, y el corte es por `level`, **no** por «trae
versión»:

- `observations:` — lo que contestó algo
- `consulted_and_found_nothing:` — se preguntó y no estaba ahí
- `never_asked:` — no habla la capacidad, luego nadie llegó a preguntarle

`Invalid` **se queda arriba** aunque no traiga versión, porque fallo cerrado es
una **respuesta**, no una ausencia, y esconderlo bajo las ausencias sería
esconder el único motivo por el que a este comando le interesaría un código de
salida distinto.

Y `observations: none` existe porque una lista vacía y una línea que se olvidó de
imprimir son la misma línea — la clase de hueco que este bloque persigue.

---

## Consecuencias

### Positivas

- La pregunta «¿por qué resolvió eso?» tiene respuesta sin abrir un fichero, y
  la respuesta **menciona lo que no se comprobó**.
- La conformidad de un provider es comprobable: `version_provider_contract`
  fija C1..C10 contra providers reales, no contra una descripción de ellos.
- Un repo sin declaración de versión funciona: el comando resuelve **solo la
  raíz**, sin `RuntimeContext`, porque abrir contexto exige identidad de
  proyecto y el comando fallaba en un repo sin adoptar — que es exactamente el
  repo cuyo release no resuelve.

### Negativas y costeadas

- **Un repo sin ninguna declaración no produce informe**: sale con 1 y
  `no release target found`. Es una decisión, no una omisión. Se podría
  «arreglar» haciendo `VersionInspection::target` opcional, y el coste es que
  `target` es el campo que todos los lectores usan para saber **sobre qué** se
  habló: volverlo opcional obliga a cada uno a tratar un caso que ya tiene un
  mensaje que dice la causa exacta y que `release plan` devuelve igual. El
  código no cero es lo correcto: aquí no hay versión, y un agente que se llevara
  un 0 leería «todo bien» de un repo que no declara nada.
- El texto **reparte** en secciones y el JSON **no**: el JSON las mantiene todas,
  porque una máquina no lee secciones. Los dos son el mismo dato con dos
  readerships, y `el_json_no_pierde_ninguna_fila_que_el_texto_aparta` lo fija.

### Deuda declarada, no abierta aquí

- `DeclaredAuthorityProvider::provider_id()` es `sddk.gateway/<ruta>` mientras las
  demás son `sddk.gateway.declaration-file/<fichero>`. La lista de
  `providers_considered` sale heterogénea. **No se arregla aquí**: los provider
  ids son superficie pública, y renombrarlos cambia lo que un consumidor externo
  ve. Se registra, no se toca.

---

## Verificación

| # | Ley | Suite |
|---|-----|-------|
| T1 | El informe declara lo que no comprobó | `m1_caveats_dropped_murio` |
| T2 | Un solo declarante no es cross-validación | `m2_cross_validation_invented_murio` |
| T3 | Los providers filtrados no desaparecen | `m3_skipped_hidden_murio` |
| T4 | Encontrar una versión no certifica un release | `m4_certifier_guarantee_murio` |
| T5 | **El acuerdo no silencia el registro** | `m5_agreement_does_not_silence_the_record_murio` |
| T6 | **Una ausencia no es una respuesta** | `m6_an_absence_is_not_an_answer_murio` |
| T7 | Cada nivel se clasifica por su nombre | `cada_nivel_se_clasifica_por_su_nombre` |
| T8 | `NOT_CHECKED` es derivado y no escrito | `lo_que_no_se_comprobo_esta_derivado_y_no_escrito` |
| T9 | La resolución no escribe en el árbol | `resolver_no_escribe_en_el_arbol` |
| T10 | **El veredicto del informe es la reducción única** | `el_veredicto_del_informe_es_la_reduccion_unica` |
| T11 | Las ausencias no entierran a las que respondieron | `las_ausencias_no_enterran_a_las_que_si_respondieron` |
| T12 | El conflicto se lee sin pasar por doce ausencias | `el_conflicto_se_lee_sin_pasar_por_doce_ausencias` |
| T13 | Sin declaraciones no llega a informe, y dice por qué | `un_repositorio_sin_declaraciones_no_llega_a_informe_y_dice_por_que` |
| T14 | El cierre no repite lo que dice `NOT_CHECKED` | `el_cierre_no_repite_lo_ya_dice_not_checked` |
| T15 | El JSON no pierde filas que el texto aparta | `el_json_no_pierde_ninguna_fila_que_el_texto_aparta` |
| C1..C10 | Diez puntos del contrato de un provider | `version_provider_contract` |

### Falsificación

Seis mutantes, y **los dos últimos nacieron de una medición, no de una
sospecha**: T5 y T6 describen defectos que el código tenía y que nadie habría
encontrado leyendo el informe de un caso favorable.

- `M1 caveats-dropped` — el mutante del silencio: un informe que no declara lo
  que no comprobó.
- `M2 cross-validation-invented` — el mutante del exceso: un solo declarante
  presentado como agreement entre fuentes. **El más grave**, porque quien lee
  «verificado con dos fuentes» toma una decisión de publicación.
- `M3 skipped-hidden` — los filtrados desaparecen, y «nadie dijo nada» se lee
  como «no había nadie que decir».
- `M4 certifier-guarantee` — el defecto textual de este bloque.
- `M5 agreement-does-not-silence-the-record` — lee las observaciones **del
  veredicto reducido**, que es la poda. Muere en la fila donde el código real
  conserva las cinco consultas y el mutante conserva dos.
- `M6 an-absence-is-not-an-answer` — cuenta como `providers_answering` a todo el
  que se consultó. Muere porque el código dice dos y el mutante dice cuatro, y
  porque lleva la fila negativa: `Invalid` **sí** cuenta como respuesta.

Y `el_veredicto_del_informe_es_la_reduccion_unica` lleva **caso negativo** a
propósito: dos registros con providers distintos tienen que dar veredictos
distintos, o la comparación no probaría nada. Una aserción que no puede fallar no
es una ley.

**Una lección del instrumento, la segunda de este bloque.** La primera versión de
este falsador atacó el defecto del defecto desde el sitio equivocado: midió
`findings` contra `verdict.observations` en un caso cuyo veredicto **era** el
conflicto, y ahí las dos listas coinciden. El mutante sobrevivió no porque el
código estuviera bien, sino porque **el caso no distinguía**. La fila que lo mata
—dos fuentes de acuerdo, que es donde `reduce` poda— se eligió después de leer
por qué la primera no fallaba. Un falsador que pasa porque su caso no
distingue está midiendo el caso, no la ley.
