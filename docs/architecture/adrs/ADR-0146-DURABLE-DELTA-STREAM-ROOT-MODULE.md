---
id: ADR-0146-DURABLE-DELTA-STREAM-ROOT-MODULE
status: accepted
supersedes_history: false
proposed_at: 2026-09-29
accepted_at: 2026-09-29
accepted_by_cycle: c3j
references:
  - docs/sddk-hypermedia-workflow-platform-evolution-2026-09-29/03-specs/SPEC-005-DURABLE-CONTEXT-SESSION-HANDOFF.md
  - docs/roadmap/ROADMAP.md#c3j
  - docs/architecture/adrs/ADR-0145-DURABLE-CONTEXT-STORE-ROOT-MODULES.md
  - crates/sddk-engine/src/durable_delta_store.rs
  - crates/sddk-engine/src/context_bridge.rs
---

# ADR-0146: `durable_delta_store` como módulo raíz (DeltaStore durable)

- **Estado**: Accepted
- **Fecha**: 2026-09-29
- **Decisión**: Aceptar `durable_delta_store.rs` como módulo raíz nuevo del
  engine porque materializa CTX-008 (DeltaStore durable) como **tercer seam
  independiente** de la capa de contexto durable, y porque el store **no es
  autoridad** sobre la base: solo la observa y la reporta.

## Contexto

SPEC-005 define CTX-008: un `ContextDelta` debe sobrevivir entre procesos para
que un `ContextBridge` rehidratado pueda consumirlos. Antes de este módulo el
`ContextBridge` era estrictamente efímero: `crates/sddk-engine/src/context_bridge.rs`
rechaza un delta cuyo `from_revision` no coincide con la base actual, así que
un stream publicado por un proceso anterior se rechazaba entero al
rehidratarse. El resultado era que el handoff durable no existía.

El store nuevo persiste un delta por fichero
(`delta-<seq>.json`), con secuencia monotónica y watermark `last_seq`, escrita
atómica (temporal + `rename`) y replay ordenado. Cierra CTX-008.

Como ADR-0145 ya\vio\ el precedente: un módulo raíz nuevo de contexto exige una
ADR, y el guard `no_new_root_level_context_module_without_adr` de
`crates/sddk-cli/tests/context_fitness.rs` lo hizo RED con
`new root-level context modules require an ADR (none found): ["crates/sddk-engine/src/durable_delta_store.rs"]`.

## Opciones

### (a) Meter el DeltaStore dentro de `context_bridge.rs`

Rechazada. `context_bridge.rs` es el **contrato de consumo** en memoria: qué
significa aplicar un delta y cuándo se rechaza uno stale. Persistir el stream
dentro de ese fichero mezcla la semántica de aplicación con el formato en
disco, y hace que "rechazar un delta stale" dependa de si el fichero se pudo
leer. El rechazo stale es un hecho semántico; la corrupción es un hecho de
almacenamiento. Son errores distintos y deben vivir separados.

### (b) Un módulo genérico `durable_stream.rs` reutilizable para los tres stores

Rechazada por ahora. Los tres stores durables comparten una técnica (write +
`rename` + replay ordenado) pero no un contrato: el watermark del delta stream
es una secuencia, el de capsules es una revisión, el de bindings no tiene
watermark. Una abstracción genérica ahora sería jerarquía especulativa sobre
tres implementaciones con contratos distintos; se revisita si aparece un
cuarto store con el mismo contrato que el delta stream.

### (c) `durable_delta_store.rs` como módulo raíz, sin autoridad sobre la base — **aceptada**

El módulo expone lectura/escritura del stream y el **replay** sobre un
`ContextBridge` que el caller ya rehidrató. No escribe la base del binding: el
binding durable sigue siendo la autoridad sobre lo que la sesión cree, y el
servicio CLI decide si actualiza o no. Un store que reescribiera la base sería
una segunda autoridad sobre el mismo hecho, que es exactamente el fallo que el
guard de ADR-0145 previene.

## Decisión

1. `FilesystemDeltaStore` persiste un `ContextDelta` por fichero bajo
   `delta-<seq>.json`, con secuencia monotónica y watermark `last_seq`.
2. `append` asigna el siguiente slot libre y escribe con temporal + `rename`;
   la lectura nunca ve un delta a medio escribir.
3. `apply_to` devuelve tres cosas, no una: `applied`, `rejected` (stale
   `from_revision` / seq no monótona, con su razón) y `replay_skipped`
   (ficheros ilegibles o corruptos). Un fichero corrupto **nunca** se convierte
   en contexto válido: se reporta y se salta.
4. `origin_basis` expone el `from_revision` del primer delta. Es la única forma
   honesta de rehidratar: rebobinar al origen del stream en vez de arrancar en
   la base actual, que rechazaría todo lo ya consumido.
5. El store no muta el binding. La decisión de avanzar la base es del servicio.

## Consecuencias

- CTX-008 es material: un delta publicado por un proceso es visible y aplicado
  por el siguiente, con rechazo stale y corrupción reportados.
- `ContextBridge` sigue siendo efímero por diseño; lo durable es el stream, no
  el bridge. La rehidratación es "reconstruir el bridge y volver a aplicar el
  stream", que es una operación explícita y testeable.
- La corrupción es visible en la salida del comando, no silenciosa. Un stream
  con un fichero corrupto entrega el resto y nombra el hueco.
- La escritura no es multi-proceso segura frente a dos `append` simultáneos con
  el mismo seq calculado. Cada writer serializa por sesión en la práctica
  (un host, un ledger, una sesión) y la consecuencia de una carrera sería un
  delta pisado, no un stream corrupto: el `rename` sigue siendo atómico. Queda
  declarado como límite conocido, no como garantía.

## Alternativas rechazadas y revisitas

- Locking por sesión: no se añade todavía. El coste (un lock file más un
  protocolo) solo se justifica si aparece evidencia de dos writers
  concurrentes reales.
- Un stream por `ContextBasis` en vez de por sesión: se revisita si un mismo
  workspace necesita compartir stream entre sesiones; hoy el binding es
  por sesión, así que el stream lo es también.
