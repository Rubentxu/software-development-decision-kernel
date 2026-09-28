---
id: INC-DEBT-030-LOCAL-RELEASE-BLOCKED-AT-SIGNING-IDENTITY
title: "El publish local de v2.2.4 aborta en 8c/14 por identidad de firma ausente; la ruta local no puede publicar un release instalable"
status: blocked
severity: medium
priority: P2
created: 2026-09-28
discovered_by: session-30 (OBSERVED, ejecución real de scripts/release.sh)
cluster_id: CL-SUPPLY-CHAIN
related: [INC-DEBT-024-LOCAL-KEYLESS-IDENTITY-UNPINNABLE, INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY]
fingerprint: "release_local_aborts_at_cosign_identity_precheck"
---

## Qué pasó (OBSERVED, session-30)

`bash scripts/release.sh` (sin flags) para 2.2.4 avanzó por los pasos
0/14 → 8b/14 sin un solo fallo:

| paso | resultado |
|------|-----------|
| 0/14 preflight | ✓ admitted, last-publish=2.0.1 → 2.2.4 |
| 1/14 fmt + clippy + test workspace | ✓ suite completa verde |
| 1d/14 EXT auto-activation | ✓ skipped (EXT vars sin set, tests siguen `#[ignore]`) |
| 3/14 build release | ✓ versión 2.2.4, tag v2.2.4 |
| 4/14 manifest | ✓ 377 ficheros hasheados y verificados |
| 5/14 bundle tarball | ✓ 669237 bytes |
| 6/14 BUNDLE.toml v2 | ✓ escrito |
| 7/14 unified tarball | ✓ 12249054 bytes, exec bit + BUNDLE.toml OK |
| 8/14 sha256 + CHECKSUMS + sbom | ✓ binary sha256 `c23f7393d4f74083…` |
| 8b/14 vault ADR mirror | ✓ 48 ADRs, `created: 0, skipped: 48` |
| **8c/14 cosign signatures** | **✗ aborted** |

Mensaje exacto del gate:

```
the project's signing identity does not exist on this host.
Required issuer: https://token.actions.githubusercontent.com
This host: not a GitHub Actions runner (GITHUB_ACTIONS != true).
```

**Estado tras el aborto, verificado:**

- `gh release view v2.2.4` → `release not found`
- `git ls-remote origin refs/tags/v2.2.4` → vacío
- ningún proceso `cosign` vivo

Es decir: **no hay estado parcial**. Abortó antes de publicar, que es
exactamente lo que el gate de INC-DEBT-024 debía impedir.

## Por qué NO es un defecto

Este gate es el que se implementó en session-25 para cerrar
INC-DEBT-024-LOCAL-KEYLESS-IDENTITY-UNPINNABLE. Sin él, `cosign sign-blob`
en local emitiría un certificado de keyless con issuer
`https://oauth2.sigstore.dev/auth` y subject = la persona, mientras que
`install.sh` y `sddk dev update` pinan issuer
`https://token.actions.githubusercontent.com` + `release.yml@tag`. El release
quedaría **publicado, firmado e instalable por nadie**. El guard prefiere no
publicar.

## Las dos salidas, y por qué esta sesión no las tomó

1. **Publicar desde GitHub Actions** (`.github/workflows/release-automation.yml`).
   Es la vía correcta: produce la identidad que los instaladores pinan. Coste:
   los minutos del plan free de GitHub están agotados (§2.5 de AGENTS.md), así
   que puede que ni ejecute. Además, publicar un release es irreversible.
2. **Publicar sin firmar, con `SDDK_SKIP_SIGNING=1`.** El propio mensaje lo
   ofrece. Trade-off real: el release queda instalable solo si el operador
   también exporta `SDDK_ALLOW_UNSIGNED=1` (o `SDDK_ALLOW_UNSIGNED_UPDATE=1`
   para `dev update`). Es decir, **cambia el contrato de instalación para
   todos los usuarios** de una release firmada a una release que exige una
   variable de entorno extra. Eso degrada la garantía de la cadena de
   suministro a cambio de que este host pueda publicar, y es una decisión de
   política de distribución, no una corrección técnica.

## Por qué no se eligió ninguna sin el operador

Ambas son irreversibles o cambian el contrato de distribución, y ninguna
es necesaria para el objetivo de esta sesión. La sesión-29 ya había
documentado `INC-DEBT-027` como "no recuperable sin operador" y la
session-30 lo refutó con una ruta rootless; aquí la búsqueda se ha hecho y
**la restricción es real, no una limitación de permisos**: no hay identidad
de proyecto en este host y no se puede fabricar una keyless que los
instaladores acepten, porque el issuer lo emite el OIDC provider de GitHub,
no la máquina.

La recomendación es la vía 1 (Actions). Si los minutos del plan free lo
impiden, la vía 2 es aceptable **como decisión explícita del operador**,
documentando el cambio de contrato en el changelog de la release.

## Qué SÍ quedó verificado en esta sesión a pesar del aborto

Los pasos 0 → 8b producen artefactos reales, y dos correcciones anteriores
se validaron contra ellos:

- El guard de estaticidad (fix de session-30, `b1d89743`) aceptó el binario
  musl real: el release pasó el paso 3/14 sin quejarse del `static-pie linked`.
- El cálculo corregido de `manifest_sha256` (INC-DEBT-025 parte 1) quedó
  confirmado en el artefacto: `BUNDLE.toml` declara
  `manifest_sha256=32cfd79f5c2915d3…` y `sha256sum MANIFEST.sha256` da
  exactamente `32cfd79f5c2915d3…`. Coinciden.

## Relación con otros INCs

- `INC-DEBT-024` (**closed**) — el gate que aborta aquí es su implementación.
  Su trabajo está hecho: ahora el fallo es limpio y temprano.
- `INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY` (distribution-open) — publicar
  desde Actions y verificar un release real firmado es justamente lo que
  dejaría ese INC en `code-closed`.
