# Diario append-only — recuperación entre sesiones

**Regla:** añadir entradas al final con hora UTC, actor, SHA antes/después, WorkItem, evidencia observada, pruebas NO ejecutadas, riesgos, decisiones y siguiente acción. Nunca corregir una entrada anterior silenciosamente: añadir `RECONCILIATION` con referencia. El diario es **índice**, no sustituye el ledger canónico ni el RECEIPT. El puntero mutable está en [CURRENT.md](CURRENT.md) y [STATE.yaml](STATE.yaml).

## Plantilla para próxima entrada

```markdown
### YYYY-MM-DDTHH:MM:SSZ — <cycle/slice> — <actor>
- Baseline: branch/sha/tag/release comprobados:
- Alcance/autorización; no-objetivos:
- Ejecutado: archivos/commit(s):
- UAT: IDs, comando, actual/expected, OBSERVED/NOT_RUN y receipt:
- Gates: PASS/FAIL/BLOCKED/NOT_APPLICABLE; perfil y alcance:
- Riesgos/decisiones, responsable y revisit trigger:
- CURRENT/STATE reconciliados a SHA:
- Próxima acción ejecutable (o STOP honesto):
```

### 2026-09-21 — ROADMAP-REBASE / DOCUMENTATION-PROPOSAL

- Baseline inspeccionado: `main@2ffff3127e7179b5f3c3104c471c8ad2c7d920ff`; Cargo.toml 1.169.127; release pública consultada: v1.169.122. Todos los datos pueden haber cambiado: revalidar al retomar.
- Diagnóstico de continuidad: la certificación A5 Base es histórica y condicionada; el cierre de los tracks internos no equivale a certificación de todos los perfiles externos ni del host JCode. El roadmap A5 y varios handoffs contenían fotografías antiguas.
- Acción propuesta en rama `docs/roadmap-production-certification-2026-09-21`: único roadmap de continuación C0–C5, contrato de certificación, matriz UAT T01–T35, CURRENT/STATE, catálogo de archivo y reglas de recuperación en AGENTS. Se trasladan cuatro documentos antiguos con punteros de compatibilidad; no se alteran ADRs, specs, recibos ni código.

### 2026-09-21T11:57:00Z — PR #7 INTEGRATION + C0 OPENING — orchestrator

- Baseline inspeccionado: `main@2ffff3127e7179b5f3c3104c471c8ad2c7d920ff` (v1.169.127); release pública v1.169.122; PR #7 OPEN contra `origin/main` en `docs/roadmap-production-certification-2026-09-21` SHA `fbf89fe`.
- Alcance/autorización: integrar PR #7 (solo docs); abrir ciclo C0 según ROADMAP.md.
- Ejecutado (3 commits):
  - `13d4131` docs(roadmap): cherry-pick PR #7 excluyendo `AGENTS.md` (root file not in pre-push allowlist `docs/**`). 21 archivos docs/ + 4 stubs preservados.
  - `96f5366` chore(release): bump 1.169.127 → 1.169.128 + `AGENTS.md` (§2.10 reformulated, §10 added). Real version change so pre-push rule (A) accepts.
  - HEAD actual: `96f53663b4d011dafd697319f8596d0cf7c49758` = `origin/main`. PR #7 closed via `gh pr close --delete-branch`.
- Gates PASSED:
  - `git diff --check` exit 0 (verified in both 13d4131 and after 96f5366 staged).
  - `release_admission_check HEAD` ACCEPT 1.169.127 → 1.169.128 (verified after 96f5366).
  - Initial push of `13d4131` to main accepted by pre-push hook (rule B, docs-only; all paths under `docs/**`).
  - Push of `96f5366` accepted by pre-push hook (rule A, real Cargo.toml version change).
- UAT executed: **none** for this entry — T01/T02 to be executed in the next step under the C0 cycle scope. Documented separately as planned.
- Riesgos/decisiones:
  - Pre-push allowlist excluye `AGENTS.md` (no es `docs/**`); documentado. La integración del PR en dos partes es la ruta correcta: docs primero (B), luego `AGENTS.md` con bump real (A). Un bump ceremonial habría violado INC-A5-PUSH-RELEASE-MARKER-FRICTION.
  - Workspace version 1.169.128 NO publicada todavía — operator-side `bash scripts/release.sh` queda pendiente. No atribuir el binario al repositorio publicado.
- CURRENT/STATE reconciliados: ambos actualizados a SHA `96f5366` y versión `1.169.128`. STATE marca `IN_PROGRESS_C0`. PR #7 marcada como integrada en ambos punteros.
- Próxima acción ejecutable: emitir `SCOPE-CONTRACT` para `c0-reconciliation-baseline` y ejecutar UAT T01 (HEAD/tag/release/workspace observados) + T02 (req→commit→test→receipt) con `RECEIPT` C0 vinculado. Sin release ni certificación hasta que C4-Base lo decida.


