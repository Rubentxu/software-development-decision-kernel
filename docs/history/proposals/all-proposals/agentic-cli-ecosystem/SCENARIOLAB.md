# ScenarioLab — ejecución y verificación de escenarios de integración

**Estado:** propuesta exploratoria de CLI Rust autónomo. **Responsabilidad única:** materializar, ejecutar y limpiar **un escenario de prueba de integración identificable** para un SUT, local, remoto o híbrido. No es un scheduler, túnel, runner de tests universal ni fuente de autoridad.

## 1. Necesidad y valor diferencial

Un agente modifica un microservicio que depende de otro stack en Kubernetes/OpenShift. Puede compilarlo localmente, pero no reproducir su interacción con servicios, configuración y datos reales sin desplegar toda la plataforma. ScenarioLab describe **qué constituye el experimento y qué condiciones lo hacen válido**: SUT y revisión, dependencias consumidas, preparación, rutas de tráfico, datos de prueba, aislamiento, probes, expectativas, teardown y condiciones de comparación.

**Pensamiento lateral:** tratar las dependencias remotas como *fixtures prestados* con alcance, exclusividad, tiempo de vida y prohibición de efectos no autorizados. No fingir un «sandbox» cuando se comparte una base de datos real.

## 2. Ecosistema: construir sobre, no reemplazar

- [mirrord](https://metalbear.co/mirrord/docs/using-mirrord/steal) permite mirror/steal de tráfico y filtros; mirror puede duplicar procesamiento/escrituras si ambos lados actúan.
- [Telepresence](https://telepresence.io/docs/howtos/attach) ofrece replace, intercept, wiretap e ingest; intercept puede filtrar tráfico y no equivale a un aislamiento completo. El acceso al cluster, permisos, instalación de agentes y daemons auxiliares dependen de su modalidad.
- [DevSpace](https://www.devspace.sh/docs/configuration/dev/) ya ofrece desarrollo remoto, sync y port-forward; [Testcontainers Rust](https://github.com/testcontainers/testcontainers-rs) monta dependencias locales en tests; Docker Compose y Helm resuelven otros alcances.
- [Dagger](https://docs.dagger.io/getting-started/introduction/) ofrece composición de tareas en contenedores; ScenarioLab no debe convertirse en un duplicado suyo.

**Hipótesis de diferenciación que hay que falsificar:** una ejecución unificada y auditable de *intención de escenario + lease de recursos + condiciones de comparación + cleanup verificado* aporta valor que los scripts y herramientas anteriores no entregan cómodamente juntos.

## 3. Propuesta de flujo

\`\`\`text
scenario.yaml → validar contexto, permisos y scope
              → resolver adaptador local/remote
              → preflight: SUT, versiones, RBAC, endpoints, test data
              → reservar únicamente recursos necesarios (lease)
              → preparar túnel/namespace/red/dependencias según modo
              → arrancar SUT y realizar probes de readiness
              → invocar runner de test existente / Chronos opcional
              → observar resultado + estado del escenario + completeness
              → limpiar recursos de propiedad conocida y verificar cleanup
              → reporte JSON + artefactos referenciados
\`\`\`

El plan de acciones mutantes requiere autoridad explícita y revisión previa; el modo \`inspect/plan\` debe ser read-only. Un escenario no puede interceptar globalmente el tráfico compartido por defecto. Las estrategias de enrutamiento, si se usan, necesitan identificador de sesión, aislamiento de peticiones y garantías de que no se duplican efectos externos.

## 4. Contrato candidato (borrador, no schema implementado)

Entrada: \`scenario_id\`, \`sut_revision\`, comando de arranque/localización del SUT, dependencias nombradas, modo \`local|remote-hybrid\`, ámbito/namespace autorizado, política de tráfico, dataset fixture, probes esperados, límites y referencias de secretos (nunca valores serializados). La entrada puede ser un YAML local específico de ScenarioLab; no es un nuevo modelo canónico de SDDK.

Salida: \`scenario_id\`, \`attempt_id\`, hash del manifiesto, base del código, fingerprint de entorno (puede producirlo EnvLens), recursos prestados y creados, modalidad de tráfico, herramientas invocadas/versiones, resultados de probes, refs de logs/trazas/tests, \`complete|partial|failed|cancelled\`, cleanup \`confirmed|partial|unknown\`. El digest de inputs no identifica por sí solo dos experimentos con efectos externos como iguales.

## 5. Arquitectura Rust emergente

CLI con \`clap\`; caso de uso \`plan/execute/inspect/cleanup\`; puertos estrechos \`EnvironmentAccess\`, \`ScenarioRunner\`, \`TrafficAttachment\`, \`Probe\` y \`ArtifactSink\` **solo cuando exista segunda implementación o contrato testable**; adapters iniciales para entorno local + un proveedor Kubernetes elegido en spike. Separar planificación pura de efectos. Las operaciones de cleanup poseen identificadores de recursos creados por el intento; no borrar recursos ajenos aunque compartan nombre. Tests de puertos con Fake antes de una prueba real de cluster.

## 6. MVP / UAT de falsificación

1. SUT HTTP local con una dependencia mock/controlada: dos ejecuciones sobre el mismo fixture producen resultados de probes reproducibles, con attempt distinto.
2. Modo remoto autorizado: SUT local accede a servicio remoto permitido; conexión errónea deja resultado de preparación fallida, no «test PASS».
3. Mirror con operación de escritura: rechazar la modalidad o exigir aislamiento demostrado; cero escritura duplicada inducida por el laboratorio.
4. Dos agentes paralelos: sesiones no capturan tráfico ni recursos del otro; conflicto de lease produce bloqueo tipado.
5. Desconexión y SIGTERM: liberar solo recursos propios; cleanup desconocido visible y procedimiento de reconciliación.
6. RBAC insuficiente, namespace equivocado, ausencia de herramienta, timeout y dato fixture ausente: fallan cerrado.
7. Reinicio de proceso: el manifiesto y los receipts de preparación/cleanup permiten diagnosticar recursos huérfanos sin relato LLM.
8. Cambiar revisión/entorno invalida equivalencia de resultados; la observación no demuestra causas sin investigación.

**Salida de decisión del spike:** construir CLI independiente si los tests 1–7 muestran valor adicional frente a un script de mirrord/Telepresence + runner; si no, publicar patrones de integración y descartar el nuevo binario.

## 7. Consumo por SDDK y CI

SDDK emite encargo autorizado y captura \`attempt\`, \`scenario\`, bases y referencias en contratos existentes; Chronos puede observar el mismo intento; CogniCode aporta relaciones estáticas, no decide aislamiento. Un pipeline convencional puede invocar ScenarioLab por shell y utilizar exit code operativo más el JSON detallado; una claim de verificación exige revisar scope, resultados y completitud. No acoplarlo a un framework agéntico concreto.
