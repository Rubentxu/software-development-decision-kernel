# Cross-context dependency rules

## Reglas obligatorias

1. `alignment` puede depender de contratos de lectura de `knowledge` y `decision`; nunca de `storage` concreto.
2. `verification` orquesta; no implementa heurísticas de Alignment.
3. `governance` no interpreta SOLID/FP/Hexagonal; sólo consume hechos/contratos explícitos según policy.
4. `agent_experience::instructions` no depende de Alignment. `agent_experience::context` sí puede consumir `AlignmentAdvisoryView`.
5. `knowledge` no depende de CogniCode/Chronos; sólo de `Evidence`/provider-neutral observations.
6. `extension` conoce ports SDDK y DTO adapters; `domain` no conoce RPC/tonic/protobuf.
7. Workbooks son proyecciones. Ningún workbook puede mutar Decision Memory o Knowledge directamente.
8. `verify` nunca usa `deb_verify` como subrutina full-scan.
9. `deb_verify` puede reutilizar primitives de verify, pero posee estrategia global distinta.
10. Shared Kernel sólo contiene conceptos con semántica idéntica entre contextos; no es un cajón de utilidades.

## Fitness tests sugeridos

- imports prohibidos por módulo;
- no `sddk_storage::*` desde domain contexts;
- no `cognicode_*` / `chronos_*` fuera de adapters;
- no `AlignmentAssessment` en policy evaluation input salvo transformado a evidence/explicit contract fact;
- no `advisory_context` dentro de `EffectiveInstructions`.
