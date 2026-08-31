# ADR-0064 — `sddk dev reconcile`: reconciliación autoritativa de agentes sddk → IDEs

**Status:** accepted (2026-08-26)
**Date:** 2026-08-26
**Cycle:** 29 (accepted)
**Trigger:** producto (drift silencioso entre `assets/agent-models.yaml` y config de IDEs)
**Supersedes:** ninguno (convive con ADR-0018)
**Relacionado:** [SPEC-RECONCILE-001](../reconciliation-spec.md)

---

## Open Questions (resolved 2026-08-26)

1. **Numeración ADR:** el handoff externo referenciaba `ADR-0021`, pero `ADR-0021-phase1-hexagonal-enforcement.md` ya está ocupado. Se renumera a **ADR-0064** (siguiente libre tras ADR-0063). Cambio en [SPEC-RECONCILE-001 §encabezado](../reconciliation-spec.md). Si el handoff se generó con numeración tipo "claude code" (reseteo por década), la decisión sigue siendo renumerar localmente para no colisionar. — **Resuelto:** ADR-0064 confirmado en spec y ROADMAP.
2. **Cycle-29 reservation conflict:** el handoff de cycle-28 (`HANDOFF-2026-08-26-cycle-28-map-max-concurrency-error-aggregation.md §1`) reservaba cycle-29 para "Map source-context isolation + cross-tick replay". Esta propuesta toma cycle-29. **Decisión necesaria:** ¿(a) cycle-29 = reconcile y Map replay → cycle-30; o (b) cycle-29 = Map replay (preservar) y reconcile → cycle-30? — **Resuelto:** opción (a) — cycle-29 = reconcile, Map replay → cycle-30. Razón: drift visible al usuario + alineable con `sddk dev doctor`.

## Context

ADR-0018 (first-write-only) hace que `sddk dev link` **nunca** sobrescriba una entrada existente del IDE. Esto preserva el "ownership del usuario" sobre su config (decisión explícita y valiosa) pero introduce **drift silencioso**:

- El usuario edita `assets/agent-models.yaml` y cambia `sddk-design` de `deepseek/deepseek-chat` a `zai-coding-plan/glm-5.3`.
- Ejecuta `sddk dev link`.
- El IDE **sigue usando** `deepseek/deepseek-chat` (porque la entrada ya existía). No hay error, no hay warning legible.

El mismo problema aplica a:
- Cambios de `description` (documentación que el IDE muestra al usuario)
- Cambios de `mode`/`hidden` (opencode/zcode)
- Cambios de `prompt`/`body` (refactor de un agente)
- Cambios de `tools` (claude)
- Cambios de `developer_instructions` (codex)

El usuario descubre el drift solo cuando algo falla o nota el comportamiento obsoleto. **No hay comando de diagnóstico.**

Adicionalmente, los `model_reasoning_effort` / `model_reasoning_summary` de codex y otros campos nativos del IDE son **del usuario** — deben preservarse intactos (marcadores de ownership no son aceptables; ver §Decision).

## Decision

### D-1: Comando nuevo `sddk dev reconcile`

Añadir `sddk dev reconcile` como comando nuevo. `sddk dev link` **NO cambia** (ADR-0018 sigue vigente para `link`). El flujo de trabajo esperado:

1. `sddk dev link` — primera instalación / onboarding (first-write-only, seguro).
2. `sddk dev reconcile` — sincronización post-cambios (autoritativo sobre campos sddk, dry-run por defecto).
3. `sddk dev reconcile --apply` — escribir los cambios mostrados.
4. `sddk dev reconcile --check` — exit 1 si hay drift (CI).

### D-2: Alcance "campos de sddk"

sddk reconcilia **solo** los campos que modela. v1 = `model`, `description`, `body`/`prompt`, `mode`, `hidden`. La lista es **cerrada y versionada** (extensible via nuevas capabilities, ver D-4). Todo lo demás se preserva tal cual.

### D-3: Regla de ownership (reemplaza heurística de prefijos)

