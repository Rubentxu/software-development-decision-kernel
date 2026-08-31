# Spec — Mecanismo de reconciliación autoritativa de agentes sddk → IDEs

> **Trace ID:** SPEC-RECONCILE-001
> **Cycle binding (propuesto):** cycle-29 — **RESUELTO 2026-08-26:** cycle-29 = reconcile (Map replay → cycle-30).
> **ADR de soporte:** [ADR-0064](./adr/ADR-0064-sddk-authored-reconciliation.md)
> **Roadmap entry:** [ROADMAP §Cycle-29 candidate](sddk-decision-kernel-architecture/02-roadmap/ROADMAP.md)
> **Estado:** propuesta aceptada (2026-08-26). Pendiente ejecutar ciclo A-min (cycle-29).
> **Fecha:** 2026-08-26

> **NOTA DE TRAZABILIDAD — renumeración de ADR:**
> El documento fuente de handoff referenciaba `ADR-0021`. Esa numeración está **ocupada**
> en el repo (`ADR-0021-phase1-hexagonal-enforcement.md`, aceptada 2026-08-19). Se
> renumera a **ADR-0064** (siguiente libre después de ADR-0063). Las referencias en
> este spec usan ADR-0064. La numeración original queda registrada en el changelog
> del ADR-0064 §Open Questions.

---

## 1. Contexto y objetivo

`sddk-framework` es un CLI en Rust (crate `sddk-cli`) que registra agentes en cuatro IDEs agénticos: **opencode**, **zcode**, **claude code** y **codex**. Hoy el registro sigue la regla "primera escritura solamente" ([ADR-0018](./adr/ADR-0018-user-owns-ide-config.md)): el framework escribe el `model`/`description` de un agente solo la primera vez y **nunca** sobrescribe una entrada existente. Consecuencia: la fuente de verdad (`assets/agent-models.yaml` y el frontmatter de `agents/*.md`) puede quedar desincronizada del config del IDE sin que nada lo detecte.

**Objetivo**: añadir un comando `sddk dev reconcile` que haga que todo lo definido en sddk sea lo que queda aplicado en cada IDE, mediante **adaptadores específicos que conocen las capacidades y el esquema nativo de cada IDE**, con una abstracción extensible a nuevos IDEs.

**Decisiones ya tomadas (vinculantes):**
1. **Alcance**: "campos de sddk" — sddk sobrescribe SOLO los campos que modela, sobre agentes que sddk define; preserva campos desconocidos del usuario y agentes no-sddk.
2. **Superficie**: comando nuevo `dev reconcile` con **dry-run por defecto** y `--apply`. `dev link` NO cambia (mantiene ADR-0018).
3. **Campos de la v1**: núcleo = `model`, `description`, `body`/`prompt`, `mode`, `hidden`. El framework de capacidades debe quedar preparado para añadir `permission`, `color`, `tools`, `metadata` después.

## 2. Estado actual (punto de partida exacto)

Base: `/var/mnt/DiscoChino2-fast/Proyectos/agentesIA/sddk-framework` (o el checkout equivalente). Ficheros clave:

### 2.1 Capa de adaptadores
`crates/sddk-cli/src/dev/editor_adapters/mod.rs` (282 líneas)
- Trait `EditorAdapter` (líneas 50-53): `editor_name()` + `register(&RegistrationContext) -> AdapterReport`. `register()` nunca hace panic ni devuelve `Result`; los errores van a `AdapterReport.errors`.
- `AgentSource` (22-28): `name`, `description`, `tools: Option<String>`, `body`.
- `RegistrationContext` (31-35): `root`, `agents: &[AgentSource]`, `models: Option<&AgentModelsConfig>`.
- `AdapterReport` (38-47): `editor`, `registered`, `updated_stale`, `skipped_existing`, `skipped_unresolved`, `pruned`, `errors`.
- `EditorDirs` (56-62), `LinkProfile` (66-98), `adapters_for` (101-124).
- `PRIMARY_AGENTS = ["orchestrator", "book-orchestrator"]` (19).
- `parse_agent_file` (138-161): solo lee `description:` y `tools:` (y el body tras `---`). El `model:` del frontmatter **no** se lee (inerte, ADR-0017).
- `load_agent_sources` (165-193): lee `root/agents/*.md`, `name` sale del nombre de fichero.
- `is_framework_namespaced` (196-198): prefijos `sddk-`/`sdd-`/`gentle-`.
- `resolve_for_models` (203-215): `None` (config ausente) → `Ok(None)` (registrar sin `model`); `Model(m)` → `Ok(Some(m))`; `NoModelConfigured` → `Err(())` (omitir agente).

