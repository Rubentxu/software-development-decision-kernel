# RECEIPT — dev update legacy root layout: merge en vez de swap destructivo

**Fecha (UTC):** 2026-09-29
**Session:** session-34
**Fix commit:** `a409fe45` (sobre `9255117d`)
**Defecto origen:** observado en vivo contra v2.2.23 (binario `b1f3b94d…`)

## Síntoma (OBSERVADO)

`sddk dev update --root ~/.local/share/sddk/framework --version v2.2.23`
eliminó en un solo golpe `framework/2.2.21/`, `framework/2.2.22/` y el
symlink `framework/current`, extrajo el bundle en la RAÍZ y dejó
`all_present: false` (doctor: `binary.bundle_coherence: missing`).

## Causa raíz (dos capas, ambas confirmadas con código)

1. **El tarball standalone del release nunca llevó `BUNDLE.toml`.**
   `release.sh` step 5 empaquetaba el tar ANTES de que step 6 escribiera
   el fichero; solo el unified tarball (step 7) lo heredaba vía `cp -r`.
   Verificado contra v2.2.23 publicado: `tar tzf` no muestra
   `BUNDLE.toml`; el unified sí lo lleva (`framework/BUNDLE.toml`).
   Sin `BUNDLE.toml`, el fix de ayer (`2bc92c20`) cae al path legacy.
2. **El path legacy usa `copy_tree(CopyMode::Always)` con
   `target == raíz del framework`.** Ese modo hace rename de la raíz
   COMPLETA a `.old-<pid>`, renombra el staging a la raíz y borra el
   parked. Los version dirs de un install.sh previo y `current` viven
   dentro de la raíz y mueren con el swap.

El RED→GREEN de ayer fue honesto para el mecanismo (fixture CON
BUNDLE.toml), pero la asunción de integración (el tar del release lleva
BUNDLE.toml) era falsa. Lección: falsar también el contrato del
productor, no solo el del consumidor.

## Fix (`a409fe45`)

- **Productor** (`scripts/release.sh` step 5): stage-then-pack. BUNDLE.toml
  se escribe en el staging (schema v2, version/min/max de `$VERSION`,
  manifest_sha256 correcto) y el tar se crea desde el staging con el
  xform uniforme. Step 6 pasa de inyección a ASERCIÓN (fallo ruidoso si
  el tar no trae el fichero o la versión no coincide).
- **Consumidor** (`crates/sddk-cli/src/dev/update.rs`): si
  `install_root == root` (legacy), merge con `CopyMode::IfChanged` en
  vez del swap. Seguro: el staged bundle ya pasó `verify_manifest`
  completo antes de copiar.

## Evidencia

### RED (test nuevo, código de ayer)

```
test update_legacy_root_layout_preserves_existing_root_content ... FAILED
panicked: pre-existing version dir must survive the root-layout update
```

### GREEN

```
test update_legacy_root_layout_bundle_still_installs_at_root ... ok
test update_legacy_root_layout_preserves_existing_root_content ... ok
cargo test -p sddk-cli --lib dev::  ->  409 passed; 0 failed
cargo fmt -p sddk-cli -- --check -> limpio
cargo clippy -p sddk-cli --lib -- -D warnings -> ok
```

### Falsaciones del mecanismo de empaquetado (dos via corta descartadas)

- `tar` con path absoluto para BUNDLE.toml: el xform filtra el prefijo
  mktemp → miembro `software-development-decision-kernel/tmp/...`.
  DESCARTADO.
- `tar -C A ... -C B file`: el xform se aplica también al segundo bloque
  → doble namespace. DESCARTADO.
- Staging único + xform uniforme: OK (587 miembros,
  `software-development-decision-kernel/BUNDLE.toml` presente).
- Anidación de `prompts/sddk`: `cp -r prompts/sddk dst/` aplana a
  `dst/sddk` si `dst/prompts` no existe. Fijado con `mkdir -p` previo;
  tar del harness contiene 48 miembros bajo `prompts/sddk/`.

### Falsación E2E con el binario 2.2.23 instalado (el que falló en vivo)

Fixture: framework con `2.2.22/` + `current -> 2.2.22`; tar NUEVO
(con BUNDLE.toml) servido por file://.

```
framework: 2.2.24 (...) 377 files content-verified via MANIFEST.sha256
  into /tmp/.../fw/2.2.24
OK: 2.2.24/BUNDLE.toml presente (layout versionado)
OK: contenido preexistente 2.2.22 intacto
```

### Falsación negativa del gate nuevo de step 6

Extraído el tar de v2.2.23 publicado (sin fix): NO contiene BUNDLE.toml
→ la aserción nueva habría abortado ese release. Confirmado.

## Gap registrada (NO fixeada aquí)

Con `--root <dir>` explícito, el repoint de `current` no corre (el
código solo repunta cuando `root == '.'`). Los flujos reales usan root
por defecto o `sddk dev use`. Candidata a deuda.

## Alcance de verificación (change-scoped)

- SUT: `crates/sddk-cli/src/dev/{update.rs,common.rs}` (update_bundle,
  copy_tree), `scripts/release.sh` (steps 5-6).
- Tests: módulo `dev::` completo (409) — cubre update_bundle, copy_tree,
  manifest, receipt. Suite shell completa se ejecuta tras este commit
  (22 contratos x2 rondas).
- No ejecutado aquí: `cargo test --workspace` completo (reservado para
  verify/release); benchs (sin cambios de rendimiento).
