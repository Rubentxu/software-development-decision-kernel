---
id: INC-DEBT-021-MUSL-ASSET-NAME-LIE
title: "El asset `sddk-<tag>-sddk-linux-x86_64-musl.tar.gz` no contiene un binario musl"
status: closed
resolved: 2026-09-27T20:59Z
resolved_by: session-19
severity: high
priority: P1
created: 2026-09-27
discovered_by: session-16 audit (OBSERVED, ejecución real del e2e)
closed_by: session-19 (OBSERVED: build musl real compilado y ejecutado en Debian 12 y Alpine 3.20)
cluster_id: CL-SUPPLY-CHAIN
fingerprint: "release_mislabelled_musl_asset_glibc_build"
---

## Qué es

`scripts/release.sh` compila **un único** binario con
`cargo build --release` (step 3/14) y lo empaqueta tal cual en el asset
unificado:

```bash
UNIFIED="$TMP/sddk-${TAG}-sddk-linux-x86_64-musl.tar.gz"
cp "$BIN" "$PACK/bin/sddk"
```

`$BIN` es el binario del **host de build**, dinámicamente enlazado
contra el glibc de ese host. El nombre del asset promete un binario
musl (estático o enlazado contra musl) que **no existe**.

## Evidencia (OBSERVED, no inferida)

Binario publicado en `v2.0.1`, ejecutado en un contenedor limpio:

```
$ docker run --rm debian:12-slim /tmp/sddk --version
/tmp/sddk: /lib/x86_64-linux-gnu/libc.so.6: version `GLIBC_2.39' not found
            (required by /tmp/sddk)
```

Confirmado en el host de build:

```
$ file .../release/sddk
ELF 64-bit LSB pie executable, x86-64, dynamically linked,
interpreter /lib64/ld-linux-x86-64.so.2, for GNU/Linux
$ ldd --version
ldd (GNU libc) 2.43
```

Es decir: el asset dice `musl`, contiene un binario **glibc 2.39**.

## Por qué importa

1. **Compatibilidad no entrega.** Un usuario en una distribución con
   glibc < 2.39 (debian:12 ships 2.36, ubuntu 22.04 ships 2.35)
   **no puede ejecutar el binario publicado**. La instalación falla.
2. **El nombre es una promesa incumplida.** La única garantía real de un
   nombre `musl` es "no depende de la libc del sistema". Aquí no se
   cumple, y el nombre es la única señal que tiene el usuario.
3. **Enmascaró el e2e.** `scripts/e2e-install.sh` usaba
   `debian:12-slim` (glibc 2.36) como imagen base. El binario no podía
   ejecutarse, así que **todos** los checks fallaban y la suite nunca
   pudo pasar. Nadie lo ejecutó en verde porque era imposible.

## Impacto observado en la suite e2e

Con `debian:12-slim` (imagen original), la variante `a` fallaba
**11 de 11** checks con el único error real siendo el libc:

```
❌ install.sh exit code
❌ binary not installed
✅ sha256 verified
❌ agents linked: 0
... (11 checks, 10 fallos)
```

En session-16 se cambió la imagen base a `debian:13-slim` (glibc 2.41)
como **mitigación del entorno de test**, con override
`SDDK_E2E_IMAGE`. Resultado: 4/4 variantes PASS. Eso valida el
instalador, **pero no arregla el artefacto**: sigue siendo glibc.

## Opciones

- **(a) Build musl real.** Compilar con
  `--target x86_64-unknown-linux-musl` y empaquetar ese binario. Es lo
  que el nombre ya promete. Coste: requiere el target toolchain en el
  host de release, y verificar que las dependencias (ring/sqlite/
  openssl si las hay) compilan contra musl. Riesgo: no trivial.
- **(b) Renombrar el asset** a lo que realmente es
  (`sddk-<tag>-linux-x86_64.tar.gz`) y documentar el requisito de glibc
  mínimo. Coste: bajo, pero rompe la URL pública existente y obliga a
  reemitir `install.sh` y la matriz de assets.
- **(c) Ambos**: publicar musl como asset principal y mantener el glibc
  como variante secundaria. Es lo que hace la mayoría de CLIs Rust
  (ripgrep, fd, bat).

## Recomendación

**(a) o (c)**, con (a) como mínimo. El nombre `musl` es una afirmación
técnica; mientras sea falsa, cualquier consumidor razonable asumirá
compatibilidad que no existe. Si se opta por (b) o (c), el cambio de
nombre debe ser un cambio **major** del contrato de distribución, no un
detalle cosmético.

## Causa raíz real (corregida en sesión-17)

Session-16 registró esto como "el nombre del asset miente". La
formulación es correcta pero **incompleta**: el problema de fondo es que
**existen dos pipelines de release en el repositorio, y sólo uno cumple el
contrato de assets**.

### Pipeline A — `scripts/release.sh` (el AUTORITATIVO)

- Se ejecuta **localmente** y es el que produce los releases reales
  (verificado: `v2.0.1` lo publicó este script, 14/14 pasos).
- Step 3/14 compila **una** vez: `cargo build --release --offline --bin sddk`.
  Eso produce un binario **glibc**, dinámicamente enlazado al host.
- Empaqueta ese mismo binario glibc en
  `sddk-${TAG}-sddk-linux-x86_64-musl.tar.gz` (L436) y lo publica
  junto a un `sddk` pelado con el mismo contenido glibc.
- **No tiene toolchain musl**: compilar con
  `--target x86_64-unknown-linux-musl` falla con
  `cc-rs: failed to find tool "x86_64-linux-musl-gcc"`.

### Pipeline B — `.github/workflows/release.yml` (manual, nunca automático)

Este pipeline **SÍ hace las cosas bien**:

```yaml
- target: x86_64-unknown-linux-musl          # L29
- name: Install musl cross tools             # L53
  run: sudo apt-get install -y musl-tools    # L57
  env: CARGO_TARGET_..._LINKER: musl-gcc     # L61-62
  run: cargo build --release --target ...    # L63