### 2.2 Adaptadores concretos
`crates/sddk-cli/src/dev/editor_adapters/json.rs` (203 líneas)
- `OpenCodeAdapter` escribe `<opencode_dir>/opencode.json`; `ZCodeAdapter` escribe `<zcode_dir>/zcode.json`; ambos comparten `upsert_json_agents` (55-174).
- Entrada por agente: `description`, `mode` (`primary`|`subagent`), `prompt` (`{file:<root>/agents/<name>.md}`), `model` (solo si resuelve), `hidden` (solo subagentes). Esqueleto nuevo: `{"$schema":"https://opencode.ai/config.json","agent":{},"mcp":{}}`.
- First-write-only en 95-123: para una entrada existente, solo refresca `prompt` si el path parece de una instalación sddk previa (`looks_like_framework_prompt`, 186-199); en otro caso `skipped_existing`. Nunca toca `model`/`description`/`mode`/`hidden`.
- Prune (145-156): elimina solo nombres con prefijo de framework que ya no están en el bundle.

`crates/sddk-cli/src/dev/editor_adapters/claude.rs` (97 líneas)
- Escribe un `.md` por agente en `<claude_dir>/agents/`. Frontmatter: `name`, `description`, `tools:` (opcional), `model:` (opcional), luego body.
- `claude_model_valid` (11-13): el modelo debe ser `sonnet|opus|haiku|inherit` o contener `/`.
- **No lee archivos existentes**: `if target.exists() { skipped_existing; continue }` (33-36). Prune por prefijo (70-90).

`crates/sddk-cli/src/dev/editor_adapters/codex.rs` (94 líneas)
- Escribe un `.toml` por agente en `<codex_dir>/agents/`. `to_toml` (19-34) emite exactamente `name`, `description`, `developer_instructions` (= body), `model` (opcional). Omite deliberadamente `model_reasoning_effort`/`model_reasoning_summary` (ADR-0019).
- **No lee archivos existentes**: `if target.exists() { skipped_existing; continue }` (50-53). Prune por prefijo (67-87).

### 2.3 Config de modelos
`crates/sddk-cli/src/dev/agent_models.rs` (311 líneas)
- `ModelTier` (11-28, `premium`/`fast`), `IdeKey` (52-71, `opencode`/`zcode`/`claude`/`codex`).
- `AgentModelsConfig` (96-100): `tiers: BTreeMap<ModelTier, BTreeMap<IdeKey,String>>` + `agents: BTreeMap<String, AgentModelEntry>`.
- `ModelResolution` (111-116): `Model(String)` | `NoModelConfigured{agent,ide}`.
- `resolve` (227-248): orden estricto override → tabla del tier → `NoModelConfigured`. Sin fallback cruzado ni hardcodeado.
- `from_file` (209-218): fichero ausente → `Ok(None)` (config ausente ≠ error).

### 2.4 Comando `dev link` y reporte
`crates/sddk-cli/src/dev/link.rs` (439 líneas)
- `run_dev_link` (245-429). `--root` por defecto `.`; si es explícito y no `.`, hace "dogfooding": `sync_assets(root/assets → bundle-activo/assets)` + regenera manifest (255-261).
- Carga modelos de `root/assets/agent-models.yaml` (263-270; error de parseo → warning + `None`).
- `LinkArgs` en `mod.rs:228-255` (flags `--root`, `--editor`, `--opencode-dir`, `--zcode-dir`, `--claude-dir`, `--codex-dir`, `--write-registry`, `--format`).

