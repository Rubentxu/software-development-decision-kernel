---
id: INC-DEBT-027-MUSL-TOOLCHAIN-ABSENT-LOCAL-PUBLISH
title: "El publish local de v2.2.1 aborta: falta el cross-compiler musl, que exige root"
status: open
severity: high
priority: P1
created: 2026-09-28
discovered_by: session-29 (OBSERVED, ejecución real de scripts/release.sh)
cluster_id: CL-SUPPLY-CHAIN
related: [INC-DEBT-021-MUSL-ASSET-NAME-LIE, INC-021]
fingerprint: "release_publish_local_requires_musl_cross_gcc"
---

## Qué es

`scripts/release.sh` publica el asset unificado `sddk-linux-x86_64-musl.tar.gz`
y por eso compila **obligatoriamente** contra `x86_64-unknown-linux-musl`
(`release.sh:454-457`). La crate `ring v0.17.14` necesita un compilador C
musl. En esta máquina el target de Rust está instalado pero el compilador C no:

```text
$ rustup target list --installed | grep musl
aarch64-unknown-linux-musl
x86_64-unknown-linux-musl          # target Rust: OK

$ command -v x86_64-linux-musl-gcc
                                          # vacio: falta el compilador C
$ dpkg -l | grep musl
                                          # vacio: musl-tools no instalado
```

Resultado observado del intento real de publicar v2.2.1 (session-29):

```text
error occurred in cc-rs: failed to find tool "x86_64-linux-musl-gcc": No such file or directory
  ✗ cargo build failed para target x86_64-unknown-linux-musl
```

El script hace `die` (exit no-cero) en el paso 3/14, **antes** de cualquier
paso que publique. No hay estado parcial.

## Por qué NO es un bug de la ruta de release

El guard es correcto y deliberado. `INC-DEBT-021-MUSL-ASSET-NAME-LIE` se cerró
en session-19 construyendo un binario musl **real** justamente para no volver
a publicar un asset llamado musl que contiene un binario glibc
(`release.sh:466-469` aborta si `file` no dice "statically linked"). Publicar
en otro target rompería el contrato de `install.sh`. El fallo es la ausencia
del toolchain, no una debilidad del gate.

## Por qué no es recuperable sin operador

`sudo -n true` falla: sudo **requiere password** en esta máquina. No existe vía
no interactiva para instalar `musl-tools` / `gcc-x86-64-linux-musl`. Por eso no
se lanza `sudo apt-get install` — sería un prompt de password no respondible.

## Recuperación (una de estas, ambas del operador)

1. **Instalar el toolchain** (con sudo, en terminal humana):
   ```bash
   sudo apt-get install -y musl-tools gcc-x86-64-linux-gnu
   ```
   Después `bash scripts/release.sh` publica v2.2.1 sin más cambios.
2. **Publicar v2.2.1 por CI**, que sí tiene el toolchain. El guard de
   `release-automation.yml` ya es correcto (fixed en session-28), así que
   push a main con el bump ya presente es exactamente el disparador esperado.

Opción 2 es la de menor fricción: el push ya está hecho (`ebd3ed8d`) y el
workspace ya está en 2.2.1.

## Impacto

- v2.2.1 **no** se ha publicado. Sin tag, sin release, sin assets parciales.
- El código de release (doble bump + `manifest_sha256`) sigue **sin publicar**;
  la release pública sigue siendo v2.0.1.
- El push a main sí está hecho y el gate pre-push lo aceptó por clause (A).

## Colisión de id detectada (NO resuelta aquí)

`INC-DEBT-021` está reclamado por **dos** deudas distintas:

- `docs/debt/INC-DEBT-021.md` → clippy baseline (low/P2, closed)
- `docs/debt/INC-DEBT-021-MUSL-ASSET-NAME-LIE.md` → asset musl (high/P1, closed)

Renumerar historia publicada está prohibido por `docs/history/README.md`, así
que esto queda declarado y no renumerado. Se registra aquí como observación
para una decisión de política futura.
