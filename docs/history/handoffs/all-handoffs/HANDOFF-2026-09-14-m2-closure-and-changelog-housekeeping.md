# Handoff — 2026-09-14 — M2 approval loop closure + changelog housekeeping

## Goal

Cerrar el ciclo WU-C4-7 / R-4-002 (M2 approval loop) del SDDK
framework's authority engine y, post-cierre, sincronizar `CHANGELOG.md`
y `manifest.toml` con el historial de releases v1.169.x que se había
quedado sin documentar.

## Estado final (verificado)

- **HEAD = origin/main = `de3483c`**
- **`sddk` 1.169.10** activo (último binary release publicado).
- Doctor: `c4.authority_single_admission: present`, `all_present: true`.
- Workspace full profile (en HEAD `a323c7d` base, byte-identical a
  `de3483c` salvo docs/version markers): 3877/3877 tests ok,
  `cargo fmt --check` clean, `cargo clippy --workspace --all-targets
  -- -D warnings` clean, 1110/1110 cli tests ok.
- **Releases binarios publicados**:
  - **v1.169.8** (feature, 2026-09-14T08:44:08Z) — M2 approval loop
    end-to-end.
  - **v1.169.10** (fix, 2026-09-14T09:41:35Z) — Fix FK noise + audit
    event loss en `record_admission_decision`.
- **Workspace version** = `manifest.toml` `pack.version` = `1.169.12`
  (drift de 4 minor versions detectado y arreglado durante el ciclo
  changelog-housekeeping).
- **`CHANGELOG.md` sincronizado** con el historial v1.169.x completo
  (1.169.0-1.169.11): 4 entradas nuevas en la cabecera, latest entry
  `[1.169.11]` 2026-09-14.

## Trabajo realizado en la sesión

### Fase 1 — M2 closure (cycle `m2-approval-loop-closure`, ya cerrado)

| Etapa | Resultado |
|---|---|
| e2e flow probe con binario 1.169.8 | `adopt apply` + `cycle start` + `lock acquire` + `supersede` + `approval list/grant` funcionan. Estamos en stage `LowMedium` (M1), `RequireApproval` es advisory en High surfaces por diseño (M4 = blocking). |
| 2 bugs latentes descubiertos en `record_admission_decision` | (1) FK noise: cross-project broadcast iteraba todos los dirs del state global. (2) Audit event loss: `SqliteEventStore::append` upsert violaba NOT NULL constraints (`display_name`, `scope`, `created_at`) que Storage migrations añaden al mismo `projects` table, dejando el row vacío → FK fail. |
| Fix (`3499d40`) | `record_admission_decision` escribe solo en el `project_id` del `ApprovalLoopContext` (M2 path) o en el único proyecto bootstrapped (legacy path). Nuevo helper `ensure_project_row` que abre vía `Storage` (full schema) y upserts con todas las columnas requeridas. |
| Test reescrito (`admission_deny_e2e.rs::human_evaluate_gate_denied_with_zero_side_effects`) | Mide en `events_v1` directamente con `rusqlite::Connection::open_with_flags(READ_ONLY)`. Storage::list_events lee `ledger_events` (tabla separada). Aserciones: `total_non_authority == 0` + `total_authority == 1`. |
| E2E pin (`d5f5d85`) | `crates/sddk-cli/tests/cli_approval_loop_e2e.rs` con 2 tests contra el binario publicado: `m2_approval_loop_request_grant_list_empty` (S1 → S2 → ledger invariants) + `m2_approval_deny_after_grant_is_rejected` (deny fail-closed). |
| Refactor (`c3459db`) | 4 `cli_wrapper_*` tests migrados de `enforce_admission_or_block[_in]` legacy wrappers a `enforce_admission_or_block_ctx` directo con `ApprovalLoopContext::none()`. Wrappers + `#[allow(dead_code)]` markers eliminados. |
| Release v1.169.10 | `bash scripts/release.sh --skip-install` + `bash scripts/install.sh --version v1.169.10 --editor all` + `sddk dev update --prune-only --keep 1`. Manual repro confirma stderr limpio (sin FK noise) y 1 `authority.admission.decided` event en `events_v1`. |

### Fase 2 — Changelog housekeeping (cycle `changelog-housekeeping-169`, CLOSED)

**Trigger**: durante la auditoría post-M2 se detectó que `CHANGELOG.md`
estaba estancado en `[1.168.31]` (2026-09-12). Los 8 bumps ceremoniales
(1.169.0-1.169.7 + 1.169.8 + 1.169.10 + 1.169.11) entre el 2026-09-12 y
2026-09-14 NO actualizaron el changelog porque se hicieron manualmente
vía `sed -i 's/^version = ...'` sin invocar `scripts/release-bump.sh`.
Además `manifest.toml` `pack.version` quedó en `1.168.31` — drift de
4 minor versions.

**Path**: B-direct (sin fork arquitectural, bounded).

**Commits entregados**:

| SHA | Subject |
|---|---|
| `bb83651` | docs(changelog): sync 1.169.x release history (1.169.0-11) |
| `de3483c` | chore(release): bump version 1.169.11 → 1.169.12 (changelog housekeeping) |

