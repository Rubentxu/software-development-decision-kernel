# SPEC-009 — CLI, MCP and protocol surface

## Objetivo

Ofrecer una superficie pequeña y discoverable para agentes y humanos, manteniendo compatibilidad con comandos actuales durante strangler.

## Porcelain propuesto

Nombres exactos quedan sujetos a spike de collision con command registry, pero la semántica objetivo es:

```text
sddk context bootstrap
sddk resource get <sddk://...>
sddk resource actions <sddk://...>
sddk action invoke <action-id> --input <json>

sddk workflow validate <file>
sddk workflow compile <file>
sddk workflow run <ref>
sddk workflow inspect <run>

sddk step inspect <step-ref>
sddk capability list
sddk provider status
sddk explain action <action-id>
```

## Requisitos

### API-001 — One semantics

CLI/MCP adapters llaman el mismo application service. No duplicar reglas en MCP tool implementations.

### API-002 — Machine-first output

JSON contract versionado. Text es proyección humana.

### API-003 — Hypermedia default

`resource get` devuelve relations/actions; el caller no necesita otro command registry para continuar.

### API-004 — Compatibility aliases

Comandos legacy pueden delegar al nuevo application service mientras existan consumers. Deprecation explícita y medible.

### API-005 — No shell interpolation

Action invocation usa typed args/direct process APIs y conserva reglas existentes.

### API-006 — Explainability

`explain action` muestra state/policy/capability/evidence reasons sin revelar secretos.

### API-007 — Provider diagnostics

`provider status` separa lifecycle, negotiated protocol, capabilities y basis. No confundir product version con availability.

### API-008 — Discovery

MCP debe permitir descubrir schemas/actions dinámicamente o exponer generic resource/action tools; evitar una nueva tool por cada workflow custom.

### API-009 — Bounded surface

No publicar todos los internals como CLI commands. Porcelain resource/action/workflow/context; plumbing puede quedar internal/experimental.

### API-010 — Agent profiles

Command/action admission respeta AgentProfile authority y side-effect class ya existentes.

## MCP recomendado

Preferencia conceptual:

```text
sddk_resource_get(uri)
sddk_action_invoke(action_ref, input)
sddk_context_expand(context_ref, resource_ref)
sddk_workflow_validate(definition)
```

frente a generar docenas de tools estáticas por step.

## Acceptance

API-UAT-001..010.
