# SPEC-010 — Security, determinism and compatibility invariants

## Seguridad

### SEC-001 — Authority on use

La authority se reevalúa al invocar una affordance. Nunca confiar sólo en el snapshot que la anunció.

### SEC-002 — Capability least privilege

Augmentation sólo añade capabilities justificadas por StepDefinition/skill/profile/policy. No wildcard.

### SEC-003 — Provider trust boundary

Provider output es untrusted observation hasta normalización/validation. Provider no puede inyectar executable instructions por campos de texto.

### SEC-004 — Secret redaction

Args/env/stdout/stderr/errors/observations/receipts/context/hypermedia representations pasan policies de redacción aplicables.

### SEC-005 — Pack isolation

Un pack no obtiene capabilities por estar instalado. Declared != granted.

## Determinismo

### DET-001 — Compilation

Mismas definitions/registry snapshots/policies => mismo IR digest.

### DET-002 — Augmentation

Misma basis + registries + policy => mismo resolved step plan, salvo fields explícitamente non-deterministic registrados fuera del identity digest.

### DET-003 — Time/randomness

Domain/engine pure paths reciben timestamps/ids cuando afecten replay. Evitar wall-clock oculto.

### DET-004 — Affordance derivation

Affordances son función de state/policy/authority/capability snapshot/basis. Un receipt guarda inputs de derivación relevantes.

### DET-005 — Contributions

Canonicalization + stable idempotency evita duplicates bajo retry.

## Compatibilidad

### COMP-001 — Additive first

Nuevos schemas arrancan versionados y aditivos. Legacy readers se mantienen hasta cutover UAT.

### COMP-002 — Unknown fields

Consumers deben seguir reglas explícitas por contract; no asumir globalmente permissive o strict. Security-sensitive contracts default fail-closed.

### COMP-003 — Legacy workflow strangler

No borrar WorkflowManifest/prompt path hasta equivalence + restart + failure UAT del nuevo runtime.

### COMP-004 — Base remains first-class

Base sin providers debe funcionar. Un custom workflow con capability REQUIRED puede bloquear ese workflow, no todo SDDK.

### COMP-005 — Schema migrations

Toda nueva tabla/column de persistent context/run state requiere crash/reopen, previous-schema fixture y rollback/forward policy documentada.

### COMP-006 — No silent provider fallback

Fallback entre providers sólo si policy lo permite y queda registrado. REQUIRED evidence no se convierte en PASS al caer a Base.

### COMP-007 — Version pin in receipts

WorkflowRun registra exact WorkflowDefinition, StepDefinition, skills, policy y provider snapshots resueltos.

## Acceptance

SEC-UAT-001..012 + migration UAT.
