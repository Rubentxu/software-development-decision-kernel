# FEATURE-CANDIDATES — Capacidades de usuario pendientes

> **Estado:** Documento de inventario (no es un backlog ejecutable por sí mismo).
> Cumple ROADMAP §C0 acción "inventariar `IMPLEMENTED/VERIFIED/CERTIFIED/DEFERRED`
> por feature".
>
> **Cuándo usarlo:** cuando el operador (o el agente en AUTO) esté considerando
> añadir features genuinas que justifiquen un minor bump en el siguiente
> `bash scripts/release.sh` (i.e. commits con prefijo `feat:` SemVer-compliant).
>
> **No-objetivo:** reabrir ROADMAP-§C5-DEFERRED sin disparador explícito.

## 0. Cómo se conecta con SemVer

Cada candidato aquí, si se ejecuta con `feat:` commits, **empujará minor**
en el próximo `scripts/release.sh` (no patch). Esto es independiente del
workspace version (que es ceremonial per CONTRIBUTING-SEMVER.md §3).

Mecánica del script de release (resumida, ver CONTRIBUTING-SEMVER.md §4 para
detalle):

```
git log $last_tag..HEAD --pretty=%s
  ├─ si hay `feat:` o `feat(...):` → MINOR bump
  ├─ si hay `BREAKING CHANGE:` en body → MAJOR bump
  └─ si solo `fix:/test:/docs:/chore:` → PATCH bump
```

## 1. Capacidades candidatas (ordenadas por madurez)

### FC-1: `sddk uat run --filter` (filtrado selectivo de UATs)

**Origen:** ROADMAP §C0 — "matriz UAT T01–T35 congelada"; observación operativa:
ejecutar los 35 UATs cada vez es costoso.

**Descripción:** Añadir un subcomando `sddk uat run` que acepte filtros:
- `--scenario T01,T05,T08` (lista explícita).
- `--tag c0,c1` (por hito del roadmap).
- `--exclude-flaky` (omite UATs marcados flakey).
- `--format json|yaml` (machine-readable).

**Superficie:**
- `crates/sddk-cli/src/uat.rs` (existe, contiene la lógica actual).
- Tests nuevos: `crates/sddk-cli/tests/uat_filter.rs`.

**Tamaño estimado:** 80-150 líneas (no destructivo, no rompe CLI actual).

**Tipo de commit:** `feat(uat):` → **MINOR bump**.

**UAT relacionados:** T01-T35 (todos se benefician).

**Bloqueos:** Ninguno. No requiere provider externo ni cambios arquitectónicos.

**Trigger:** Operador o usuario reporta "ejecutar 35 UATs cada vez es lento".

---

### FC-2: `sddk doctor --format json` (machine-readable doctor)

**Origen:** `sddk dev doctor` existe (visto en scripts/release.sh §11). Output
actual es texto. Falta formato JSON.

**Descripción:** Añadir flag `--format json` a `sddk dev doctor` que devuelva
un payload JSON con `binary.bundle_coherence`, `all_present`, `missing[]`,
etc. Útil para integrar con herramientas externas (CI, dashboards).

**Superficie:**
- `crates/sddk-cli/src/doctor_cmd.rs` (probablemente existe).
- Tests: extender tests existentes con caso JSON.

**Tamaño estimado:** 30-60 líneas.

**Tipo de commit:** `feat(cli):` → **MINOR bump**.

**UAT relacionados:** T28 (full verify).

**Bloqueos:** Ninguno.

**Trigger:** Necesidad de integración CI/dashboards (preguntar al operador).

**Estado:** ✅ IMPLEMENTED (commit 21fcfff, session-11). `DoctorOutput` ahora
tiene `#[derive(serde::Serialize)]`; el path JSON funciona y emite un payload
machine-readable con `binary.bundle_coherence`, `all_present`, `missing[]`.

---

### FC-3: `sddk cycle export --to jsonl` (export de cycle state)

**Origen:** Operador podría querer migrar cycles entre instalaciones o auditar
estado de cycles sin acceso al SQLite.

**Descripción:** Nuevo subcomando que vuelca un cycle (o todos los cycles de un
project) en formato JSONL con eventos, leases, gates. Simétrico a un eventual
`sddk cycle import`.

**Superficie:**
- `crates/sddk-cli/src/cycle_cmd.rs` (existe).
- `crates/sddk-domain/src/storage.rs` (queries existentes).
- Tests: `crates/sddk-cli/tests/cycle_export.rs`.

**Tamaño estimado:** 100-200 líneas.

**Tipo de commit:** `feat(storage):` o `feat(cli):` → **MINOR bump**.

**UAT relacionados:** T30 (rebuild projections).

**Bloqueos:** Ninguno (no toca Authority).

**Trigger:** Necesidad de backup/migración/auditoría.

**Estado:** ⚠️ DEFERRED — DUPLICATED por `sddk ledger export` (sesión-12 audit).

`sddk ledger export --cycle <id> --output <file>` ya cubre el caso de uso
principal (volcar eventos del cycle a JSONL en archivo). Diff vs FC-3 propuesto:

