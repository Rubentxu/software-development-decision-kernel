# RECEIPT — session-45 · release v2.2.33

- **UTC**: 2026-09-30T07:16Z
- **Tag**: `v2.2.33` (anotado, `4d2f0cbd`) → commit `e737b04a6e101b45c79970418356c158d189b6e9`
- **released_baseline**: `v2.2.27`
- **development_head**: `e737b04a` == `origin/main`
- **workspace_version**: `2.2.33`
- **workflow run**: `36681891807` — `completed success`, 12/12 jobs
- **binario sha256**: `42b86e6d2bf55d664a49d777ab63f67fe078191d8b8a745408b1f044711de48c`
- **bundle sha256**: `ade5a756353127a944fe791b1329ca62f89fad6990a3dd640367a2ffc4c97cd3`

## Gates ejecutados (OBSERVED)

| Gate | Comando | Resultado |
|---|---|---|
| fmt | `cargo fmt --all -- --check` | FMT_OK |
| clippy | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0, 0 warnings |
| tests | `cargo test --workspace` | 5150 passed / 0 failed / 19 ignored |
| lock | `cargo metadata --locked` | LOCKED_OK |
| shellcheck | `shellcheck` (9 ficheros) | **28 hallazgos preexistentes, 0 introducidos** — ver abajo |
| hook | `githooks/pre-push` | HOOK_EXIT=0 tras bump real |
| admission | `release_admission_check_v2 HEAD` | `ACCEPT last-publish=2.2.27 -> 2.2.33` |
| manifest | `test_release_public_gate.sh` | PASS=13 FAIL=0 |
| debt | `test_debt_index_coherence.sh` | PASS=8 FAIL=0 (guard falsificado: FAIL→PASS) |
| diff | `git diff --check` | OK, árbol limpio |

## Public-release gate (9b) — OBSERVED

- `isDraft=false`, `isPrerelease=false`, publicado `2026-09-30T07:14:30Z`
- tag SHA anclado vía `git ls-remote` (no `origin/main`): coincide con `e737b04a`
- **27 assets** publicados
- **7/7 assets HTTP 200** desde `releases/download/v2.2.33/`
- CDN sin staleness: sha256 servido == sha256 del asset (`42b86e6d…`)
- `cosign verify-blob` del binario: **Verified OK** (issuer `token.actions.githubusercontent.com`, subject `release.yml`)
- `cosign verify-blob` del bundle: **Verified OK**
- `CHECKSUMS`: bundle `La suma coincide`
- bundle: `MANIFEST.sha256` en la raíz, 379 ficheros (INC-DEBT-034 sostenido)

## Instalación local — OBSERVED

- `bash scripts/install.sh --version v2.2.33 --editor all` sin `SDDK_BASE_URL`, sin
  `SDDK_ALLOW_UNSIGNED`, sin `SDDK_SKIP_SIGNING` → **exit 0**, `all_present: true`
- `sddk --version` → `sddk 2.2.33`
- `framework/current` → `2.2.33`
- `sddk dev doctor` → `content.manifest: present`, `binary.bundle_coherence: present`, `all_present: true`
- `sddk dev update --prune-only --keep 1` → `removed 1 stale bundle(s); kept 2.2.33`

## Blocker de calidad abierto (NO gate de este release)

`ci.yml:47` ejecuta `shellcheck` sin flags sobre `tests/test_*.sh scripts/*.sh
tests-e2e/tui/run.sh` con `|| exit 1`. Con shellcheck 0.11.0 el default severity
es `style`, así que **info, style y warning fallan el job**.

Medido sobre el árbol de `origin/main` (`git archive origin/main`, sin mis commits):
**28 hallazgos en 9 ficheros** — 25 `info`, 2 `style`, 1 `warning` (SC2034
`GATE_END` en `tests/test_release_public_gate.sh:55`, código muerto).

Comparación `origin/main` vs `HEAD`: **idénticos**, mi bump no introduce ni uno.
Los 9 ficheros ya existían en `origin/main`; mis dos ficheros nuevos
(`scripts/check_debt_index_coherence.sh`, `tests/test_debt_index_coherence.sh`)
salen **limpios**.

Contexto: el último run de `ci.yml` sobre `main` ya estaba en `failure`
(`ed0e3c47`, 2026-09-28) antes de este trabajo, y `release.yml` no depende de
`ci.yml`, por lo que no bloqueó el release (AGENTS.md §2.5: la nube es evidencia
asíncrona, el gate es local). **Se registra como blocker de calidad preexistente,
no se maquilla como verde.** No se ha abierto INC: queda para triaje.
