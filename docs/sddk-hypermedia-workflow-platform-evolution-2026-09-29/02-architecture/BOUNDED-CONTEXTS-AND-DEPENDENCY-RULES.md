# Bounded contexts y reglas de dependencia

## Contextos propuestos

### Workflow Definition

Responsable de definiciones versionadas, referencias a steps, wiring de inputs/outputs, guards, composición y defaults. No ejecuta.

### Workflow Execution

Responsable de WorkflowRun, StepRun, Attempt, runtime operators, convergence, replay y reconciliation. No conoce CogniCode/Chronos.

### Step Catalog

Responsable de StepDefinition, versionado, compatibilidad y contratos tipados. No concede authority ni contiene adapters de provider.

### Agent Experience / Hypermedia

Proyecta recursos, relaciones, affordances, context bootstrap/expand y transport adapters. No persiste hechos arbitrariamente ni posee transcript.

### Context & Knowledge

Compone ContextBasis/Capsule/Delta a partir de planning, memory, semantic graph, vault y observations. No decide workflow transitions.

### Intelligence Extension Platform

Resuelve capabilities a providers, lifecycle, negotiation y anti-corruption adapters. Produce Observation, nunca Decision/Authority directamente.

### Planning & Decision

WorkItems, dependencies, decisions y provenance. Sigue siendo la autoridad del trabajo planificado.

### Evidence & Semantic Graph

EvidenceRef/CAS + relaciones semánticas/projections. Una observación puede producir evidence; no se duplica store.

### Governance / Authority

Decide qué side effects y transitions están permitidos. Affordances consultan esta autoridad, no la replican.

### Packs

Agrupan StepDefinitions, skills, workflows, augmentation profiles y mappings de capacidades propios de un dominio. Un pack no crea un scheduler ni un store paralelo.

## Reglas de dependencia

```text
Domain contracts
    ↑
Engine pure services
    ↑
Ports
    ↑
Adapters / Storage / Providers / Transports
    ↑
CLI composition root
```

### Prohibiciones

- `sddk-domain` no importa SDKs de CogniCode/Chronos/host.
- `WorkflowIR` no contiene prompts ni transport details.
- `StepDefinition` no contiene API keys, executable paths o provider DTOs.
- `Agent Experience` no escribe canonical facts sin pasar por un reconciler autorizado.
- `Pack` no puede registrar wildcard capabilities.
- `Provider` no puede insertar decisiones o approvals directamente.
- `Hypermedia projection` no se convierte en segunda state authority.
- `ContextCapsule` no almacena transcript completo.

## Relación con crates actuales

No se fuerza un split inmediato. Primero se establecen módulos/fronteras y fitness tests. Un split físico sólo se justifica cuando las métricas de cambio/compilación/ownership lo avalen, coherente con R11 del roadmap vigente.

Posible evolución tardía, no objetivo inmediato:

```text
sddk-domain
sddk-engine
sddk-storage
sddk-gateway
sddk-agent-experience     (solo si métricas lo justifican)
sddk-extension-platform  (solo si segundo consumidor lo justifica)
sddk-pack-sdk             (cuando C6e lo necesite)
```

## Fitness functions nuevas

1. Ningún provider type en domain/workflow/agentic generic contracts.
2. Ningún prompt de fase contiene el recipe completo de lifecycle command cuando exista affordance/reconciler equivalente.
3. Un StepDefinition no contiene nombre de provider salvo campo de override explícito y opcional.
4. Toda Contribution productiva se valida contra output contract antes de persistir.
5. ContextCapsule/Delta durable sobreviven restart.
6. Hypermedia affordances cambian al cambiar state/authority/basis; no se reutilizan stale.
7. Los workflows software por defecto compilan a WorkflowIR; el viejo manifest sólo queda como compat/projection durante migración.
8. Los packs no escriben directamente event log fuera de APIs del kernel.