| Capacidad | `ledger export --cycle X` | FC-3 propuesto |
|---|---|---|
| Filtro por cycle | ✅ `--cycle X` | ✅ `--cycle X` |
| Output a archivo | ✅ `--output <file>` (requerido) | ✅ `--output <file>` (opcional) |
| Output a stdout | ❌ | ✅ |
| Formato JSONL | ✅ (default) | ✅ |
| Formato JSON envelope | ❌ | ✅ |
| Formato texto human-readable | ❌ | ✅ |
| Filtro por frame | ✅ `--frame` | ❌ |
| Límite de eventos | ✅ `--limit` | ❌ |

FC-3 como subcomando nuevo violaría "evita código duplicado" (regla del
operador). Las 3 diferencias reales (stdout, JSON envelope, text) se cubren
mejor extendiendo `ledger export` con `--format text|json|jsonl` y haciendo
`--output` opcional.

**Acción propuesta (futuro, no en este ciclo):**
- Extender `LedgerExportArgs` con `--format text|json|jsonl` (default jsonl) y `--output <PathBuf>` opcional.
- Reusar `run_ledger_export` y `ExportOutput`.
- Marcar FC-3 como RESUELTO vía la extensión propuesta (cuando se haga).
- Si la extensión no se ejecuta, mantener como DEFERRED con disparador "operador
  necesita formato JSON envelope o text en backups".

**Sesión-12 finding:** working tree de FC-3 con 251 líneas en `cycle.rs` y
~120 líneas de integration test → RECHAZADO, no se commitea. Reproducción en
sesión-11/12 cerraba sin haber ejecutado `sddk --help | grep -i "ledger"` para
verificar que `ledger export` ya existía.

---

### FC-4: `sddk uat replay --release v1.169.122` (re-ejecutar UATs contra release pinned)

**Origen:** ROADMAP §C0 + observación: los UATs son contra el HEAD actual; no
hay forma de verificar que una release anterior sigue pasando.

**Descripción:** Subcomando que clona el binario + bundle de un release tag
específico, lo ejecuta en un workspace temporal, corre los UATs, y devuelve
PASS/FAIL con digest del binario.

**Superficie:**
- `crates/sddk-cli/src/uat.rs` (extender).
- `scripts/release.sh` referencia (instalar binario previo).
- Tests: integración con `gh release download`.

**Tamaño estimado:** 200-400 líneas (no trivial, integración con GH API).

**Tipo de commit:** `feat(uat):` → **MINOR bump**.

**UAT relacionados:** T30, T31 (compatibilidad, replay).

**Bloqueos:** Requiere acceso a GH Releases (ya hay `gh auth` configurado).

**Trigger:** Preocupación de "release X sigue siendo válido tras refactor Y".

---

### FC-5: `sddk cycle diff <cycle-a> <cycle-b>` (diff entre cycles)

**Origen:** Operador comparando cycles para debugging o auditoría.

**Descripción:** Compara dos cycles (eventos, leases, gates, decisiones) y
devuelve un diff estructurado. Útil cuando un cycle falla y se compara con uno
anterior exitoso.

**Superficie:**
- `crates/sddk-cli/src/cycle_cmd.rs`.
- `crates/sddk-domain/src/diff.rs` (nuevo módulo).
- Tests: `crates/sddk-cli/tests/cycle_diff.rs`.

**Tamaño estimado:** 150-250 líneas.

**Tipo de commit:** `feat(cli):` → **MINOR bump**.

**UAT relacionados:** T21 (crash/reopen), T22 (contention).

**Bloqueos:** Ninguno.

**Trigger:** Debugging de cycles fallidos en producción.

**Estado:** ⚠️ DEFERRED — DUPLICATED parcialmente por `sddk fork diff` y `sddk memory diff`.

`sddk fork diff` compara el prefijo de un fork contra el parent state (cycle-scoped).
`sddk memory diff` compara árboles de decisión memory.

Diferencias reales vs FC-5 propuesto:
- `sddk fork diff` requiere crear un fork (fork create) antes de poder diffear.
  Está orientado a "qué cambia si yo divido el cycle aquí".
- `sddk memory diff` opera sobre decision_memory trees, no sobre cycle events crudos.
- FC-5 propuesto operaría sobre dos cycle IDs ya existentes sin necesidad de
  fork o memory tree.

**Acción propuesta (futuro, no en este ciclo):**
- Validar con el operador si la necesidad de "comparar dos cycles sin fork" es
  distinta de "comparar el prefijo de un fork contra parent".
- Si es distinta, FC-5 tiene valor. Si no, mantener como DEFERRED.
- No implementar hasta tener respuesta del operador.

---

### FC-6: `sddk vault show <adr-id>` (renderizar ADR desde vault)

**Origen:** ADR-0099 define el Vault como "fuente humana". No hay CLI para
leer/visualizar un ADR concreto sin saber el path.

**Descripción:** Resuelve un ADR por ID (e.g. `ADR-0099-VAULT-AS-HUMAN-KNOWLEDGE-SOURCE`),
lo lee del vault local, y lo renderiza (markdown → TUI pager o terminal
output).

