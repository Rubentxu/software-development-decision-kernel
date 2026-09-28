---
id: INC-DEBT-034-CI-RELEASE-ASSET-LAYOUT-DIVERGES-FROM-GATE
title: "La publicación por Actions produce un layout de assets que el gate local rechaza, y arrastra el directorio assets/ del repo al release"
status: open
severity: high
priority: P1
created: 2026-09-28
discovered_by: session-31 (OBSERVED, publicación real de v2.2.6)
cluster_id: CL-RELEASE
related: [INC-DEBT-030-LOCAL-RELEASE-BLOCKED-AT-SIGNING-IDENTITY, INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY]
fingerprint: "ci_release_assets_star_globs_repo_assets_dir"
---

## Qué pasó (OBSERVED, session-31)

Publicación real de **v2.2.6** por la vía de Actions (la que parecía
bloqueada, y no lo estaba). El resultado es un release **publicado,
incompleto y contaminado**.

Estado real tras el run `36452986413` (conclusion `failure`):

- `gh release view v2.2.6` → existe, `isDraft=false`, `isPrerelease=false`.
- Tag `v2.2.6` → `be13b539a7a283fb5a15149e767c7a5a4d899fb9` (= `origin/main`).
- **11 assets**, de los cuales:
  - `agent-models.yaml` (7648 bytes) **no pertenece al release**: es un
    fichero del directorio `assets/` del repo, que viaja en el bundle
    (55 entradas de `assets/` en `MANIFEST.sha256`).
  - 0 firmas `.sig`, 0 `.pem`, 0 `.bundle.json`.
- Jobs `Unified artifact`, `Sign release assets` y `Smoke test installer`:
  **skipped** (dependen de los jobs que fallaron).

## Causa raíz 1 — el glob `assets/*` barre el directorio del repo

`release.yml` stagea con:

```yaml
- name: Stage assets
  run: |
    mkdir -p assets
    cp dist-out/dist/sddk "assets/${{ matrix.asset }}"
    ...
- name: Publish to GitHub Releases
  run: |
    gh release upload "$TAG" assets/* --clobber --repo ...
```

`assets/` **ya existe en el repo** — es la superficie del bundle
(`assets/agent-models.yaml`, `assets/agent-models/tui.sh`,
`assets/explorer/template.html`; 17 ficheros versionados). El
`mkdir -p` no crea un staging limpio: se mezcla con lo que ya está
versionado, y `assets/*` lo arrastra al release.

Error exacto en el log del job:

```
Post "…/releases/398438640/assets?label=&name=agent-models":
  read assets/agent-models: is a directory
```

`assets/agent-models` es un subdirectorio, y `gh release upload` no lo
sabe manejar. El `--clobber` tampoco ayuda: el problema no es colisión
de nombre, es que el glob Rekurse.

Este es un fallo **nuevo**, no observado antes: hasta session-31 el
workflow de CI nunca se había ejecutado (0 runs históricos). La
pregunta natural es si el bundle job y el publish job comparten el
working directory — **sí**, ambos jobs corren en el mismo workspace del
runner, y "Bundle framework assets" corrió **antes** y con `success`.

## Causa raíz 2 — el layout de CI y el contrato del gate local divergen

`tests/lib_public_release_gate.sh:68` define los assets canónicos:

```
sddk
sddk.sha256
sddk-v$VERSION-sddk-linux-x86_64-musl.tar.gz
sddk-v$VERSION-sddk-linux-x86_64-musl.tar.gz.sha256
CHECKSUMS
sbom.json
gh-release-receipt.json
software-development-decision-kernel.tar.gz
software-development-decision-kernel.tar.gz.sha256
```

El workflow de CI produce **nombres distintos**:

| gate local espera | CI produce |
|---|---|
| `sddk`, `sddk.sha256` | `sddk-linux-x86_64-musl`, `sddk-linux-x86_64-musl.sha256` |
| `CHECKSUMS`, `sbom.json` | `sddk-linux-x86_64-musl.CHECKSUMS`, `sddk-linux-x86_64-musl.sbom.json` |
| `gh-release-receipt.json` | **nada** — solo `scripts/release.sh` lo genera |

O sea: **`gh-release-receipt.json` solo existe en la vía local**, y la vía
local está bloqueada por la identidad de firma (`INC-DEBT-030`). Las dos
vías de publicación no cubren el mismo contrato, y la que puede publicar
(no bloqueada) es la que no cumple el contrato del gate.

Esto no es un defecto del gate: es un **gap estructural** entre dos
autores del mismo contrato que nunca se cruzaron, porque hasta ahora
ninguno de los dos se había ejecutado end-to-end.

## Por qué no lo arreglo aquí

El arreglo de la causa raíz 1 es de una línea y es obvious: usar un
staging con nombre propio (`dist-out/release-assets/`) en vez de `assets/`.

El de la causa raíz 2 **no es una línea**, y decidirlo aquí sería
inventar política de distribución:

- ¿Se adapta el gate local al layout de CI, o al revés?
- Si es el gate: cambia el contrato que 4 releases públicos ya satisfacen.
- Si es CI: hay que decidir si `gh-release-receipt.json` se genera en CI
  (y con qué contenido: el receipt declara el actor, el tag y los hashes
  locales), y si se abandona el layout por-target o se publica un único
  `sddk` canónico.

Además, el release publicado está **contaminado**: tiene un asset ajeno y
le faltan las firmas. Eso exige una decisión de reemisión (borrar y
republicar, o publicar 2.2.7 encima), y publicar es irreversible.

## Lo que sí está verificado

- v2.2.6 **existe** y es instalable-en-principio, pero no cumple el
  contrato de assets ni lleva firma. **No se puede declarar publicación
  completa.**
- La vía de Actions **sí funciona** (refuta el supuesto de
  `INC-DEBT-030` de que los minutos estuvieran agotados), y produce
  binarios musl reales.
- El job `Bundle framework assets` tuvo `success`, así que el bundle
  con el ancla corregida se generó y subió.

## Acción siguiente propuesta

1. Corregir el staging de CI (`assets/` → directorio propio) — slice de una
   línea, verificable con `act` o con un run real.
2. Resolver la **causa raíz 2** como decisión explícita: un único contrato
   de assets, con un único autor. Hoy hay dos y no coinciden.
3. Decidir qué hacer con v2.2.6: se deja como está (documentando que está
   incompleto y sin firmar) o se publica 2.2.7 que lo corrija. **No borrar
   un release ya publicado sin autorización del operador.**

Mientras tanto: **v2.0.1 sigue siendo el último release íntegro.** Cualquier
instalación debe apuntar a esa etiqueta, no a v2.2.6.
