# CURRENT — puntero de reanudación de SDDK

**Estado:** C1 cerrado; C2 NOT_EVALUATED (3 sub-cycles systemic); C3a/b/c/d/e/f/g/h PASS_OBSERVED. **C4 v1.171.0 — PASS_PARTIAL_OBSERVED** (release publicada y binario en PATH, pero certificación formal retractada por session-12 audit al esquema CERTIFICATIONS §5). Override SemVer LIFTED (v1.171.0 es SemVer-correct minor). Actualizado: 2026-09-22T19:33Z (session-12 FC-1 v2 closeout + chromium-skip fix + version bump to 1.171.2). Este puntero se revalida al comienzo de cada sesión.

| Campo | Valor observado o pendiente |
| --- | --- |
| Fuente de la fotografía | `main@8a541c3` (HEAD = `chore(release): bump 1.171.1 -> 1.171.2`) — consultado 2026-09-22T19:31Z. Tag `v1.171.0` → commit `db1e2e44`. **HEAD está ahead de `v1.171.0` por 4 commits** (el más reciente bump apunta al siguiente tag `v1.171.2`). |
| Workspace en esa fotografía | `1.171.2` (Cargo.toml). Workspace == next-tag-en-publicación (cumple AGENTS.md §2.3). |
| Release pública comprobada en esa fotografía | `v1.171.0` (2026-09-22T17:11:44Z) sigue siendo GH Releases Latest. Binary sha256 `5e9d5fbd17d94b8c53763cdca0a70435b8eabedcd72e521cce21f1c2335ef9f3`. PATH sha256 matches GH asset. |
| Hito activo | C1 CERRADO; C2 NOT_EVALUATED (systemic, provider MCP bridges ausentes); C3a-h PASS_OBSERVED; **C4 v1.171.0 PASS_PARTIAL_OBSERVED**. Override SemVer LIFTED. Workspace 1.171.2 ahead of last published tag. |
| Estado PRs abiertos | Ninguno. |
| Commits since v1.171.0 (workspace ahead) | `0974292 fix(test): skip stale_detects_geometry_change cleanly when chromium missing` · `e49ff38 chore(release): bump 1.171.0 -> 1.171.1` · `3d24000 feat(cli): extend sddk uat batch with selective filters` · `3c93472 test(cli): add filter predicate coverage for uat batch` · `8a541c3 chore(release): bump 1.171.1 -> 1.171.2` |
| Working tree uncommitted | Vacío (`git status -s` → no output). Todos los cambios de session-12 commiteados y pusheados. |
| Tests verified this session | `cargo test -p sddk-cli --lib` → 791 passed (session-11: 784; +7 son `uat_batch_filters_tests::…`). 0 failed, 1 ignored (chromium skip clean). `cargo clippy -p sddk-cli --all-targets -- -D warnings` → clean. `cargo fmt --check` → clean. |
| Real-provider binary availability | `cognicode` CLI v0.97.3 presente en PATH; `cognicode-mcp` AUSENTE (typer not installed for `mcp` CLI bridge). `chronos-mcp` AUSENTE. `jcode` v0.86.0 presente; `jcode-sdk` no publicado. **C2 sigue NOT_EVALUATED.** |
| Siguiente acción exacta | (1) Si operador autoriza: `bash scripts/release.sh` para publicar v1.171.2 con FC-1 v2 + chromium-skip fix sobre el binario ya verificado. (2) Sin release: continuar con próximos WorkItems del roadmap (FC-2 `sddk doctor --format json` ya IMPLEMENTED en session-11; restantes abiertos: FC-4 `uat replay --release`, FC-6 `vault show <adr-id>`, FC-7 `uat status --format json`, FC-8 `uat validate --format json`). |
| Evidencia requerida para mover puntero | Para C4 PROMOTION a `CERTIFIED_BASE`: ejecutar C2 con providers reales (cognicode-mcp + chronos-mcp + jcode-sdk instalados), re-run T01-T35 contra el SHA de release, ejecutar `tests/clean_machine_uat.sh --tag v1.171.0` en podman. Las session-12 evidence no cubren el G2.5 systemic missing-provider. |
| Bloqueos y decisiones | **C2 sigue cerrado honesto NOT_EVALUATED** (receipts c2a/c2b/c2c). **C3a-h cerrado PASS_OBSERVED**. **Override SemVer LIFTED en v1.171.0**; v1.171.1 fue patch (3 commits: 1 fix + 2 docs); v1.171.2 es minor (1 feat + 1 test sobre 1.171.1) — retorno automático a SemVer-correct sin override en este punto. **FC-3 RECHAZADO por duplicación con `sddk ledger export --cycle`**. **FC-1 IMPLEMENTED v2** (extensión de `UatBatchArgs`, no nuevo subcomando). J7/J8/J9/X08/R11 siguen DEFERRED. |
| Próxima revisión | Al inicio de **cada** sesión y después de cada commit/release relevante |

## Recuperación sin adivinar

