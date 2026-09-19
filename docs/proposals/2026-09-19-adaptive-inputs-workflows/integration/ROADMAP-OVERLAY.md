# Overlay propuesto sobre roadmap ACTUAL (no sustitutivo hasta reconciliación)

## Fuente de autoridad

**Base:** `docs/architecture/README.md` (entrada normativa), `docs/architecture/a5/A5-CURRENT-ROADMAP.md`, `docs/SDDK-Production-Readiness-Alignment-2026-09-14/02-MINI-ROADMAP.md`; en HEAD `da37510`. El texto de este overlay NO debe copiarse sobre el archivo actual entero ni editar el estado de entregas históricas. Importar únicamente como delta en el próximo roadmap-sync aprobado. Antes de aplicar, verificar si HEAD, WorkItems, receipts o roles han cambiado.

## Alineamiento slot por slot

| Slot vigente | Estado fuente (2026-09-19) | Delta propuesto (no cambiar ese estado sin receipt) |
|---|---|---|
| A0–A4 / A5-C | Certificaciones históricas cerradas; A5-C v1.169.88 | Mantener intacto; lo nuevo no recertifica ni invalida BASE. |
| A6 / R7 / STATIC_ENHANCED | Roadmap marca A6 `NOT STARTED` aunque CC-S0 seam/ADR-0137 ya existen | Sincronizar subestado «CC-S0 seam shipped, A6 enhanced NOT delivered»; admitir AIW-S0/S1 con proveedor REAL→Observation→Verify (AC10). AIW-S1b solo si contradicción durable necesita nuevo shape. |
| A7 / R8 / RUNTIME_ENHANCED | Reservado | AIW-S5: Chronos REAL, contexto de escenario + Verification; sin A7 fake como exit. |
| A8 / FULLY_ENHANCED | Bloqueado por A6+A7 | AIW-S6: composición de ambas fuentes y contradicciones persistentes cuando exigido, sin autoridad de proveedores. |
| J2..J6 / JCode | Paralelo P1, J2 propuesta no abierta | AIW-S8: probar `arch-spec-030` antes de nueva API específica JCode, respetando SEC-1. |
| R9 / atención-watchlists | Parte del roadmap Context-First | AIW-S7: consulta read-only sobre Planning/Verify/Execution con consumidor real; no Agenda DB. |
| Workflow evolution | Presenta IR/expansion propuesta histórica | AIW-S4: primer Task añadido por trigger verificable, sin introducir segundo BT runtime; scope y WorkItem nuevos solo por reconciliación. |
| R11 | Evaluación de separación de crates P3 | AIW-S8: medir segundo consumidor, necesidad de packaging, concurrency y UX antes de dividir CLIs. |
| Experimento Jev | No es prerequisito del roadmap | No abrir slot salvo dataset etiquetado + mejora demostrada; nunca bloquear A6. |

## Texto sugerido para la próxima revisión aprobada de A6 (añadir, no sustituir filas)

> A6/CC-S0: el seam SDDK-owned (`CodeIntelligencePort`, fake/Null, pruebas de protocolo) y ADR-0137 existen a `da37510`. Esto no constituye `STATIC_ENHANCED` ni satisface AC10. El siguiente alcance A6 debe demostrar un proveedor CogniCode real, normalización a `observation::SoftwareObservation`, basis SHA-256 canónica, consumo end-to-end de una claim por Verify, errores/faltas tipados y Base green sin proveedor. La forma sucesora de evidencia de contradicciones permanece condicional a UAT de persistencia. Las piezas de captura de test/gate y handoff son habilitadores con consumer/owner propios, no sustitutos de A6.

## Texto sugerido para nota de adopción en `docs/architecture/README.md`

> `docs/proposals/2026-09-19-adaptive-inputs-workflows/` es un paquete de especificaciones y ADR **propuestas**, alineado con A6/A7/A8/J2/R9/R11. No crea un roadmap paralelo ni modifica cierres A5/M0–M9. La autoridad de ejecución permanece en `docs/architecture/a5/A5-CURRENT-ROADMAP.md`; cada slice se incorpora a través del WorkItem/ADR/receipt vigente tras aprobación y UAT.

## Cómo «sustituir» la propuesta anterior sin falsear autoridad

1. Aceptar este paquete como **guía consolidada** para nuevo trabajo, no aceptar implícitamente cada ADR ni iniciar todos sus hitos.
2. En los seis documentos origen, añadir una nota al inicio, cuando se autorice: «Consolidado como propuesta de evolución en `docs/proposals/2026-09-19-adaptive-inputs-workflows/README.md`; hallazgos históricos conservados, la autoridad de ejecución sigue en el roadmap canónico». No reescribir hallazgos observados ni borrar su historia.
3. Reconciliar Planning Ledger/roadmap con un WorkItem **existente** para el primer slice. Si no existe, crear uno solo siguiendo el procedimiento autorizado, con scope y gate, no seis por importar el ZIP.
4. Eliminar de prompts/skills únicamente afirmaciones obsoletas o invocaciones que el UAT demuestre incorrectas, mediante su cambio versionado. «Cero menciones en auditoría» por sí solo no significa que debamos insertar llamadas a todos los comandos.
