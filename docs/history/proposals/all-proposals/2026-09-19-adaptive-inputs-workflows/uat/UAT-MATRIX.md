# Matriz UAT falsable — trazabilidad por hito

**Ningún test se ha ejecutado al generar este paquete.** «PASS» solo puede afirmarse con comandos ejecutados en checkout limpio/entorno documentado, logs/JSON/receipt y revisión de tests que *podían fallar*. Separar tests unitarios (UT), integración (IT), end-to-end real (E2E), negativas (NEG), durabilidad (DUR), adversariales (SEC) y proveedores externos (EXT). Fake valida port; nunca sustituye EXT.

## A6 / datos / verificación

| ID | Clase | Entrada/operación | Resultado requerido / falsificación |
|---|---|---|---|
| A01 | EXT+E2E | CogniCode real negociado sobre fixture SDDK; una relación que anuncie | Observación `SoftwareObservation` con basis completa y refs; Verify de claim específica cambia por evidencia real. Sin el proveedor se mantiene el resultado Base, no enhanced. |
| A02 | EXT | Cambiar una relación en revisión B respecto a A | Nueva observación/basis distinta; no reutilizar A como evidencia vigente para B. Repetir A con same semantic input produce identidad estable SOLO bajo contrato de reproductibilidad probado. |
| A03 | NEG | `Null`, OFF, timeout, cancelación, incompatible, partial | Estados diferenciados; `Unknown/NotEvaluated` o fallo explícito para REQUIRED, ningún falso PASS. |
| A04 | SEC | Proveedor devuelve dato fuera del scope o input adversarial | Rechazar/quarentena, no importar SDK/heavy AST ni instrucción del proveedor como autoridad. |
| A05 | IT | `verify_kernel_cmd` ruta real | `ObservationSet` NO vacío con dato válido; verificar qué path consume exactamente las observaciones, no una prueba privada que llama VerifyKernel directamente. |
| A06 | IT | `DigestSha256` de CC-S0 con mismo/diferente input | Ref durable de A6 usa SHA-256 canónico válido de 64 hex; FNV provisional nunca se registra como `sha256:`. Compatibilidad de spike identificada. |
| A07 | NEG | observación missing scope/producer revision o result truncated | Artefacto bruto permitido si política lo deja, pero no evidencia factual verificadora. |
| A08 | IT | base Git dirty/rename/delete/changed contract locator | Sin coincidencia injustificada por string; path overlap ≠ callgraph. Cambios de basis invalidan. |
| A09 | REG | Base sin CogniCode | Batería Base existente pasa sin instalación, arranque ni migración obligatoria del proveedor. |
| A10 | DUR+COND | Dos observaciones contradictorias con fuentes/basis distintas | Sobreviven reboot con identidades y dos relaciones; no `latest wins`, no score agregado. Solo si S1b admitida. |
| A11 | NEG+COND | relation desconocida, ref inexistente, hash inválido | Error fail-closed, writer no deja adjunto falso ni CAS huérfano. |
| A12 | DUR+COND | lectura de adjuntos legacy anteriores a migración | Sin pérdida/interpretación alterada ni actualización destructiva; compatibilidad documentada. |
| A13 | CONC+COND | escritores concurrentes de la misma observación | At most one canonical relation para clave idempotente, sin huérfanos CAS en UNIQUE collision, replay coherente. |

## Shell / runner / tests