`crates/sddk-cli/src/dev/framework_check.rs` (69 líneas)
- `LinkReport` (11-24) y `link_report_text` (28-48).

### 2.5 Registro del comando `dev`
`crates/sddk-cli/src/dev/mod.rs` (422 líneas)
- `LinkEditor` (64-75, valores `opencode`/`zcode`/`claude`/`codex`/`all`).
- `DevCommand` enum (78-108) — **aquí se añade `Reconcile`**.
- `run_dev` dispatch (312-326) — **aquí se añade el brazo**.
- `atomic_write` está en `crates/sddk-cli/src/dev/common.rs` (282 líneas, helper 28-82): temp + `sync_all` + rename atómico. Reusar para toda escritura.

### 2.6 Tests existentes
`crates/sddk-cli/src/dev/tests/`: `json_adapter_tests.rs`, `claude_adapter_tests.rs`, `codex_adapter_tests.rs`, `agent_models_tests.rs`, `link_e2e_tests.rs`, `models_cmd_tests.rs`, `reconciliation_tests.rs` (este último es solo de symlinks/prune, no de modelos).

> **NOTA:** los números de línea del doc fuente han sido validados contra el repo en este fork. Pequeños desfases de ±1 línea son normales (no invalidan la spec).

## 3. Diseño a implementar

### 3.1 Abstracción de capacidades (nueva)
Añadir a `crates/sddk-cli/src/dev/editor_adapters/` un módulo (sugerencia: `reconcile.rs`) con estos tipos:

```rust
pub(super) struct EditorCapabilities {
    pub ide: IdeKey,
    pub supports_mode: bool,          // opencode/zcode
    pub supports_hidden: bool,        // opencode/zcode
    pub supports_prompt_ref: bool,    // opencode/zcode ({file:...})
    pub supports_tools: bool,         // claude (v1)
    pub model_validator: Option<fn(&str) -> bool>, // p.ej. claude_model_valid
}

pub(super) struct ReconcileTarget {
    pub model: Option<String>,
    pub description: String,
    pub prompt: Option<String>,       // json {file:...}
    pub body: Option<String>,         // claude .md / codex developer_instructions
    pub mode: Option<&'static str>,   // "primary" | "subagent"
    pub hidden: Option<bool>,
}

pub(super) struct ExistingEntry {
    pub model: Option<String>,
    pub description: Option<String>,
    pub prompt: Option<String>,
    pub mode: Option<String>,
    pub hidden: Option<bool>,
    pub body: Option<String>,
    pub extras: serde_json::Map<String, serde_json::Value>, // campos ajenos a sddk, a preservar
}

pub(super) struct FieldDiff {
    pub agent: String,
    pub field: String,   // "model" | "description" | "prompt" | "mode" | "hidden" | "body"
    pub before: Option<String>,
    pub after: Option<String>,
}

pub(super) struct ReconcileReport {
    pub editor: String,
    pub added: usize,          // agentes del bundle sin entrada previa
    pub reconciled: usize,     // agentes con >=1 campo cambiado
    pub unchanged: usize,
    pub pruned: usize,
    pub skipped: usize,        // NoModelConfigured o modelo inválido
    pub diffs: Vec<FieldDiff>,
    pub errors: Vec<String>,
}
```

### 3.2 Trait de reconciliación (nuevo, no toca `register`)
```rust
pub(super) trait ReconcileAdapter {
    fn capabilities(&self) -> &EditorCapabilities;
    fn read_existing(&self, name: &str) -> Result<Option<ExistingEntry>, String>;
    fn reconcile(&self, ctx: &RegistrationContext<'_>, apply: bool) -> ReconcileReport;
}
```
Las cuatro structs de adaptador existentes (`OpenCodeAdapter`, `ZCodeAdapter`, `ClaudeAdapter`, `CodexAdapter`) implementan tanto `EditorAdapter` (sin cambios) como `ReconcileAdapter`.

