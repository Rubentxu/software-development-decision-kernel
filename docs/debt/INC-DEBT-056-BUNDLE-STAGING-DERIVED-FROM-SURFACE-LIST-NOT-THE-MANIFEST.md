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

Consecuencia: **`manifest_sha256` en `BUNDLE.toml` no describía el propio
tarball**. Dos ficheros viajaban sin digest, no verificados por
`verify_manifest` en la instalación, y no reinstallables de forma
reproducible. La ruta cloud (`release.yml`) no sufre (b): empaqueta un
checkout limpio, donde ese debris no existe.

## Criterio verificable

El defecto (b) medido antes del arreglo, con el tar real de la fase 5:

```
$ tar tzf bundle.tar.gz | grep -E '\.bak|\.atl'
software-development-decision-kernel/agents/.atl/.skill-registry.cache.json
software-development-decision-kernel/assets/agent-models.yaml.bak
```

Y el defecto (a), RED medido sobre la fase 5 aislada: `exit=2`.

## Resolución — Ruta 1: la autoridad manda

El staging se **deriva de `MANIFEST.sha256`**, que step 4 ya verifica
fail-closed y que por tanto es la única declaración de qué se publica:

```bash
awk '{print $2}' MANIFEST.sha256 | xargs -d '\n' cp --parents -t "$BUNDLE_STAGE"
```

Esto elimina las dos clases de defecto a la vez: lo que el manifest lista
viaja, y **nada más puede viajar**. La lista escrita a mano desaparece, con
lo que desaparece también el coste de editarla cada vez que se añade una
superficie — que es exactamente lo que produjo (a).

Se añade un contrato fail-closed en la misma fase: el conjunto de ficheros
del staging debe ser **exactamente** el del manifest más `BUNDLE.toml`.

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
