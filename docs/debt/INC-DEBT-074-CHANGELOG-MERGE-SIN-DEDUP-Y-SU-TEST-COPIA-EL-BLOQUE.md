---
id: INC-DEBT-074
title: "El merge del changelog anade las entradas del bump a ciegas a la seccion que ya existe, y el unico test que vigila ese bloque COPIA el codigo en vez de invocarlo: hoy duplica el contenido publicado, y cualquier arreglo del merge seria invisible para su propio guard"
status: resolved
severity: medium
priority: P2
fingerprint: "changelog_merge_appends_without_dedup_and_its_test_copies_the_block"
fingerprint_aliases: []
cluster_id: CL-VERIFY
created: 2026-10-05
created_by: miniMax Code (mvs_fac515d56e784fc081d64fefb38323aa)
owner: miniMax Code (mvs_fac515d56e784fc081d64fefb38323aa)
detected_at: 2026-10-05
detected_in_session: session-81
resolved_at: 2026-10-05
component: release pipeline / changelog
surface: scripts/release-bump.sh (bloque de merge de CHANGELOG) y tests/test_changelog_merge.sh
related: [INC-DEBT-047, INC-DEBT-070, INC-DEBT-073]
references:
  - scripts/release-bump.sh
  - tests/test_changelog_merge.sh
  - tests/test_changelog_coverage.sh
  - CHANGELOG.md
---

# INC-DEBT-074 — RESUELTO — el merge deduplica, y el test ejecuta el codigo que vigila

## Que se midio

**MEDIDO publicando 2.10.0.** La seccion `## [2.10.0]` de `CHANGELOG.md`
quedo con **las 6 entradas escritas dos veces**: las escritas a mano en las
lineas 8-15, con su prosa, y el bloque autogenerado por el bump en las
lineas 19-27, con el subject pelado. El corte se hizo a mano y no es
repetible.

La causa es que `scripts/release-bump.sh:357-377` hace

```bash
tail -n +2 "$ENTRY_FILE"
```

al final de la seccion existente, **sin comparar contra lo que la seccion
ya declara**. Y el orden del flujo hace inevitable el solapamiento: el
preflight (`scripts/release.sh:316`) exige que HEAD sea
`chore(release): bump version`, luego la seccion del artefacto que se va a
publicar tiene que existir **antes** del bump. Escribirla antes es la
norma, no el error.

## Por que el gate 2b no lo ve

`tests/test_changelog_coverage.sh` comprueba **presencia** por huella
(tipo + scope + 4 primeras palabras del payload). Duplicar no la
incumple: MEDIDO `PASS=5 FAIL=0` con la seccion duplicada puesta. El gate
contesta la pregunta que sabe hacer —"¿el changelog describe el trabajo?"—,
que no es la que hace falta —"¿lo describe una vez?"—. Ninguno de los dos
tiene la culpa y por eso no basta con añadir una asercion al 2b.

## La segunda cara, y es la que impide arreglarlo

`tests/test_changelog_merge.sh:30-43` **pega el bloque del merge
literalmente** en el test, con el comentario "copied verbatim from
release-bump.sh". Consecuencia medida: cambiar el merge en
`release-bump.sh` **no mueve el test**, y el test seguiria dando verde
contra la copia antigua. El guard de un bloque no puede ser una copia del
bloque — es la clase que este repo ya ha pagado mas de una vez, y aqui es
peor que en las anteriores porque **el objeto exclusivo del guard es
justamente ese bloque**: no hay otra asercion que lo cubra.

Ademas el test actual solo comprueba que el item nuevo se mergia (que no
salga una segunda cabecera `## [`). **No comprueba la ausencia de
duplicados**, luego un dedup correcto tampoco estaria vigilado por el test
que existe hoy.

## Severidad: medium/P2, y por que no mas

No hay perdida de datos ni un verde falso sobre una propiedad de
seguridad. El modo de fallo real es un artefacto publicado que lista el
mismo trabajo dos veces, lo cual es la clase que INC-DEBT-047 ya
persigue — y por eso P2 y no P3: el coste no fue el duplicado en si, fue
un recorte manual no repetible que habria que repetir en cada release
mientras esto siga abierto.

## Resuelto

Las tres salidas, en el orden que la propia deuda fijaba, y la razon por la
que el orden no era negociable.

### 1. El merge vive en una libreria que EJECUTAN los dos