**Regla de ownership (reemplaza la heurística de prefijos en reconciliación)**: un agente es "de sddk" si su `name` está en `ctx.agents` (el bundle). Solo esos se reconcilian y se prunean por desaparición del bundle. Todo lo demás (agentes de usuario, campos ajenos dentro de una entrada sddk) se conserva.

### 3.3 Comportamiento por adaptador (reconcile)

**opencode / zcode (`json.rs`)**
- `read_existing`: leer el JSON, extraer `agent[name]` como `ExistingEntry`. `extras` = todas las claves del objeto salvo `model`/`description`/`prompt`/`mode`/`hidden`.
- `reconcile`: para cada agente del bundle, construir `ReconcileTarget` (modelo desde `resolve_for_models(ctx.models, name, IdeKey::Opencode|Zcode)`; `description` desde `AgentSource.description`; `prompt` = `{file:<root>/agents/<name>.md}`; `mode`/`hidden` desde `PRIMARY_AGENTS`).
  - Si no existe: crear entrada (igual que hoy) → `added`.
  - Si existe: comparar campo a campo los 5 campos sddk; si difieren, registrar `FieldDiff`. Si `apply`, mutar **solo** esas claves conservando `extras`.
  - Si el modelo resuelto no pasa el validador del IDE, reportar y no pisar `model`.
- Prune: eliminar del mapa `agent` las claves cuyo nombre esté en el bundle "anterior" y ya no esté (misma lógica de namespace actual, incluyendo `orchestrator`/`book-orchestrator` por ser del bundle). Nunca tocar agentes no-sddk.

**claude (`claude.rs`)**
- Añadir un parseador de frontmatter que devuelva claves ordenadas (para preservar desconocidas) + body. Reutilizar el patrón de `parse_agent_file` (mod.rs:138-161), pero genérico: leer todas las líneas `clave: valor` hasta el cierre `---`, y el resto como body.
- `read_existing`: parsear `<claude_dir>/agents/<name>.md` si existe. `extras` = claves del frontmatter distintas de `name`/`description`/`tools`/`model`.
- `reconcile`: actualizar `name`/`description`/`model` (si resuelve y pasa `claude_model_valid`) y body; **preservar claves de frontmatter desconocidas**. Reescribir con `atomic_write`.
- Prune: igual que hoy (por prefijo + desaparición del bundle), sobre `.md`.

**codex (`codex.rs`)**
- `read_existing`: `toml::from_str` (la dep `toml = "0.8"` ya existe) del `.toml`. `extras` = claves distintas de `name`/`description`/`developer_instructions`/`model` (p.ej. `model_reasoning_effort`).
- `reconcile`: actualizar `name`/`description`/`developer_instructions`/`model`; preservar el resto. Reescribir con `toml::to_string_pretty` + `atomic_write`.
- Prune: igual que hoy, sobre `.toml`.

**Regla común**: si `resolve_for_models` devuelve `Err(())` (`NoModelConfigured`), el agente se cuenta como `skipped` (y en `apply` no se elimina una entrada existente, solo se reporta). Config ausente (`models == None`) → `model = None` (no se escribe `model`), igual que hoy.

### 3.4 Comando `sddk dev reconcile`

Nuevo fichero `crates/sddk-cli/src/dev/reconcile.rs`:

```rust
#[derive(Debug, Clone, Args)]
pub(super) struct ReconcileArgs {
    #[arg(long, default_value = ".")]
    pub(super) root: std::path::PathBuf,
    #[arg(long, value_enum, default_value_t = LinkEditor::All)]
    pub(super) editor: LinkEditor,
    #[arg(long)] pub(super) opencode_dir: Option<PathBuf>,
    #[arg(long)] pub(super) zcode_dir: Option<PathBuf>,
    #[arg(long)] pub(super) claude_dir: Option<PathBuf>,
    #[arg(long)] pub(super) codex_dir: Option<PathBuf>,
    /// Aplicar cambios (por defecto: dry-run de solo lectura).
    #[arg(long)] pub(super) apply: bool,
    /// Salir con código no-cero si hay drift (para CI).
    #[arg(long)] pub(super) check: bool,
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(super) format: OutputFormat,
}
```