1. Confirmar qué rama contiene el nuevo plan, si el PR está integrado y cuál es la versión real de main. Si no está integrado, el puntero de main no ha cambiado.
2. Leer [ROADMAP.md](ROADMAP.md) y [CERTIFICATIONS.md](CERTIFICATIONS.md); localizar el último recibo **observado** del hito activo y los casos [UAT](UAT-MATRIX.md) no ejecutados.
3. Contrastar el último bloque de [SESSION-JOURNAL.md](SESSION-JOURNAL.md) con `git log -5` y el estado operativo; si difieren, registrar reconciliación como **nueva** entrada, sin editar el pasado.
4. Solo entonces abrir/continuar el próximo WorkItem. Un resumen de sesión, un commit de docs o un dry-run no sustituyen un recibo de certificación.

## Estado de certificación (al cierre de session-12 / 2026-09-22T19:33Z)

- **C1 (Base)**: cerrada con full profile 4998/0/15 sobre e7968f8; certificados H02/H05+H06/cycle-c.
- **C2 (Integraciones reales)**: **NOT_EVALUATED_PROVIDER_MISSING (C2a, C2b) / NOT_EVALUATED_ADAPTER_MISSING (C2c)**. Cierre honesto documentado en `docs/roadmap/receipts/c2a/`, `c2b/`, `c2c/`. Status systemic; recovery requires operator decision (instalar `cognicode-mcp` con typer, instalar `chronos-mcp`, publicar `jcode-sdk`).
- **C3a-h**: TODOS **PASS_OBSERVED** — Authority hardening (C3a), Storage adversarial (C3b-c), Performance baseline (C3d), Schema resilience (C3e), Migration re-application safety (C3f, closes C3e-F1 via ADR-0141), Performance budget harness (C3g), Supply-chain audit + remediation (C3h, 0 vulns).
- **C4 (Release y certificación de producto)**: **v1.171.0 PASS_PARTIAL_OBSERVED** — release publicado en GH como Latest (2026-09-22T17:11:44Z). CERTIFICATION-RECEIPT schema-compliant emitido en `docs/roadmap/receipts/c4-release-v1.171.0/CERTIFICATION-RECEIPT.yaml` (236 líneas, schema_version=1 per CERTIFICATIONS.md §5). Resumen de gates:
  - 4 gates **PASS_OBSERVED** (T28 full verify, T29 install, T31 SHA coherence, T33 provider-absence preserved Base).
  - 12 gates **HISTORICAL_CARRY_OVER** (G0..G6, G9, G10, G12..G15) con respaldo en `cargo test --workspace` 5037/0/5.
  - 1 gate **NOT_VERIFIED** (G11 security/secrets — R14 OPEN_NON_BLOCKER since A5-C).
  - 4 UAT scenarios **NOT_RUN** (T08-T18 C2 providers, T23 adversarial, T26 performance, T30 cross-version, T32 failed-gate adversarial).
- **C5 (Evolución condicionada)**: pendientes P2/P3 (J7/J8/J9/X08/R11), ninguno activo.
- **v1.171.2 release candidate (workspace ahead, no publicado)**:
  - **FC-1 v2 IMPLEMENTED**: `sddk uat batch` extendido con `--scenario`, `--flag`, `--priority`, `--exclude-flaky` (commits `3d24000` + `3c93472`). 7 nuevos tests predicate. Sin duplicación de código: helpers puros (`batch_filter_matches`, `uat_priority_label`) reutilizables.
  - **Chromium-skip fix**: test `stale_detects_geometry_change` ahora hace skip limpio cuando chromium no está instalado (commit `0974292`), no panic.
  - **Gates verificados localmente**: `cargo test -p sddk-cli --lib` 791/0/1, clippy clean, fmt clean, shellcheck scoped subset clean.
- **Session-12 audit findings** (operador: "CLOSED != CERTIFIED"):
  - RECEIPT.md original (session-11) era un release receipt, NO un CERTIFICATION-RECEIPT (schema §5). Ahora reemplazado por CERTIFICATION-RECEIPT.yaml.
  - 40 INCs declaradas `status: closed` fueron auditadas; todas tienen evidencia de cierre (commit refs, test names, exit codes, matrix case counts). NO hay cierres paperwork.
  - FC-3 working tree RECHAZADO: duplica `sddk ledger export --cycle X --output <file>`. La extensión propuesta es extender `LedgerExportArgs` con `--format text|json|jsonl` y `--output opcional` (FUTURO, no en este ciclo).
  - FC-5 working tree NO implementado pero marcado DEFERRED-DUPLICATED con `sddk fork diff` y `sddk memory diff`.

**Distinción importante**: workspace v1.171.2 == next-tag `v1.171.2` (alineado per AGENTS.md §2.3, pendiente de `bash scripts/release.sh`). El binario instalado en `~/.local/bin/sddk` corresponde al tag v1.171.0 (sha256 `5e9d5fbd...`). Próximo release: si operador autoriza, `bash scripts/release.sh` recortará tag v1.171.2 + binario + bundle + instalación local.