| ID | Clase | Entrada/operación | Resultado requerido / falsificación |
|---|---|---|---|
| T01 | E2E | Agente ejecuta test por ruta autorizada | SDDK registra salida/exit/attempt/basis; el informe LLM es presentación, no hecho. |
| T02 | NEG | exit=1 con texto `passed` | No PASS ni gate verde. |
| T03 | NEG | exit=0 pero suite no cubre claim | Resultado de comando exitoso; Verify sigue no verificado. |
| T04 | NEG | timeout, cancel, spawn failure, assertion fail, infra fail | Clases distintas; un error de infraestructura no cuenta como test failed ni passed. |
| T05 | NEG | stdout/stderr/report truncado | `incomplete` propagado; si testcase necesario faltante, Unknown. |
| T06 | SEC | secretos canary en args/stdout/stderr/error/denied | No se fugan en host view, diagnostic prefix, receipts, logs o artefactos CAS accesibles; cero efectos en deny. |
| T07 | IT | ejecutar test dos veces con mismos inputs | Dos intentos físicos enlazados; ninguna dedup hace desaparecer segunda ejecución, manteniendo idempotency de la solicitud cuando proceda. |
| T08 | IT | test report nativo de familia seleccionada | Normalización específica por testcase/suite con versión fijada, sin afirmar que RunOutcome breve ya lo ofrece. |
| T09 | NEG | CLI externo ejecutado fuera de gateway | No atribuir a SDDK recibo de ejecución inexistente; agente puede aportar Contribution separada. |
| T10 | REG | herramienta inexistente/no instalada/allowlist denegada | Fallo tipado; no cambiar estado canónico salvo registro permitido; sin invocar shell arbitrario por fallback. |

## Agenda/Secretary/handoff

| ID | Clase | Entrada/operación | Resultado requerido / falsificación |
|---|---|---|---|
| G01 | IT | snapshot Planning reconciliado, A bloquea B | Agenda indica candidato/causa y refs; NO autorización de ejecución por `project_next`. |
| G02 | NEG | múltiples Active, dependency ID inexistente, spine no reconciliado | Mostrar conflicto/desconocido; nunca «READY» falso ni ocultar asimetría entre `next` y `blocked`. |
| G03 | REG | mismo snapshot semántico consultado 2 veces | JSON/texto dan mismas razones semánticas, timestamp de presentación puede variar. |
| G04 | IT | Secretary L0 reacciona a evidencia relevante | Propuesta/consulta observable en ruta productiva, no solo test de módulo; sin añadir WorkItem automático. |
| G05 | SEC | Secretary intenta release/gate/lease/receipt fuera closed-set | Denegación antes de efecto; Orchestrator no eleva la autoridad de Secretary. |
| G06 | IT | evento irrelevante/duplicado | No se lanza análisis pesado ni se crea asunto duplicado. |
| G07 | NEG | autoridad de leer documento/capability no otorgada | No se prepara ejecuta «trabajo adelantado» fuera de permisos. |
| G08 | PERF | consultas read-only sobre fixture grande | Baseline de tiempo/CPU antes/después; no implementar incrementalidad si no hay mejora justificada. |
| G09 | IT | salida advisory MISALIGNED o hallazgo heurístico | No se convierte automáticamente en deny ni crea un nuevo WorkItem. |
| H01 | DUR+E2E | dos tareas estáticas con handoff tras cerrar proceso | Segunda tarea recupera cápsula desde storage real sin transcript ni InMemoryStore. |
| H02 | NEG | ref de evidencia obligatoria ausente/stale | Handoff incompleto y causa explícita; no continuar fingiendo evidencia. |
| H03 | IT | output parcial y comentario del agente «completado» | Runtime/Verify no convierten Partial en Done/Pass. |
| H04 | REG | varias revisiones del workflow apuntan a mismo trabajo | Refs WorkItem→PlanRevision→Node→Attempt→Evidence estables, sin duplicar tareas. |
| H05 | SEC | propuesta de delegación supera capability/permiso | Denegación gobernada, sin efectos ni filtración de args. |
| H06 | IT | disenso de riesgo alto y evidencia obligatoria | Synthesis conserva disposición+refs, no omite silenciosamente. |
| H07 | PERF | mismo caso antes/después ContextCompiler | Registrar tokens y tamaño de cápsula, tiempo y missing required refs; NO inventar reducción de tokens. |
| H08 | REG | cambiar proveedor/agente del segundo encargo | Mismo contrato semántico y policy; no dependencia de transcript del agente anterior. |
| H09 | NEG | CAPSULE compile encuentra dos bases incompatibles | Declarar conflicto de bases o reconstrucción; no presentar snapshot falso. |

