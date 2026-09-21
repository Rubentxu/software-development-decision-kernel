# SCOPE-CONTRACT — C1 — Hardening de contratos y aislamiento

> **Slice id:** `p-63676b11dc0ef88f/c1-contracts-hardening`
> **Baseline SHA:** `eae4f7f` (origen/main)
> **Working SHA:** `eae4f7f` (will be updated as slices land)
> **Date (UTC):** 2026-09-21T13:55:00Z
> **Status:** OPEN — ready to begin with H01 (shape_matches descriptor validation)
> **ROADMAP ref:** `docs/roadmap/ROADMAP.md` §2 C1

## §1 Goal

Endurecer los contratos de autoridad y aislamiento de SDDK sin romper Base ni su certificación histórica. C1 se descompone en 4 sub-slices H0X (per ROADMAP §2 C1):

- **H01:** `structured_work::shape_matches` no debe devolver `true` para descriptor no soportado.
- **H02:** `structured_work::submit` debe rechazar `request_id` duplicado con contenido distinto.
- **H05:** El seam `set_process_service_for_tests` debe estar aislado de producción.
- **H06:** La redacción canónica (`redact()`) debe aplicarse consistentemente en todos los entry points.

## §2 Substance already collected (preflights)

| Slice | Substance |
|---|---|
| H01 | Bug verbatim en `crates/sddk-engine/src/structured_work.rs:198-208` con `_ => true` fallthrough. Único call site: línea 154. 5/5 SAW tests pasan hoy (sin cobertura del path unknown-descriptor). Mock probe ejecutable reproduce bug. Test propuesto: `saw007_unknown_descriptor_rejected`. |
| H02 | `proposal::IdempotencyKey` (proposal.rs:39) usa `(project_id, cycle_id, capability, request_hash)` — NO solo `request_id`. Dual `IdempotencyKey` (`workflow_run.rs:147`) son conceptos distintos. Comportamiento a verificar: same-key-different-payload rejection; interpolación de errores sensibles. |
| H05 | `set_process_service_for_tests` en `crates/sddk-engine/src/authority_ticket_service.rs:111`. Solo `#[doc(hidden)]`, NO `#[cfg(test)]`. 0 callers activos. Fix de 1 línea: `#[cfg(test)]`. |
| H06 | `pub fn redact` canónico en `crates/sddk-gateway/src/lib.rs:233` + `fn redact_text:277`. Tests SEC1 cubren stdout/receipts. Verificación pendiente: ¿se aplica consistentemente en todos los entry points? |

## §3 Method (RED→GREEN)

Para cada slice:

1. **Caracterización RED**: añadir test que reproduce el bug actual y falla con código existente.
2. **Aplicación mínima**: el cambio más pequeño que satisface el test.
3. **Caracterización GREEN**: el test pasa.
4. **No-regresión**: suite del crate + SAW tests siguen verdes.

## §4 STOP conditions

- Si el fix rompe algún test A5-* certificado → STOP, marcar R-WAVE antes de avanzar.
- Si el fix es breaking para host adapters (JCode, etc.) → STOP, requerir ADR o adapter de compatibilidad.
- Si el test RED no es caracterizable (e.g. bug no observable) → STOP, marcar como ya fixed.
- Si los pre-push o admisión fallan → STOP, no eludir con flag, solicitar al operador.

## §5 Hard constraints

- **C1.1:** No modificar A5-* certified artifacts (history is sacred).
- **C1.2:** No publicar release (system-law `git.release`).
- **C1.3:** Pre-push allowlist respetado: el rango debe admitir por rule A (real bump) o rule B (docs-only).
- **C1.4:** Release admission debe ser verde al final del slice para que el operador pueda publicar limpiamente.
- **C1.5:** Cambios atómicos por concernencia (AGENTS §2.1): un commit por H0X.

## §6 Deliverables (per slice)

| Deliverable | Path |
|---|---|
| SCOPE-CONTRACT (este) | `docs/roadmap/receipts/c1/<sha>/SCOPE-CONTRACT.md` |
| Test code | `crates/sddk-engine/src/structured_work.rs` (mod tests) |
| Fix code | `crates/sddk-engine/src/structured_work.rs` (function) |
| UAT-EVIDENCE | `docs/roadmap/receipts/c1/<sha>/UAT-EVIDENCE.yaml` |
| C1-RECEIPT (al final del ciclo) | `docs/roadmap/receipts/c1/<sha>/C1-RECEIPT.md` |

## §7 Out of scope (defended)

- C2 (CogniCode, Chronos, JCode real) — distinto DAG.
- C3 (seguridad adversarial) — paralelo a C2, después de C1.
- Performance budgets (C3 §5 del ROADMAP) — sin baseline previo.
- Nuevos crates o refactors cosméticos.
- R11 (crate split) — DEFERRED.
- AIW-S8 X08 (Jev corpus) — DEFERRED.

## §8 First slice (H01)

**Working file:** `crates/sddk-engine/src/structured_work.rs`

**Bug:** function `shape_matches` (línea 198) tiene `_ => true` que acepta cualquier descriptor desconocido como match. H01 dice que un descriptor no soportado NO debe dar `true`.

**Test RED:** añadir `saw007_unknown_descriptor_rejected` que verifica `shape_matches(json!("hello"), "this-descriptor-does-not-exist") == false`. Test debe fallar con código actual.

**Fix mínimo:** cambiar `_ => true,` → `_ => false,` en línea 206.

**Test GREEN:** el nuevo test pasa. Los 5 SAW tests existentes siguen verdes (no tocan el path unknown).

**No-regresión:** `cargo test -p sddk-engine --lib structured_work` debe reportar 6/6 pass.

**Commit:** `fix(engine): H01 — reject unknown descriptors in shape_matches`. Single atomic commit per AGENTS §2.1.

**Bump companion:** `chore(release): bump version a 1.169.131 — admission for H01+release`. Esto restaura admisión para que el operador pueda publicar limpiamente tras la sesión (pre-push rule A sobre el rango entero).

**Push:** ambos commits en el mismo push. Pre-push rule A acepta por el bump; admisión verde porque HEAD = bump.

## §9 Operator action post-C1

Si C1 cierra con admisión verde:

1. Operador corre `bash scripts/release.sh` desde HEAD.
2. Operador notifica al orquestador para re-correr T01.
3. Orquestador marca C1 como CLOSED si los gates lo permiten.
4. Siguiente: C2 (integraciones reales) — `c2a-cognicode`, `c2b-chronos`, `c2c-jcode`.
