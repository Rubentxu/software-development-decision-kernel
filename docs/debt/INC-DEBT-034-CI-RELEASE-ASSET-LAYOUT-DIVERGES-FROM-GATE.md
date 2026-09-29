---
id: INC-DEBT-034-CI-RELEASE-ASSET-LAYOUT-DIVERGES-FROM-GATE
title: "La publicación por Actions produce un layout de assets que el gate local rechaza, y arrastra el directorio assets/ del repo al release"
status: closed
severity: high
priority: P1
created: 2026-09-28
discovered_by: session-31 (OBSERVED, publicación real de v2.2.6)
resolved: 2026-09-29 (session-33)
cluster_id: CL-RELEASE
related: [INC-DEBT-030-LOCAL-RELEASE-BLOCKED-AT-SIGNING-IDENTITY, INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY]
fingerprint: "ci_release_assets_star_globs_repo_assets_dir"
---

## Decisión de política (session-33) — el layout raíz ES el contrato

El INC pedía una decisión explícita: ¿se adapta el gate local al layout de CI,
o al revés? La pregunta estaba mal planteada, porque daba por hecho que eran
dos layouts en conflicto. **No lo son: hay un layout, y el de CI es el
correcto. Lo que estaba mal era la documentación que describía el otro.**

### Por qué el layout raíz no es arbitrario

Es **forzado por la estructura del propio pipeline**, no una elección de
estilo. El job `unified-artifact` de `release.yml` consume el bundle así:

```bash
tar xzf "$BUNDLE" -C "$WORK/framework"      # línea 193, SIN --strip-components
test -f "$WORK/framework/MANIFEST.sha256"   # línea 196, gate de fallo
```

Si el bundle se publicara envuelto bajo `software-development-decision-kernel/`,
el manifest caería en `$WORK/framework/software-development-decision-kernel/`
y el `test -f` de la línea 196 **fallaría**. El layout raíz no es una
convención: es la única forma que el consumidor del pipeline admite sin
desempaquetar.

`release.sh` (ruta local) sí envuelve, y por eso necesita desempaquetar a mano
para escribir `BUNDLE.toml` y rearmar el unified (`cp -r "$FW_DIR/." ...`).
Es más trabajo, no un contrato distinto: el envoltorio es un detalle interno
de cómo el script prepara el unified, y el artefacto final
(`framework/BUNDLE.toml`, `framework/MANIFEST.sha256`) sale idéntico.

### Qué se cambió

**El contrato documentado, no el productor.** `AGENTS.md` §8 paso 5 describía
la forma envuelta como si fuese la del bundle publicado. Se alineó con lo que
se publica y se consume.

El consumidor ya era correcto: `tarball_wraps_all_members_under_one_dir()`
(`2f7d5064`) detecta el layout antes de aplicar `--strip-components=1`, así que
tolera ambos y no borra el manifest en ninguno. Se conserva la tolerancia
deliberadamente: un consumidor que acepta las dos formas no se rompe cuando
la otra ruta de producción (local) produce la envuelta.

### Lo que NO se cambió, y por qué

- El gate de `verify_manifest` fail-closed. El rechazo de v2.2.17 fue
  **correcto**: el manifest no estaba donde el guard lo buscaba. Lo que estaba
  mal era la búsqueda, no el guard.
- El staging de CI. La causa raíz 1 (el glob `assets/*`) ya estaba corregida
  en `release.yml` con el staging `dist-out/release-assets/`, y v2.2.18 lo
  confirma: 27 assets, contrato de 9 canónicos + paquetes de plataforma
  + firmas, sin `agent-models.yaml`.

### Verificación de la decisión contra el release real

Sobre el `software-development-decision-kernel.tar.gz` publicado en v2.2.18
(descargado de la release, no de un artefacto local):

| Comprobación | Observado |
|---|---|
| Miembros bajo `software-development-decision-kernel/` | **0** |
| `MANIFEST.sha256` en la raíz | **1** |
| `agents/` con sus subdirectorios | sí (`agents/sddk-archive.md`, …) |
| `dev update` extrae y verifica | `369 files content-verified via MANIFEST.sha256` |
| Instalación real | `all_present: true`, `binary.bundle_coherence: present` |

La línea de releases v2.2.12→v2.2.18 (siete publicaciones) consumió seis
ciclos de CI en revelar seis defectos encadenados de la ruta de instalación.
Este era el último. La causa de fondo era que **el contrato del bundle estaba
escrito en un sitio y el bundle se producía en otro, y nada cruzaba ambos
campos**. Ahora coinciden y el consumidor tolera las dos formas.

## HALLAZGO session-33 — resuelto el conocimiento negativo de session-32

