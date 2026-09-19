# EffectCheck — comprobación de efectos reales de acciones y automatizaciones

**Estado:** candidato de capacidad transversal, **NO abrir un CLI nuevo en primera fase**. Implementar un caso vertical sobre el Gateway/receipts/Verification de SDDK; extraer binario Rust autónomo solo cuando un workflow externo lo necesite sin SDDK.

## 1. El problema y la innovación

\`exit 0\` demuestra únicamente que un proceso terminó con ese código. No demuestra que un despliegue está sirviendo la imagen correcta, que una migración aplicó el esquema esperado, que una acción externa fue idempotente o que una configuración llegó a la instancia real.

**Pensamiento lateral:** el resultado de una automatización se obtiene con dos testigos independientes: \`execution receipt\` (**acción invocada y resultado operativo**) y \`effect observation\` (**estado externo comprobado bajo scope y ventana temporal**). Una discrepancia genera una necesidad de investigación, no autorización para reintentar automáticamente una operación potencialmente no idempotente.

## 2. Ecosistema y fronteras

- Los sistemas CI/CD y el Gateway de SDDK ya registran ejecutores, intents y receipts; no escribir otro runner o Authority Engine.
- [Conftest](https://www.conftest.dev/) evalúa políticas sobre datos estructurados; es útil para «el manifiesto declarado cumple», pero no necesariamente «el servicio desplegado cumple».
- Los probes de HTTP, Kubernetes, SQL y cloud CLIs dan estados concretos; [OpenTelemetry CI/CD](https://opentelemetry.io/docs/specs/semconv/cicd/) ayuda a correlacionar runs, pero las trazas no reemplazan checks de estado o autoridad.
- Chronos aporta observaciones temporales cuando hay un escenario comparable; no es el scheduler de EffectCheck.

## 3. Propuesta de caso mínimo

Después de una operación autorizada \`apply\` sobre un servicio de **entorno de pruebas controlado**, ejecutar una observación read-only que verifique (a) identidad del artefacto servido, (b) readiness dentro de un deadline, (c) condición observable del contrato, con referencias al mismo WorkItem/run/attempt. La comprobación solo puede comenzar si la operación obtuvo una admisión válida y la política permite observación del destino.

En un despliegue real puede existir consistencia eventual: definir ventana y criterio de estabilización. \`unknown\` tras timeout es distinto de \`not matched\` en una lectura válida. Una comprobación externa debe registrar los permisos utilizados y el momento observado, sin otorgar privilegios adicionales.

## 4. Futura interfaz autónoma, si se justifica

Especificación declarativa de una **pregunta acotada** (recurso/atributo esperado/probe/timeout/identidad de ejecución), no motor de scripting universal. Conmutadores \`plan\`, \`verify\` y \`explain\` hipotéticos; ejecución via adaptadores tipados read-only y sin shell arbitrario en las expresiones de condición. El CLI independiente devolvería observación con compleción y refs; la autoridad de hacer rollback, retry o continuar pertenece al consumidor.

## 5. UAT negativo/positivo

1. Operación retorna exit 0 pero digest desplegado es distinto → efecto no demostrado.
2. Operación retorna exit distinto de 0 pero estado externo parece correcto → evidencia contradictoria, no reinterpretar la ejecución como éxito ni reintentar ciegamente.
3. Condición tarda en converger: durante ventana se informa pending; al final prueba por observación fresca o conserva timeout.
4. Destino equivocado/namespace sin permiso no puede convertirse en PASS.
5. Dos aplicaciones con igual nombre en namespaces distintos no se cruzan.
6. Observación temporalmente anterior a la operación no verifica su efecto posterior.
7. Reinicio entre operación y comprobación: recuperar refs sin duplicar el efecto; si el intento está en estado incierto, consultar antes de reintentar.
8. Resultado redirigido/falsificado por una CLI externa sin fuente verificable queda inconcluso.
9. El mismo contrato funciona en un consumidor CI independiente: gate indispensable para promover a CLI propio.

## 6. Encaje y descarte

Reutilizar \`CapabilityReceipt\`, \`VerificationClaim\`, \`SoftwareObservation\`, refs de CAS y run/attempt de SDDK según los contratos vigentes: no proponer \`EffectCheckRecord\` o tabla nueva. Si el conjunto de probes existente del pipeline resuelve las mismas preguntas sin trabajo manual ni pérdida de evidencia, registrar patrón de integración y no extraer nada.

El resultado puede ayudar a Secretary a abrir una pregunta de continuación, pero **no** crear WorkItems o ejecutar compensaciones por defecto. Orchestrator decide y Authority media efectos.
