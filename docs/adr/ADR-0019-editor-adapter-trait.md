---
status: accepted
date: 2026-08-18
deciders: [orchestrator]
linked_cycles: [c-20260818-145237]
---

# ADR-0019 — EditorAdapter trait as the registration seam

## Status

Accepted — implemented in `crates/sddk-cli/src/dev/editor_adapters/`.

## Contexto

Registration was opencode-only (`register_opencode_agents` in
`framework_check.rs`), and zcode/claude/codex received symlinks but no agent
registration. Adding three more per-editor writers inside `framework_check.rs`
would re-bloat a module that ADR-016-era extraction had already split once.

## Decision

Registration is expressed through a narrow `EditorAdapter` trait:

```rust
trait EditorAdapter {
    fn editor_name(&self) -> &'static str;
    fn register(&self, ctx: &RegistrationContext<'_>) -> AdapterReport;
}
```

with four implementations:

- **OpenCode** — JSON upsert in `opencode.json` (`agent` map; refactor of the
  old `register_opencode_agents`),
- **ZCode** — JSON mirror in `zcode.json` (same parameterized core, `IdeKey::Zcode`),
- **Claude** — native `.md` files with YAML frontmatter in `~/.claude/agents/`
  (model gate: `sonnet|opus|haiku|inherit` or full provider/model IDs),
- **Codex** — native TOML files in `~/.codex/agents/` (`.md` body →
  `developer_instructions`; serialized via the `toml = "0.8"` crate).

`register()` never panics and never returns `Result` — per-file failures are
captured into `AdapterReport.errors`, so one editor's failure never aborts
the others (PerIdeErrorIsolation); any failure still yields a non-zero
`dev link` exit (PerEditorReporting).

Adapters own the agents surface of their editor: for claude/codex,
`link_editor` receives a surface profile (`LinkProfile`) that skips the
agents symlink surface, so native files and symlinks never collide.

Codex fields the framework does not model (`model_reasoning_effort`,
`model_reasoning_summary`) are deliberately not written — documented
limitation, not an oversight.

## Alternativas rechazadas

- **Monolito `register_all_editors`** → re-infla el módulo que ADR-016 ya
  dividió una vez; viola el precedente de techo de LOC.
- **Registro en gateway/engine** → preocupación exclusiva de desarrollo; el
  módulo `dev` de sddk-cli es el hogar establecido.
- **Codex vía JSON** → el formato nativo de Codex es TOML; inventar un shim
  JSON añade una capa de traducción sin testear.

## Consecuencias

- `framework_check.rs` se reduce a helpers de reporte/sync (151 → 67 LOC).
- Nuevo árbol `dev/editor_adapters/` con cada archivo bajo el techo de LOC.
- `LinkEditor` gana `Claude|Codex` (uninstall/doctor/update lo reflejan).
- Los archivos claude/codex escritos por el framework nunca se actualizan
  tras la primera escritura (consecuencia conjunta de ADR-0018, documentada
  en las notas de apply).
