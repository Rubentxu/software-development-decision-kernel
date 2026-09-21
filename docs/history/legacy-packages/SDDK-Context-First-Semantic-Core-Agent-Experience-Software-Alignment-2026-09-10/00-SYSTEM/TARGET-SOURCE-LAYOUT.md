# Target Source Layout — reorganizar ahora, separar crates sólo si se justifica

## Decisión

Mantener los crates por capa actuales durante la primera etapa, pero **reubicar módulos dentro de cada crate por bounded context antes de implementar Software Alignment**.

```text
crates/
  sddk-domain/src/
    shared/
    planning/
    execution/
    decision/
    knowledge/
    alignment/
    verification/
    governance/
    agent_experience/
    extension/

  sddk-engine/src/
    planning/
    execution/
    decision/
    knowledge/
    alignment/
    verification/
    governance/
    agent_experience/
    extension/

  sddk-storage/src/
    canonical/
    objects/
    projections/
      planning/
      execution/
      decision/
      knowledge/
      alignment/
      verification/

  sddk-cli/src/
    commands/
      planning/
      execution/
      decision/
      knowledge/
      alignment/
      verification/
      governance/
      extension/
    composition/

  sddk-gateway/src/
    agents/
    providers/

  sddk-vault/
    # sigue siendo adapter/KnowledgeSource, no Knowledge authority
```

## Regla para futuros crates por BC

No crear `sddk-alignment`, `sddk-knowledge`, etc. automáticamente. Abrir un ADR de split sólo si al menos dos señales se mantienen durante varios hitos:

- change coupling interno alto y externo bajo;
- build/test boundary clara;
- ownership/API pública estable;
- dependencia unidireccional comprobable;
- el split reduce blast radius o permite release/feature isolation;
- no obliga a duplicar Shared Kernel.

## Compatibilidad durante movimientos

Mover módulos con `pub use` temporales y tests de ruta/compilación. No mezclar el movimiento físico con cambios semánticos grandes en el mismo commit.