**Superficie:**
- `crates/sddk-cli/src/vault_cmd.rs` (existe, contiene mirror logic).
- Tests: integración con vault local.

**Tamaño estimado:** 80-120 líneas.

**Tipo de commit:** `feat(vault):` → **MINOR bump**.

**UAT relacionados:** (nuevo, no estaba en T01-T35).

**Bloqueos:** Ninguno.

**Trigger:** "Quiero ver ADR-0141 sin buscar el archivo".

**Estado:** ✅ IMPLEMENTED (commit post-release v1.170.3 / session-11 cierre).
`VaultShowOutput { node, backlinks }` con `#[derive(serde::Serialize)]`;
`vault_show_text` para rendering legible. Capability `vault.show` añadida
a `workflow/workflow.yaml` (risk: low, consequence: read). 6 tests
unitarios (3 nuevos + 3 existentes normalize_cycle_target_*).

---

### FC-7: `sddk uat status --format json` (machine-readable uat status)

**Origen:** `sddk uat status` existe pero solo emite texto. Falta formato JSON
para integración con CI dashboards y reportes automatizados.

**Descripción:** Añadir flag `--format json` que devuelva un payload con
`release`, `plan`, `report` (estado, escenarios, veredictos). Permite a UAT
status ser consumido por herramientas externas.

**Superficie:**
- `crates/sddk-cli/src/uat.rs` (existe).

**Tamaño estimado:** 30-60 líneas.

**Tipo de commit:** `feat(uat):` → **MINOR bump**.

**UAT relacionados:** T01, T05, T18 (status reports).

**Bloqueos:** Ninguno.

**Estado:** ✅ IMPLEMENTED (commits 13c9690/a06083c/95f2250, session-11).
Estructura `UatStatusOutput { release, plan, report }` con `#[derive(serde::Serialize)]`,
text rendering a través de fn nombrada `uat_status_text`.

---

### FC-8: `sddk uat validate --format json` (machine-readable validate)

**Origen:** `sddk uat validate` tiene flag `--format` pero el JSON path era
inactivo: devolvía `Ok(())` y `render_result` serializaba el unit como `null`.

**Descripción:** Añadir `UatValidateOutput { schema_version, plan_features,
scenarios_total, form_dsl_errors }` con `#[derive(serde::Serialize)]`. Path
Text sigue emitiendo `uat validate: OK`.

**Superficie:**
- `crates/sddk-cli/src/uat.rs` (existe).

**Tamaño estimado:** 30-50 líneas.

**Tipo de commit:** `feat(uat):` → **MINOR bump**.

**UAT relacionados:** T28 (full verify), T01 (matriz).

**Bloqueos:** Ninguno.

**Estado:** ✅ IMPLEMENTED (commit 1b2795b, session-11). El path JSON retorna
conteos del plan parseado; nuevo test `validate_plan_json_output_has_counts`
cubre el contrato.

---

## 2. Capacidades diferidas (C5) — NO ejecutar sin trigger explícito

Las del ROADMAP §C5 (X08, J7-J9, R11) son **futuras features** con disparadores
definidos. NO son backlog ejecutable por sí solas. Listadas aquí solo para
trazabilidad:

| ID | Descripción | Disparador requerido |
|---|---|---|
| X08 | Jev benchmark con corpus etiquetado | Corpus etiquetado + baseline definido |
| J7 | MCP pull (vs SDK/push actual) | Consumidor real que demuestre necesidad |
| J8 | Operaciones host avanzadas | Negociación de capabilities + UAT propios |
| J9 | Segundo host real antes de `AGENTIC_API_STABLE/1.0` | Host real implementado + AG4 validado |
| R11 | Split de crates | Métricas sostenidas de dependencia + frecuencia |

## 3. Capacidades EXPLÍCITAMENTE fuera de inventario

Las siguientes son **NO-features** y NO deben proponerse:

- Cambiar el workspace version de SemVer-style a algo más estricto (sería
  cosmético; el problema real está resuelto por CONTRIBUTING-SEMVER.md y
  `scripts/release.sh`).
- Reorganizar el layout de crates sin métricas (R11 lo prohíbe).
- Refactors sin nuevas capacidades de usuario (no son `feat:`).

## 4. Plantilla para añadir un candidato

```markdown
### FC-N: <nombre corto>

**Origen:** ROADMAP §X — "<cita del roadmap>".
**Descripción:** <1-2 párrafos>.
**Superficie:**
- `<path>` (existe / nuevo).
- Tests: `<path>`.

**Tamaño estimado:** <N líneas>.
**Tipo de commit:** `feat(<scope>):` → **MINOR bump**.
**UAT relacionados:** <T-ID list>.
**Bloqueos:** <ninguno / X>.
**Trigger:** <cuándo abrir este ciclo>.
```

---

## Referencias

- `docs/roadmap/ROADMAP.md` §C0 (inventario de features).
- `docs/roadmap/ROADMAP.md` §C5 (features diferidas).
- `docs/architecture/CONTRIBUTING-SEMVER.md` (mecánica workspace-vs-release).
- `docs/roadmap/UAT-MATRIX.md` (T01-T35).