## Expansión, externos y empaquetado

| ID | Clase | Entrada/operación | Resultado requerido / falsificación |
|---|---|---|---|
| W01 | E2E | evidencia real+claim gap produce propuesta | Orchestrator decide, Authority admite, compiler+validator añaden Task ejecutable, versión padre+hija persistente. |
| W02 | NEG | trigger duplicado, proposal reenvío | Una expansión y sin doble efecto. |
| W03 | DUR | reinicio entre proposal/commit y tras commit | Recuperación determina exactamente una revisión adoptada; completed nodes preservados. |
| W04 | NEG | permiso denegado/budget agotado/capability ausente | No mutación IR ni ejecución; motivo trazable. |
| W05 | NEG | basis stale, parent revision cambiado, conflicto worktree | Rechazo/rebase autorizado; no apply parcial. |
| W06 | IT | operador no construido `Loop/Wait/Join/...` | Rechazo explícito, no ejecutar variante declarada pero no soportada. |
| W07 | IT | `Choice` guard sin fallback en fallo | El test constata la semántica REAL; no etiquetar Selector erroneamente. |
| W08 | REG | workflow SDD estático | Sigue funcionando sin selección dinámica y con mismos gates. |
| W09 | NEG | evento del proveedor con prompt injection | Se trata como input no fiable; ninguna acción de autoridad por texto del evento. |
| W10 | CONC | dos propuestas simultáneas con mismo parent | Control de concurrencia sobre versión y no pérdida/duplicación de cambios. |
| W11 | PERF | trigger irrelevante | 0 llamadas cognitivas y 0 nuevas tareas si reglas lo resuelven. |
| P05 | EXT | Chronos real escenario A/B comparable | Datos con env/instrumentation/complete, observación no presume causalidad. |
| P06 | NEG | traza parcial, escenario no comparable o instrumento incompatible | No se afirma verificación de claim. |
| P07 | NEG | Chronos ausente | Base conserva comportamiento; requerido explícito falla su operación. |
| P08 | SEC | trace con tokens/secretos | No exponer raw sin política/permiso; refs y retención correctos. |
| P09 | E2E | CogniCode y Chronos acuerdan | Dos fuentes y bases visibles, correlación descrita sin convertir en autoridad. |
| P10 | E2E | CogniCode y Chronos se contradicen | Ambas se mantienen y Verify refleja conflicto/Unknown cuando proceda. |
| P11 | NEG | dependencia de proveedor requiere instalación obligatoria para Base | Fallo UAT; prohibido.
| P12 | REG | reconstrucción de proyección combinada | Misma base produce mismos campos y refs, sin store combinado canónico.
| X01 | IT | mismo caso de uso desde CLI/host | Resultado semántico/Authority equivalentes, variantes de presentación distintas aceptadas. |
| X02 | SEC | host accede a raw/denied args/secretos | Debe ser imposible por API estable o denegarse; 0 leak. |
| X03 | REG | host offline/no daemon | Base sigue operativo. |
| X04 | CONC | dos CLIs contra mismo storage con update | No doble autoridad ni state divergence. |
| X05 | IT | experimento segundo binario read-only | No duplica engine/storage/policy y mantiene salida semántica. |
| X06 | REG | un CLI antiguo consulta esquema nuevo | Compatibilidad o fallo explícito documentado, nunca interpretación falsa. |
| X07 | ARCH | ninguna lógica de dominio en nuevo `main` | Prueba de dependencia/fitness + segundo consumidor real. |
| X08 | DEC | Jev mejora baseline en corpus y es optional | Si no mejora o falta contrato estable, no integración; replay no vuelve a invocar modelo. |
