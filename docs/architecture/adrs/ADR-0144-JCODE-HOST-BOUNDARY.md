---
id: ADR-0144-JCODE-HOST-BOUNDARY
status: proposed
supersedes_history: false
proposed_at: 2026-09-29
accepted_at: null
accepted_by_cycle: null
references:
  - docs/roadmap/receipts/c2c/UAT-EVIDENCE-2026-09-29T1510.yaml
  - docs/roadmap/ROADMAP.md#c2c-jcode
  - docs/roadmap/UAT-MATRIX.md#t15-t18
  - docs/architecture/adrs/ADR-0140-agentic-integration-root-modules.md
---

# ADR-0144: Boundary de integración con el host JCode

- **Estado**: Proposed (decisión de operador requerida)
- **Fecha**: 2026-09-29
- **Decisión**: Elegir **una** frontera de autoridad para la integración SDDK↔JCode,
  declararla explícitamente y no mantener implementaciones paralelas.

## Contexto

C2c (T15–T18) estaba `NOT_EVALUATED_ADAPTER_MISSING` desde 2026-09-22 por dos
razones: no existía un SDK público de JCode, y el repo no tenía adapter. La
situación cambió y el recibo `receipts/c2c/UAT-EVIDENCE-2026-09-29T1510.yaml`
la documenta con evidencia observada:

- `@1jehuang/jcode-sdk` **1.1.0** existe en npm, protocolo **v1** estable.
- El propio SDK verifica el drift de schema **en ambas direcciones** con tests
  mecánicos (un test Rust falla si falta una variante; un test Node falla si
  divergen los tag sets). Eso es una garantía que un adapter propio no hereda.
- Ejercitado contra `jcode v0.89.1` real: `launch`/`connect`, `session` ≠ `run`,
  aislamiento entre instancias, `noReply` sin turno, `structured_schema_invalid`
  local, `readFile` acotado al working dir, `unknown_session` tipado.
- El repo de JCode (`1jehuang/jcode`, ~20k estrellas) mantiene el SDK como
  **subconjunto curado** con un test que difiere la API contra todo lo que la
  app terminal usa y hoy reporta cero huecos.

El roadmap (C2c) pedía explícitamente: *"Verificar si el adapter público vive en
otro repo antes de implementar uno duplicado"*. **Verificado: vive fuera.**

## Opciones

### (a) Sidecar Node sobre el SDK público — recomendado

SDDK habla con un proceso Node delgado que usa `@1jehuang/jcode-sdk`. La frontera
pública es el paquete npm versionado; el schema drift lo vigila su propio repo.

- **A favor**: cero duplicación de protocolo; hereda el control de drift; idioma
  natural para el host (el SDK es TS); el boundary se documenta como equivalencia
  y es auditable contra el paquete publicado.
- **En contra**: introduce un runtime Node en la superficie de despliegue; hay que
  fijar la versión del SDK como dependencia (no como rango abierto).

### (b) Adapter Rust habla el protocolo v1 del socket directamente

Implementar el cliente en `sddk-engine` (donde ya viven los ports de provider).

- **A favor**: un solo lenguaje; sin proceso extra; encaja con el patrón existente
  (`code_intelligence_port_mcp`, `runtime_evidence_port_mcp`).
- **En contra**: duplica un protocolo ya implementado y mantenido; el drift de
  schema pasa a ser **nuestro** problema, y el propio SDK avisa de que el surface
  evoluciona en minors; el repo de JCode documenta explícitamente que v1 acepta
  eventos y métodos nuevos, o sea que un cliente propio necesita un filtro de
  unknown frames que alguien va a mantener a mano.

### (c) Un `.sidecar` propio dentro de este repo, reexportando el SDK

- Descartada: es (a) con la dependencia mal ubicada — el sidecar pertenece al
  host, no a SDDK. Un fork local de un paquete ajeno es exactamente el duplicado
  que C2c prohíbe.

## Decisión

**Propuesta (a): sidecar Node sobre el SDK público**, con estas condiciones:

1. El paquete `@1jehuang/jcode-sdk` se fija a **versión exacta** (`1.1.0`), nunca a
   rango. El protocolo v1 es estable pero *admiten* miembros nuevos; un rango
   abierto convierte un minor del host en cambio de comportamiento nuestro.
2. El sidecar expone **una sola** operación tipada por caso de uso, y traduce a
   los tipos canónicos de SDDK (`Outcome`, `Receipt`, `EvidenceRef`). No reexporta
   el SDK como API propia.
3. `session` ≠ `run` se respeta: la sesión es un objeto durable con identidad; un
   run es un turno dentro. El sidecar no colapsa ambos.
4. La transcripción es **host-owned**: SDDK la referencia como `EvidenceRef`, no la
   copia ni la reescribe.
5. Los eventos se coalescen en el límite del sidecar; el interior del
   `turn_done` no se propaga como N eventos sueltos al ledger.
6. Ausencia de provider o de capability opcional produce `EvidenceGap` /
   `NOT_SUPPORTED`, nunca un PASS degradado (T18 ya observado así en el SDK real).
7. `JCODE_CORE_GA` **no** se declara con esto. La milestone C2c del roadmap
   exige AG0–AG2 y un recibo específico; este ADR solo fija la frontera.

## Consecuencias

- C2c deja de estar bloqueada por falta de adapter: el lado SDDK del boundary es
  un sidecar pequeño, y el contrato queda escrito antes de escribir código.
- Si el host publica un SDK Rust, la opción (a) se revisa; no se mantiene (b) en
  paralelo.
- La dependencia de Node pasa a ser requisito de despliegue del perfil Agentic.
  El perfil Base no la requiere y no debe transitivamente.

## Alternativa si el operador rechaza (a)

Entonces se elige (b) **con un ADR propio y obligaciones explícitas**: filtro de
unknown frames, tabla de eventos soportados, y un test de paridad contra el SDK
publico que falle cuando el host añade un evento. Sin esas obligaciones, (b)
es deuda disfrazada de integración.
