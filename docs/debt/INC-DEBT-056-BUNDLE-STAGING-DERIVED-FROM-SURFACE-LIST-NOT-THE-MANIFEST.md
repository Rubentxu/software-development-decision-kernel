---
id: INC-DEBT-056-BUNDLE-STAGING-DERIVED-FROM-SURFACE-LIST-NOT-THE-MANIFEST
title: "El staging del bundle era una quinta copia del contrato, no el manifest"
severity: high
priority: P1
status: resolved
opened: session-65h
resolved: session-65h
resolution: Ruta 1 (la autoridad manda — el staging se deriva del manifest)
component: sddk-cli, release
surface: scripts/release.sh, tests/test_bundle_surface_coverage.py, .github/workflows/release.yml
---

# INC-DEBT-056: el staging del bundle era una quinta copia del contrato

> **RESUELTA (session-65h).** `scripts/release.sh`aba las superficies con una
> lista escrita a mano (`cp -r agents skills assets …`) y luego las nombraba
> otra vez en el `tar`. Dos defectos medidos, **ambos naciendo de la misma
> causa**: el staging no se derivaba de la autoridad que ya existía.

## Los dos defectos

**(a) El `tar` nombraba superficies que el staging nunca copiaba.**

Al añadir `specs` y `docs/impeccable-reference` a la lista del `tar` se
obvió añadirlas a la del `cp -r`. Reproducido con la fase 5 aislada:

```
tar: specs: No se puede efectuar stat: No existe el fichero o el directorio
tar: docs/impeccable-reference: No se puede efectuar stat: No existe el fichero o el directorio
tar: Se sale con estado de fallo debido a errores anteriores
exit=2
```

`set -euo pipefail` (`release.sh:55`) convierte esto en un release abortado,
no en uno corrupto — el fallo es ruidoso. Pero el release **no se habría
completado**, y la causa es que la sexta superficie se añadió a una de las dos
copias.

> Un subdirectorio necesita además su **padre** creado antes del `cp -r`, o
> aterriza plano como `dst/<hoja>` y el `tar`, que pide el camino con
> prefijo, no encuentra nada que empaquetar. Regla ya documentada para
> `prompts/sddk`; `docs/impeccable-reference` la sufria por primera vez.

**(b) El `cp -r` copiaba ficheros que el manifest no lista.**

`cp -r <superficie>` copia lo que hay en disco, **incluido lo que `.gitignore`
excluye**. Contando ficheros reales del tar contra entradas del manifest:

```
superficie                     tar  manifest
agents                          73        72
skills                         245       245
prompts/sddk                    44        44
assets                          18        17
specs                           14        14
docs/impeccable-reference         2         2
```

Los dos sobrantes, ambos untracked y ambos cubiertos por `.gitignore`:

| fichero | regla | qué es |
|---|---|---|
| `agents/.atl/.skill-registry.cache.json` | `.gitignore:26` (`.atl/`) | caché local de `skill-registry` |
| `assets/agent-models.yaml.bak` | `.gitignore:17` (`*.bak`) | backup de la tabla de modelos |

**(c) Los dos que introdujo el arreglo mismo.** No se plagaron: los
**produjo** la derivación por manifest, y solo aparecieron al **ejecutar**
el bundle completo, no al leer el código.

- **`MANIFEST.sha256` dejó de viajar.** El manifest no puede listarse a sí
  mismo —un fichero no puede contener su propio digest—, luego derivar el
  staging de él descarta el fichero que el bundle más necesita.
  `release.yml:230` aborta con `bundle lacks MANIFEST.sha256` y `update.rs`
  lo trata como **required**: el release se habría roto en la ruta cloud y
  en cada instalación.
- **El prefijo del tarball se duplicó en los 396 miembros.** El `--xform`
  transforma el *nombre del miembro*, y al recibir el directorio
  `software-development-decision-kernel` ya envuelto le prepende el mismo
  prefijo otra vez:
  `software-development-decision-kernel/software-development-decision-kernel/…`.
  Medido: **608 miembros** con doble prefijo.

Que (c) lo produjera el arreglo de (a) y (b) es la parte incómoda: la
corrección se verificó con el guard de superficies —12/12 verde y 9
mutaciones— sin que ninguno **empaquetara**. Un guard que no empaqueta no
puede observar un tarball mal empaquetado. Ambos nacen de la misma razón:
todo lo que se verificó fue el *staging*, nunca el artefacto.

## Criterio verificable

Defecto (b), medido antes del arreglo, con el tar real de la fase 5:

```
$ tar tzf bundle.tar.gz | grep -E '\.bak|\.atl'
software-development-decision-kernel/agents/.atl/.skill-registry.cache.json
software-development-decision-kernel/assets/agent-models.yaml.bak
```

Defecto (a), RED medido sobre la fase 5 aislada: `exit=2`.

Defecto (c), RED medido ejecutando el paso 5 completo y extrayendo el
resultado (`tests/test_release_bundle_step5.sh`, entonces sin corregir):

```
  FAIL — MANIFEST.sha256 does not ship
  FAIL — BUNDLE.toml missing
  FAIL — doubled wrapper prefix on 608 members
```

Consecuencia de (b): **`manifest_sha256` en `BUNDLE.toml` no describía el
propio tarball**. Dos ficheros viajaban sin digest, no verificados por
`verify_manifest` en la instalación, y no reinstallables de forma
reproducible. La ruta cloud (`release.yml`) no sufre (b): empaqueta un
checkout limpio, donde ese debris no existe.

## Resolución — Ruta 1: la autoridad manda

