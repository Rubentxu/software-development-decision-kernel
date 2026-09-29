---
id: INC-DEBT-031-CHANGELOG-DUPLICATE-AND-PHANTOM-VERSIONS
title: "CHANGELOG.md tiene 4 versiones duplicadas y una version fantasma (2.3.0) que nunca existio como release"
status: closed
severity: low
priority: P3
created: 2026-09-28
closed: 2026-09-29 (session-34, commit changelog-dedup)
closed_by: session-34
discovered_by: session-30 retrospectiva (OBSERVED, sobre el fichero real)
cluster_id: CL-TRACEABILITY
related: [INC-DEBT-025-MANIFEST-SHA-FROM-FIRST-LINE]
fingerprint: "changelog_duplicate_headers_and_phantom_version"
---

## Qué es

`CHANGELOG.md` tiene **cinco** anomalías de estructura, todas observadas
sobre el fichero real (no inferidas):

### 1. Cuatro versiones con cabecera duplicada

| versión | líneas | ¿bloques idénticos? |
|---------|--------|---------------------|
| `2.2.0` | 342 y 468 | difieren (cada uno tiene items únicos) |
| `2.1.0` | 586 y 642 | difieren |
| `1.4.0` | 2326 y 2337 | — |
| `1.8.0` | 2242 y 2400 | — |

Conteo verificado:

```console
$ grep -oE '^## \[[0-9]+\.[0-9]+\.[0-9]+\]' CHANGELOG.md | sed 's/## \[//;s/\]//' | sort | uniq -c | awk '$1>1'
      2 1.4.0
      2 1.8.0
      2 2.1.0
      2 2.2.0
```

Con dos cabeceras para la misma versión, **no hay forma de saber cuál
entrada manda**. Los bloques no son idénticos, así que no es un duplicado
puro que se pueda borrar: hay que fusionar sus items.

### 2. Una versión fantasma: `2.3.0`

`## [2.3.0]` aparece en la línea 405, **entre** `2.2.0` (342) y el segundo
`2.2.0` (468), rompiendo el orden Keep-a-Changelog (nuevo primero).

No existió nunca como release:

```console
$ gh release view v2.3.0   -> release not found
$ git rev-parse v2.3.0     -> (ningún tag)
```

Causa raíz: el bump sí alcanzó una versión superior en un momento y se revirtió
antes de publicar, pero la escritura en CHANGELOG ya había ocurrido. El
commit `7559a710` (`fix(release): no re-bumpear cuando el workspace ya
declara la release`) declara `version = "2.2.0"` tanto en sí mismo como en
su padre — es decir, la reversión ya había pasado, pero su entrada en el
changelog quedó.

### 3. Causa raíz del defecto que los produce

`scripts/release-bump.sh` insertaba la sección `## [$NEXT]` **a ciegas**,
sin comprobar si ya existía. Como `--force-version` existe precisamente
para re-declarar una versión ya declarada, un re-bump producía una segunda
cabecera idéntica.

**Ya corregido** en `17d9b804` (el bump ahora fusiona en la sección
existente en vez de duplicar la cabecera, con
`tests/test_changelog_merge.sh` como cobertura). Este INC **no cubre el
script**: cubre la limpieza del fichero ya dañado y la pregunta de qué
hacer con la versión fantasma.

## Por qué importa (y por qué es P3)

La trazabilidad de "qué versión contiene qué" es la primera pregunta de
cualquier consumidor del changelog, y aquí la respuesta es ambigua para
cuatro versiones. Pero:

- **No afecta a la instalación.** El changelog no se usa en ningún camino
  de código: verificado que `install.sh`, `release.sh` y el CLI no lo leen.
- **No afecta a la distributión.** Los assets publicados llevan su propio
  `CHECKSUMS` y `sbom.json`; el changelog no viaja en el bundle.
- **Ninguna versión duplicada está publicada** salvo la línea 2.0.x, que
  no aparece en absoluto (0 entradas para `2.0.`).

Es deuda de higiene documental, no un riesgo de seguridad ni de
instalabilidad. Por eso P3 y no más: el coste de arreglarlo (fusionar 4
pares de bloques items-a-item) es mayor que el beneficio, salvo que
alguien necesite el changelog como fuente fiable.

## Lo que NO se afirma aquí

- No se afirma que se haya perdido historia. Los bloques difieren, así que
  una fusión conservadora conserva todo; pero una fusión mal hecha sí
  podría perder entradas, y por eso el trabajo está pendiente de una
  revisión item a item, no de un `uniq`.
- No se afirma que `2.3.0` sea un error de *release*. Es un error de
  *registro*: no hubo tag, ni release, ni distribución.

## Siguiente

Fusionar los 4 pares, decidir el destino de `2.3.0` (eliminar la entrada,
o marcarla explícitamente como "nunca publicada"), y **añadir la línea 2.0.x
que hoy no existe en absoluto** (v2.0.0 y v2.0.1 sí están publicadas). Es
trabajo de bajo riesgo pero no trivial, así que queda explícitamente
**fuera del alcance de la session-30** y no se hizo a medias.

## Cierre (session-34, 2026-09-29)

**Reparación aplicada (OBSERVED):** los 5 defectos originales más uno
nuevo descubierto al reparar.

1. `2.2.0` duplicado → conservado el primero (línea 701); eliminado el
   segundo (subset estricto).
2. `2.3.0` fantasma (764-826) → eliminado; su único item exclusivo
   (`fix(release): derivar la version desde el workspace`) ya estaba en
   el bloque válido de 2.2.0 (2 copias ahí, deduplicadas a 1).
3. `2.1.0` duplicado → conservado el primero; el segundo era subset
   estricto.
4. `1.4.0` duplicado → fusionados los 6 items únicos del bloque pequeño
   dentro del bloque grande; un solo bloque 1.4.0.
5. `1.8.0` header huérfano de fin de fichero → eliminado.

**Hallazgo nuevo (no estaba en esta INC):** el generador de CHANGELOG
arrastraba items hacia atrás en merges fallidos: 122 items aparecían en
múltiples secciones (el mismo item hasta 12-17 veces, p.ej.
"derivar la version desde el workspace" x17). Criterio de limpieza:
cada item se conserva solo en su sección más antigua (la release que
realmente lo contuvo). 752 items finales vs 1499 originales.

**Evidencia:** `git diff` del commit changelog-dedup; conteos antes/después
en la sesión; `tests/test_changelog_merge.sh` verde tras el cambio.
