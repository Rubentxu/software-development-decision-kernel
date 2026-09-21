# SCOPE-CONTRACT — C0 — Reconciliación y baseline reproducible

> **Slice id:** `p-63676b11dc0ef88f/c0-reconciliation-baseline`
> **Macro-cycle:** nuevo roadmap (`docs/roadmap/ROADMAP.md`)
> **Baseline:** `main@2ffff3127e7179b5f3c3104c471c8ad2c7d920ff` (v1.169.127)
> **Working SHA:** `e8964accfb4832690aaf78a8c305df6556f60dcf` (v1.169.128)
> **Status:** IN_PROGRESS_C0 (no CERTIFIED — UAT T01/T02 ejecutados pero recibo en draft)

## §1 Goal

Ejecutar el primer hito del nuevo roadmap:

1. **Reconciliar la fotografía** del repo: HEAD, tag, release pública, workspace version, AGENTS, mini-roadmap 14/09, A5-C, AIW state, recibos AIW/A6/A7/A8/J, histórico de `docs/`. No modificar el pasado; registrar nuevas entradas de reconciliación si difieren.
2. **Inventariar features** en `IMPLEMENTED / VERIFIED / CERTIFIED / DEFERRED / SUPERSEDED` con `commit + test + receipt` enlazados cuando exista. `CLOSED` sin recibo = `NOT_RUN` por defecto (no PASS por texto).
3. **Ejecutar UAT T01 (HEAD/tag/release/workspace observados)** y **T02 (req→commit→test→receipt mapping)** con `OBSERVED/NOT_RUN` honestos y outputs reales.
4. **Generar `C0-RECEIPT.md`** en `docs/roadmap/receipts/c0/<sha>/` con el resultado real, riesgos aceptados, limitaciones explícitas. `status: PASS_OBSERVED` solo si **todos** los gates aplicables están PASS_OBSERVED; no certificar Base retroactivamente.

## §2 STOP conditions

| Condition | Action |
|---|---|
| Tag / release pública no coinciden con workspace (HEAD) | STOP: registrar discrepancia; si la release está pendiente por `bash scripts/release.sh` del operador, documentar como `NOT_RUN` y no PASS |
| Receipt existente afirma PASS pero el commit no está en `origin/main` o falta test observable | STOP: marcar como `NOT_RUN/DEFERRED` con la causa; emitir reconciliación sin reescribir el pasado |
| Falta binario externo (`cognicode-mcp`, `chronos-mcp`) para UAT EXT | NO simular: marcar como `BLOCKED` con `acceptance_blocked` explícito |
| AIW-S7/S8 afirma DELIVERED pero solo está en commits locales sin push | STOP: reclamar push + UAT observado antes de aceptar el cierre |

## §3 Hard constraints

- **C1**: Sin modificaciones a ADRs aceptados, specs, tests o recibos certificados.
- **C2**: Sin release publicado (`bash scripts/release.sh` es operator-only, system-law `git.release`).
- **C3**: Sin invocar UAT contra binarios EXT inexistentes. `NOT_RUN` honesto.
- **C4**: Sin crear crate/gráfico/autoridad de escritura duplicados.
- **C5**: Sin manipular histórico. Reconciliación = nueva entrada, no reescritura.
- **C6**: Pre-push allowlist respetado: cambios en `AGENTS.md` requieren bump real (rule A).

## §4 Deliverables

| Deliverable | Path | Status |
|---|---|---|
| SCOPE-CONTRACT | `docs/roadmap/receipts/c0/e8964ac/SCOPE-CONTRACT.md` | ✅ este archivo |
| UAT-EVIDENCE | `docs/roadmap/receipts/c0/e8964ac/UAT-EVIDENCE.yaml` | 🔲 |
| RECEIPT | `docs/roadmap/receipts/c0/e8964ac/C0-RECEIPT.md` | 🔲 |
| CURRENT.md reconciled | `docs/roadmap/CURRENT.md` | ✅ |
| STATE.yaml reconciled | `docs/roadmap/STATE.yaml` | ✅ |
| SESSION-JOURNAL.md entry | `docs/roadmap/SESSION-JOURNAL.md` | ✅ (entrada `2026-09-21T11:57:00Z`) |
| AGENTS.md §10 enforced | `AGENTS.md` | ✅ (PR #7 ya integrado) |

## §5 Method

1. **AGENTS.md §10** (entrada): `git fetch`, `git status -sb`, `git rev-parse HEAD`, `git log -5`, `git tag`, `gh release list`.
2. Inventario manual de 40 cycle artifacts, A5-C cert (v1.169.88), recibos A6/A7/A8/J/AIW.
3. UAT T01: `git rev-parse HEAD` ↔ `git tag` ↔ `gh release list` ↔ `Cargo.toml` — todos observados, no asumidos.
4. UAT T02: mapa `feature → commit → test → receipt` por cada perfil (`BASE`, `STATIC_ENHANCED`, `RUNTIME_ENHANCED`, `FULLY_ENHANCED`, `JCODE_CORE_GA`).
5. RECEIPT: SHA, version, gates aplicables, riesgos aceptados, limitaciones.

## §6 Out of scope

- Publicar release v1.169.128 (`bash scripts/release.sh` queda para el operador).
- EXT live activation (cognicode-mcp / chronos-mcp no instalados en este entorno).
- AIW-S8 X08 (Jev corpus + baseline) — DEFERRED per ROADMAP.
- R11 (crate split) — DEFERRED evidence-driven.
- J7 / J8 / J9 — DEFERRED.

## §7 References

- `docs/roadmap/ROADMAP.md` §2 C0
- `docs/roadmap/CERTIFICATIONS.md` §1-§6
- `docs/roadmap/UAT-MATRIX.md` T01, T02
- `docs/history/legacy-packages/architecture-a5-a6/architecture-a5/A5-C-BASE-PRODUCTION-READY-CERTIFICATION.md` (A5-C Base v1.169.88)
- `docs/history/proposals/all-proposals/2026-09-19-adaptive-inputs-workflows/STATE-OF-AIW.md` (AIW snapshot)
- `docs/history/research/all-research/2026-09-21-roadmap-gaps-deep-research.md` (gap survey)
- `tests/cycle-artifacts/p-63676b11dc0ef88f/` (40 cycle dirs)
- `githooks/pre-push` (release admission contract)