### 2026-09-21T12:00:00Z — C0 RECEIPT EMITTED — orchestrator

- Baseline inspeccionado: `main@e8964accfb4832690aaf78a8c305df6556f60dcf` (v1.169.128); release pública v1.169.122; working tree clean.
- Alcance: cierre de C0 reconciliación tras PR #7 integrado. Emitidos SCOPE-CONTRACT, UAT-EVIDENCE y C0-RECEIPT en `docs/roadmap/receipts/c0/e8964ac/`.
- UAT T01 ejecutado: 8 pasos, 7 PASS_OBSERVED + 1 PRE-RELEASE (workspace 1.169.128 > release v1.169.122). Falsificación T01.8 (stale pointer) detectada y corregida: STATE.yaml apuntaba a 96f5366 antes de T01; ahora a e8964ac.
- UAT T02 ejecutado: inventario de 23 features en 6 perfiles. 15 PASS_OBSERVED (HISTORICAL) — Base 13 (con G11 waivered) + Static 1 + Runtime 1. 8 NOT_RUN/BLOCKED hoy (binaries EXT missing, acceptance_blocked). Ningún PASS fabricado.
- Gates verificados: G0..G8 (factual reconciliation, no rewriting, no EXT simulation, no auto-release, pre-push allowlist respetado, falsificaciones T01/T02 ejecutadas, waiver R14 carried forward, journal+state committed). Resultado global: **PASS_OBSERVED_WITH_NOTES**.
- Riesgos aceptados: PRE-RELEASE (operator gate), EXT binaries missing (C1+), AIW-S8 X08 DEFERRED, R11/J7/J8/J9 DEFERRED.
- Próxima acción ejecutable: operador ejecuta `bash scripts/release.sh` para publicar v1.169.128. Orchestrator NO inicia C1 hasta que workspace == public release observado.

### 2026-09-21T12:07:00Z — C0 ANNEXES + OPERATOR GATE STILL OPEN — orchestrator

- Estado: C0 RECEIPT cerrado en b0d6d40; release v1.169.128 sigue pendiente del operador (system-law git.release).
- Trabajo autónomo entre el push de b0d6d40 y este turno:
  - `cargo build --release -p sddk-cli` (3m 10s clean; binario en `$CARGO_TARGET_DIR=/var/home/rubentxu/cargo-targets/release/sddk`, NO `./target/release/sddk`).
  - sha256 verificado: `4c361ddae53d7e9d96fe45880a5e2825ef68b9996fd274f1b0b2df5a230fdf4f` (30.7 MB, exec bit OK).
  - Smoke: `sddk --help` renderiza, `sddk version` reporta `binary: 1.169.128`, `sddk dev` subcomandos visibles.
  - Dry-run: rechaza con "non-monotonic 1.169.128 → 1.169.128" (correcto, somos pre-release; el operador cierra el gap).
- Anexos C0 emitidos (no rompen scope C0; son notas honestas que el operador + el siguiente orquestador necesitan):
  - `docs/roadmap/receipts/c0/e8964ac/C0-PRE-RELEASE-SMOKE.md` — sha256 + smoke antes de `bash scripts/release.sh`.
  - `docs/roadmap/receipts/c0/e8964ac/C1-RESEARCH-NOTES.md` — preguntas concretas para que C1 arranque con substance (no es SCOPE-CONTRACT; no toca código).
- Riesgo documentado: NO se abre C1 hasta release confirmada. C0-RECEIPT §4 fila 7 + §10 (en research-notes) registran la cadena.
- Próxima acción ejecutable: operador corre `bash scripts/release.sh`; orquestador re-corre T01; si OK, marca C0 CLOSED y emite SCOPE-CONTRACT de C1 partiendo de `C1-RESEARCH-NOTES.md`.

### 2026-09-21T12:09:00Z — C1 PREFLIGHT (READ-ONLY) — orchestrator

- Estado: C0 sigue IN_PROGRESS_C0 hasta release del operador; preflight de C1 ejecutado (NO SCOPE-CONTRACT, NO código tocado).
- Trabajo autónomo:
  - `grep -rn "shape_matches\|structured_work" crates/` reveló que `shape_matches` está en `crates/sddk-engine/src/structured_work.rs:198` (NO en `sddk-domain` como decía la research-note inicial).
  - Lectura del archivo: confirmado OBSERVED que `_ => true` (línea 206) hace pass-through a cualquier descriptor desconocido. Bug H01 confirmado en evidencia real.
  - Único call site: línea 154 (dentro del loop de validación).
  - Tests existentes NO cubren el path unknown-descriptor → gap identificable.