Flujo de `run_dev_reconcile`:
1. Resolver `root` igual que `link` (`. ` → `resolve_active_framework_root`; si no, `canonicalize`). NO hacer `sync_assets` (reconcile es de solo lectura del bundle; no debe mutar el bundle activo).
2. `load_agent_sources(&root)` + `AgentModelsConfig::from_file(root/assets/agent-models.yaml)`.
3. Construir `EditorDirs` (replicar la lógica de `link.rs:285-306`).
4. Para cada editor seleccionado (dispatch `reconcilers_for`, análogo a `adapters_for`), llamar `reconcile(ctx, args.apply)`.
5. Renderizar: en texto, una línea por `FieldDiff` (`<agent>: <field> '<before>' -> '<after>'`) + resumen; en JSON, la `ReconcileReport` serializada.
6. Exit codes: `0` sin drift/éxito · `1` drift detectado (`--check`) o error de escritura · `2` config inválida · `3` root/bundle no resoluble.

**Registro**: en `dev/mod.rs`:
- Añadir `mod reconcile;` a los `mod` (líneas 7-26).
- Añadir `Reconcile(ReconcileArgs)` al enum `DevCommand` (78-108), con doc-comment.
- Añadir `DevCommand::Reconcile(args) => self::reconcile::run_dev_reconcile(args, environment),` en `run_dev` (312-326).

clap expone automáticamente `sddk dev reconcile`.

## 4. Cambios fichero a fichero (checklist de implementación)

**Nuevos**
1. `crates/sddk-cli/src/dev/editor_adapters/reconcile.rs` — tipos de §3.1, trait de §3.2, y helper `reconcilers_for(editor, dirs)` análogo a `adapters_for` (mod.rs:101-124).
2. `crates/sddk-cli/src/dev/reconcile.rs` — `ReconcileArgs`, `run_dev_reconcile`, render de texto/JSON, exit codes.
3. `docs/adr/ADR-0064-sddk-authored-reconciliation.md` — decisión formal.
4. `docs/agent-reconciliation.md` — guía de uso + tabla de capacidades/mapeo por IDE.
5. `crates/sddk-cli/src/dev/tests/reconcile_tests.rs` — tests (ver §6).

**Modificados**
6. `crates/sddk-cli/src/dev/mod.rs` — `mod reconcile;`, variante `Reconcile` + dispatch.
7. `crates/sddk-cli/src/dev/editor_adapters/mod.rs` — `pub(super) use` de los tipos nuevos; añadir `ReconcileAdapter` al scope; `EditorCapabilities` por IDE.
8. `crates/sddk-cli/src/dev/editor_adapters/json.rs` — `read_existing` + `reconcile` para opencode/zcode.
9. `crates/sddk-cli/src/dev/editor_adapters/claude.rs` — parseador de frontmatter + `read_existing` + `reconcile`.
10. `crates/sddk-cli/src/dev/editor_adapters/codex.rs` — `toml::from_str` + `read_existing` + `reconcile`.
11. `docs/agent-models-registration.md` — añadir nota: `reconcile` complementa (no reemplaza) `link`.

## 5. ADR-0064 (contenido mínimo)

Ver: [ADR-0064](./adr/ADR-0064-sddk-authored-reconciliation.md).

Resumen:
- **Contexto**: ADR-0018 estableció "primera escritura solamente"; produce drift silencioso entre `agent-models.yaml` y el config del IDE.
- **Decisión**: nuevo comando `sddk dev reconcile` autoritativo sobre los **campos que sddk modela**, limitado a agentes definidos en el bundle. `dev link` conserva first-write-only.
- **Propiedad**: agente "de sddk" ⇔ su nombre está en `agents/*.md`. Campos ajenos a sddk y agentes de usuario se preservan (sin marcadores de ownership ni reescritura completa — se rechazan ambas alternativas).
- **Capacidades por IDE**: cada adaptador declara qué campos soporta y sus validadores; los campos no soportados se descartan con nota, no con error fatal.
- **Seguridad**: dry-run por defecto; escritura solo con `--apply`; `--check` para CI.
- **Consecuencias**: claude/codex pasan de "no leer el archivo" a leer-modificar-escribir; se añade parseado de frontmatter YAML (claude) y TOML (codex).

