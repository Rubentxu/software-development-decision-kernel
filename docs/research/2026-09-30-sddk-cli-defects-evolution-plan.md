# Plan evolutivo — Defectos de SDDK 2.2.33 reportados desde agent-secretless

**Fecha:** 2026-09-30 · **Origen:** `agent-secretless` `docs/receipts/sddk-2.2.33-defects.md` (evidencia OBSERVED vía CLI, sin checkout del framework en esa máquina). **Método de este documento:** cada defecto confirmado o matizado contra el código fuente de `sddk-framework` (rama `main`, rango `b5abf758..1927d215`, workspace 2.2.37) con `file:line`, y aterrizado como hito **C3k** en `docs/roadmap/ROADMAP.md`.

**Estado tras la investigación:** los 7 defectos (D1–D7) y los 4 items de sección 3 del report están **confirmados en el código fuente**, con matices que el report no podía ver sin checkout. Nada está implementado todavía: este documento es la base de evidencia; ROADMAP.md es el plan.

---

## 1. Tabla de confirmación (report → código fuente)

Todas las causas raíz son OBSERVED (leídas en el código de este checkout). Los matices INFERRED se marcan.

| ID | Defecto (report) | Veredicto | Causa raíz |
|---|---|---|---|
| D1 | `uat sign-off` firma plan vacío, sin evidencia, actor libre | **CONFIRMADO** | `run_uat_signoff` (`crates/sddk-cli/src/uat.rs:1687`): nunca parsea el plan (solo `plan_path.exists()`), snapshot literal `"sha256:{}"` sin manifest (uat.rs:1728), `outstanding_findings: Vec::new()` hardcodeado (uat.rs:1734), `--actor` sin validación de patrón de agente |
| D2 | Identidad derivada de la URL; cambio de case → ledger nuevo en silencio | **CONFIRMADO con matices** | `normalize_remote_path` (`crates/sddk-domain/src/identity.rs:310`) preserva el case del path del repo (el host sí se normaliza a lowercase en identity.rs:296); `ensure_project_row` (`crates/sddk-cli/src/admission.rs:593`) materializa fila+ledger nuevos sin warning. **Matices:** (a) `SDDK_PROJECT_ID` SÍ existe pero solo como fallback de `uat` (`uat.rs:1314`), indocumentado e inconsistente con el resto de la CLI — el report lo dio por inexistente; (b) no hay `sddk project set/pin`: `ProjectCommand` (`lib.rs:547`) solo expone `Resolve` |
| D3 | `backlog discard` destruye hallazgos sin requisito de sucesor | **CONFIRMADO** | `BacklogDiscardArgs` (`crates/sddk-cli/src/backlog.rs:162`) sin `--superseded-by`; `require_transitionable` (backlog.rs:454) solo valida estado terminal; el schema del evento solo exige `item_id/reason/discarded_at` (`sddk-domain/src/event_registry/schemas.rs:381`) |
| D4 | `uat status` resuelve rutas por CWD | **CONFIRMADO** | `run_uat_status` (`uat.rs:1998`): construye `uat-plan-<release>.yaml` y `uat-report-<release>.yaml` relativos al CWD, sin `--root` ni anclaje al workspace |
| D5 | `uat plan --from` no valida el release de partida | **CONFIRMADO** | `run_uat_plan` (`uat.rs:771`): `last_uat_release: args.from` se copia literal; no hay validación contra tags |
| D6 | `uat plan` nunca genera escenarios | **CONFIRMADO** | `features: Vec::new()` hardcodeado (`uat.rs:782`); el esqueleto es por diseño, pero ningún output ni doc lo declara |
| D7 | `backlog render` sobrescribe sin check de drift | **CONFIRMADO** | `BacklogCommand` (`backlog.rs:27`) expone Capture/Triage/List/Show/Promote/Discard/Render; no hay `Check` ni `render --check` |
| S3.1 | Gate `release-receipt` sin evaluador registrado | **CONFIRMADO** | `WorkflowEngine::register_evaluator` (`crates/sddk-engine/src/lib.rs:1165`) no tiene **ningún call site en el CLI** (solo tests del engine); `evaluate_gate` falla con `UnregisteredEvaluator` para cualquier gate |
| S3.2 | Prompt de release desalineado del motor | **CONFIRMADO** | `prompts/sddk/phases/release.md:109` y `:176-181` instruyen `evaluate-gate --gate release-receipt / no-pending-effects --evaluator sddk.cli`; con S3.1 ese comando está condenado a fallar |
| S3.3 | Sin schema de `report` ni `session` en `uat validate` | **CONFIRMADO** | `run_uat_validate` (`uat.rs:823`) parsea por `schema_version` y trata todo como plan; `UatSession` existe en domain (`sddk-domain/src/uat.rs:1559`) pero no tiene validador de superficie propio |
| S3.4 | Ingest multi-sesión pisa el resultado | **CONFIRMADO** | `upsert_uat_result` (`crates/sddk-storage/src/control_plane.rs:255`): `ON CONFLICT(project_id, tag_version) DO UPDATE` pisa todo; `session_count` cuenta **results de una sesión** (`uat.rs:1134`), no número de sesiones |

**Lo que el report pedía verificar y ya está bien (sección 4 del report):** sin cambios; `ledger verify`, idempotencia de `render`, rechazo de sign-off sin plan, etc. siguen vigentes en este checkout.

---

## 2. Interpretación (Knowledge → Alignment)