**Un agente es "de sddk" si su nombre está en `ctx.agents`** (leído de `root/agents/*.md` por `load_agent_sources`).

- **Solo** los agentes de sddk se reconcilian.
- **Solo** los agentes de sddk se prunean si desaparecen del bundle.
- **Todo lo demás** (agentes de usuario, campos ajenos dentro de una entrada sddk) **se conserva intacto**.

Se rechazan explícitamente:
- **Marcadores de ownership** (`__sddk_owned: true`, sidecar `.sddk.json`): innecesarios; la fuente de verdad es `root/agents/*.md` (ya gestionada por el bundle).
- **Reescritura completa**: destruye trabajo del usuario.

### D-4: Framework de capacidades (`EditorCapabilities`)

Nuevo módulo `editor_adapters/reconcile.rs`:

```rust
pub(super) struct EditorCapabilities {
    pub ide: IdeKey,
    pub supports_mode: bool,          // opencode/zcode
    pub supports_hidden: bool,        // opencode/zcode
    pub supports_prompt_ref: bool,    // opencode/zcode ({file:...})
    pub supports_tools: bool,         // claude (v1)
    pub model_validator: Option<fn(&str) -> bool>, // p.ej. claude_model_valid
}
```

Cada adaptador declara capacidades. **Campos no soportados se descartan con nota, no con error fatal** (p.ej. `mode` en claude → omitido silenciosamente del `ReconcileTarget`).

### D-5: Trait `ReconcileAdapter` (nuevo, no toca `EditorAdapter`)

```rust
pub(super) trait ReconcileAdapter {
    fn capabilities(&self) -> &EditorCapabilities;
    fn read_existing(&self, name: &str) -> Result<Option<ExistingEntry>, String>;
    fn reconcile(&self, ctx: &RegistrationContext<'_>, apply: bool) -> ReconcileReport;
}
```

Cada adaptador implementa ambos traits (`EditorAdapter` para `link`, `ReconcileAdapter` para `reconcile`). **No se fusionan los traits** (responsabilidades distintas: `link` es write-only first-write, `reconcile` es read-modify-write).

### D-6: Comportamiento por adaptador

| Adaptador | read_existing | reconcile | extras preservadas |
|---|---|---|---|
| opencode | parsea `opencode.json["agent"][name]` | sustituye 5 claves sddk en sitio | todas las claves no-sddk del objeto |
| zcode | parsea `zcode.json["agent"][name]` | análogo a opencode | análogo |
| claude | parsea frontmatter YAML completo + body | reescribe `.md` con frontmatter conocido + claves extra + body | claves de frontmatter no-sddk |
| codex | `toml::from_str` del `.toml` | reescribe `.toml` con claves sddk + extras | claves no-sddk (incl. `model_reasoning_*`) |

### D-7: Seguridad y contrato de ejecución

| Bandera | Default | Comportamiento |
|---|---|---|
| `--apply` | `false` | dry-run; imprime `FieldDiff`, no escribe |
| `--check` | `false` | exit 1 si drift detectado, útil en CI |
| `--format json` | text | `ReconcileReport` serializada |
| `--editor <X>` | `all` | limita a un IDE |

**Exit codes:**
- `0` éxito / sin drift
- `1` drift detectado (`--check`) o error de escritura
- `2` config inválida (`agent-models.yaml` malformado)
- `3` root/bundle no resoluble

### D-8: `NoModelConfigured` ≠ error

Si `resolve_for_models` devuelve `Err(())`, el agente se cuenta como `skipped`:
- **dry-run:** aparece en `skipped`, sin diff.
- **`--apply`:** si la entrada ya existía, se conserva; **no se borra** (al usuario le costó configurarla).
- Si no existía: se omite (mismo comportamiento que `link` hoy).

`models == None` (config ausente) → `model = None` (se escribe sin `model`, igual que `link`).

## Consequences

### Positivas
- Drift detectable y corregible (`--check` en CI).
- Campos del usuario preservados (alineado con ADR-0018).
- Cero ambigüedad sobre qué hace `link` vs `reconcile` (first-write-only vs autoritativo).
- `link` sigue siendo el onboarding seguro (idempotente, no destructivo).

