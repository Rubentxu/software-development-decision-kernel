---
id: C4-RELEASE-v2.0.0
cycle: session-14
type: RELEASE-RECEIPT
status: PUBLISHED
tag: v2.0.0
tag_sha: db043b4c545a028a4e4d9348bcf9789e1c9c2bfb
head: db043b4
workspace_version: 1.173.0
bundle_version_installed: 1.173.0
published_at: "2026-09-27T18:19:02Z"
is_draft: false
is_prerelease: false
---

# C4 — Release receipt v2.0.0 (session-14)

## Resumen

Release publicada con los 14 pasos del script en verde. El tag es
`v2.0.0`; el binario y el bundle reportan `1.173.0`. La discrepancia
está documentada abajo y es intencionada (decisión del operador).

## Release

- Tag: `v2.0.0` → `db043b4c545a028a4e4d9348bcf9789e1c9c2bfb`
- `tag_sha == HEAD == origin/main` (verificado con `git ls-remote`)
- draft=false, prerelease=false
- 9/9 assets públicos, HTTP 200 desde el CDN
- URL: https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v2.0.0

## Assets (contrato de 9)

```
CHECKSUMS
gh-release-receipt.json
sbom.json
sddk
sddk-v2.0.0-sddk-linux-x86_64-musl.tar.gz
sddk-v2.0.0-sddk-linux-x86_64-musl.tar.gz.sha256
sddk.sha256
software-development-decision-kernel.tar.gz
software-development-decision-kernel.tar.gz.sha256
```

- bundle tarball: 669.239 bytes
- unified tarball: 12.108.327 bytes, exec bit + BUNDLE.toml verificados
- BUNDLE.toml schema v2, `manifest_sha256=608c6d9ced950456e9d453d8e54529b6c3dc06e45302189c3c15c01c738fd39f`
- MANIFEST.sha256 regenerado: 377 ficheros, verificado
- binary sha256 prefijo: `aa13f6f8b9e61423…`

## Gates ejecutados por el script

| Paso | Resultado |
|---|---|
| 0 preflight (admisión v2) | ACCEPT |
| 1 workspace green | PASS |
| 1c sync HEAD → origin/main | ya en remoto |
| 1d EXT auto-activation | skipped (opt-in) |
| 2 read version | 1.173.0 → v1.173.0 |
| 2.5 semver tag | **override → v2.0.0** (ver abajo) |
| 3 build --release | OK |
| 4 manifest | 377 ficheros, OK |
| 5 bundle tarball | OK |
| 6 BUNDLE.toml v2 | OK |
| 7 unified tarball | exec bit OK |
| 8 sha256 + CHECKSUMS + sbom | OK |
| 8b vault ADR mirror | 0 created, 48 skipped (ya existen) |
| 9 gh release create | publicado |
| 9b public-release gate | **PASS 9/9** |
| 10 install desde URL | CDN sirvió el sha correcto a los 10s |
| 11 dev doctor | `all_present: true` |
| 12 prune | 1 bundle stale eliminado, conservó 1.173.0 |
| 13 round-trip distrib | OK |
| 14 final state | shipped + installed |

## Discrepancia tag ↔ bundle (documentada, no oculta)

**El tag es `v2.0.0`; el software dentro es `1.173.0`.**

Causa: el paso 2.5 ejecuta `scripts/release-bump.sh` en dry-run. Sin
`--force-version`, ese script deriva `major` porque el rango contiene
`refactor(cli)!`. El paso 2.5 sobrescribe el `TAG` derivado del
workspace (`release.sh:373-375`) y el tag publicado pasa a `v2.0.0`.

El override de versión aplicado al `Cargo.toml` (1.171.2 → 1.173.0) sí
se respetó en el contenido: binario, bundle y
`binary_min_version`/`binary_max_version` son `1.173.0`. Lo único que
tomó el valor del algoritmo fue el **tag**.

Corregir esto requiere pasar `--force-version 1.173.0` al propio
`release.sh`; el flag ya está plumbéado (`release.sh:361-363`), luego
**no hay bug de implementación**: fue una invocación incompleta. Se
registra en `INC-AUDIT-S14-RELEASE-FORCE-VERSION-ERGONOMICS`.

**Decisión del operador: se mantiene `v2.0.0`.** No se reetiqueta,
porque (a) v2.0.0 es defendible per se — cuatro módulos `pub`
desaparecieron de la API, — (b) reetiquetar un release público es
churn visible sin ganancia funcional, y (c) el bundle instalado es
funcionalmente correcto. El coste es una afirmación pública más fuerte
que la estrictamente necesaria, que queda anotada aquí y en el INC.

## Estado local instalado

- bundle: `~/.local/share/sddk/framework/1.173.0` (current → 1.173.0)
- bundle 1.171.2 eliminado por prune
- `sddk dev doctor`: `binary.bundle_coherence: present`,
  `all_present: true`
- round-trip de distribución (install → prune → re-install): OK

## Qué contiene esta release

Rango `7dbd886..db043b4` (8 commits), todos verificados:

1. `f1d5fbb` fix(gateway): `evidence.bundle.write` escribe de verdad.
   Antes devolvía éxito sin persistir y verificaba su postcondición
   contra el outcome que él mismo fabricaba.
2. `8d49f11` fix(cli): el gate de clippy de `sddk release` pasa a
   `-D warnings` (antes `-D errors`, más débil que el contrato).
3. `97b900b` refactor(cli)!: −1.403 LOC de spikes muertos.
4. `b018ec5` fix(engine): 4 referencias colgantes a esos spikes.
5. `24dd3da` docs(debt): 3 INCs de auditoría + premisa stale de C2
   corregida.
6. `701f73b` docs(roadmap): punteros de sesión + claim de verificación.
7. `8a82d26` chore(release): bump 1.171.2 → 1.173.0.
8. `db043b4` docs(roadmap): cierre de session-14.

## Verificación previa a publicar

| Gate | Resultado |
|---|---|
| `cargo fmt --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo test --workspace --offline --no-fail-fast` | 5034 passed; 0 failed; 19 ignored |

## Regresión cazada antes de publicar

La primera pasada del perfil completo falló en
`arch_ratchet_mutations::conf09_universal_evidence_only`: la limpieza de
spikes había dejado su entrada viva en `CONF09_TYPE_ALLOWLIST`. El test
lleva la aserción que lo cazó (*"shrink the allowlist instead of
keeping dead entries"*), y se corrigieron 4 referencias colgantes en
`b018ec5`. Sin el perfil completo, esta release habría publicado un
commit rojo.

## Deuda abierta que esta release no cierra

- `INC-AUDIT-S14-TEST-PORTS-UNCONSUMED` (high/P1)
- `INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY` (high/P1)
- `INC-AUDIT-S14-NO-STRUCTURED-LOGGING` (medium/P2)
- `INC-AUDIT-S14-RELEASE-FORCE-VERSION-ERGONOMICS` (low/P2)