- **Los 11 hallazgos comparten dos raíces estructurales.** (1) *Proyecciones sin gate de realidad*: `uat plan`/`sign-off`/`status` y `backlog render` emiten o aceptan estado sin contrastarlo con su fuente (ledger, tags, report). (2) *Autoridad sin mecanismo*: el engine tiene el hook de evaluadores de gates pero nadie lo puebla, y la identidad del proyecto no tiene camino de fijación sancionado — el prompt manda hacer algo que la máquina rechaza (S3.2), y el usuario no puede fijar lo que la máquina deriva frágilmente (D2).
- **D1 es el único con capacidad de contaminar la evidencia de release** (un agente puede fabricar una aceptación humana indistinguible). D2 es el de mayor daño potencial silencioso (writes a un ledger equivocado con éxito). El resto son ergonomía o señal faltante.
- **No-objetivo:** no re-diseñar el modelo de identidad ni el motor de gates; poblar y cerrar los mecanismos ya existentes.

---

## 3. Work items (aterrizados como hito C3k en ROADMAP.md)

Orden por valor/riesgo (critica el report, sección 5; se mantiene tras ver el código):

| WI | Alcance | Criterio de salida falsable |
|---|---|---|
| W1 (D1) | `uat sign-off`: parsear y exigir plan con ≥1 escenario; rechazar `evidence_snapshot_sha256: "sha256:{}"` salvo `--allow-empty-evidence` explícito; `--agent-authored` obligatorio para `--actor agent:*` con decision `accepted`; rechazar justificación vacía | sign-off de plan vacío falla con código no-cero; registro con `agent:*` sin flag no se emite; tests RED→GREEN negativos |
| W2 (D2) | (a) normalizar case del path del repo en `normalize_remote_path` (fix de 1 línea + regresión con URLs del report); (b) warning fail-loud (no-cero con `--strict`) cuando un write materialice un ledger vacío para un `project_id` sin eventos; (c) `sddk project pin` que persista identidad en `adoption.json` y que `resolve` respete; (d) documentar o eliminar `SDDK_PROJECT_ID` en uat | case-cambio resuelve al mismo `p-*`; pin sobrevive a cambio de remote; regresión con los 3 comandos de la sección 6 del report |
| W3 (D3) | `backlog discard --superseded-by <id>` requerido cuando `reason=superseded`, verificando que el sucesor exista y mencione al predecessor | discard superseded sin sucesor falla; con sucesor válido pasa; wontfix/duplicate sin cambio |
| W4 (S3.1+S3.2) | Registrar evaluadores `sddk.cli` para los gates declarados del workflow (`release-receipt`, `no-pending-effects`) en la construcción del engine del CLI, con evaluación material (artefactos presentes, tag↔SHA binding); o si la decisión de modelo es otra, corregir los prompts. Requiere decisión: **evaluador real vs prompt corregido** | `evaluate-gate --gate release-receipt` con `--evaluator sddk.cli` emite receipt pass/fail real; release.md ejecutado al pie de la letra no produce `ENGINE_UNREGISTERED_EVALUATOR` |
| W5 (D4+D5+D6) | `uat status --root` (o anclaje al workspace); `uat plan --from` valida contra `git tag`; `uat plan` deriva features del diff contra `--from` o declara en output que es esqueleto manual | status consistente desde subdirectorio; `--from banana` falla; plan generado declara su naturaleza |
| W6 (D7) | `backlog render --check` (exit no-cero ante drift) para CI | archivo mutado a mano → check falla; archivo limpio → pasa |
| W7 (S3.3+S3.4) | `uat validate` distingue plan/session/report por `schema_version`/forma; `uat_results` acumula sesiones (fila por sesión o contador acumulativo) y `session_count` cuenta sesiones reales | 9 ingests de 9 sesiones → `session_count=9` con coverage agregado; validate rechaza session malformada con mensaje de session |

**Dependencias:** W1..W3, W5..W7 independientes entre sí. W4 depende de una decisión de modelo del operador (evaluador material en CLI vs prompt corregido) porque toca la autoridad de gates. Tras arreglar W3, el guard `scripts/backlog-lineage-check.py` de `agent-secretless` puede mudarse al framework como test de contrato (sugerencia del report).

**Clasificación de deuda:** al abrir el ciclo, D1→INC nueva high/P1; D2→INC high/P1; S3.4→medium/P2; W4→medium/P2 (bloquea a quien siga el prompt de release); resto low/P3. Las INC se abren contra `docs/debt/` con su guard de coherencia.

---

## 4. UAT propuesto (para UAT-MATRIX.md cuando C3k pase a READY)

- `UAT-SIGN-001..004`: sign-off (plan vacío, actor agente sin flag, evidencia vacía, flujo completo sano).
- `UAT-IDEN-001..004`: identidad (case-cambio, pin, ledger vacío fail-loud, SDDK_PROJECT_ID documentado o ausente).
- `UAT-BLG-001..003`: discard con/sin sucesor, render --check.
- `UAT-GATE-001..002`: evaluate-gate release-receipt/no-pending-effects con evaluador registrado.
- `UAT-UAT-001..004`: status root, plan --from inválido, plan esqueleto, ingest multi-sesión.

---

## 5. Reproducción de las causas raíz (framework checkout)

```bash
# D1: sign-off sin parsear plan ni evidencia
grep -n "plan_path.exists()" crates/sddk-cli/src/uat.rs        # uat.rs:1701
grep -n 'sha256:{}' crates/sddk-cli/src/uat.rs                  # uat.rs:1728
# D2: case del path preservado
sed -n '310,330p' crates/sddk-domain/src/identity.rs            # sin to_lowercase del path
# S3.1: cero registros de evaluadores en el CLI
grep -rn "register_evaluator" crates/sddk-cli/src/ | wc -l      # 0
# S3.4: upsert pisa
sed -n '255,270p' crates/sddk-storage/src/control_plane.rs      # ON CONFLICT DO UPDATE
```
