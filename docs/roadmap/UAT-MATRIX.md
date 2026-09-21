# UAT — matriz de aceptación falsable (nuevo roadmap)

**Estado inicial:** todos los casos `NOT_RUN` en el nuevo baseline; las pruebas históricas se enlazan como `HISTORICAL`, no se transcriben como PASS actual. Cada `UAT-EVIDENCE.yaml` debe fijar SHA, tag si aplica, versión binario/bundle, entorno, precondiciones, input/expected/actual, ejecución real/mock, stdout/stderr sanitizados, digest y referencia de recibo. Cuando un escenario no aplique al perfil, explicar `NOT_APPLICABLE`.

| ID | Hito / nivel | Escenario y falsador | Exit criterion |
| --- | --- | --- | --- |
| T01 | C0 / T0 | HEAD, versión Cargo, tag y release comprobados independientemente; inyectar puntero stale | Se rechaza confundir código dev con artefacto publicado. |
| T02 | C0 / T1 | Mapeo requisito→commit→test→receipt; introducir evidencia ausente | Estado queda NOT_RUN/BLOCKED, nunca PASS. |
| T03 | C1 / T0 | Descriptor de tipo desconocido en structured_work | Error tipado; nunca Contribution. |
| T04 | C1 / T0 | Required missing, wrong type, objetos anidados, extra fields según schema | Resultado inválido rechazado; válido aceptado. |
| T05 | C1 / T1 | Enviar request_id duplicado con payload distinto e igual | Conflicto explícito o idempotencia contractual, no silent overwrite. |
| T06 | C1 / T1 | Canario secreto en salida inválida | Error/receipt no incluye valor del canario. |
| T07 | C1 / T1 | Llamar test seam y producción en fixture de compilación | API de test no accesible en producción o justificación/contrato explícito; singleton no se sustituye silenciosamente. |
| T08 | C2a / T4 | CogniCode binario real, handshake/capabilities/protocolo | Identidad/versión y capacidades registradas; no capabilities inferred. |
| T09 | C2a / T4 | Grafo incompleto / usage no indexado / claim sin prueba | UNKNOWN/EvidenceGap; nunca Verified por 0 usages. |
| T10 | C2a / T4 | Proveedor ausente, timeout, crash/restart | Error tipado; Base continua sin false PASS. |
| T11 | C2a / T4 | Re-ejecutar contrato estático en dataset/grafo completo acordado | Cobertura/frescura observables y digest estable. |
| T12 | C2b / T4 | Chronos real captura >0 y claim no satisfecha | Verdict conforme a eventos reales; no inferir exit_status. |
| T13 | C2b / T4 | Capture 0, probe falla, timeout, restart | UNKNOWN/ERROR tipado, sin éxito falso. |
| T14 | C2b / T4 | Dos programas y presupuestos recursos/protocolo | Evidencia por programa/contrato, sin filtrado silencioso. |
| T15 | C2c / T4 | Adapter separado con SDK público, host real, connect + attach | Compila fuera del repo interno; Session != Run; versiones y SHA exactos. |
| T16 | C2c / T4 | Host event burst + turn_done + cambios irrelevantes | Bounded Verify; no segunda fact log ni spam de contexto. |
| T17 | C2c / T4 | ContextDelta stale/reordered + trabajo estructurado invalid/timeout | Reject/dedup correcto; Outcome y receipt trazables, sin promesa ficticia. |
| T18 | C2c / T4 | Host sin capability opcional y proveedor ausente | NOT_SUPPORTED / EvidenceGap; utilidad Base preservada. |
| T19 | C3 / T3 | Deny + policy swap entre issue y consume | Cero efectos tras invalidación. |
| T20 | C3 / T3 | Doble CLI y policy digest concurrente | No ticket cruzado ni global ordering asumido. |
| T21 | C3 / T3 | Crash/reopen alrededor de event append, CAS y receipt | Rebuild determinista; sin evento duplicado ni ACK perdido. |
| T22 | C3 / T3 | Contention SQLite en superficies IMMEDIATE activas | Retry limitado e idempotente o error explícito; no writes parciales. |
| T23 | C3 / T6 | Canarios en env, args, stdout/stderr, error/reason, receipt y CAS | Cero filtraciones en todos los sinks esperados. |
| T24 | C3 / T6 | Bytes no-UTF8/NUL y payload+args oversized | Denegación estructurada sin eco de datos ni ejecución. |
| T25 | C3 / T6 | Provider/host intenta bypass de Authority/Storage | Rechazo inequívoco; cero escritura canónica. |
| T26 | C3 / T6 | Baseline p50/p95 y consumo Base/static/runtime | Métrica reproducible y presupuesto aprobado; sin umbrales inventados. |
| T27 | C3 / T3 | Schema future incompatible, upgrade/rollback probado | Fail closed o migración soportada; DB intacta. |
| T28 | C4 / T5 | Full verify: fmt + clippy + workspace tests + shell/contract suites | Todas PASS en SHA actual, ignored y flakes inventariados. |
| T29 | C4 / T5 | Instalar binario/bundle público en máquina limpia | Identidad, manifest, SBOM y checksum del tag verificados. |
| T30 | C4 / T5 | Reabrir estado real de versión previa y reconstruir proyecciones | Compatibilidad/replay conforme al contrato declarado. |
| T31 | C4 / T5 | Comparar source SHA, tag, recibo y binary/bundle hashes | Coinciden; sin copiar evidencia entre versiones. |
| T32 | C4 / T5 | Fallo forzado de un gate obligatorio | No se emite certificación, aunque otros gates sean verdes. |
| T33 | C4 / T5 | Ausencia de binario EXT en perfil enhanced | BLOCKED/NOT_RUN; Base puede permanecer válido. |
| T34 | C5 / T4 | Segundo host real implementa contratos mínimos | AG4 validado antes de API estable, sin semántica JCode filtrada. |
| T35 | C5 / T2 | Corpus Jev etiquetado + baseline reproducible | Mejora demostrada o experimento descartado; sin score infundado. |

**Criterio de cierre:** no marcar un hito VERIFIED por contar tests. Vincular cada fila aplicable a un resultado ejecutado, al SHA exacto y a un recibo verificable. Un fallo requiere incidencia y corrección RED→GREEN; un blocker debe tener owner y revisit trigger. Los gates de [CERTIFICATIONS.md](CERTIFICATIONS.md) deciden las promociones.
