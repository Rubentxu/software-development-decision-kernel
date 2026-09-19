# EnvLens — huella de entorno y comparabilidad de ejecuciones

**Estado:** propuesta de prototipo Rust, posible CLI/librería separada. **Responsabilidad única:** capturar **solo el entorno relevante para una observación** y determinar qué diferencias afectan a su comparabilidad según reglas explícitas. No instala toolchains, despliega infraestructura ni garantiza reproducibilidad absoluta.

## 1. Problema

Un test pasa en portátil y falla en CI; un benchmark cambia tras modificar una imagen; una observación de Chronos procede de otra instrumentación. Comparar stdout sin identificar la base observacional conduce a diagnósticos inválidos. EnvLens responde: **qué se observó, qué se declaró y qué falta para comparar**, preservando \`unknown\` ante datos no disponibles.

## 2. Qué existe y qué no replicar

- [mise](https://mise.jdx.dev/dev-tools/mise-lock.html) resuelve versiones concretas de herramientas mediante \`mise.lock\`; [Nix](https://nixos.org/) y gestores de contenedores construyen otros niveles de reproducibilidad. No duplicar esa función.
- Los sistemas CI, Kubernetes y OCI ofrecen metadatos de ejecución, recursos y digest de imágenes; no asumir que su presencia prueba identidad de entorno.
- [OpenTelemetry CI/CD](https://opentelemetry.io/docs/specs/semconv/cicd/cicd-spans/) define convenciones de observabilidad de runs; pueden correlacionar trazas, no sustituir una base de evidencia admitida.

**Diferenciación a validar:** fingerprint **reducido y justificado por el experimento**, no una instantánea del sistema completo ni un gestor de configuración.

## 3. Reglas de captura

Entrada: \`purpose\` (test, benchmark, escenario remoto, ejecución de agente), lista explícita de dimensiones que influyen en el resultado y origen confiable de cada dimensión. Ejemplos: SHA/dirty state, compilador y flags, lockfiles, arquitectura OS/CPU, digest de imagen, flags de instrumentación, configuración permitida, versión de servicio remoto y clase de recursos. No incluir automáticamente HOME, credenciales, endpoints privados o entorno completo.

Salida propuesta: dimensiones declaradas vs observadas, productor, fecha de observación, scope, valor normalizado/redactado, completitud, digest canónico por **semántica y algoritmo versionados**; informe diff que distinga \`same\`, \`different\`, \`unknown\`, \`incomparable\`. \`same\` en un subconjunto no implica entorno idéntico.

**Dos fingerprints diferentes:** el de *condiciones configuradas* puede permanecer estable; la identidad del *intento ejecutado* cambia aunque las condiciones sean iguales. No derivar identidad de ejecución de un hash de entradas.

## 4. Arquitectura Rust y estrategia incremental

\`clap\` CLI opcional, captura mediante lectores puros + adaptadores de sistema/CI, normalizador y comparador puro con fixtures. Un registro de dimensiones se mantiene inicialmente **dentro del CLI**, con tipos explícitos para campos sensibles, dinámicos y relevantes. No crear un catálogo universal de factores de reproducibilidad ni exigir integración con todos los CI.

MVP: \`capture\` desde proyecto Rust/mise y desde un runner CI de fixture; \`compare\` con reglas explícitas de equivalencia y salida JSON/texto. Integrar ScenarioLab como productor delegado de huella solo si el contrato se demuestra estable, sin duplicar observación de runtime de Chronos.

## 5. Casos UAT

1. Dos capturas idénticas bajo dimensiones definidas producen misma huella semántica aunque tengan distinto timestamp.
2. Cambio de compilador/flags o digest de imagen aparece como diferencia relevante; aumento de hora de presentación no altera la huella.
3. Falta una dimensión marcada obligatoria: \`incomparable\`, jamás \`same\`.
4. Entorno remoto no accesible y volumen parcial: informe parcial sin fabricar valores ni utilizar variables del host como sustitutos.
5. Un canary de secreto introducido en env/config/log no aparece en JSON, stderr, archivos ni digest reversible; no registrar nombres de variables prohibidas.
6. Comparar Linux y Windows para un benchmark que depende del SO exige regla explícita, no equivalencia por tener mismo hash de código.
7. Ejecuciones repetidas con la misma huella conservan attempt IDs distintos y métricas independientes.
8. Desactivar un colector no cambia reglas de autoridad ni «aprueba» otros resultados.

**Criterio de descarte:** si consumir metadatos del runner actual + \`mise.lock\` y un comparador puro satisfacen los dos consumidores iniciales, construir solo esa librería; no lanzar CLI propio.

## 6. Relación con SDDK

\`ObservationBasis\` y los receipts existentes reciben refs de fingerprint cuando la claim depende del entorno. EnvLens no escribe \`KnowledgeBasis\` ni crea un segundo \`ContextCompiler\`. El usuario/agente recibe la explicación de por qué resultados no son comparables; el kernel conserva el juicio de verificación y la autoridad.
