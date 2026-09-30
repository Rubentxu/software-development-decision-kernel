# RECEIPT — session-47: deuda severa (INC-DEBT-041, INC-DEBT-038) + release v2.3.1

**Fecha (UTC):** 2026-09-30 · **Baseline al inicio:** `1e37b08e` (v2.3.0, C3k completo)
**HEAD al cierre:** `accd4911` == `origin/main` == peel del tag `v2.3.1`

## Alcance

Petición: continuar corrigiendo regresiones si las hay, deuda técnica severa
reciente si la hay, ciclos de roadmap si lo anterior está cubierto. Alerta
"deuda" sin criterios vigentes no es deuda real. SDDK como única autoridad.

Hallazgo del pre-flight: las dos deudas más recientes con criterios vigentes
verificados en código eran **INC-DEBT-041** (medium/P2, session-45) e
**INC-DEBT-038** (medium/P2, session-37). Ambas verificadas vigentes antes de
tratarlas como deuda real.

## INC-DEBT-041 — shellcheck gate ciego con hallazgos latentes → resolved

- Re-medición con el comando exacto del step (`shellcheck -S style` sobre
  `tests/test_*.sh scripts/*.sh tests-e2e/tui/run.sh`): **≈49 hallazgos en 15
  ficheros**, no los 28/9 de session-45 (globo parcial: quedaban fuera
  `test_install_asset_contract.sh` y `apply_banner.sh`).
- Patrón dominante: `SC2016` **intencional** (grep de literales `$VAR` contra
  YAML/sh — patrón de contrato). `GATE_END` (SC2034) no es código muerto: es
  anchor espejo del bloque del gate en release.sh.
- Tratamiento (ruta 1 del triaje: sin tocar ci.yml, sin bajar tolerancia):
  ~30 directivas `# shellcheck disable=SCnnnn` justificadas por línea;
  1 refactorización mínima (SC2129 en `apply_banner.sh`, grupo `{ … } > tmp`,
  smoke de ambas ramas OK); anotación de SC1091 en release.sh.
- **Evidencia OBSERVED:** 0 hallazgos en el globo completo. Tests de los 10
  ficheros tocados PASS (release ×7, install asset contract, vault ×2) más
  vecinos (`receipt_authority`, `pipeline_consistency`).
- Commits: `3c746a88` (código), `1e45f810` (deuda docs).
- Queda clasificada como propiedad estructural (no defecto) la ceguera
  secuencial de jobs de CI.

## INC-DEBT-038 — `dev install --source` recibo mentiroso → resolved

- Opciones 2+3 del propio documento, juntas:
  - `InstallReceipt.layout` (opcional); `dev install --source` escribe
    `layout: "flat"` con `bundle_version: null`. El hash de BUNDLE.toml se
    conserva; la versión del bundle sigue validándose fail-closed contra el
    binario antes de escribir.
  - `dev doctor`: si el resolver versionado no encuentra bundle pero el data
    root lleva recibo flat, opera sobre las superficies planas y reporta
    `binary.bundle_coherence` N/A en verde con detalle "flat-install";
    `receipt.version` sigue verificando.
- **TDD:** RED (`doctor_flat_install_receipt_reports_notsynthetic_coherence`)
  → GREEN. Doctor 9/9, dev_install 5/5, manifest commiteado 1/1,
  pipeline_consistency OK, fmt/clippy -D warnings limpios.
- **E2E OBSERVED:** install `--source` en prefix aislado exit 0 con recibo
  `{layout: flat, bundle_version: null}`; doctor sobre ese prefix
  `all_present: true`.
- Higiene pillada por el guard: `BUNDLE.toml` fósil 2.2.37 (post-bump v2.3.0
  sin regenerar; el guard `test_dev_install_source_guard` lo detectó rojo al
  re-chequear) → regenerado a 2.3.0 (commit del fix).
- Commits: `d1d59df6` (código+BUNDLE), `a3751cc0` (deuda docs).

## Defecto de herramienta pillado en el camino

- `scripts/reconcile_state_pointer.sh` escribía versiones en prosa en el
  comentario de `current_sha` (`(2.3.1)`), que su propio guard
  (`test_release_state_pointer`, check 3c de session-44) rechaza. Fix: el
  comentario deja de afirmar versiones; la versión vive solo en
  `workspace_version_at_current`. OBSERVED: reconciliación con guard PASS.
