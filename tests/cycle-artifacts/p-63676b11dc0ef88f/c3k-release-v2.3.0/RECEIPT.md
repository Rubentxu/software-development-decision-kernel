# RECEIPT — session-46b/c3k cierre: release v2.3.0 (hito C3k completo)

**Fecha UTC:** 2026-09-30 · **Actor:** jcode (orquestador, autonomía total delegada por operador) · **Ciclo:** p-63676b11dc0ef88f / C3k (11 defectos agent-secretless)

## Alcance publicado (C3k completo)

1. **D1** (f3ae7e96): `uat sign-off` fail-closed — plan con escenarios, evidencia real, actor honesto.
2. **D2** (52182522): `normalize_remote_url` en minúsculas — case-change del remote ya no forkea el ledger. + W2a.
3. **D2/D7** (521289d8): `backlog discard --superseded-by` obligatorio con reason superseded; `backlog render --check` compara sin escribir. E2E `backlog_guard_e2e` (5).
4. **D2** (dd9d2002): warning fail-loud cuando admission crea un ledger nuevo (`ensure_project_row`).
5. **D2/W2d** (107e71d0): env vars de runtime documentadas en arch-spec-049 §6 (SDDK_STATE_HOME, SDDK_DATA_HOME, SDDK_PROJECT_ID, XDG_CACHE_HOME).
6. **D4/D5** (d2083c18): `uat plan --from` validado contra `git rev-parse refs/tags/`; `uat status` ancla en `--root`. E2E `uat_paths_e2e` (2).
7. **S3.3/S3.4** (696502c4, 54726afb): `uat validate` discrimina artifact_kind (session|report|plan) y valida contra el modelo tipado; `upsert_uat_result` acumula sesiones/duración en ingests sucesivas. Test de acumulación green.
8. **W4/S3.1/S3.2** (610a3aea): decisión de modelo — los 23 gates del release son los declarados (ya reciben `sddk.cli`); `release-receipt`/`merge-receipt` son ARTIFACTS, no gates. Prompt `prompts/sddk/phases/release.md` corregido (repo + bundle), hint artifact-vs-gate en `UnregisteredEvaluator`, **ADR-0082**, E2E `release_gate_contract_e2e` (walk A-min de 8 gates green).
9. **W2c** (9c3e027e): `sddk project pin/unpin` — pin persistido en `.sddk/project-pin.json` (schema 1); `project resolve` reporta `identity_source: pinned`; `RuntimeContext::open` honra el pin. `IdentitySource::Pinned` en sddk-domain. E2E `project_pin_e2e` (2 green).
10. **INC-DEBT-040 variante (3)** (37c90b51): ruta **(A-v2, tag-baseline)** en `githooks/pre-push` — push admitido cuando la versión del tip excede el máximo tag `v*` del remote (o bootstrap). Fail-closed si `ls-remote` falla. Matriz 48/48 PASS, 3 expectativas de rename enmendadas (`AMENDED:`), shellcheck limpio. Debt marcada **resolved** + índice actualizado.
11. **Higiene del árbol** (94a7516d, f2fed84b, 7231a09f, 09f3cabc, 9d5c13d9): MANIFEST.sha256 regenerado (lo detectó `cli_dev_install_accepts_committed_manifest`), BUNDLE.toml fósil 2.2.32→2.2.37 (guard `test_dev_install_source_guard`), puntero STATE reconciliado x2, comentario sin versiones en prosa.

## Git y versión

- **released_baseline (tag):** v2.3.0 → anotado, peel = `9d5c13d9` (== HEAD == origin/main al publicar)
- **release_sha:** 9d5c13d9 (bump `11f8f5aa` 2.2.37→2.3.0, minor por 3 feats; reconciliación docs `9d5c13d9` es posterior al tag pero docs-only)
- **workspace_version:** 2.3.0 · **binary:** sddk 2.3.0 (musl, sha256 `3c5d5b88fdae53b2…`) · **bundle:** 2.3.0
- **19 commits** desde v2.2.37 publicados en un solo push (64f2d73b..11f8f5aa) — **primera admisión por la ruta tag-baseline del propio fix INC-DEBT-040** (el rango no contenía bump; tip 2.3.0 > tag v2.2.37).

