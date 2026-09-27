---
id: C4-RELEASE-v2.0.1
cycle: session-15
type: RELEASE-RECEIPT
status: PUBLISHED
tag: v2.0.1
tag_sha: 5ce4bcacfb8ec4855cd6c543b59f6bda940b7872
head: 5ce4bca
origin_main: 5ce4bca
workspace_version: 2.0.1
binary_version: 2.0.1
bundle_version_installed: 2.0.1
binary_sha256: 417a7163a286f75bcd3ece778e6d33eb7031f312626d2639513db47b9c672e22
published_at: "2026-09-27T19:26:59Z"
is_draft: false
is_prerelease: false
admission_mode: v2
semver_derivation: v2.0.0 + fix => v2.0.1 (patch)
---

# C4 — Release receipt v2.0.1 (session-15)

## Resumen

Release de seguridad que **cierra el gap de distribución detectado en la
auditoría previa**: el guard de escritura arbitraria
(`ensure_safe_tarball_members` en `sddk dev update`) estaba commiteado en
`main` pero **no distribuido** — el tag anterior `v2.0.0` apuntaba a
`db043b4`, siete commits antes del fix `79e9e2d`. Todo el que instalaba
desde la release seguía con la primitiva de traversal expuesta.

Este corte publica ese fix. Pipeline completo 14/14 en verde, gate
público 9/9, instalación local verificada.

Esta release cierra además la divergencia de serie de versiones: el
workspace arrastraba la serie `1.17x` mientras el tag publicado era
`v2.0.0`, lo que dejaba el workspace **por debajo** del máximo de tags
publicados y hacía fallar la admisión v2 con `not-above-last-publish`.
El workspace se alinea ahora en `2.0.1` con la serie publicada real.

## Release

- Tag: `v2.0.1` → `5ce4bcacfb8ec4855cd6c543b59f6bda940b7872`
- `tag_sha == HEAD == origin/main` (verificado con `git ls-remote origin refs/tags/v2.0.1`)
- draft=false, prerelease=false
- 9/9 assets públicos
- URL: https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v2.0.1

## Assets (contrato de 9)

```
CHECKSUMS
gh-release-receipt.json
sbom.json
sddk
sddk-v2.0.1-sddk-linux-x86_64-musl.tar.gz
sddk-v2.0.1-sddk-linux-x86_64-musl.tar.gz.sha256
sddk.sha256
software-development-decision-kernel.tar.gz
software-development-decision-kernel.tar.gz.sha256
```

- unified tarball: 12.141.631 bytes
- bundle tarball: 669.236 bytes
- MANIFEST.sha256: `32cfd79f5c2915d3d92e949ea02c2c71…`

## Verificación del binario publicado (independiente del script)

El digest publicado por la API de GitHub coincide **byte a byte** con el
binario compilado localmente:

```
publicado (gh):  sha256:417a7163a286f75bcd3ece77…
local (sha256sum): 417a7163a286f75bcd3ece77…
```

## Gates ejecutados sobre el árbol publicado

```text
cargo fmt --check                                  PASS
cargo clippy --workspace --all-targets -D warnings PASS (0 diagnostics)
cargo test --workspace --offline --no-fail-fast   5041 passed, 0 failed, 19 ignored
```

Salida completa del perfil completo: exit 0, ninguna suite con fallos.

El release script volvió a ejecutar el perfil completo durante step 1
(sin `--skip-tests`).

## Fix de seguridad incluido (el motivo de este corte)

`79e9e2d fix(cli): cierra la primitiva de escritura arbitraria en sddk dev update`

- `ensure_safe_tarball_members` rechaza rutas absolutas y traversal
  **antes** de extraer.
- Extracción con `--no-same-owner` y `--no-same-permissions`.
- 7 tests, incluido el exploit `software-development-decision-kernel/../../etc/x`.

Verificado en el código publicado, no sólo en `main`:

```
git show v2.0.1:crates/sddk-cli/src/dev/update.rs | grep -c ensure_safe_tarball_members
→ 10
git show v2.0.1:crates/sddk-cli/src/dev/update.rs | grep -c no-same-owner
→ 1
```

## Estado instalado tras el release

```text
sddk --version                     → sddk 2.0.1
~/.local/share/sddk/framework/current → 2.0.1
sddk dev doctor --prefix ~/.local/bin:
  binary.bundle_coherence: present
  all_present: true
```

Coherencia binario ↔ bundle verificada: el binario instalado y el bundle
instalado son ambos `2.0.1`.

## Reconciliación de punteros (previa a este release)

`STATE.yaml` declaraba `current_sha: 87fef0e` mientras `HEAD` y
`origin/main` estaban en `3ed92ed`. El drift venía de que el commit de
punteros se escribía antes del push de sí mismo. Corregido en `ee9064a`
sin reescribir historia; la entrada anterior se conserva en el journal.

## Derivación SemVer

`scripts/release-bump.sh` (algoritmo canónico, sin override manual)
derivó `v2.0.0 → v2.0.1`: patch por un `fix` sin cambio de contrato.
No se pasó `--force-version`. El tag SemVer y el workspace `2.0.1`
coinciden por primera vez en esta serie.

## Pendiente (no resuelto por este release)

- **Firma out-of-band**: el `.sha256` se sigue descargando del mismo
  origen que el payload. La primitiva de traversal está cerrada, pero la
  autenticidad del artefacto sigue sin trust root. Bloquea
  `v2.0.0`+ como canal de distribución confiable.
- **C2**: `chronos-mcp` sigue sin artefacto instalable; no evaluable.
- **C5**: change-scoped verification pendiente; `test_ports.rs` no se toca.
