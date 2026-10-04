---
id: INC-DEBT-070
title: "El gate 2b de changelog lee la ultima version publicada del clon LOCAL, y el clon queda viejo en cuanto se publica: el gate pasa a exigir en la siguiente release commits que ya estan publicados"
status: open
severity: medium
priority: P2
fingerprint: "changelog_gate_reads_local_tag_baseline_and_drifts_after_publish"
fingerprint_aliases: []
cluster_id: CL-AUTHORITY-SPLIT
created: 2026-10-04
created_by: miniMax Code (mvs_fac515d56e784fc081d64fefb38323aa)
owner: miniMax Code (mvs_fac515d56e784fc081d64fefb38323aa)
detected_at: 2026-10-04
detected_in_session: session-79
component: tests
surface: tests/test_changelog_coverage.sh:66
related: [INC-DEBT-064, INC-DEBT-068, INC-RELEASE-TAG-FIX]
references:
  - tests/test_changelog_coverage.sh
  - scripts/lib/release_admission.sh
  - scripts/release-bump.sh
---

## Que es

La pregunta "cual es la **ultima version publicada**" tiene **dos respuestas** en
este pipeline, y `tests/test_changelog_coverage.sh:66` usa la que se queda
vieja:

```bash
LAST_TAG="$(git -C "$ROOT" tag --sort=-version:refname | head -1)"
```

`git ls-remote` lee el remoto; `git tag` lee el clon local. **`gh release create`
publica en el remoto y nada del pipeline actualiza el clon**, luego la respuesta
local va cayendo atras/version tras version, y el gate 2b compara el rango
`v<local>..HEAD` creyendo que es trabajo sin publicar.

## MEDIDO, session-79, inmediatamente despues de publicar v2.8.0

| Medida | Valor |
|---|---|
| Tag mas alto **local** | **v2.7.0** |
| Tag mas alto **remoto** | **v2.8.0** |
| `LAST_TAG` que habria usado el gate 2b | **v2.7.0** |
| Rango que habria comparado | `v2.7.0..HEAD` = **11 commits** |
| De esos, `feat`/`fix`/`test` **ya publicados en 2.8.0** | **4** |

O sea: la proxima release habria exigido que los commits de la 2.8.0 —
`feat(cli)`, `test(engine)`, `test(cli)`, `test(uat)` — estuvieran representados en
la seccion de **la version siguiente**. Y aqui esta la parte uncomfortable: la
forma "facil" de hacerlo verde es **duplicar esas entradas** en el changelog
nuevo, que es un changelog que **describe trabajo que ya salio**.

La mitigacion aplicada fue `git fetch --tags origin`, que deja el clon
veraz. **No es un arreglo**: nada impide que la proxima publicacion vuelva a
dejarlo viejo, y el patron se repite cada release.

## Por que NO se ha arreglado aqui, y por que la severidad no baja

El arreglo son pocas lineas —leer la misma autoridad que ya usa
`release-bump.sh` (`scripts/lib/release_admission.sh:120`,
`last_published_version()`, con sus tres resultados: remoto contesta /
bootstrap legitimo / **query failed que debe fallar cerrado**)— **pero tocar un
gate exige su propio falsador**, y el falsador de la linea base del bump
(`test_release_bump_remote_baseline_mutation.sh`, `PASS=2 FAIL=0 SKIP=0`) esta
**atado al otro consumidor**, no a este. Sin un falsador propio, el cambio seria
una declaracion sin dientes, que es **exactamente la forma de defecto que este
repo lleva tres sesiones corrigiendo**. Abrirlo aqui seria abrir un frente que
el bloque no cierra.

## Denominador: por que esto NO es "varios gates con fuentes distintas"

Medido, porque la hypothesis mas comoda era "varios gates duplican la pregunta"
y **resulto falsa**: tras el arreglo de session-75, `release-bump.sh` y
`release_admission.sh` leen el remoto, y los dos unicos ficheros que aun
mencionan `git tag` fuera de este gate son **fixtures de repos aislados**
(`test_release_admission.sh`, `test_release_bump_remote_baseline_mutation.sh`),
donde el clon local **si** es la autoridad porque no hay remoto.

**Este gate es el unico consumidor de produccion que queda en el clon local.**
El denominador se escribe porque cambiarlo —de "varios" a "uno"— es la
diferencia entre un problema de convencion y un bug en un fichero, y la
medicion es la que dice cual de las dos es.

## Lo que esta medicion NO dice

- **No se ha ejecutado el gate con el clon viejo**: la conclusion de "exigiria
  4 commits ya publicados" se deduce del rango y del filtro de tipos del
  propio gate, leidos, no de una corrida en rojo. Fabricar un clon viejo para
  verlo es trabajo de su propia sesion.
- **No afirma que el gate haya fallado nunca** por esto. Es una
  **superficie de fallo**, no un fallo observado.
- **El arreglo tiene un detalle que no es trivial** y por eso no se ha hecho a
  ojo: el gate necesita el tag **localmente** para calcular `git log
  "$LAST_TAG"..HEAD`. Leer el remoto sin traerse el objeto deja el rango
  incalculable, y un rango incalculable no se puede rellenar con una
  suposicion sobre el nivel de SemVer — que es el mismo punto por el que
  session-75 tuvo que traerse la etiqueta.

## Cierre

Cierra cuando el gate 2b lea la **misma** autoridad que el resto del pipeline
—`last_published_version()`, con los tres resultados que no se pueden
confundir— **y tenga su propia falsacion** con los tres casos: remoto al dia,
remoto por delante del clon, y remoto que no responde. **No cierra por
antiguedad**, y no cierra por "traerse los tags a mano": eso es la mitigacion,
no el contrato.
