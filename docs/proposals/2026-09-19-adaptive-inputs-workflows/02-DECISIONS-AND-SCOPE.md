# Decisiones consolidadas y control de alcance

## D1 — Producir antes que interpretar

Una necesidad informativa recorre en orden lógico: consultar hechos existentes → activar productor determinista específico → proveedor semántico directo → delegación cognitiva para ambigüedad residual → decisión del Orchestrator. **No es una obligación de hacer cinco llamadas**. En tareas creativas se puede delegar directamente. Cada efecto, incluso observar con acceso a recursos protegidos, pasa por Authority ANTES de la ejecución.

## D2 — Misma semántica, distintas vistas

Agenda = vista de Planning + Verification + Knowledge + Execution; no tabla/aggregate Agenda. «Árbol de decisión» = vista de trabajos, alternativas, decisiones y evidencia vinculada; no grafo canónico nuevo. «Behaviour Tree» = composición declarativa de operadores existentes, con semántica demostrada por runtime; no segundo motor de scheduler ni enum espejo. No crear WorkGoal/WorkAssignment/WorkHandoff/WorkSession/KnowledgeProducer o global `Insight` por estética.

## D3 — Fact vs Object vs Projection vs Ephemeral

Los productores capturan hechos/observaciones atribuibles. CAS conserva cuerpos admitidos y el ledger conserva identidad, relación y procedencia mediante escritor canónico. Un projector reconstruye vistas; ninguna vista escribe autoridad. `KnowledgeBasis` no obtiene inputs por sí mismo. Las afirmaciones inferidas conservan su clase y sus supuestos; reportar «no encontrado en scope» no equivale a «no existe».

## D4 — Agentes proponen; herramientas reportan resultados propios

Un agente elige comandos y estrategias si la tarea lo requiere, pero un `exit=0`/timeout/artefacto pertenece al runner/recorder, nunca a una frase del agente. Una contribución interpretativa conserva referencias verificables, incertidumbre y dissent. `ExecutionOutcome` documental no se da por implementado si faltan contratos reales. No obligar al registro de cada comando exploratorio: capturar los necesarios para decisión/gate/aceptación.

## D5 — Progresar el roadmap vigente, no multiplicar planes

A6 productor estático real→observación→Verify sigue línea prioritaria; captura de tests/gates es habilitador independiente admitido solo si consumidor claro. A7/runtime, A8/combinación, J2/host y R11/separación conservan autoridad y dependencias. No abrir etapa E0–E5 nueva. No reabrir A5-C, SEC-1, histórico M0–M9 ni convertir propuestas en receipts.

## D6 — Condiciones para añadir cualquier tipo, tabla, puerto o CLI

1. Identificar pregunta del consumidor, productor real y UAT que falla sin el cambio.
2. Mostrar por qué `WorkItemV1`, `DecisionRecordV1`, Evidence/Observation, ContextCapsule, WorkflowIR, AgentContributionEnvelope, gateway o proyecciones **no** cubren el caso.
3. Justificar identidad/ciclo de vida/invariantes propios frente a una operación, adaptación o proyección.
4. Especificar owner, autoridad, migración, compatibilidad, recuperación, observabilidad, soporte Base y código antiguo eliminado.
5. Aceptar la forma mínima; si no hay evidencia de necesidad, diferir. Una evolución puntual de persistencia de contradicciones puede estar justificada si las relaciones actuales son insuficientes; no implica licencia para crear un segundo Knowledge Store.

## D7 — Rechazos expresos

- No daemon obligatorio, watcher continuo o polling de todo el repositorio.
- No «Knowledge Scan = Truth» ni importación/autoridad automática del Vault.
- No LLM entre `CodeIntelligencePort` y CogniCode.
- No reutilización de FNV bajo etiqueta SHA-256 ni almacenamiento de `ObservationSet` entero como único blob opaco.
- No prometer `Selector` con fallback-on-failure usando `Choice` actual si solo selecciona por guard; no simular `Loop/Wait` con código privilegiado generado por LLM.
- No CAS de stdout sin redacción, retención ni tratamiento de truncado/secretos.
- No auto-creación de un WorkItem por cada finding, comando, evento o clasificación.
- No TypeSafe/Jev en aceptación de evidencias, permisos, verificación, scheduling o ruta crítica Base.