**Cycle artifacts** (en
`~/.local/share/sddk/projects/p-63676b11dc0ef88f/cycle-artifacts/p-63676b11dc0ef88f/changelog-housekeeping-169/`):

- `implementation-receipt.md` — Build phase receipt
- `verification-report.md` — Verify phase report
- `release-receipt.json` — Release phase receipt (no-binary; UAT waived por scope docs-only)
- `merge-receipt.json` — push receipt
- `inventory.json` — cycle-scoped files inventory
- `archive-manifest.md` — durable ground-truth (also copied to
  `.sddk/cycles/p-63676b11dc0ef88f-changelog-housekeeping-169/archive-manifest.md`)

**Gates**:
- `implementation-complete` (passed)
- `tests-pass` (passed; rationale: docs-only, source tree byte-identical
  al HEAD pre-changelog `a323c7d` con full profile ya verde)
- `policy-compliant` (passed; AGENTS.md §2.1, §2.2, §2.3, §2.6,
  §2.10, §4.1, §5 todas verificadas)
- `no-pending-effects` (passed; zero binary to install)
- `release-uat-approved` (waived por scope docs-only; no binary
  shipped)
- `ledger-valid` (passed)
- `vault-index-current` (passed)

**Verificación**:
- `verify-references` → `PASSED`, 0 dangling refs.
- 6 eventos en ledger (3 transitions + 3 lease.released): seq=4
  build→verify, seq=6 verify→release, seq=8 release→archive.
- Cycle status: `CLOSED`, `phase: archive`, `path: B-direct`.

## Estado del roadmap tras esta sesión

- **M2 cerrado** con dos releases binarios publicados (1.169.8 + 1.169.10)
  y un refactor test-only (1.169.11 / 1.169.12 ceremonial bumps).
- **CHANGELOG y manifest en lockstep** con el historial de versions.
- **Sin deuda obligatoria abierta.** 3 followups evaluados:
  1. *Pre-push guard para drift `manifest.toml` ≠ workspace version* →
     no necesario: `scripts/release-bump.sh` líneas 100-103 ya sincroniza
     `manifest.toml`; el problema M2 fue de proceso (no se invocó
     release-bump.sh), no de tooling.
  2. *Wrapper script bump+changelog+manifest atómico* → redundante:
     `scripts/release-bump.sh` ya cubre los 4 archivos
     (`Cargo.toml` + `manifest.toml` + `Cargo.lock` + `CHANGELOG.md`).
  3. *Investigar `duplicate_event_id` warning upstream* → trabajo de
     investigación no bounded; fuera de scope de housekeeping.

## Incidencias/problemas resueltos durante la sesión

1. **`manifest.toml` drift de 4 minor versions**: cerrado con el bump
   1.169.12.
2. **`record_admission_decision` FK noise + audit event loss**: cerrado
   en commit `3499d40`; release v1.169.10.
3. **Deny e2e test silenciosamente verde con caso degenerado**: cerrado
   en commit `3499d40` (test reescrito para medir en `events_v1`
   directamente).
4. **Manual bumps sin `release-bump.sh` durante M2**: cerrado al
   sincronizar CHANGELOG manualmente en commit `bb83651`. Recomendación:
   todo bump ceremonial futuro debe invocar `scripts/release-bump.sh`
   para mantener el contrato (cubre los 4 archivos).

## Gotchas / lecciones

- `sddk dev install` auto-linkea agents/skills/prompts pero NO
  regenera el bundle: el bundle se actualiza al hacer release binario.
- **El error `event_store:duplicate_event_id:authority-approval-system-cli_run-advisory_high_approval`**
  es fail-soft por diseño del storage (rechaza idempotent retries con
  content_hash distinto); la transición del cycle progresa aunque el
  evento del audit trail no se grabe. Investigar upstream si la
  generación determinística de event_id incluye el actor
  (`system:rubentxu` vs `user:*`).
- **`record_admission_decision` opera sobre `state/sddk/projects/*`**,
  no sobre el proyecto del `--root` CLI arg. Por eso el M2 cycle-aware
  path usa `ApprovalLoopContext.project_id` explícito.

## Next steps

- Nada pendiente del roadmap inmediato.
- Próximo trabajo natural (cuando toque):
  - Promover SPEC-005 / SPEC-006 a `implemented` (M3 — SemanticGraph
    + Vault consolidation). Requiere spike SP-03 + revisión de los
    ADRs ADR-0098 + ADR-0099.
  - M4 stage `All` para activar `RequireApproval` blocking en High
    surfaces (cambio 1 línea en `EnforcementStage` enum + tests).
  - Cualquier release binario futuro debe invocar
    `scripts/release-bump.sh` para mantener CHANGELOG + manifest.toml
    sincronizados.

## Persistencia durable

- **Cycle archive-manifest M2**:
  `.sddk/cycles/p-63676b11dc0ef88f-m2-approval-loop-closure/archive-manifest.md`
- **Cycle archive-manifest changelog-housekeeping-169**:
  `.sddk/cycles/p-63676b11dc0ef88f-changelog-housekeeping-169/archive-manifest.md`
- **Memoria de sesión** Jcode memory scope project:
  `sddk-m2-closure-and-changelog-housekeeping-2026-09-14`
