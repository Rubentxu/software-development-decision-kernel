---
id: INC-DEBT-021-MUSL-ASSET-NAME-LIE
title: "El asset `sddk-<tag>-sddk-linux-x86_64-musl.tar.gz` no contiene un binario musl"
status: open
severity: high
priority: P1
created: 2026-09-27
discovered_by: session-16 audit (OBSERVED, ejecución real del e2e)
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