- cp dist-out/dist/sddk "assets/sddk-linux-x86_64-musl"   # L71
```

Compila musl estático de verdad y publica el asset **con el nombre que
`install.sh` esperaba**.

Pero su trigger es `workflow_dispatch` (L11-12) y su propio encabezado
declara: *"MANUAL-ONLY: SDDK never depends on CI/CD. The automatic
`push: tags: v*` trigger was removed so no run is ever queued by a
release"*.

**Consecuencia**: el pipeline que construye el artefacto correcto existe
y funciona, pero nunca corre. El que corre no construye el artefacto
correcto. Y `install.sh` fue escrito contra el contrato de **A** (el del
pipeline B), que **A** nunca cumplió — de ahí el 404 de
INC-DEBT-022: `install.sh` pedía `sddk-linux-x86_64-musl`, que es
exactamente el nombre que publica B.

### Las dos salidas

- **(a) Build musl real en `release.sh`.** Requiere `musl-tools` en el
  host de release. **Bloqueado en el entorno actual**: sin `sudo`, sin
  `apt`, sin `musl-gcc` (`command -v musl-gcc` → ausente). El target de
  Rust `x86_64-unknown-linux-musl` sí está instalado, pero sin el linker
  de C `rusqlite bundled` no compila. Verificado empíricamente, no
  supuesto.
- **(b) Declarar `release.sh` como la autoridad y renombrar sus assets**
  a lo que son (glibc), eliminando la palabra `musl` del contrato.
  Rompe la URL pública del tarball: cambio **major** del contrato de
  distribución.
- **(c) Declarar `release.yml` como la autoridad** y hacer que el release
  real pase por él. Contradice el principio de la propia cabecera del
  workflow (*"SDDK never depends on CI/CD"*) y devuelve el gate a la
  nube, que según AGENTS.md §2.5 **no debe** ser bloqueante.

## Recomendación

**(a) a medio plazo, (b) como parche inmediato si (a) no puede hacerse
ya.** Instalar `musl-tools` en el host de release no debería requerir
sudo en un host de build controlado; si lo requiere, eso es un problema
de infraestructura del host, no del repo, y conviene resolverlo ahí.

Lo que **no** es admisible es mantener el estado actual: un asset cuyo
nombre afirma una propiedad técnica que su contenido no tiene.

## Estado

**Abierto.** Session-16 aplicó sólo la mitigación del entorno de test
(imagen base del e2e) y dejó el artefacto sin tocar, porque arreglar el
build es un cambio de release pipeline que no cabe en un fix de
installer. La regresión está pineada por
`tests/test_install_asset_contract.sh` check 8, que falla mientras el
nombre siga prometiendo musl sin que exista build musl.

## Relación

- El mismo patrón de "nombre que no describe el contenido" causó el bug
  de nombre de asset del binario (`sddk-linux-x86_64-musl` vs `sddk`),
  corregido en session-16. Ver INC-DEBT-022.
- La ausencia de firma (integridad sí, autenticidad no) es
  `INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY`, abierta e independiente.

---

## Resolución (session-19, 2026-09-27T20:41–20:59Z)

### La premisa de session-17 era falsa: el build musl NO estaba bloqueado

Session-17 concluded que el build musl era imposible en este host:

> "el target de Rust **sí** está instalado; falta el linker de C que
> necesita `rusqlite bundled`. El host no tiene `sudo`, ni `apt`, ni
> `musl-gcc`."

Eso era verdad **y no-era la causa**. `musl-gcc` es el wrapper de los
toolchains glibc cruzados (Debian/Ubuntu, vía `musl-tools`); no existe
en Alpine, donde la libc ES musl y `gcc` ya compila contra ella. Busqué
un componente que solo hace falta en la plataforma que no estoy usando.

La vía real, verificada:

```text
$ podman run --rm -v "$PWD":/src:z --security-opt label=disable \
    rust:1.91-alpine sh -c 'apk add musl-dev build-base; CC=gcc \
    cargo build --release --target x86_64-unknown-linux-musl --bin sddk'
    Finished `release` profile [optimized] target(s) in 6m 48s