## 6. Tests a escribir (mínimo)

`crates/sddk-cli/src/dev/tests/reconcile_tests.rs` (y/o tests por adaptador):
- **json dry-run**: no escribe nada; devuelve `FieldDiff` correctos.
- **json apply**: corrige `model` (p.ej. `deepseek/deepseek-chat` → `zai-coding-plan/glm-5.3`) y `description`; una clave desconocida (`"temperature"`) sobrevive; agente no-sddk (`my-agent`) intacto; idempotencia (segundo `apply` = 0 cambios).
- **claude/codex**: leen archivo existente, actualizan `model`/`description`/body y **preservan** claves de frontmatter/TOML desconocidas; dry-run no escribe.
- **prune**: un agente del bundle que desaparece se elimina; un agente de usuario se conserva.
- **comando**: `--check` devuelve exit 1 con drift y exit 0 sin drift; `--format json` estable; `--editor opencode` solo toca opencode.
- **regresión**: los tests de `link` y `models` existentes siguen pasando.

## 7. Verificación

1. `cargo build` en el workspace (o `cargo build -p sddk-cli`).
2. `cargo test -p sddk-cli` — todos verdes (nuevos + regresión).
3. Demo manual no destructiva:
   ```bash
   sddk dev reconcile --editor opencode --root <repo> --opencode-dir /tmp/oc-reconcile-test
   # dry-run: imprime diff, no escribe
   sddk dev reconcile --editor opencode --root <repo> --opencode-dir /tmp/oc-reconcile-test --apply
   # aplica y reimprime
   sddk dev reconcile --editor all --root <repo> --format json --check
   ```

## 8. Criterios de aceptación

- Existe `sddk dev reconcile` con dry-run por defecto, `--apply` y `--check`.
- Reconciliar `model`/`description`/`prompt`/`mode`/`hidden` (y `body`/`developer_instructions` en claude/codex) hace que el IDE refleje exactamente `agent-models.yaml` + `agents/*.md`.
- No se borran campos ajenos a sddk ni agentes de usuario en ningún IDE.
- Los 4 IDEs (opencode, zcode, claude, codex) funcionan y son extensibles vía la abstracción de capacidades.
- `dev link` no cambia de comportamiento.
- Documentación: ADR-0064 + `docs/agent-reconciliation.md` actualizados.

## 9. Fuera de alcance (v1) — anotar como seguimiento

- Mapear `permission`, `color`, `tools`, `metadata` por IDE (el framework de capacidades lo deja preparado).
- Limpiar la duplicidad cosmética del `model:` inerte en el frontmatter fuente (sigue inerte; `agent-models.yaml` manda).
- Migrar `dev link` a reconciliar por defecto (no en v1).

## A. Trace map

| Trace ID | Tipo | Path | Estado |
|---|---|---|---|
| SPEC-RECONCILE-001 | spec | este doc | propuesta aceptada (2026-08-26) |
| REQ-RECONCILE-001 | spec | `/home/rubentxu/.sddk-knowledge/sddk-framework/specs/cli/REQ-Dev-Reconcile-Authoritative-IDE-Reconciliation.md` | proposed (cycle-29 sddk-spec output, sha256 `eb331ee2983ddf03d011c5e63b3bbebc52daa977f953e5ec72942630067161e2`, 12 REQ + 23 scenarios) |
| ADR-0064 | decisión | `docs/adr/ADR-0064-sddk-authored-reconciliation.md` | aceptado (2026-08-26) |
| ROADMAP-cycle-29-candidate | planificación | `docs/sddk-decision-kernel-architecture/02-roadmap/ROADMAP.md` | aceptado (2026-08-26) |
| cycle-29 (A-min) | ejecución | sddk-cli (TBD) | NO iniciado |