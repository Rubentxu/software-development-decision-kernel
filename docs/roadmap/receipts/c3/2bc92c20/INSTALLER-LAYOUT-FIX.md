# RECEIPT — Installer layout unification (session-33b defect closed)

> **Slice id:** `p-63676b11dc0ef88f/installer-layout-unification`
> **Baseline SHA:** `37e3cc82` (v2.2.22) → fix en `2bc92c20`, release v2.2.23
> **Date (UTC):** 2026-09-29T13:16:00Z
> **Status:** VERIFIED (RED→GREEN observado)

## El defecto (session-33b, causa raíz confirmada hoy en código)

1. `dev update` (update_bundle): extraía el bundle DIRECTO en
   `framework/` (raíz) y `swap_current_to(&bundle_root, &bundle_root)`
   apuntaba `current -> framework/`.
2. `install.sh`: instala en `framework/<version>/` y apunta `current`
   al dir de versión.
3. Mezcla: `dev update` desde un binario viejo + `install.sh` después →
   el dir de versión queda vacío y los 69 symlinks de editor se rompen
   (`all_present: false`).

## El fix (`2bc92c20`)

- `update_bundle`: si el tarball trae `BUNDLE.toml` con `bundle.version`,
  extrae en `framework/<version>/` (mismo layout que install.sh); sin
  BUNDLE.toml (bundles legacy raíz) conserva la extracción en raíz.
- `current`: apunta a la versión más nueva existente tras la extracción
  (o a la raíz en layout legacy).

## Evidencia

- **RED:** con el fix en `git stash`, el test
  `update_installs_versioned_bundle_into_version_dir` va ROJO
  (panicked: instalaba en raíz).
- **GREEN:** con el fix, el mismo test pasa: los ficheros aterrizan en
  `target/9.9.9/agents/a.md`, la raíz queda limpia, el output menciona
  la versión.
- **Legacy pinado:** `update_legacy_root_layout_bundle_still_installs_at_root`
  verde — el contrato del layout raiz no se rompió.
- `cargo clippy -p sddk-cli`: limpio.
- Suite update completa: 5/5.

## Qué NO cubre

- La migración de un estado ya roto (dir de versión vacío + enlaces
  rotos) sigue requiriendo reinstalación limpia con `install.sh`: este
  fix evita que se VUELVA a romper, no repara el estado en disco.
- `test_install_asset_contract.sh` sigue validando install.sh; los dos
  pipelines quedan alineados en layout pero cada uno con su guard.
