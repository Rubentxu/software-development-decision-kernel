---
id: INC-DEBT-070
title: "El gate 2b de changelog lee la ultima version publicada del clon LOCAL, y el clon queda viejo en cuanto se publica: el gate pasa a exigir en la siguiente release commits que ya estan publicados"
status: resolved
severity: medium
priority: P2
fingerprint: "changelog_gate_reads_local_tag_baseline_and_drifts_after_publish"
fingerprint_aliases: []
cluster_id: CL-AUTHORITY-SPLIT
created: 2026-10-04
created_by: miniMax Code (mvs_fac515d56e784fc081d64fefb38323aa)
owner: miniMax Code (mvs_fac515d56e784fc081d64fefb38323aa)
resolved_at: 2026-10-04
resolved_in_session: session-79b
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

## COMO SE RESOLVIO (session-79b, mismo dia, bloque siguiente)

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


---

## Resolucion (session-79b)

El arreglo se hizo en el bloque siguiente, con su propia autofalsacion, que es
justo lo que esta entrada pedia antes de cerrarse.

**El gate** (`tests/test_changelog_coverage.sh`) ahora:
- consulta la MISMA autoridad que `release-bump.sh` y el gate 9b
  (`scripts/lib/release_admission.sh`, `_last_published_resolve`) y trata sus
  **tres resultados por separado**: `bootstrap` -> skip limpio; `query_failed` ->
  **FAIL CERRADO** que NOMBRA la causa; tag publicado -> autoridad;
- **trae el objeto del tag** cuando el clon no lo tiene, porque el rango se
  calcula como `${LAST_TAG}..HEAD` y esa referencia tiene que existir aqui, y
  si no se puede traer **falla cerrado en vez de adivinar**;
- usa la lista local **solo** cuando no hay remoto configurado, que es el unico
  caso en que no puede estar vieja respecto de una autoridad;
- **imprime de donde salio la linea base** en cada corrida, porque un lector que
  no ve que autoridad respondio no puede distinguir un bootstrap limpio de un
  clon viejo.

**El test** (`test_changelog_coverage_baseline.sh`, 4 casos) construye fixtures
donde **las dos respuestas dicen cosas distintas**: con el clon en `v0.9.0` y el
remoto en `v1.0.0`, el rango remoto tiene 1 commit —cubierto por la seccion— y el
rango local tiene 3, dos de ellos ya publicados. Con la autoridad correcta el
gate **pasa**; con la defectuosa **falla pidiendo trabajo ya salido**. El caso
C0 es el control de no-vacuidad: exige que el fixture en orden pase, luego el
test distingue certeza de pesimismo. `PASS=15 FAIL=0`.

**La autofalsacion** (`test_changelog_coverage_baseline_mutation.sh`, paso
**3l/15** de `release.sh`): tres mutaciones, cada una exigiendo que caiga el caso
nombrado. `PASS=4 FAIL=0 SKIP=0`, restauracion byte-identica por sha.

## El falsador encontro dos defectos, y los dos eran mios

1. **El needle leia un caso al reves.** `cae()` buscaba `[ok]   C1:` y un caso
   tiene **varias** aserciones, luego un caso medio caido —con lineas ok y lineas
   FAIL a la vez— se leia **verde**. Es el needle ambiguo de siempre con otra
   forma: las tres mutaciones se declararon "sin dientes" cuando en realidad dos
   si los tenian. Un caso CAE si **alguna** de sus aserciones es FAIL.
2. **El fixture no discriminaba.** Las tres lineas de commit estaban en la
   seccion, luego el rango contra `v0.9.0` (3 commits) tambien estaba cubierto y
   **las dos lineas base pasaban**. Un caso que no discrimina no mide, por mucho
   que el gate este roto; y `v1.0.0` estaba en `HEAD~2` en vez de `HEAD~1`, luego
   el rango remoto tenia dos commits y la seccion solo podia cubrir uno.

Y un tercero, encontrado al construir el fixture: **un fixture roto puede
producir un verde**. El `git-wrapper` de esta maquina se niega a firmar commits en
un repo sin identidad, luego los commits no se creaban, el gate leia un repo
vacio, y **C0 pasaba por el motivo equivocado** — un fixture sin integridad
ocupa el lugar del que si mediria. Por eso el test afirma **integridad del
fixture antes de medir nada sobre el**.

## Lo que la resolucion NO arregla

- **El clon local sigue quedandose viejo tras cada publicacion.** Ahora el gate
  lo detecta y va a por el tag correcto, asi que el efecto esta neutralizado;
  pero el clon viejo sigue siendo el estado normal de este checkout y lo que se
  pago hoy fue **un `git fetch --tags`**. La causa —`gh release create` publica
  en el remoto y nada actualiza el clon— sigue viva y no tiene guard.
- **La mitigacion sigue siendo manual.** Si alguien borra el remoto de un clon,
  el gate vuelve legitimamente a la lista local, que es el comportamiento
  correcto, y ese clon vuelve a estar viejo respecto de un remoto que ya no ve.
