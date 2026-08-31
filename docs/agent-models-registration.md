# Agent model registration — apply notes (cycle c-20260818-145237)

> Notas de usuario para la gestión de modelos de agentes por IDE.
> Decisiones de diseño: ADR-0017…0020.

## Cómo funciona

- `assets/agent-models.yaml` es la **única fuente** del mapeo agente→modelo.
  Por agente: `tier` (`premium|fast`) y opcionales `overrides` por IDE
  (`opencode`, `zcode`, `claude`, `codex`). Resolución: override → tabla del
  tier → el agente se omite con un warning.
- Gestión interactiva: `bash "$(sddk dev models tui-path)"` (gum; degrada a
  menús bash si gum no está). CLI equivalente: `sddk dev models
  list|set|validate`.
- `sddk dev link` registra agentes en **los cuatro editores**: opencode.json,
  zcode.json, `~/.claude/agents/*.md`, `~/.codex/agents/*.toml`.

## Relación con `dev reconcile`

`dev reconcile` complementa (no reemplaza) a `dev link`:

- **`dev link`**: Registro inicial, primera escritura solamente (ADR-0018). No actualiza entradas existentes.
- **`dev reconcile`**: Sincronización continua. Actualiza entradas existentes para reflejar el estado actual del bundle.

Usa `dev link` para la instalación inicial. Usa `dev reconcile` tras `dev update` para propagar cambios del bundle a las configs de IDE.

## Límites y tradeoffs (intencionados)

- **Primera escritura solamente (ADR-0018).** El framework nunca sobreescribe
  `model`/`description` de una entrada existente. Consecuencia: los archivos
  claude/codex escritos por el framework **no se actualizan** tras la primera
  escritura — los cambios de `description`/body del bundle no se propagan.
  Para regenerar uno, borra el archivo y vuelve a ejecutar `dev link`.
- **El `model:` del frontmatter de `agents/*.md` es inerte** para el registro
  (ADR-0017). Se conserva en los archivos para otras herramientas; el
  registro solo lee `assets/agent-models.yaml`.
- **Codex: campos no modelados se omiten.** `model_reasoning_effort`,
  `model_reasoning_summary` y cualquier otro campo que el framework no
  conoce no se escriben (ADR-0019). Si los necesitas, edita el `.toml`
  manualmente — el framework no lo tocará (primera escritura solamente).
- **Sin fallback hardcoded.** Un agente sin modelo configurado se omite con
  un warning; `dev link` termina con exit 0. Sin `agent-models.yaml`, los
  agentes se registran sin clave `model`.
- **Los comentarios de `agent-models.yaml` no se preservan** al editar con la
  TUI/`dev models set`: el archivo es dato gestionado por el framework y se
  reescribe desde el modelo tipado.
- El prune solo elimina entradas con prefijo `sddk-`/`sdd-`/`gentle-` que ya
  no existen en el bundle; las entradas de usuario nunca se tocan.
