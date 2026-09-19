# Escenario ejecutable por hitos: fallo intermitente, handoff y nueva comprobación

**Especificación de escenario, NO resultado de UAT.** Fixturizar un proyecto de prueba con una relación estática controlable A/B, contrato arquitectónico declarado, test conocido y un cambio que produce una necesidad adicional de verificación. El proveedor CogniCode debe confirmar que soporta una relación demostrable; si no, elegir otra relación, no inventar análisis de impacto.

## Primera etapa — sin LLM para capturar hechos

1. Usuario: «investiga y corrige fallo intermitente sin romper contrato X»; SDDK abre/sigue WorkItem existente de la tarea, con scope y policy.
2. Planning snapshot produce candidato y dependencias; `architecture_cmd` obtiene paths+base y solapamiento con locator X. Eso solo identifica **contratos declarados asociados al cambio**.
3. Orchestrator admite investigación. Un agente configura la reproducción y elige test; el gateway lo ejecuta y conserva resultado. Caso negativo: imprime `100 tests passed` pero exit=1; Verify sigue sin PASS.
4. Si la claim requiere estructura y A6 real está disponible, SDDK invoca `CodeIntelligencePort` directamente; adapter produce `SoftwareObservation` con base. Verify compara con requisito sin atribuir autoridad a proveedor.

## Segunda etapa — handoff

5. Agente investigador devuelve `AgentContributionEnvelope` con hipótesis, evidence refs, riesgos, alternativas y scope; el attempt y receipts contienen resultado factual. La síntesis registra disposition/dissent.
6. Se cierra proceso; otro proceso carga store y recompila ContextCapsule de siguiente encargo, con accepted decision, evidence refs válidos, artifact outputs y pending acceptance. El agente implementador no lee todo el transcript.

## Tercera etapa — expansión mínima

7. Tras un cambio, CogniCode observa una relación relevante y Verify devuelve «falta evidencia de test T». L0 detecta el gap; L1 propone Task T si está dentro de closed-set; L2 o un agente solo prepara alternativas si el caso excede reglas disponibles.
8. Orchestrator decide ejecutar T; Authority verifica los efectos y permisos de la capacidad. Compiler valida delta y crea revisión hija conservando los nodos completados. No usar `Loop/Wait/Join` si la ruta real los rechaza.
9. Test T produce receipt factual; Verification determina outcome según scope. Secretary actualiza su vista sin crear una Agenda DB. Si sigue Unknown, propone nueva investigación o escalado; ningún PASS de silencio.

## Condiciones de contraste

- **Baseline estático:** mismo test y mismo gate bajo workflow SDD existente, sin expansión ni proveedor enhanced. Debe continuar funcionando.
- **Provider off:** enhanced no se anuncia; Base conserva sus resultados legítimos.
- **Replay:** mismo trigger no añade segundo Task ni repite un side effect completado; nuevo attempt físico conserva su propia identidad si se ordena de nuevo.
- **Antiadopción decorativa:** implementar comando nuevo sin cambiar el resultado de Verify o próximo encargo NO satisface el escenario.
- **Scope:** fixture A/B, comandos exactos y salida UAT se fijan en el ciclo real; no se fabrican en este documento.