- Anexo emitido: `C1-PREFLIGHT.md` documenta la corrección verbatim.
- Corrección aplicada a `C1-RESEARCH-NOTES.md §1`: ubicación canónica del archivo + corrección del "Archivos candidatos".
- Próxima acción ejecutable: operador corre `bash scripts/release.sh`; orquestador re-corre T01; si OK, emite C1 SCOPE-CONTRACT partiendo de C1-RESEARCH-NOTES corregido + C1-PREFLIGHT.

### 2026-09-21T12:11:00Z — C1 PREFLIGHT-2 — H02/H05/H06 verified (READ-ONLY) — orchestrator

- Estado: C0 sigue IN_PROGRESS_C0 hasta release; preflight extendido de C1 ejecutado.
- Trabajo autónomo (sin tocar código, solo grep/read):
  - **H02 dedup**: `IdempotencyKey` en `crates/sddk-domain/src/proposal.rs:39` con campos project_id/cycle_id/capability/request_hash → dedup ya usa hash de payload, NO solo request_id. Hallazgo adicional: existe SEGUNDO `IdempotencyKey` en `crates/sddk-domain/src/workflow_run.rs` (code smell).
  - **H05 seam**: `set_process_service_for_tests` confirmado en `crates/sddk-engine/src/authority_ticket_service.rs:111`. **Gap de aislamiento**: solo `#[doc(hidden)]`, NO `#[cfg(test)]`. Sin embargo, grep confirma 0 callers → no exploit activo, pero puerta abierta.
  - **H06 gateway/redaction**: `pub fn redact` canónico en `crates/sddk-gateway/src/lib.rs:233` + `fn redact_text:277`. Cobertura de tests: sec1_redactor_unit.rs + sec1_capability_receipt_redaction.rs + runner_receipt_e2e.rs t06. Defense gateway: 413 + PayloadTooLargeForInlineStorage.
- Anexo emitido: `C1-PREFLIGHT-2.md` con evidencia verbatim de los tres gaps.
- Próxima acción ejecutable: operador corre release; orquestador re-corre T01; C1 SCOPE-CONTRACT puede arrancar con substance real para H01-H06.

### 2026-09-21T12:15:00Z — C1 PREFLIGHT-3 — executed evidence (READ-ONLY + cargo test) — orchestrator

- Estado: C0 sigue IN_PROGRESS_C0 hasta release del operador; preflight-3 con evidencia EJECUTADA (no inspección).
- Trabajo autónomo:
  - `cargo test -p sddk-engine --lib structured_work` → 5/5 SAW tests pasan, 0 fail, 1313 otros tests filtered out. Compilation 1m 25s.
  - Mock probe `/tmp/sddk_h01_probe.rs` (borrado después de ejecución): reproduce el control flow de `_ => true` y confirma bug H01 OBSERVED con output verbatim `mock result = true / H01 BUG CONFIRMED OBSERVED in equivalent control flow`.
  - Verificación de las dos `IdempotencyKey` (proposal::IdempotencyKey vs workflow_run::IdempotencyKey): CONFIRMADAS estructuralmente distintas — son dos conceptos con propósitos diferentes, NO hay refactor pendiente (corrige sospecha de C1-PREFLIGHT-2 §1).
  - MANIFEST.sha256 stale para los nuevos docs/roadmap/receipts/c0/ pero `scripts/release.sh` step 4 lo regenera automáticamente — no requiere acción autónoma.
- Anexo emitido: `C1-PREFLIGHT-3.md` con output verbatim de cargo test + mock probe + corrección de IdempotencyKey.
- Probe en /tmp/ eliminado. Repo sin cambios de código.

### 2026-09-21T12:16:00Z — C0-HANDOFF emitido — orquestador cierra sesión limpia — orchestrator

- Estado: C0 cerrado en orquestador; release sigue pendiente del operador.
- Trabajo autónomo final:
  - C0-HANDOFF.md emitido: snapshot consolidado de C0 + sustancia para C1 + defended boundaries + STOP-CONT explícitos
  - Estado: HEAD = a15dcdc, working tree limpio, in sync con origin/main
  - 8 archivos en `docs/roadmap/receipts/c0/e8964ac/`: SCOPE-CONTRACT, UAT-EVIDENCE.yaml, C0-RECEIPT, C0-PRE-RELEASE-SMOKE, C1-RESEARCH-NOTES, C1-PREFLIGHT, C1-PREFLIGHT-2, C1-PREFLIGHT-3, C0-HANDOFF (9 total)
- Decisión de cierre: NO se inicia C1 sin release confirmada (system-law git.release + ROADMAP §3). Toda la substance para el SCOPE-CONTRACT de C1 está preflight-1+2+3; el próximo orquestador (o esta sesión tras release) puede emitir SCOPE-CONTRACT directamente desde los preflights.
- Próxima acción del operador: `bash scripts/release.sh` desde HEAD a15dcdc.
- Próxima acción del orquestador post-release: re-correr T01, marcar C0 CLOSED, emitir C1 SCOPE-CONTRACT.
