# ContractProbe — matriz verificable de compatibilidad entre versiones

**Estado:** spike de CLI candidato. **Responsabilidad única:** correlacionar evidencias de compatibilidad por **par exacto consumidor/proveedor, contrato, versión y entorno** cuando existen herramientas heterogéneas. No reimplementar Pact Broker, Buf ni fuzzers de API.

## 1. Necesidad real

Un cambio en OpenAPI, Protobuf, esquema de eventos o contrato de un servicio puede afectar a consumidores distintos y a versiones desplegadas en varios entornos. «El esquema compila» no demuestra compatibilidad de las implementaciones ni que se verificó el par de versiones de producción.

**Pensamiento lateral:** construir una matriz con «celdas de evidencia» que distingan declaración, comprobación estática y ejecución contractual. Las celdas desconocidas son huecos de investigación, no aprobaciones por omisión.

## 2. Alternativas investigadas

- [Pact Broker y can-i-deploy](https://docs.pact.io/pact_broker/can_i_deploy) ya ofrecen la matriz de verificaciones de consumidor/proveedor y comprueban versiones desplegadas. **Si el caso de uso es exclusivamente Pact, utilizar Pact directamente; descartar ContractProbe.**
- [Buf breaking](https://buf.build/docs/breaking/) detecta rupturas bajo reglas configurables entre versiones Protobuf: resultado declarativo, no garantía de comportamiento de un servicio.
- [Schemathesis](https://schemathesis.readthedocs.io/en/stable/quick-start/) genera pruebas basadas en OpenAPI/GraphQL y puede detectar incumplimientos de runtime. No sustituirlo.
- OpenAPI diff y analizadores de esquemas/eventos pueden sumarse en otro spike **solo si existen contratos reales**.

## 3. MVP acotado

Entrada: un servicio proveedor, un consumidor, dos versiones explícitas, contrato identificado (hash y fuente), entorno de destino y resultados existentes de **Pact o Buf**, sin ejecutar automáticamente una batería completa. Construir matriz \`declaration_checked\` / \`consumer_provider_verified\` / \`unknown\` con refs de evidencia y vigencia. La «capa de correlación» solo merece binario independiente si al menos dos tecnologías/servicios aportan valor imposible de obtener directamente con el broker.

Las pruebas que sí requieran ejecución se delegan a las herramientas nativas; el CLI puede prepararlas y recoger resultados, pero no convertirse en su motor.

## 4. Semántica de resultados

\`compatible_for_checked_contract\` requiere versión/scope/resultados pinados; \`incompatible\` exige prueba de ruptura dentro del alcance; \`not_evaluated\` cubre faltas de contrato, versión, resultado o acceso al broker. Una verificación de contrato no implica que todo el sistema o todos los efectos de negocio sean correctos.

Los pares \`consumer@sha × provider@sha × environment\` no deben colapsarse a un score agregado ni a \`latest\` sin identidad reproducible. El servicio desplegado se obtiene de fuente de deployment verificable, no del último tag de registry por aproximación.

## 5. Arquitectura Rust

CLI \`matrix/inspect\` y, solo si hace falta, \`verify\`; adaptadores separados para cada formato nativo; comparador puro y presentación JSON/texto. Broker/registry se consulta por puertos con autenticación explícita y permisos mínimos, sin replicar toda su base de datos. La autoridad de «puede desplegar» pertenece al pipeline/política consumidora, no a ContractProbe.

## 6. UAT y regla de descarte

1. Pact muestra compatibilidad solo en una pareja concreta; otra pareja sin prueba queda \`not_evaluated\`.
2. Cambio Protobuf que rompe un campo detectado por Buf se registra con base y regla; un check estático verde no constituye prueba de runtime.
3. Contrato HTTP válido pero proveedor viola respuesta: el resultado dinámico contradictorio se conserva, no se promedia.
4. Versión de consumidor no desplegada no se sustituye por \`latest\`.
5. Broker inaccesible/resultado parcial/timeout mantiene desconocido; no emite verde.
6. Dos ejecuciones contra las mismas identidades producen matriz semánticamente igual con distintos intentos.
7. Segundo formato real cambia una consulta que Pact solo no responde; **si no ocurre, no crear CLI autónomo**.
8. Consumo por Jenkins, SDDK y un script simple produce los mismos pares/refs, sin dependencia de agente.

## 7. Encaje con SDDK

CogniCode localiza dependencias posibles, no verifica compatibilidad. ContractProbe entrega observaciones/ref de tests; SDDK decide qué claims cubren, prepara trabajos pendientes y aplica gates/autoridad. Un resultado \`can-i-deploy\` no sustituye la política corporativa, la aprobación humana ni las demás verificaciones.
