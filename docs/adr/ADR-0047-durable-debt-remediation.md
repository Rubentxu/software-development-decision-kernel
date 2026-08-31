# ADR-0047 — Remediacion durable y priorizada de deuda tecnica

**Status:** Accepted  
**Date:** 2026-08-21

## Context

`debt-verify` produce un informe inmutable por ciclo y `archive` puede crear
incidencias en el vault, pero el contrato actual no obliga al orchestrator a
recuperar esas incidencias, priorizarlas y convertirlas en trabajo de ciclos
posteriores. Una deuda puede quedar registrada sin entrar nunca en un alcance de
remediacion.

Tambien falta separar dos conceptos que no significan lo mismo:

- **Severity** mide el impacto tecnico intrinseco del problema.
- **Priority** decide cuando debe corregirse respecto al resto del trabajo.

Una deuda HIGH puede ser P1 por afectar un camino critico o P3 si esta aislada,
tiene workaround y riesgo aceptado vigente. La prioridad no cambia la severidad
ni puede ocultar evidencia.

## Decision

### 1. Informe de ciclo e incidencia durable son conceptos distintos

`debt-report.json` permanece como evidencia inmutable del ciclo. Una incidencia
`INC-NNN-{slug}.md` representa el estado acumulativo de una deuda entre ciclos.
El nombre sigue siendo legible para humanos; los fingerprints son propiedades de
correlacion, no nombres de archivo.

La incidencia durable incorporara, como minimo:

```yaml
status: open | accepted_risk | resolved
severity: critical | high | medium | low
priority: P0 | P1 | P2 | P3
canonical_fingerprint: sha256:...
fingerprint_aliases: []
first_observed_in_cycle: "[[CYC-...]]"
last_observed_in_cycle: "[[CYC-...]]"
observed_in_cycles: ["[[CYC-...]]"]
owner:
deferral_count: 0
max_deferrals:
due_by:
accepted_risk_reason:
accepted_risk_expires_at:
resolved_in_cycle:
reopen_evidence: []
```

Cada cambio de estado conserva procedencia y se anade al changelog bi-temporal.
Un cambio de regla o normalizacion de fingerprint exige un alias o una relacion
explicita de sustitucion; nunca puede desconectar silenciosamente el historial.

### 2. Debt-verify observa y propone; archive escribe conocimiento durable

`debt-verify` lee directamente las incidencias `open` y `accepted_risk` del vault,
calcula un digest canonico del conjunto consultado y lo vincula al informe. El
digest demuestra que baseline se uso, pero no crea un subsistema de snapshots.

`debt-report.json` sera el unico origen de `incidence_proposals`. El nodo de ciclo
y los informes humanos son proyecciones derivadas. Las propuestas permitidas son:

```text
create | observe | reopen | reprioritize
```

`debt-verify` no resuelve incidencias ni escribe el vault. Durante `archive`, un
caso de uso interno de reconciliacion aplica las propuestas de forma idempotente
y despues valida el vault. No se introduce una fase ni un agent nuevos.

Una incidencia ausente en un analisis no se resuelve automaticamente: el scope de
un ciclo puede no haber inspeccionado su modulo. Resolver exige evidencia positiva
de correccion vinculada a verify/debt-verify y al SHA publicado. Si reaparece un
fingerprint resuelto, la incidencia vuelve a `open` y registra la nueva evidencia.

#### El gate fallido se corrige en el ciclo actual

La persistencia durable no es una salida para aplazar deuda que impide superar el
gate:

| Verdict de debt-verify | Tratamiento |
|---|---|
| `FAIL` | Bloquear release, corregir en la misma rama y ciclo, y repetir verify y debt-verify. No convertir el blocker en follow-up para poder publicar. |
| `INCONCLUSIVE` | Bloquear, reintentar la cobertura fallida o exigir revision humana. La incertidumbre no se registra como deuda aceptada. |
| `PASS_WITH_WARNINGS` | Permitir el handoff y proponer incidencias durables solo para hallazgos no bloqueantes. |
| `PASS` | Continuar; archive puede cerrar incidencias seleccionadas solo con evidencia positiva de correccion. |

Si el launch plan selecciono una incidencia previa como alcance obligatorio y no
queda resuelta, verify/debt-verify no puede devolver un resultado aprobatorio. La
deuda seleccionada forma parte de los criterios de aceptacion del ciclo.

### 3. El orchestrator mantiene una cola durable de remediacion

Antes de adquirir el lock y fijar el launch plan, el orchestrator consulta las
incidencias abiertas, comprueba riesgos aceptados expirados y construye una cola
determinista. Cada entrada incluye prioridad, razones, antiguedad, aplazamientos,
owner, paths afectados y ciclos donde fue observada.

La prioridad se deriva de reglas visibles, no de una puntuacion opaca del LLM:

| Priority | Politica de planificacion |
|---|---|
| `P0` | Riesgo inmediato de seguridad, perdida/corrupcion de datos o ruptura de invariantes criticas. Bloquea el inicio de un ciclo no orientado a remediarlo hasta decision humana explicita. |
| `P1` | Riesgo alto, recurrente o con presupuesto de aplazamiento agotado. Debe entrar en el siguiente ciclo compatible o provocar un ciclo dedicado. |
| `P2` | Deuda planificable. Se incorpora cuando el nuevo cambio toca su contexto, alcanza `due_by` o existe capacidad declarada. |
| `P3` | Deuda menor o de observacion. Se mantiene visible y se reevalua cuando reaparece o cambia el contexto. |

La prioridad considera severidad, confianza, criticidad del area, recurrencia,
edad, numero de aplazamientos, expiracion del riesgo aceptado y solapamiento con
el scope propuesto. Un override humano requiere owner, justificacion y caducidad.

