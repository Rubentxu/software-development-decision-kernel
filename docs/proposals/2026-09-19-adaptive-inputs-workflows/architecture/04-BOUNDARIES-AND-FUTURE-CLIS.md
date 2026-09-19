# Fronteras externas y futura división de CLI

## CogniCode — A6

Mantener el `CodeIntelligencePort` SDDK-owned de CC-S0. A6 posterior exige adapter real/negociación en runtime, resultado con basis completa, normalización específica a `observation::SoftwareObservation` **no** al `ObservationSet` homónimo y textual del spike, integración real Verify/AC10 y Base intacto cuando falte proveedor. Provider SDK, AST y grafo pesado nunca contaminan Knowledge/Verification. Revisar `DigestSha256::of`: el nombre promete SHA-256 pero usa FNV-1a, no apto para identidad durable/CAS/recibo de seguridad. Alinear con digest canónico ya usado en SDDK, sin doble identidad nueva.

## Chronos — A7

Port runtime propio de SDDK según arch-spec-021 y capabilities efectivamente negociadas. Escenario, ejecutable, build, env, instrumentation, sampling/trace completeness y ventana temporal son parte de basis. Static/runtime pueden contradecirse: conservar ambas observaciones y su contexto, sin promediar a score ni atribuir causalidad a correlación. Si A7 no tiene proveedor real, Base continúa sin fabricar evidencias RUNTIME_ENHANCED.

## JCode — J2..J6

Reutilizar primero el contrato genérico de `arch-spec-030` (API/SDK transporte-neutral, invocación on-demand). Anti-corruption SEC-1 protege secretos, recibos y denegaciones: un host no puede ver args secretos/raw unredacted ni importar almacenamiento privado. J2 propone trait+view específicos de JCode: demostrar qué falta en el API genérico antes de añadirlos. Una ruta host→gateway y otra CLI→gateway **comparten** las mismas políticas/captura. Probar segundo host antes de prometer API 1.0, según AIS-008.

## TypeSafe/Jev — NO dependencia Base

El modelo es posible clasificador/reranker de *alternativas semánticas admisibles*, no productor de hechos, calculadora de EIG, autorizador ni scheduler. Hacer spike offline sobre corpus de routing ya etiquetado: baseline deterministic+retrieval vs LLM habitual vs Jev; medir exactitud/calibración/abstención/p95 coste/latencia/sensibilidad a contexto adversarial. Sin mejora robusta sobre baseline o sin input suficiente, no abrir adaptador. Si se admite, opción bajo un puerto existente de decisión/selección (no otro store); registrar model version, input hash, alternativas, salida, validez y no invocarlo en replay como si fuese determinista.

## ¿Varios CLI?

Crates actuales: `sddk-domain`, `sddk-engine`, `sddk-gateway`, `sddk-storage`, `sddk-vault`, `sddk-cli`, `sddk-testkit`, `sddk-pack-uat`. `sddk-cli/src/main.rs` delega en `sddk_cli::run_from`, y `lib.rs` concentra composición y muchos handlers. **No dividir por nombre ni por volumen del CLI**: primero mover únicamente lógica que necesita segundo consumidor probado a un caso de uso/puerto de su bounded context, conservando presentación CLI separada. Posibles candidatos si un segundo consumidor lo pide: `architecture_cmd::build_context` compartido con cápsulas/Verify; `knowledge_ingest` solo después de demostrar integración útil; runner compartido entre host y shell.

R11 es el lugar para evaluar, no adelantar, `sddk-dev`/`sddk-inspect` u otros binarios. Criterios objetivos: independencia de release/deps/privilegios, menor blast radius, segundo consumidor, compatibilidad, overhead, transacciones concurrentes y no duplicar state stores. Un sub-CLI **no** invoca otro sub-CLI por stdout como API canónica. Si se separa por proceso, usar frontera semántica y misma autoridad/persistencia, con lock/lease/idempotencia probados. `sddk` mantiene comandos compatibles o deprecación explícita; no crear daemon obligatorio ni cinco composition roots divergentes.

## R11 spike de falsificación

Crear segundo `main` experimental *sin shipping* que invoque un mismo caso de uso de lectura, comparando JSON semántico, exit codes y policy snapshot con `sddk`. Simular escritura concurrente por dos procesos y upgrade de un binario viejo; bloquear split si se pierde idempotencia, seguridad, migraciones o se requiere copiar lógica de engine/storage al CLI.