El staging se **deriva de `MANIFEST.sha256`**, que step 4 ya verifica
fail-closed y que por tanto es la única declaración de qué se publica:

```bash
awk '{print $2}' MANIFEST.sha256 | xargs -d '\n' cp --parents -t "$BUNDLE_STAGE"
cp MANIFEST.sha256 "$BUNDLE_STAGE/"   # el manifest no puede listarse a sí mismo
```

Esto elimina las dos clases de defecto a la vez: lo que el manifest lista
viaja, y **nada más puede viajar**. La lista escrita a mano desaparece, con
lo que desaparece también el coste de editarla cada vez que se añade una
superficie — que es exactamente lo que produjo (a). El segundo `cp` es la
corrección de (c): el manifest es el único fichero que el manifest no
puede declarar, luego se añade explícitamente.

El `--xform` se aplica a `./ruta` y no al nombre del directorio, para que el
prefijo envuelto salga una sola vez (corrección de (c)).

Se añade un contrato fail-closed en la misma fase: el conjunto de ficheros
del staging debe ser **exactamente** el del manifest más `MANIFEST.sha256` y
`BUNDLE.toml`, y ambos deben estar presentes — que es lo que
`update.rs` exige al desinstalar.

Las dos rutas de producción ya no enuncian el mismo hecho de la misma forma,
lo cual es correcto y no una divergencia:

| ruta | cómo declara el contenido | por qué |
|---|---|---|
| `release.sh` (local) | deriva del manifest | corre en un árbol de trabajo donde **sí** hay debris |
| `release.yml` (cloud) | lista explícita | empaqueta un checkout limpio; la lista es legible y auditable |

## El guard no vio ninguno de los dos

`tests/test_bundle_surface_coverage.py` (session-65g) comparaba la lista de
superficies del `tar` de `release.sh` contra `MANIFEST_SURFACES`. Ese test
pasaba con las dos partes rotas, y no por casualidad: **los dos defectos son
invisibles a una comparación de listas**.

- (a) metía `specs` en la lista del `tar`, así que la comparación pasaba. El
  `cp -r` no estaba en ninguna lista que el gate mirara.
- (b)metía ficheros que **no están en ninguna lista**, porque no deben
  estarlo. Ninguna comparación de listas puede ver un fichero que
  correctamente no aparece en ninguna.

El gate se reescribió para atar la propiedad, no la lista: el staging se
deriva del manifest, el `tar` empaqueta el árbol entero, el manifest casa
con lo que git trackea en ambas direcciones, y un **canario** untracked bajo
una superficie real no llega al staging. 12 tests, 9 mutaciones
falsificadoras observadas.

## Lo que este caso confirma sobre los guards

Un guard que compara dos listas solo detecta la **divergencia entre
declaraciones**. No detecta que una declaración deje de ser la que se
obedece, ni lo que se publica sin declarar. Las dos cosas que encontré en
esta sesión aparecieron solo al **ejecutar** el staging y contar ficheros.

Los dos puntos ciegos que la falsificación encontró en el propio guard nuevo
(ya corregidos) eran de la misma familia:

1. `assertNotIn("agents", members)` + `len(members) == 1` — una mutación que
   nombrara `software-development-decision-kernel/agents` lo satisfacía: un
   subconjunto sigue siendo un miembro, solo que el inesperado. La
   contención no es la igualdad.
2. `test_staged_tree_matches_the_manifest_exactly` comparaba el manifest
   **consigo mismo**: ambas partes se derivaban de él, así que borrar una
   entrada la borraba de las dos y el gate seguía verde mientras el fichero
   dejaba de publicarse en silencio. Tautológico. El test nuevo se ancla en
   `git ls-files`, una autoridad que el manifest no puede definir para sí
   mismo.

Sexta vez que un falsador encuentra en sí mismo lo que la inspección no.

## El test que faltaba: ejecutar el artefacto, no el staging

Los defectos (c) los produjo el arreglo y **no los vio ninguno de los dos
gates**: 12/12 el de superficies, 9/9 mutaciones, y el paso 5 seguía roto.
La razón es una sola y ya se había dicho media vuelta arriba: todo lo que se
verificó fue el *staging*, nunca el **artefacto**. El guard compara código; el
defecto estaba en lo que ese código produce.

`tests/test_release_bundle_step5.sh` lo ejecuta de punta a punta: stage →
`tar` → extraer → y preguntar al bundle **extraído** lo que preguntarían
`install.sh` y `update.rs`:

```
ok   — MANIFEST.sha256 ships (release.yml:230 and update.rs require it)
ok   — BUNDLE.toml ships at the wrapper root
ok   — single wrapper directory (no doubled prefix)
ok   — anchor matches the manifest that shipped
ok   — all 394 manifest entries present in the bundle
ok   — all digests verify
ok   — no gitignored artefact rode along
     bundle carries 396 files
```

3 mutaciones falsificadoras, una por defecto, las tres detectadas: quitar el
`cp` del manifest, volver al `--xform` sin `^\./`, y quitar el prefijo
`sha256:` del ancla. Cada una muere en el check que le corresponde.

El mismo día, `tests/test_release_ci_manifest_anchor.sh` solo miraba
`release.yml` (`WF=`), con `release.sh` apareciendo **una vez, en un
comentario**. Ese gate existía para converger los productores del ancla y
estaba estructuralmente incapaz de ver que `release.sh` seguía divergiendo:
seguía escribiendo hex desnudo. Extendido a los dos productores, y en rojo
sobre el estado real, que es la forma en que un gate demuestra que mira.
