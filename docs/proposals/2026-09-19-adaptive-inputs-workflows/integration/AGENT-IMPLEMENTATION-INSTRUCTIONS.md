# Instrucciones para un agente que integre este evolutivo

**Antes de cada ciclo, leer primero** `AGENTS.md` del repositorio, `docs/architecture/README.md`, `docs/architecture/a5/A5-CURRENT-ROADMAP.md`, el WorkItem y el receipt vigente. Esta guía no sustituye la gobernanza del repo. Al cambiar HEAD respecto a `da37510`, revalidar símbolos/consumidores antes de escribir.

## Preparación del slice

1. Localizar consumer productivo preciso: comando/host/skill que debe tomar distinta decisión con el resultado; documentar baseline observada, scope y test negativo.
2. Mapear `source → actual function → ADT → writer → read model → consumer` con rutas/símbolos reales, distinguiendo `proposed`, `implemented`, `wired` y `E2E`. Confirmar si el input se produce automáticamente, por puerto o lo elige el agente.
3. Reusar base de revisión/dirty/env/policy, `WorkItem`, `DecisionRecord`, `SoftwareObservation`, `EvidenceAttachmentRecord`, `ContextCapsule` y `WorkflowIR`. Cualquier tipo público adicional necesita declaración de clase de estado, owner e invariante; **ninguna tabla/enum universal por analogía**.
4. Si una API del documento está sólo en una spec (p. ej. `ExecutionOutcome`), no afirmar que existe: encontrar su equivalente ejecutable o presentar un adaptador justificado que no cree otra autoridad.
5. Si hay divergencia con roadmap/ADR aceptada, detener **esa modificación de autoridad** y registrar el conflicto; los tests independientes de lectura pueden continuar. No reescribir conjeturalmente documentos cerrados.

## Implementación en un ciclo mínimo

- Test red primero: al menos un negativo de ausencia/partial/stale/deny y un E2E de consumidor; que el test pase con fake no basta si el exit exige proveedor real.
- Diferencia mínima: un productor/normalizador/consumidor, usando el gateway o caso de uso existente. No construir clase `Manager`, `Registry`, cola ni CLI nuevos si no existe una responsabilidad sin owner.
- Mantener el código de lectura/normalización sin escribir Authority, proyecciones sin escribir hechos y agentes sin declarar PASS.
- Si se introduce un campo/relación, ofrecer camino compatible de lectura/escritura, prueba de datos previos y UAT de reinicio/concurrencia cuando aplique.
- Actualizar UN prompt/skill consumidor real, probar `--help`/command registry y un flujo agentico verificable. Cero menciones en prompts antiguos no es motivo para modificar todos.

## Handoff de ciclo

Entregar: source SHA, WorkItem, ADR/spec exactas aceptadas, lista mínima de archivos cambiados, comandos y outputs UAT *reales*, receipts/ref y limites, qué rutas son Fake/Null, qué no está wired, qué trabajo se pospone, evidencia de Base green y decisión «seguir/rectificar/descartar» con datos. **No emitir PASS con un resumen de un agente ni cerrar por número de tests si falta el E2E productor→consumidor.**