El launch plan registra dos listas:

```yaml
selected_debt: [INC-NNN]
deferred_debt:
  - incidence: INC-NNN
    reason: string
    deferral_count: integer
```

La deuda seleccionada pasa a proposal/spec, tasks y criterios de aceptacion. El
orchestrator no amplia silenciosamente un ciclo activo. Si aparece deuda nueva
durante verify, se remedia dentro del ciclo solo cuando el contrato del gate lo
exige; el resto se propone para ciclos futuros.

`B-direct` puede aplazar P1-P3 por su naturaleza de hotfix. No puede ignorar P0
sin un override humano de emergencia, trazable y con caducidad.

### 4. Los artefactos se conservan por defecto

Se establecen cuatro clases:

| Clase | Ejemplos | Politica por defecto |
|---|---|---|
| Durable | incidencias, ADRs, requirements, nodos de ciclo | Nunca borrar; resolver, marcar stale o superseder |
| Auditoria | debt/verify reports, receipts, archive manifest | Conservar mientras el proyecto exista |
| Trabajo | explore, proposal, design, tasks, apply progress | Conservar; una politica futura puede moverlos a almacenamiento frio |
| Presentacion | copias en `/tmp`, HTML temporal | Desechable |

No se implementa garbage collection automatico. Primero se mediran numero de
ciclos, bytes por clase y crecimiento. Cualquier compactacion futura requerira
otro ADR, `--dry-run`, aprobacion humana y prueba de restauracion. Nunca podra
eliminar evidencia alcanzable desde una incidencia abierta, riesgo aceptado, ADR,
requirement, nodo de ciclo, receipt o archive manifest.

## Consequences

### Positive

- La deuda deja de ser un informe historico y se convierte en trabajo gobernado.
- El orchestrator puede corregir deuda progresivamente sin inflar ciclos activos.
- Severity y priority dejan de confundirse.
- Los fingerprints deduplican sin sacrificar IDs legibles ni navegacion del vault.
- La correccion y reapertura conservan evidencia entre sesiones y ciclos.
- La politica conservadora evita perder artefactos antes de medir el crecimiento.

### Trade-offs / risks

- El vault crece y la consulta inicial del orchestrator gana complejidad.
- La estabilidad del fingerprint se convierte en un contrato versionado.
- `archive` necesita un reconciliador interno idempotente, aunque no una fase nueva.
- Una politica P0/P1 demasiado agresiva puede frenar trabajo de producto; los
  overrides humanos deben ser excepcionales, visibles y temporales.
- Sin compactacion automatica, el almacenamiento continuara creciendo hasta que
  exista evidencia suficiente para disenar una politica segura.

## Rejected alternatives

- **Guardar deuda solo en informes por ciclo:** conserva evidencia, pero no crea
  una cola accionable ni evita aplazamientos infinitos.
- **Usar Engram como registro principal:** facilita recuperacion entre sesiones,
  pero Engram no es autoridad durable del proyecto.
- **Nombrar incidencias por fingerprint:** rompe IDs humanos, navegacion y permite
  colisiones conceptuales cuando cambia una regla.
- **Resolver por ausencia en el siguiente scan:** un analisis acotado no demuestra
  que la deuda desaparecio.
- **Crear `sddk-debt-reconcile` como nueva fase:** aumenta el workflow y el runtime
  sin necesidad; la reconciliacion pertenece al cierre durable de `archive`.
- **Borrar por antiguedad:** puede destruir la evidencia que explica una deuda,
  decision o release todavia vigente.

## Implementation notes

La implementacion se divide en cuatro entregas trazables:

1. Informe de deuda tipado y propuestas de incidencia (`SDDK2-906`).
2. Schema y lifecycle durable de incidencias (`SDDK2-910`).
3. Reconciliacion entre ciclos y escritura idempotente en archive (`SDDK2-911`).
4. Planificacion del orchestrator por prioridad y politica de retencion (`SDDK2-912`, `SDDK2-913`).

## Compatibility/migration

Las incidencias existentes conservan sus nombres. Una migracion anade propiedades
con valores seguros: `priority` derivada de severity, fingerprints vacios hasta su
primera reconciliacion y `deferral_count: 0`. Ninguna incidencia se resuelve,
acepta o reprioriza automaticamente durante la migracion.

Los ciclos e informes existentes siguen siendo evidencia historica. El nuevo
reconciliador solo actua sobre informes que declaren la version de contrato nueva.

## Revisit trigger

Revisar esta decision cuando exista una de estas condiciones:

- Mas de 100 ciclos cerrados o 5 GiB de artefactos por proyecto.
- La consulta de incidencias supere 500 ms de forma sostenida.
- Sea necesario compartir una incidencia entre varios project IDs.
- Una exigencia regulatoria obligue a retencion o borrado con plazo fijo.

## Implementation trace

- **cycle-7a** (this commit, ratifies status): adds severity taxonomy (`docs/debt/SEVERITY.md`), priority taxonomy (`docs/debt/PRIORITY.md`), debt directory index (`docs/debt/README.md`), AGENTS.md §4 reference. Implements REQ-K7-001..003 + REQ-K7-009 (AGENTS.md §4 reference, partial).
- **cycle-7b** (next cycle): JSON Schema draft-07 for `debt-report.json` (`docs/debt/debt-report.schema.json`), INC template (`docs/debt/INCIDENCE-TEMPLATE.md`), agent updates (`sddk-debt-verify`, `sddk-archive`), workflow gates (`debt-severity-assigned`, `debt-priority-assigned`), orchestrator + phase-contracts + arsenal prompt updates. Implements REQ-K7-004..009.