```

Dos detalles no obvios, ambos encontrados por ejecución y no por lectura:

1. El bind-mount de este host (ext4 con `seclabel`) necesita
   `--security-opt label=disable` **y** `:z`. Sin ellos el montaje es
   visible en `/proc/mounts` pero inaccesible, y `cargo` falla con
   "could not find Cargo.toml" — un error que miente sobre la causa.
2. `cargo build` sin `--target` reutiliza `target/` y puede no dejar el
   binario donde uno espera. Con `--target` el output va a
   `target/<triple>/release/`.

### El binario resultante es musl de verdad

```text
$ file target/x86_64-unknown-linux-musl/release/sddk
ELF 64-bit LSB pie executable, x86-64, static-pie linked, not stripped
$ ldd  →  not a dynamic executable / statically linked
$ ./sddk --version  →  sddk 2.0.5
```

### Y funciona donde el binario glibc fallaba

Session-16 observó que el binario glibc del host no arranca en Debian 12
(`GLIBC_2.39 not found`). El mismo test con el binario musl:

| Contenedor | libc | Resultado |
|---|---|---|
| `debian:12-slim` | glibc 2.36 | `sddk 2.0.5` — funciona |
| `alpine:3.20` | musl, sin glibc | `sddk 2.0.5` — funciona |

Ese es el criterio de aceptación real: no "compila", sino "corre en un
sistema que el binario anterior no soportaba".

### El fix

`scripts/release.sh` ahora compila con el target musl
(`SDDK_RELEASE_BUILD_TARGET`, default `x86_64-unknown-linux-musl`) y
**verifica el linkage antes de publicar**: `file` debe decir
`statically linked`, si no aborta. Publicar un binario dinámico con
nombre musl es exactamente este INC, así que falla cerrado en el paso 3
y no en el paso 9, cuando ya es caro.

El target sigue siendo configurable porque no todos los hosts tienen el
toolchain, pero desviarlo del musl emite un warning explícito en vez de
ser un cambio silencioso.

### La decisión de autoridad entre los dos pipelines

Session-17 la dejó abierta por falta de toolchain. Resuelta con
evidencia, no con preferencia:

- `release.yml` es `workflow_dispatch`-only. No ha publicado nunca un
  release. Un pipeline que nunca ha publicado no puede ser la autoridad
  del contrato de assets.
- `release.sh` es el que produjo todos los tags, y es el único que puede
  correr la admisión y el round-trip de instalación localmente.

`release.sh` es autoritativo. **No se unifican los nombres de asset**: el
nombre desnudo `sddk-linux-x86_64-musl` pertenece al contrato de
`release.yml` (matriz por-arch, multi-OS), y publicar el mismo binario
bajo los dos nombres daría dos rutas de descarga para un artefacto que
volverían a divergir. `install.sh` ya consume el asset unificado
(corregido en session-16, INC-022 cerrado).

El guard deriva esa autoridad de un hecho observable
(`workflow_dispatch` presente + ausencia de trigger automático), no de
una constante que alguien pone a 1.

### Falsificación del guard

`tests/test_release_pipeline_consistency.sh` pasa en verde sin haber
tocado sus aserciones para forzar el verde. Y sigue detectando las dos
formas de reincidir:

| Inyección | Resultado |
|---|---|
| Revertir a `cargo build --release` sin `--target` | FAIL |
| Cambiar el default del target a `x86_64-unknown-linux-gnu` | FAIL |

`shellcheck` clean en ambos ficheros.

### Lo que queda pendiente

El asset público `v2.0.1` **sigue siendo glibc con nombre musl**. El
fix está en `main` pero no se publica hasta el próximo release. Mientras
exista `v2.0.1` comoLatest, un usuario que descargue ese asset tiene un
binario que no arranca en su máquina. Cerrar del todo el problema exige
un release nuevo; el código ya está listo para ello.