### Negativas / Trade-offs
- **Coste de implementación**: ~600-800 LOC nuevos (tipos, trait, 4 implementaciones, comando CLI, tests).
- **Carga cognitiva**: dos comandos similares (`link` vs `reconcile`). Mitigación: documentación + tabla de cuándo usar cada uno en `docs/agent-reconciliation.md`.
- **Riesgo de regresión en claude/codex**: pasan de "no leer" a "leer-modificar-escribir". Tests de regresión obligatorios.
- **Re-orden de claves TOML**: `toml::to_string_pretty` puede cambiar el orden visual del `.toml`. Aceptable; el contrato es semántico.

### Riesgos

| Riesgo | Severidad | Mitigación |
|---|---|---|
| `--apply` borra campos del usuario por bug | Alta | Tests E2E con claves extra; revisión adversarial; dry-run por defecto; `--check` en CI antes de `--apply` |
| `toml::from_str` falla en `.toml` corrupto | Media | `read_existing` devuelve `Err(String)`; el agente se reporta en `errors` y se omite; reconcile continúa con el resto |
| YAML frontmatter parser no maneja multi-line values | Media | Spec §3.3 indica "leer todas las líneas `clave: valor`"; si una clave ocupa varias líneas, parseo simplificado puede perderla. Aceptable para v1; documentar limitación. |
| Desincronización entre `link` y `reconcile` al añadir capacidades futuras | Media | `EditorCapabilities` es la fuente de verdad; añadir campo nuevo requiere actualizar `link` Y `reconcile` (test de paridad). |
| Cambio de orden de claves JSON rompe comparaciones byte-a-byte | Baja | Reporte es semántico (`FieldDiff`), no textual. CI debe usar `--format json` y comparar campos. |

## Tests obligatorios

Ver [SPEC-RECONCILE-001 §6](../reconciliation-spec.md#6-tests-a-escribir-mínimo).

Resumen:
- json dry-run / json apply con clave extra + agente usuario + idempotencia
- claude/codex preservación de claves desconocidas
- prune bundle-only / preserve user agent
- exit codes (--check 0/1)
- regresión `link` + `models` tests existentes

## INV Preservation

- **INV-1 (zero-intrusion):** `reconcile` opera sobre configs del usuario; no toca el bundle (NO hace `sync_assets`). Cumple.
- **INV-2 (atomicidad):** todas las escrituras via `atomic_write` (common.rs:28-82). Cumple.
- **INV-3 (idiomatic Rust):** traits separados, `Result<_, String>` para errores de I/O; sin panic en paths de producción. Cumple.
- **INV-4 (extendibilidad):** `EditorCapabilities` + `ReconcileAdapter` son la superficie para añadir IDEs futuros. Cumple.
- **INV-5 (reversibilidad):** `--apply` solo afecta campos sddk + prune sddk-only; cualquier cambio es deshacer-editable por el usuario. Cumple.

## References

- ADR-0017 — tier-based model resolution (orden override → tabla → skip)
- ADR-0018 — user owns IDE config (first-write-only para `link`)
- ADR-0019 — editor adapter trait (codex sin `model_reasoning_*`)
- ADR-0055 — P3 closure (`CountingSemaphore`, ciclo-21; referencia por patrón de trait)
- ADR-0061 / ADR-0062 / ADR-0063 — Map evolution (referencia por patrón de ciclo A-min)
- `crates/sddk-cli/src/dev/editor_adapters/mod.rs:138-161` — patrón de parseo YAML
- `crates/sddk-cli/src/dev/common.rs:28-82` — `atomic_write` reusable
- `crates/sddk-cli/src/dev/agent_models.rs:227-248` — `resolve`
- SPEC-RECONCILE-001 — este ADR's spec
- HANDOFF-2026-08-26-cycle-28-map-max-concurrency-error-aggregation.md §1 — cycle-29 reservation conflict