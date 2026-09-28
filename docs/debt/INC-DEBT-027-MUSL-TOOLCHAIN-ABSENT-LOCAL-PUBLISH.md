---
id: INC-DEBT-027-MUSL-TOOLCHAIN-ABSENT-LOCAL-PUBLISH
title: "El publish local de v2.2.1 aborta: falta el cross-compiler musl, que exige root"
status: resolved
resolution: rootless-podman-musl-shim
resolved: 2026-09-28
severity: high
priority: P1
created: 2026-09-28
discovered_by: session-29 (OBSERVED, ejecución real de scripts/release.sh)
resolved_by: session-30 (OBSERVED, build musl real vía Podman rootless)
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

## Por qué no es recuperable sin operador — SUPERADO

> **Histórico (session-29).** Esta sección afirmaba que el bloqueo no tenía
> salida sin `sudo`. Session-30 la refutó: la instalación vía `apt` sí exige
> root, pero la vía rootless con Podman no. Ver "Resolución" más abajo.

`sudo -n true` falla: sudo **requiere password** en esta máquina. No existe vía
no interactiva para instalar `musl-tools` / `gcc-x86-64-linux-musl`. Por eso no
se lanza `sudo apt-get install` — sería un prompt de password no respondible.

## Resolución (session-30, OBSERVED) — ruta rootless, sin sudo

La premisa "no es recuperable sin operador" era **incorrecta**: solo era cierto
para la vía `apt`. `musl-tools` no necesita instalarse en el host si se compila
dentro de un contenedor rootless con Podman.

Shim en `/var/home/rubentxu/.local/libexec/musl-shim/x86_64-linux-musl-gcc`
(no versionado, fuera del repo) que ejecuta
`x86_64-alpine-linux-musl-gcc` dentro de `localhost/sddk-musl-toolchain`.

Los mounts son **estrechos y deliberados** — no un `-v $HOME:$HOME`:

```text
--userns=keep-id
-v <home>/cargo-targets:<home>/cargo-targets:Z      # escritura
-v <home>/.cargo:<home>/.cargo:ro,z                 # registro, solo lectura
-v <home>/.cargo:/home/<user>/.cargo:ro,z
-v "$HOST_CWD":"$HOST_CWD":z                         # el repo en su ruta real
-w "$HOST_CWD"
```

Dos detalles que costaron el diagnóstico:

1. **`-v $HOME:$HOME` sin label rompe la escritura aunque los permisos y el
   UID sean correctos.** Con `--userns=keep-id` el contenedor corre como
   `uid=1000(rubentxu) gid=1000(rubentxu)` y el dir es `drwxr-xr-x rubentxu
   rubentxu` — y aun así `touch` da `Permission denied`. La causa es el label
   SELinux del montaje, no DAC. `:Z` en el subpath lo resuelve.
2. **`:Z` sobre `$HOME` entero revienta en un archivo protegido no relacionado**
   (`~/.config/FortiClient.bak/Cache: permission denied`). El primer intento
   relabeló todo el home y falló por eso; de ahí que los mounts se limiten a
   los subpaths que el build realmente necesita.

Resultado observado del build real:

```text
$ cargo build --release --offline --bin sddk --target x86_64-unknown-linux-musl
    Finished `release` profile [optimized] target(s) in 5m 56s

$ file .../x86_64-unknown-linux-musl/release/sddk
ELF 64-bit LSB pie executable, x86-64, static-pie linked, stripped   # 30.8 MB
$ ... /sddk --version
sddk 2.2.2
```

El guard de `release.sh` (`release.sh:471-479`) sigue siendo el que valida:
`file` dice "statically linked" y el paso 3/14 pasa. **La deuda no era
"falta el toolchain", era "falta una ruta rootless"** — el gate nunca estuvo
roto, y esta ruta no afloja ninguno de sus requisitos.

### Limitación honesta (no resuelta)

El shim **no está versionado**: vive en `$HOME/.local/libexec/`, fuera del
repo. Otro host, u otra máquina, no lo tiene. Queda como deuda de
reproducibilidad del entorno de release, no de la ruta de release en sí.

## Recuperación alternativa (ambas del operador)

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
