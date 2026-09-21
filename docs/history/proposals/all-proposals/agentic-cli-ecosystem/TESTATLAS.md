# TestAtlas — selección justificable y cobertura evidencial de pruebas

**Estado:** candidato condicionado. **Primero** finalizar el pipeline de captura/normalización de resultados de test en SDDK; solo extraer CLI al demostrar un consumidor multi-lenguaje o multi-repositorio independiente.

## 1. Problema y tesis diferenciadora

Un agente sabe ejecutar un test; no siempre sabe **cuál ejecutar por un cambio**, si el test se ejecutó sobre el código relevante o si su éxito satisface una claim. TestAtlas combinaría unidades cambiadas, relaciones estáticas, topología de tests, alcance de ejecución y criterios de aceptación. No decide que «todos los demás tests sobran».

**Pensamiento lateral:** separar \`candidate_test\` (relación inferida con cambio) de \`executed_test\` (hecho de un intento) y \`claim_verified\` (veredicto de Verification con evidencia suficiente). Tres preguntas y tres niveles epistémicos, no un número universal de cobertura.

## 2. Competencia existente

- [cargo-affected](https://github.com/max-sixty/cargo-affected) selecciona tests Rust a partir de cobertura por función y diff, pero documenta estado experimental y posibles falsos negativos en entradas externas, proc macros o cambios fuera de las fuentes instrumentadas. No sustituye la suite completa en CI.
- [cargo-nextest](https://nexte.st/) ejecuta y reporta tests Rust; pytest, Jest, Maven/Gradle y Go ya tienen ecosistemas específicos.
- [pytest-testmon](https://testmon.org/) aborda selección según dependencias/cobertura en Python.
- CogniCode puede aportar relaciones estáticas disponibles a través de su puerto **solo cuando el proveedor real las demuestre**. No asumir cobertura runtime a partir de un call graph.

La oportunidad de TestAtlas existe si aporta **normalización y trazabilidad entre cambios, tests ejecutados y claims, sobre más de una herramienta**. No reimplementar un runner ni seis parsers a la vez.

## 3. Propuesta de MVP

1. Input: revisión/diff y dirty state; tests disponibles desde un runner existente; relaciones directas explícitas o registradas entre tests y unidades; resultado estructurado por testcase de **una** familia.
2. Selector conservador: incluir tests vinculados al cambio y los obligatorios por política; registrar por qué se incluyó/excluyó cada test. Ante relaciones incompletas, seleccionar suite de fallback según contrato, no declarar «ningún test afectado».
3. Runner ejecuta fuera de TestAtlas (shell del agente/SDDK gateway o CI); el resultado se ingiere con run/attempt/revisión/env/completitud.
4. Proyección: tests recomendados, realmente ejecutados, resultados observados y gaps de claims **sin emitir veredicto de Verify por cuenta propia**.

Una futura estrategia multi-repo puede incorporar Pact/ContractProbe para contratos cruzados, pero no crear otra Knowledge DB.

## 4. Arquitectura / salida

Componer puertos existentes de SDDK cuando sea consumidor SDDK; si se justifica un binario independiente, proporcionar entradas locales JSON/git y un almacén de correlaciones reconstruible con scope explícito. No copiar la planificación ni los resultados de ejecución como autoridad paralela.

Salida candidata: \`selection_basis\`, \`candidate_tests[]\` con razón/relation refs, \`required_fallback\`, \`executions[]\` con outcome y completeness, \`uncovered_claim_refs[]\` (solo claims suministradas), \`producer_versions\` y \`unknowns[]\`. Es un DTO del CLI, no una propuesta de agregado SDDK.

## 5. UAT de aceptación y refutación

1. Cambio en función instrumentada selecciona test relacionado; cambio ajeno no inventa relación.
2. Cambio en migration/fixture/build.rs o test sin cobertura conocida activa fallback, **no** conjunto vacío presentado como seguridad.
3. Test con exit=0 sin relación registrada con la claim permanece \`not verified\`.
4. Assertion fallida, error de arranque, timeout y reporte truncado conservan estados diferenciados.
5. Un resultado perteneciente a otro commit, otro scope o un intento previo no verifica automáticamente el cambio actual.
6. Relación derivada solo de heurística es \`candidate\`, no hecho de cobertura observado.
7. Dos repositorios/lenguajes ofrecen una pregunta que el selector nativo individual no responde: **gate para extraer el CLI**.
8. Comparar tiempo/tokens y falsos negativos con suite completa de referencia; si no mejora coste manteniendo calidad, rechazar selección automática.

## 6. Encaje con SDDK, CogniCode, Chronos

Sujeto a [evolutivo previo de inputs](../2026-09-19-adaptive-inputs-workflows/README.md): primero un resultado de tests verdadero capturado por Gateway y normalizado en Verification. CogniCode enriquece candidatos, Chronos relaciona escenarios ejecutados; ambos son opcionales. Secretary propone comprobaciones ausentes, Orchestrator decide; Authority admite la ejecución. TestAtlas nunca convierte un test omitido en PASS ni reemplaza al Verification Kernel.