Session-32 dejó escrito, y así consta en su commit `2f7d5064`, que la
mutación "siempre `true`" **no puso los pins en rojo** pese a `cargo clean` y
recompilación forzada, y que "el mecanismo por el que el pin no detectaría ese
mutante sigue sin explicar". **Está explicado, y la explicación no es la
rutina.**

La causa: **`root_level_ci_layout_is_not_detected_as_wrapped` prueba la
FUNCIÓN, no el SITIO donde se la invoca.** El detector
`tarball_wraps_all_members_under_one_dir` sigue siendo correcto bajo el
mutante, así que el pin — que lo llama directamente — sigue verde. El
defecto que session-32 insertó estaba en el `if` del call site
(`if true { ... push("--strip-components=1") }`), y ningún test que ejercite
la función puede verlo.

**La forma del defecto es "función correcta detrás de un `if` equivocado"**,
que es justo la forma que session-32 no sospechó: se buscó un fallo en la lógica
y la lógica estaba bien.

Confirmado por ejecución en session-33: con `if true` en el call site, el
guard da **6/6 PASS** y el test unitario del detector da **verde**. Ninguna de
las dos capas existentes lo detecta. De ahí nace
`tests/test_release_bundle_layout.sh` (6 checks), que sí lo detecta, porque
parsea el bloque del push y exige que su condición nombre el flag que el
detector asigna.

### Falsificación del guard nuevo (OBSERVED, session-33)

| Dirección | Mutación | Resultado |
|---|---|---|
| Verde | árbol correcto | PASS 6/6, identifica `if strip_components` (línea 362) |
| **ROJO (productor)** | reenvolver el bundle en `release.yml` con `--xform` | **FAIL** check 1 |
| **ROJO (consumidor)** | `if true` en el call site del strip | **FAIL** check 5, nombrando la condición |

El case 5 se escribió **cuatro veces** y las tres primeras versiones eran
débiles, todas demostradas por la misma mutación:

1. "el detector aparece antes del push" → pasa con `if true`.
2. "el `if` de menor indentación arriba" → se sale de la función y coge uno
   de la ruta de firma (línea 85): **falso positivo** sobre código correcto.
3. "el `if` más cercano a indent-4" → igual: el cuerpo de la función es
   indent 4, así que un `if` sin relación en la misma función gana.
4. "el `if` más cercano **a la indentación del bloque**" → correcta.

Las tres primeras pasaban la mutación. Se documentan porque un guard que no
ha sido mutado no está verificado, y uno que se desataca de la mutación
produce falsos positivos que acaban borrados.

Nota sobre el primitive: buscar "el `if` más cercano" es la primitiva
equivocada. Lo que identifica la guarda es que el push esté dentro del bloque
que abre la condición, así que el `if` debe estar exactamente a la indentación
del bloque que contiene el push (4 espacios por debajo).



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

---

## Reconciliación session-43 — la "causa raíz 2" estaba obsoleta

Este documento ya figuraba `status: closed` desde session-33, pero
`docs/debt/README.md` lo seguía listando como `high/P1 open` y repetía
que la divergencia de layout seguía abierta. **Verificado en
session-43: la divergencia no existe.**

Lo que el documento daba por roto era que el layout de CI y el contrato
de `tests/lib_public_release_gate.sh` no coincidieran. Hoy ambos
sourcean `scripts/release-assets-contract.sh`, que es la fuente única, y
el escenario 12 de la suite lo comprueba explícitamente:

```console
$ bash tests/test_release_public_gate.sh
=== Scenario 12: shared production/test asset contract ===
  ✓ release.sh and gate tests source the same asset contract helper
  PASS=13  FAIL=0
```

Y los nombres que CI publica son exactamente los que el contrato exige
(`.github/workflows/release.yml`):

```rust
UNIFIED="sddk-${TAG}-sddk-linux-x86_64-musl.tar.gz"     # línea 314
(cd "$STAGE" && sha256sum "$UNIFIED" software-development-decision-kernel.tar.gz > CHECKSUMS)   # 319
for asset in sddk sddk.sha256 "$UNIFIED" "$UNIFIED.sha256" CHECKSUMS sbom.json; do           # 336
```

El release real más reciente lo confirma de forma independiente:
`v2.2.27` publica los 9 assets canónicos con esos nombres, más los
paquetes de plataforma y las firmas.

**El aviso "no instalar desde v2.2.6" sigue siendo válido** como hecho
histórico de esa tag concreta. Lo que caducó es la premisa de que el
defecto siga abierto en el HEAD actual.

### Lección de método

El índice de deuda y los documentos de deuda divergen, y **el índice es
lo que se lee** para priorizar. Una entrada que dice `open` cuando el
fichero dice `closed` no es un error de redacción: es un work item
falso que consume atención en cada sesión que lo revise. La regla que
sugiere este caso es que una reconciliación de deuda debería incluir un
check mecánico de coherencia índice↔documento, no sólo la corrección
manual. Queda anotado como candidato, no implementado.
