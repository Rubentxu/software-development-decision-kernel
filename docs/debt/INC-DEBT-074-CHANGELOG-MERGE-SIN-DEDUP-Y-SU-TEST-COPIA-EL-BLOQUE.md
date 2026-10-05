---
id: INC-DEBT-074
title: "El merge del changelog anade las entradas del bump a ciegas a la seccion que ya existe, y el unico test que vigila ese bloque COPIA el codigo en vez de invocarlo: hoy duplica el contenido publicado, y cualquier arreglo del merge seria invisible para su propio guard"
status: open
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
resolved_at:
component: release pipeline / changelog
surface: scripts/release-bump.sh (bloque de merge de CHANGELOG) y tests/test_changelog_merge.sh
related: [INC-DEBT-047, INC-DEBT-070, INC-DEBT-073]
references:
  - scripts/release-bump.sh
  - tests/test_changelog_merge.sh
  - tests/test_changelog_coverage.sh
  - CHANGELOG.md
---

# INC-DEBT-074 — el merge del changelog no deduplica, y su test copia el bloque

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

## Salidas

1. Extraer el merge a una funcion invocable y que **tanto**
   `release-bump.sh` **como** el test la ejecuten, de modo que el guard
   deje de ser una copia. Esto va PRIMERO: sin esto, el punto 2 no es
   verificable.
2. Deduplicar por la misma regla que el gate 2b (tipo + scope + 4
   primeras palabras), para que las dos herramientas no tengan cada una
   su definicion de "mismo commit".
3. Anadir al test una asercion de que un commit ya representado **no**
   aparece dos veces, y falsarla quitando el dedup.

Ninguna de las tres se hace aqui: publicaba 2.10.0 y hacerlas con el
release a medias seria la forma de introduzirlas sin un 1b que las mida.
