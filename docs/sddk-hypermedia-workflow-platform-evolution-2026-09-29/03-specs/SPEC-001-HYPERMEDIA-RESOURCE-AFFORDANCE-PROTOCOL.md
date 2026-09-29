# SPEC-001 — Hypermedia Resource & Affordance Protocol

## Estado

**Proposed** — requiere ADR-001.

## Objetivo

Definir un protocolo agent-first para inspeccionar estado y descubrir acciones legales sin codificar el lifecycle completo en prompts o clientes.

## Requisitos

### HYP-001 — Resource identity

Todo recurso navegable MUST tener un identificador lógico estable con esquema `sddk://`.

### HYP-002 — Transport independence

La representación MUST ser semánticamente equivalente entre CLI, MCP y futuros transports. Un transport MAY omitir decoraciones, nunca cambiar autoridad o significado.

### HYP-003 — Representation envelope

Toda representación agent-facing MUST incluir:

```text
resource
resource_type
representation_version
state
basis
relations[]
actions[]
provenance
```

Campos opcionales: `summary`, `embedded`, `warnings`, `staleness`, `cost_hints`.

### HYP-004 — Typed relation

Una relación describe navegación read-only:

```json
{"rel":"work-item","href":"sddk://work-items/W1","type":"WorkItemV1"}
```

MUST NOT implicar permission o side effect.

### HYP-005 — Typed affordance

Una acción MUST declarar como mínimo:

```text
rel
action_id
target
input_schema
output_schema
required_authority
idempotency
basis
```

MAY declarar evidence prerequisites, preconditions, estimated cost y expiration.

### HYP-006 — Derived, not canonical

Affordances MUST derivarse de estado + policy + authority + capability availability. No son una segunda fuente de verdad. Pueden quedar registradas en receipts de ejecución como snapshot reproducible.

### HYP-007 — Basis binding

Toda acción mutable MUST estar vinculada a la basis con la que fue emitida. Si state/policy/authority cambia, ejecutar una affordance stale MUST fallar tipadamente.

### HYP-008 — No prompt authority

Una relación o advisory text dentro de la representación no se convierte en InstructionSource. La autoridad de una acción procede del kernel.

### HYP-009 — Problem detail

Un bloqueo MUST devolver problema tipado + acciones legales de recuperación cuando existan.

Ejemplo:

```json
{
  "state":"blocked",
  "problem":{"type":"missing-evidence","missing":["runtime.trace"]},
  "actions":[{"rel":"resolve","action_id":"capability.observe/runtime.trace"}]
}
```

### HYP-010 — Discoverability

Un agente nuevo MUST poder continuar un run con sólo:

1. resolver un resource URI inicial;
2. leer `actions`;
3. validar schemas;
4. ejecutar una affordance;
5. seguir la representación resultante.

No debe necesitar una tabla de transición duplicada en su prompt.

## Versionado

`representation_version` evoluciona independientemente de API transport version. Cambios aditivos no requieren major si clientes deben ignorar campos desconocidos. Cambios semánticos de campos existentes sí.

## Seguridad

- Never embed secrets in representation.
- Authority MUST be evaluated server/kernel side at invocation time, not trusted from the prior representation.
- Resource IDs MUST not encode credentials.
- Embedded context MUST respect redaction policies.

## Acceptance

HYP-UAT-001..010 en `06-uat/UAT-MATRIX.md`.
