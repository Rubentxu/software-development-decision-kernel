# UAT Overlay — Acceptance Truthfulness / AIW / Context-First

Añadir estas filas a `docs/roadmap/UAT-MATRIX.md`.  
Estados iniciales: `NOT_RUN`.

| ID | Hito | Boundary | Escenario / falsador | Exit criterion |
|---|---|---|---|---|
| AT-UAT-001 | C3l.0 | PURE | Claim marcado VERIFIED sin boundary/evidence compatible | Re-clasificado; ninguna evidencia histórica se reescribe |
| AT-UAT-002 | C3l.1 | IN_PROCESS | ChallengeStrategy retorna error | Summary != ConfirmedBaseline |
| AT-UAT-003 | C3l.1 | IN_PROCESS | Todas las strategies fallan | Resultado incompleto/gap tipado |
| AT-UAT-004 | C3l.2 | IN_PROCESS | Regla registrada + ProducerEvent por `dispatch()` público | Señal esperada; engine vacío falsador falla |
| AT-UAT-005 | C3l.2 | IN_PROCESS | ProducerEvent Unknown | 0 señales y 0 side effects |
| AT-UAT-006 | C3l.3 | IN_PROCESS/SQLITE | Evidence gap → proposal → authority → PlanRevision → execution | Nueva revisión + sólo nodo nuevo |
| AT-UAT-007 | C3l.3 | IN_PROCESS/SQLITE | Replay del mismo trigger/proposal | 1 revisión y 1 ejecución total |
| AT-UAT-008 | C3l.3 | IN_PROCESS/SQLITE | Authority deny | 0 cambio canónico |
| AT-UAT-009 | C3l.4 | MCP_EXTERNAL | CHRONOS_MCP_BIN ausente | BLOCKED/NOT_RUN, nunca PASS |
| AT-UAT-010 | C3l.4 | MCP_EXTERNAL | Chronos real disponible | binary/version/hash/capabilities + captura observada |
| AT-UAT-011 | C3l.5 | SQLITE_MULTI_PROCESS | Dos CLI compiten por misma lease | Un owner, fencing monotónico, loser sin write |
| AT-UAT-012 | C3l.5 | SQLITE_MULTI_PROCESS | Winner muere y otro recupera tras condición válida | Sin doble autoridad ni divergence |
| AT-UAT-013 | C3l.6 | PROCESS | Writer + segundo consumidor real | Reader observa estado durable |
| AT-UAT-014 | C3l.6 | PROCESS | Segundo consumidor intenta write | Fail closed |
| AT-UAT-015 | C3l.7 | PROCESS | `check-architecture` sobre repo actual | 0 error no-waived o waiver vigente tipado |
| AT-UAT-016 | C3m.0 | PURE | Buscar significados KMT productivos | Exactamente una definición canónica |
| AT-UAT-017 | C3m.1 | PURE | Source cambia en una hoja | Sólo ramas afectadas invalidan |
| AT-UAT-018 | C3m.1 | PURE | Dependency/analyzer cambia sin source | Diferencia dimensional detectada |
| AT-UAT-019 | C3m.2 | PURE | revise con mismo contenido / distinto tiempo | Comportamiento coincide con ADR de identidad |
| AT-UAT-020 | C3m.3 | IN_PROCESS | Fake runtime provider ≠ Chronos | Canonical observation preserva provider fake; no "chronos-mcp" |
| AT-UAT-021 | C3m.4 | IN_PROCESS | Snapshot con log_head >0 pero evidencia insuficiente | No 0.95 inventado; estado/basis explícito |
| AT-UAT-022 | C3m.5 | PURE | Nuevo módulo root fuera del context map | Fitness falla |
| AT-UAT-023 | C3n.1 | PROCESS | Receipt E2E sin process_count/binary hash | Receipt inválido |
| AT-UAT-024 | C3n.2 | MIXED | Recertificación AIW de slices reabiertas | Estado por slice + SHA + boundary |
| AT-UAT-025 | C3n.3 | MIXED | Recertificación Context-First reabierta | Estado por hito + SHA + boundary |
| AT-UAT-026 | C3n.4 | RELEASE_ARTIFACT | Forzar fallo de un nuevo gate obligatorio | Perfil afectado no se certifica |

## Reglas

1. `NOT_RUN` no cuenta como PASS.
2. `BLOCKED_EXTERNAL_DEPENDENCY` tampoco.
3. Un test `PROCESS` debe observar PID/process boundary real.
4. `SQLITE_MULTI_PROCESS` requiere fichero durable compartido y dos procesos.
5. `MCP_EXTERNAL` registra binary path/version/hash/protocol/capabilities.
6. El falsificador debe atacar la garantía, no sólo cambiar datos irrelevantes.
