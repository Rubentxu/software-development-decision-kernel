# Estrategia de arquitectura emergente

## Objetivo

Evitar dos errores simétricos:

1. diseñar hoy una plataforma universal enorme que no necesitemos;
2. seguir acumulando soluciones locales que obliguen a otra consolidación dentro de pocos meses.

La arquitectura se estabiliza mediante **interfaces pequeñas + UAT de consumidor real + promotion gates**.

## Regla de promoción

Una abstracción nueva pasa por:

```text
Observed pain
  ↓
minimal seam
  ↓
real consumer
  ↓
negative UAT
  ↓
second use or strong reuse evidence
  ↓
promote to stable contract
```

No se generaliza sólo porque el nombre suene reusable.

## Decisiones reversibles primero

### Reversibles

- shape inicial de `Affordance`;
- nombres de relaciones hypermedia;
- mapping capability→provider;
- formato de AugmentationProfile;
- transport CLI/MCP;
- Book pack experimental.

### Costosas de revertir

- Resource URI identity rules;
- StepDefinition versioning/compatibility;
- canonical event types;
- persistence schema;
- ownership entre domain/engine/gateway;
- provider authority boundary;
- WorkflowDefinition → IR semantics.

Estas últimas requieren ADR + migration story antes de implementación productiva.

## Strangler boundaries

### Workflow

```text
old WorkflowManifest ───────┐
                            ├─ equivalence adapter → WorkflowDefinition/IR
new WorkflowDefinition ─────┘
```

Primero compilar y comparar. Sólo después ejecutar por nuevo runtime.

### Prompts

```text
procedural prompt
   ↓ remove one responsibility at a time
prompt + typed affordances
   ↓
minimal cognitive prompt
```

No hacer un rewrite masivo de prompts.

### Context

```text
current resume commands
    +
new context bootstrap projection
    ↓ compare
new durable capsule
    ↓
old recipe becomes compatibility fallback
```

### Providers

Mantener adapters actuales y colocar CapabilityResolver por delante. No reescribir los adapters sólo para encajar nombres nuevos.

## Lateral thinking incorporado

### 1. Affordances como documentación ejecutable

La misma representación sirve para agentes, CLI discoverability, debugging y auditoría. `sddk explain action <id>` puede explicar por qué una acción está disponible/no disponible.

### 2. Evidence-carrying affordances

Una acción puede declarar qué evidence falta para desbloquearla. En lugar de error genérico:

```text
release unavailable
```

se devuelve:

```text
missing: verification:E91
missing: approval:A12
next legal actions: verify, request-approval
```

### 3. Semantic memoization

Un StepRun read-only puede ser reutilizable si coinciden:

```text
step_definition_digest
+ canonical input digests
+ context_basis
+ provider capability snapshot
+ policy digest
```

No implementar cache global en C6a; medir primero en C6d. Side-effecting steps nunca se memoizan de esta forma.

### 4. Capability content negotiation

El caller puede pedir `minimal`, `normal` o `forensic` representation, pero la autoridad decide el límite. Esto reduce tokens sin crear context contracts distintos.

### 5. Provider basis como parte del conocimiento temporal

Una Observation no sólo referencia el SHA del subject; también el capability snapshot/provider basis. Así una observación puede quedar stale porque cambió el código **o porque cambió el analizador**.

### 6. Workflow lint como type checker, no style checker

El compilador detecta:
- inputs insatisfechos;
- ciclos ilegales;
- capability inexistente;
- output contract incompatible;
- authority imposible;
- subworkflow incompatible;
- provider REQUIRED sin fallback permitido.

### 7. Segundo dominio como architecture test

El Book workflow no se desarrolla por feature comercial. Su primera misión es falsar dependencias software-specific ocultas en core.

## Stop conditions

Detener y replantear si:

- implementar hypermedia exige duplicar state;
- StepDefinition comienza a almacenar scripts/prompts gigantes;
- Augmentor se convierte en un scheduler paralelo;
- CapabilityResolver necesita conocer decisiones de negocio del domain;
- Book workflow exige cambiar más core que pack code: indicio de falsa generalidad;
- equivalence tests muestran que WorkflowIR no representa una semántica crítica del workflow actual.
