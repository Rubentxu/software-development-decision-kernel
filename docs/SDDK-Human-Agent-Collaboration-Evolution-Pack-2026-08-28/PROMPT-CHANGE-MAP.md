# Mapa de cambios de prompts y skills

## Regla general

No copiar personalidad ni reporting prose en 64 agentes.

Crear un contrato central:
`prompts/sddk/human-interaction-contract.md`

## `prompts/sddk/orchestrator.md`

Añadir:
- obligación de producir/derivar InteractionEvents;
- StageReport en transitions relevantes;
- risk-based attention;
- resume summary en restart/compaction;
- Result Contract referencia `current_run_view`.

Eliminar/evitar:
- duplicación de templates de fase;
- nueva semántica CLI fuera de `cli-usage-contract.md`.

## `prompts/sddk/status-query.md`

P0:
- CLI/ledger = runtime authority;
- vault = durable knowledge;
- git = code truth;
- chat = context only;
- error explícito ante cycle id desconocido.

## `prompts/sddk/phase-contracts.md`

Extender router context:
```yaml
interaction:
  audience:
  autonomy:
  personality_profile:
  locale:
  narration_budget:
```

Cada phase envelope:
```yaml
interaction_delta:
  findings:
  decisions:
  reframes:
  assumptions_changed:
  problems:
  next:
  attention:
```

## Phase prompts

No renderizan personalidad.
Sólo producen structured delta.

### apply
Reutilizar telemetry existente; convertir únicamente eventos noteworthy.

### verify/debt
Reportar:
- verdict;
- top findings;
- impact;
- recovery;
sin volcar lens trace.

### release/archive
Mantener receipts/evidence completos; human projection es derivada.

## Agent wrappers

No añadir rules de persona.
Sólo referencia al contract si el wrapper es user-facing coordinator.

## Skills

Crear como máximo:
- `human-interaction-renderer`
- `human-decision`
si el runtime/editor necesita una unidad reusable.

Preferencia: que la lógica determinista resida en Rust y las skills sólo adapten el uso.