`scripts/lib/changelog_merge.sh`, sourceada por `scripts/release-bump.sh` y
por `tests/test_changelog_merge.sh`. El test ya no pega el bloque: lo
llama. Se comprobo que el seam no hace falta: el guard saca su codigo de
`$ROOT/scripts/lib/changelog_merge.sh`, luego el falsificador monta un repo
en miniatura con la libreria mutada y el guard intacto y corre el guard
ahi. Ningun hueco en el codigo de produccion.

### 2. Deduplica con la MISMA regla que el gate 2b

`changelog_item_fingerprint` imprime `<type(scope)>|<4 primeras palabras>`
y la usa el merge. La regla del gate 2b sigue siendo la suya y no se toco:
el gate mide PRESENCIA sobre el rango de commits y el merge decide que
items anadir, y copiar la funcion entero habria movido un gate que hoy
pasa y cuyo fallo es el mas caro del camino.

**Lo que si se unifico, y por que era obligatorio**: el concepto "¿es el
mismo commit?" tiene ahora una definicion, y tener dos se manifesto de
una forma concreta:

- La huella.quitaba la vineta **antes** que los espacios, luego
  `${line#- }` no casaba con `  - fix(cli): ...` y la clave salia
  `- fix(cli)` con la vineta pegada. El dedup SEGUIA funcionando porque
  los dos lados del merge son los dos con vineta — invisible desde
  dentro —, pero la huella ya no era la misma que la del gate 2b, luego
  la promesa de "una sola regla" era falsa. MEDIDO y corregido.

### 3. Asercion de no-duplicado, falsada quitando el dedup

`tests/test_changelog_merge.sh` con **PASS=34 FAIL=0**, y
`tests/test_changelog_merge_mutation.sh` con **PASS=9 FAIL=0 SKIP=0**: las
siete mutaciones caen, cada una por su comprobacion.

## Lo que aparecio al construirlo, y que no estaba en la deuda

Cuatro cosas, todas medidas, ninguna hipotetica.

**Un `case` cuyo patron no distinguia lo que su nombre decia.** El primer
`case` de `release-bump.sh` uso `"*disposition: merged"*`, que tambien
casa con `merged_nothing`: la rama `merged_nothing` era CODIGO MUERTO. Lo
dijo el linter (SC2221). Y el fallo va en la direccion buena: ahora la
comparacion es EXACTA, luego si algo escribiera en stdout el `case` cae
en el error en vez de tomar la rama equivocada.

**El merge no hacia NADA cuando la seccion destino era la ultima.** El
ensamblado terminaba en `[ "$after_line" -le "$total" ] && tail ...`; sin
seccion siguiente la condicion es falsa, el grupo entero sale con estado 1
y el `|| { return 1; }` de al lado abortaba ANTES de escribir. Sin
disposicion declarada, luego indistinguible de un acierto. Salio al
escribir el caso de prueba C4, que es el unico que tiene esa forma.

**La propiedad "sin grupos vacios" tenia DOS autores y por eso no se podia
falsar.** La aplicaba el filtro y tambien el awk emisor. MEDIDO: la
mutacion M7, que quita solo la del filtro, dejaba el guard en VERDE. No
es una guarda fuerte, es redundancia: una propiedad que no se puede
falsar no esta vigilada. Se dejo un solo autor —el que EMITE— y con eso
M7 cae. Es la regla de AGENTS.md 2.7 aplicada a una propiedad mia, y la
cuarta vez que esta sesion la encuentra en el mismo sitio.

**Cinco de las siete mutaciones estaban rotas de origen, no mal aplicadas.**
La primera version del falsificador paso los pares como `viejo<TAB>` sin
texto de reemplazo, luego **borro** la linea entera, rompio la libreria, y
el guard cayo por todo en vez de por su comprobacion. Una mutacion que
cae "por algo" no mide nada. Ademas el par de M5 mia `$'\n'`, que CIERRA
la cadena de comillas simples que lo contiene. Se reescribio con heredocs
de comillas y separador `%%` en vez de un TAB escrito a mano.

Y una direccion que queda escrita porque es la que importa: cuando un item
NO se puede clasificar, el merge lo **conserva** y lo declara. Un item
duplicado es ruido; un item perdido es un artefacto que no describe lo que
publica. Ante la duda, el dedup no descarta.
