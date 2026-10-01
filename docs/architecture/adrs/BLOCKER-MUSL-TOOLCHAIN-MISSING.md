---
id: BLOCKER-MUSL-TOOLCHAIN-MISSING-BLOCKS-2-5-0
title: el toolchain musl no esta instalado y bloquea la publicacion de v2.5.0
status: open
severity: high
priority: P1
detected_at: 2026-10-01
detected_in_session: session-57
component: release-toolchain
surface: scripts/release.sh (step 3/14)
references:
  - docs/debt/INC-DEBT-021-MUSL-NAME-GLBC-CONTENT.md
  - scripts/release.sh
  - scripts/install.sh
---

# BLOQUEANTE: falta el toolchain musl, y el release ABORTA a proposito

## Sintoma

`bash scripts/release.sh` pasa los steps 0 (preflight), 1 (workspace green),
1b (los 8 shell contract tests) y 2 (read version), y **aborta en el step 3**:

```
error occurred in cc-rs: failed to find tool "x86_64-linux-musl-gcc":
  No such file or directory (os error 2)
✗ cargo build failed para target x86_64-unknown-linux-musl
```

## Diagnóstico

| Pieza | Estado |
|---|---|
| Target Rust `x86_64-unknown-linux-musl` | **instalado** (`rustup target list --installed`) |
| Compilador C `x86_64-linux-musl-gcc` | **ausente** |
| Paquete `musl` | no instalado (distro **Bazzite 44**, Fedora inmutable) |
| `sudo` | requiere contraseña; no instalable sin intervención del operador |

El fallo viene de `ring v0.17.14`, que necesita un compilador C para el target
musl durante su build script.

## Por que NO se resuelve con un flag

`scripts/release.sh` documenta exactamente esta situacion (lineas 424-450):

> El target es configurable porque no todos los hosts de release tienen el
> toolchain. El default es musl porque es lo que el nombre del asset promete.
> Si se pide musl y el toolchain no esta, el script **ABORTA**: es preferible no
> publicar a publicar un binario con el nombre equivocado. **Esa era exactamente
> la mentira que INC-021 documentaba.**

Y sobre el escape hatch:

> `SDDK_RELEASE_BUILD_TARGET=...: el nombre del asset dice musl pero el target no
> es musl` → `esto reintroduce INC-021. Se requiere una decision explicita del
> operador.`

**Por tanto el flag NO se usa en esta slice.** Forzar un build glibc y llamarlo
musl seria exactamente la mentira de INC-DEBT-021: un artefacto cuyo nombre
promete portabilidad estatica y cuyo contenido la necesita. El instalador
(`scripts/install.sh`) reparte los binarios como musl, asi que un binario glibc
fallaria en Alpine y otros sistemas musl.

## Remedio (requiere operador)

Bazzite es una distro Fedora **inmutable**: `dnf install` no persiste entre
reboots. Las dos vias:

1. **Layer propio (recomendado, persistente)**
   ```bash
   rpm-ostree install --idempotent musl-gcc
   sudo systemctl reboot
   ```
   Tras el reboot, `which x86_64-linux-musl-gcc` debe resolver y
   `x86_64-linux-musl-gcc --version` funcionar.

2. **Capa con `rpm-ostree layers` sin reiniciar** (más rápido, se pierde en el
   siguiente `rpm-ostree cleanup` si no se registra).

Verificación antes de relanzar:

```bash
x86_64-linux-musl-gcc --version
bash scripts/release.sh
```

## Estado del trabajo que este bloqueo retiene

Nada se publico y **no se publico nada a medias**, que es el comportamiento
correcto. Todo el trabajo verificado queda commiteado y listo para salir en el
momento en que el toolchain este:

- `5a6f155f test(push)` — repara el caso fail-closed que media el repo
  equivocado (ver `INC-DEBT-045`).
- `f78a8bf2 docs(debt)` — registra INC-DEBT-045 y reindexa INC-DEBT-044.
- `86f2aad7 chore(release): bump version` — workspace `2.5.0`, HEAD con el
  subject que exige el step 0.

Los tres gates que fallaron en intentos anteriores **ya pasan**: workspace green,
los 8 shell contract tests (incluido `test_push_prevention_hook.sh` con
`PASS=48 FAIL=0` y `test_vault_adr_mirror_coverage.sh` con 53 ADRs espejados).

## Deriva de version declarada

Como no se publico nada, la version del workspace sigue siendo `2.5.0` y el
ultimo tag publicado sigue siendo `v2.4.2`. La regla §2.3 (el workspace version
es puntero ceremonial del release que se va a publicar) se mantiene: en cuanto
el toolchain este disponible, el release sale como `v2.5.0` sin cambios
adicionales.