## Gates locales (perfil completo, pre-bump sobre 09f3cabc)

- `cargo fmt --check` exit 0
- `cargo clippy --workspace --all-targets -- -D warnings` exit 0
- `cargo test --workspace`: **0 failed** en toda la suite tras regenerar MANIFEST (único rojo inicial: `cli_dev_install_accepts_committed_manifest`, causado por MANIFEST stale, no por código)
- 26 shell tests: **verdes** salvo `test_release_state_pointer` (FAIL esperado pre-push → PASS tras push y reconciliación) y `test_supply_chain_authenticity` (requiere `--tag`, PASS=13/13 con `--tag v2.2.37`)
- `test_push_prevention_hook` 48/48 · `test_release_admission` 24/24 · `test_release_bump_derivation` 7/7 · `pipeline_consistency` OK · `test_debt_index_coherence` PASS

## Publicación (CI, patrón session-45/46)

- `bash scripts/release.sh` local: pasos 0-8 ✓ (tests, build musl 2.3.0, MANIFEST regen+verify, bundle 652520 bytes, unified 12367180 bytes, checksums+sbom), **8c se detiene correctamente**: cosign keyless exige identidad de GitHub Actions.
- Tag `v2.3.0` pusheado + `gh workflow run release.yml --ref v2.3.0` → **run 36732655082 = completed/success** (0 jobs fallidos).
- Release público: `isDraft=false, isPrerelease=false, publishedAt 2026-09-30T15:02:53Z, 27 assets`.

## Gate 9b (public gate) — OBSERVED

- Tag anchoring: `git ls-remote origin refs/tags/v2.3.0` = `9d5c13d9f6076649…` == HEAD == origin/main ✓
- Assets HTTP 200: **18/18 verificados individualmente** (los 4 tarballs + sig/pem/sha256, sddk, sddk.sha256, sddk.sig, sddk.pem, CHECKSUMS, sbom.json, gh-release-receipt.json, bundle) ✓

## Gate 9c (autenticidad) — OBSERVED

- `sddk.sha256` servido por CDN = `3c5d5b88fdae53b2fb686796a4272dc061a50114a65e89c3803646dd30d93a7b`; binario descargado = mismo hash (CDN sin staleness) ✓
- `cosign verify-blob` **Verified OK** con identidad `https://github.com/Rubentxu/software-development-decision-kernel/.github/workflows/release.yml@refs/tags/v2.3.0`, issuer `token.actions.githubusercontent.com` ✓

## Instalación local — OBSERVED

- `bash scripts/install.sh --version v2.3.0 --editor all` exit 0 (URL pública real, sin SDDK_BASE_URL)
- `sddk --version` = 2.3.0 · `framework/current -> 2.3.0`
- `dev install --prefix ~/.local --release-receipt gh-release-receipt.json` → receipt schema 1 con `tag: v2.3.0`; `dev doctor --prefix ~/.local`: **content.manifest present, binary.bundle_coherence present, all_present true** (los 6 missing son los advisory `surface.briefness.*` preexistentes)
- `dev update --prune-only --keep 1` → removed 2.2.37

## Estado del hito

- **C3k: COMPLETO.** W1..W7 + W2a..W2d ejecutados; los 11 defectos del report agent-secretless tienen fix o clasificación documentada (D2 cubre W2a/b/c/d; D1/D3/D4/D5/D7 fixes directos; S3.x del plan de evolución; W4 por decisión de modelo ADR-0082).
- Deuda abierta relevante: INC-DEBT-041 (shellcheck CI, open), INC-DEBT-039 (low/P3, re-scoped), INC-MATRIX-LINT (31 hallazgos preexistentes). INC-DEBT-040 **resuelta por la variante (3)**, no reabierta.
- NOT_RUN: UATs CTX-UAT-011..015, HYP-UAT-001..004 (sin cambio de superficies que los cubran en este hito).

## Siguiente paso

Los pendientes del plan de evolución (docs/research/2026-09-30-sddk-cli-defects-evolution-plan.md) que excedían C3k pasan a evaluación de roadmap; sin STATE paralelo: la autoridad operativa sigue siendo SDDK (`agent-session`).
