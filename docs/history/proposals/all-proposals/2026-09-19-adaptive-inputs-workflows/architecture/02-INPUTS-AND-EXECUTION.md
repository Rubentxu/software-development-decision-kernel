# Producción de inputs: autoejecución, proveedor directo y CLI delegada

## Regla: input concreto → consumidor concreto

Toda propuesta de productor especifica: (1) pregunta consumidora; (2) fuente y comando/capability real; (3) formato de salida/contrato y alcance; (4) revision+dirty+config+provider/version+entorno+completitud; (5) clase observado/declarado/inferido; (6) writer canónico o salida efímera; (7) UAT positivo y falsificación. No introducir un registro universal `KnowledgeProducer` ni un enum `Insight` para rellenar huecos sin consumidor.

## Procesos internos, sin daemon requerido

| Trigger existente | Operación interna deseada | Prioridad, límite y destino |
|---|---|---|
| Entrada a operación sensible a contexto | Resolver identidad project/worktree/revision+dirty/config/policy y `ledger head` | Bajo coste, obligatorio para la operación; refs de basis, no una nueva tabla `InputSnapshot`. |
| `plan`/consulta de atención | `project_next`, `project_blocked` sobre snapshot identificado | On-demand, puro, candidato de planning; reportar unresolved refs/multiple Active; ningún efecto. |
| Gate/capability gobernada | Gateway runner captura exit, señal, timeout, duración, stdout/stderr redacted, output ref y receipt | **Automático en el mismo límite de ejecución**. Si falla registro obligatorio, resultado no verificable; no inventar PASS. |
| Revisión/ejecución con diff necesario | Git changed paths + ChangeBasis + contratos/locators | Bajo demanda; hook Git opcional para invalidación, jamás fuente exclusiva. No callgraph. |
| Entrada a `explore` con corpus documental relevante | `knowledge scan` + verify de cambios; consultar registry | Scan opt-in por política de ese target, delimitado; no `import` automático ni ascenso del Vault a autoridad. |
| Nueva observación/claim relevante | Normalización → Verify → advisory/WHY existentes | Ejecutar solo la evaluación que tenga claim consumidora. El projector no decide governance. |
| Nuevo event/lectura de proyección | Comprobar versión y reconstruir si obsoleta | Empezar full rebuild puro; selectividad/incrementalidad solo si coste medido y bases de invalidación completas. |
| Delegación/reanudación | `ContextCompiler` engine + `CapsuleInputs`, run state, decisiones/evidence refs | Sin transcript, tamaño acotado, refs obligatorias válidas; dos ContextCompiler homónimos: elegir el del engine, no crear tercero. |
| Bloqueo/resultado inesperado | Secretary L0/L1 aplica reglas existentes; L2 propone | Ningún nuevo WorkItem automático por cada alerta; decisión/presupuesto antes de investigaciones pesadas. |

**No promover «captura en cada transición» a reconstrucción global:** solo invalidar lo afectado y garantizar frescura al consumidor que la requiera. El `post-commit` no captura estado dirty posterior ni cubre hooks ausentes; fallback on-demand.

## Proveedor directo, sin agente intermediario

CogniCode entra por `CodeIntelligencePort` (CC-S0 tiene fake/Null; A6 necesita adaptador REAL + contrato de transporte), Chronos por un puerto semántico de runtime en A7. El Orchestrator o el agente pueden proponer **la pregunta**, pero SDDK ejecuta y recibe el resultado por puerto. Autoridad de lectura, compatibilidad negociada, timeout/cancel/partial, bases y proveedor identificados. Datos pesados AST/index/trace siguen en proveedor; SDDK conserva refs y evidencias pequeñas. Nada de prometer AC10 con Fake.

## Agente LLM ejecuta CLI especializada mediante shell

| Naturaleza del encargo | Ejemplos orientativos | Agente | Captura SDDK |
|---|---|---|---|
| Tests/corrección | cargo/nextest, pytest, Jest, Maven, Gradle, Go | Selecciona suite, contexto de fallo y parámetros | Runner/receipts registran salida estructurada; normalizador **uno por familia requerida**, sin inventar parser genérico. |
| Diagnóstico exploratorio | Git, rg, LSP CLI, perf, strace si está permitido | Formula búsquedas/hipótesis e interpreta | Capturar solo ejecuciones que sustentan decisión/gate; operaciones triviales no generan una evidencia por comando. |
| Cobertura/bench/security | llvm-cov, JaCoCo, hyperfine, Criterion, cargo-audit, Semgrep | Diseña experimento y umbrales aceptables | Conserva configuración, entorno, artefactos y resultado nativo; sin convertir hallazgos heurísticos en prohibición. |
| Diseño/código/artefactos | compiladores, formatters, git, builders | Diseña e implementa | Authority para efectos, refs de cambios y resultados verificables. |
| Interpretación/innovación | agente con herramientas y evidence refs | Aporta alternativas, motivos y límites | `AgentContributionEnvelope`/síntesis con disposition, dissent y refs; no toma salida narrada como `ExecutionOutcome`. |

### Mismo camino de captura para CLI y runtime

```text
Agent/Orchestrator -> proposal/capability -> Authority (antes de observar si la observación tiene efectos)
 -> Gateway/RunSpec argv estructurado, env allowlist, timeout, CWD autorizados
 -> RunOutcome + salida íntegra donde sea necesaria y esté autorizada
 -> redacción de secretos y clasificación de completitud
 -> CAS/receipts vía writer canónico cuando el contrato lo exija
 -> normalizador específico (si existe), con basis y relación a una claim
 -> Verify/Knowledge/Planning por sus fronteras respectivas
 -> presentación JSON para agente y texto humano desde el MISMO resultado.
```

Un wrapper CLI como `sddk dev run-and-record` es **candidato de UX** sobre el Gateway existente, no segundo runner ni permiso para `sh -c` arbitrario. Escoger subcomando/nombre definitivo después de revisar collision `dev`, `run`, `capability` y command registry. CLI externo directo fuera del gateway NO se afirma automáticamente registrado. Reutilizar redacción SEC-1 y evitar fuga en args/errores/diagnostic prefix: no asumir que una salida redacted puede mostrar 64 bytes «verbatim» si contienen secretos.

**Intentos e idempotencia:** cada ejecución física conserva identidad de intento; la deduplicación de una *operación solicitada* y el hash de un resultado repetible son claves distintas. Repetir un test no implica falsamente «no se ejecutó». `exit 0` ≠ claim PASS sin contrato+scope+testcase; stdout truncado/incomplete ≠ ausencia de fallo. Nunca almacenar salida cruda sensible sin política de redacción/retención, límite, acceso y validación.

## Selección contextual con mínima computación

Producir por demanda, no indexar todo el proyecto al abrir sesión. Orden: estado disponible → consulta barata → productor enfocado sobre paths/claims → proveedor enhanced si aporta al consumidor → agente LLM para interpretación residual. En cada fan-out: `required` vs `optional` y presupuesto de CPU, memoria, IO, tiempo/tokens; `optional` indisponible causa gap visible, `required` falla la operación sin degradar a PASS. Datos inmutables pueden cachearse solo sobre basis completa; verificaciones temporales y del entorno no se reutilizan por hash del código únicamente.