- Commit: `a3753be0`.
- Incidente propio registrado: un script python de una línea truncó
  STATE.yaml a 0 bytes; restaurado desde git (`1324ff35`) y la edición
  rehecha con `edit`. Sin daño (el estado íntegro estaba pusheado).

## Regresión pillada por el gate del release

- `cargo test --workspace` (paso 1 del release.sh) falló en
  `install_migrates_legacy_v1_receipt_to_v2_when_source_has_bundle_toml`: el
  test esperaba que la migración v1→v2 enlazara `bundle_version`, la conducta
  mentirosa que 038 elimina. Actualizado a pinnear `layout: "flat"` +
  `bundle_version` null. lib 854/0/1. Commit `accd4911`.

## Release v2.3.1 (patch: 2 fix + fix de script + test)

- Bump: `bash scripts/release-bump.sh` → derivación PATCH 2.3.0 → 2.3.1;
  commit `9556cc62` + puntero `a249225d` + comentario limpio `1324ff35` +
  fix script `a3753be0` + BUNDLE 2.3.1 `010e1c00`.
- Tag anotado `v2.3.1` → peel `accd4911`.
- **Incidente tag fantasma (patrón v2.2.25):** el primer tag se pusheó antes
  que el commit del test; el primer run de CI (36741522523) corrió sobre el
  árbol con el test rojo y creó un **draft** v2.3.1 inválido. Resolución:
  draft eliminado, tag remoto borrado, push completado, re-tag al HEAD bueno,
  segundo run **36742855897 completed success** (27 assets, isDraft=false,
  isPrerelease=false, publishedAt 2026-09-30T16:23:27Z).
- **Gate 9b OBSERVED:** ls-remote `refs/tags/v2.3.1` = `96ee1a40…` (objeto
  anotado), peel = `accd4911` == HEAD == origin/main; **27/27 assets HTTP
  200** (un 500 transitorio de CDN en el `.pem` de darwin-arm64, OK tras ~2
  min de refresco; re-chequeo completo 27/27).
- **Gate 9c OBSERVED:** `sddk.sha256` servido por CDN = `a84e5980c263d827…`
  == sha256 del binario descargado; `cosign verify-blob` **Verified OK**
  (identity `release.yml@refs/tags/v2.3.1`, issuer token.actions...).
  Nota: el asset `sddk.bundle.json` no existe en esta release (verificación
  por cert + sig; la dupla .pem/.sig del binario y los .pem/.sig/.sha256 por
  tarball sí están).
- **Pasos 10-12 OBSERVED:** `install.sh --version v2.3.1 --editor all` exit 0;
  `sddk 2.3.1`; `current → 2.3.1`; doctor `content.manifest: present`,
  `binary.bundle_coherence: present`, `all_present: true`;
  `dev update --prune-only --keep 1` removed 2.3.0.

## Cierre

- STATE.yaml reconciliado a `accd4911` / 2.3.1; ROADMAP: nota de deuda
  severa resuelta en esta sesión; JOURNAL: entrada append-only.
- Guards en PASS post-push: `test_release_state_pointer`,
  `test_release_pipeline_consistency`, `test_dev_install_source_guard`,
  `test_release_state_pointer` (19 checks).
- No se abrió ningún WorkItem de roadmap: el criterio de la petición era
  "regresiones y deuda severa primero"; ambas deudas estaban vigentes y
  verificadas, y el plan S4+ queda como evaluación futura (igual que al
  cerrar C3k).

## NOT_RUN

- INC-AUDIT-S14-* (medium/low, sin criterios nuevos desde su registro).
- INC-DEBT-039 (low/P3, re-scoped; disparador mecánico: primera fila real en
  `node_runs_v1`).
- CTX-UAT-011..015, HYP-UAT-001..004 (C3j objetivo 6 / hipermedia; futuro).

## Siguiente paso

Evaluar S4+ de `docs/research/2026-09-30-sddk-cli-defects-evolution-plan.md`
como hito propio del roadmap (pendiente desde el cierre de C3k), o abrir C3i
si se prefiere continuar la línea hypermedia.
