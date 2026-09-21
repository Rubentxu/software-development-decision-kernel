# Diario append-only — recuperación entre sesiones

**Regla:** añadir entradas al final con hora UTC, actor, SHA antes/después, WorkItem, evidencia observada, pruebas NO ejecutadas, riesgos, decisiones y siguiente acción. Nunca corregir una entrada anterior silenciosamente: añadir `RECONCILIATION` con referencia. El diario es **índice**, no sustituye el ledger canónico ni el RECEIPT. El puntero mutable está en [CURRENT.md](CURRENT.md) y [STATE.yaml](STATE.yaml).

## Plantilla para próxima entrada

```markdown
### YYYY-MM-DDTHH:MM:SSZ — <cycle/slice> — <actor>
- Baseline: branch/sha/tag/release comprobados:
- Alcance/autorización; no-objetivos:
- Ejecutado: archivos/commit(s):
- UAT: IDs, comando, actual/expected, OBSERVED/NOT_RUN y receipt:
- Gates: PASS/FAIL/BLOCKED/NOT_APPLICABLE; perfil y alcance:
- Riesgos/decisiones, responsable y revisit trigger:
- CURRENT/STATE reconciliados a SHA:
- Próxima acción ejecutable (o STOP honesto):
```

### 2026-09-21 — ROADMAP-REBASE / DOCUMENTATION-PROPOSAL

- Baseline inspeccionado: `main@2ffff3127e7179b5f3c3104c471c8ad2c7d920ff`; Cargo.toml 1.169.127; release pública consultada: v1.169.122. Todos los datos pueden haber cambiado: revalidar al retomar.
- Diagnóstico de continuidad: la certificación A5 Base es histórica y condicionada; el cierre de los tracks internos no equivale a certificación de todos los perfiles externos ni del host JCode. El roadmap A5 y varios handoffs contenían fotografías antiguas.
- Acción propuesta en rama `docs/roadmap-production-certification-2026-09-21`: único roadmap de continuación C0–C5, contrato de certificación, matriz UAT T01–T35, CURRENT/STATE, catálogo de archivo y reglas de recuperación en AGENTS. Se trasladan cuatro documentos antiguos con punteros de compatibilidad; no se alteran ADRs, specs, recibos ni código.
- Pruebas: **NO EJECUTADAS** (cambio documental por conector GitHub, sin checkout local); validación de enlaces, lints, build y release pendiente del ciclo de integración. No hay nueva certificación ni release.
- Siguiente acción: revisar diff del PR, ejecutar gates locales documentales, integrar respetando release policy y abrir C0 para fijar el SHA real de trabajo y los UAT T01/T02.
