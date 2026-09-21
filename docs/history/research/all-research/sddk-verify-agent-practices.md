# Prácticas para agentes de verificación como quality gate

**Fecha de consulta:** 2026-08-21
**Alcance:** agentes y prompts que verifican cambios de software antes de aceptarlos.
**Criterio de fuentes:** documentación oficial, estándares y publicaciones de sus autores. Se excluyeron agregadores y recomendaciones sin evidencia primaria.

## Resumen ejecutivo

Un agente verificador fiable no debe actuar como un revisor que "opina" que el cambio parece correcto. Debe operar como un orquestador de evidencia: convertir criterios de aceptación en comprobaciones observables, ejecutar controles deterministas sobre un estado identificable del repositorio, buscar implementaciones aparentes y reservar el juicio LLM para propiedades que el código no pueda decidir. El veredicto debe fallar de forma cerrada cuando falte evidencia obligatoria.

La documentación actual de OpenAI mantiene estas prácticas de evaluación, pero anuncia la retirada de su plataforma Evals el 30 de noviembre de 2026. Por tanto, conviene adoptar los principios y no acoplar el gate a esa API concreta [S1].

## Claims verificados

| ID | Claim concreto | Evidencia primaria | Traducción al gate |
|---|---|---|---|
| C1 | Una evaluación robusta empieza por objetivo, dataset representativo, métricas y evaluación continua; las métricas automatizadas deben calibrarse con juicio humano. | OpenAI [S1] | No verificar sin criterios y umbrales previos; conservar casos reales y regresiones. |
| C2 | En agentes hay que distinguir tarea, trial, grader, traza y estado final. La variabilidad exige varios trials y los graders deterministas son preferibles cuando bastan. | Anthropic [S4] | Verificar el estado producido, no solo la explicación final; repetir las evals del propio agente. |
| C3 | Un benchmark de código puede dar falsos positivos y negativos aunque ejecute tests. OpenAI encontró prompts incompletos, tests demasiado estrictos, cobertura insuficiente y prompts contradictorios en aproximadamente el 30% de SWE-Bench Pro. | OpenAI [S3] | Auditar también la especificación y los tests; un test verde no demuestra por sí solo que el gate sea válido. |
| C4 | Los graders LLM pueden sufrir sesgos y reward/grader hacking. OpenAI recomienda compararlos con etiquetas expertas; Anthropic observó indulgencia en la autoevaluación y obtuvo mejor control separando implementador y evaluador. | OpenAI [S2], Anthropic [S5] | Separar roles y validar periódicamente el juez con ejemplos humanos de PASS y FAIL. |
| C5 | Los tests deben ser correctos, útiles y capaces de fallar cuando el código está roto. El review también debe cubrir diseño, funcionalidad, complejidad, concurrencia y mantenibilidad. | Google [S6] | Exigir controles negativos y una revisión de calidad productiva independiente del resultado de tests. |
| C6 | Los dobles de prueba pueden divergir de producción. Google recomienda preferir implementaciones reales cuando sean viables y complementar los dobles con pruebas de mayor alcance y fidelidad. | Google [S7] | No aceptar una feature demostrada solo con mocks, stubs o fakes no contrastados. |
| C7 | La cobertura indica que una línea fue alcanzada, no que su comportamiento fue comprobado. Mutation testing introduce defectos deliberados y revela tests que siguen pasando. | cargo-mutants [S9] | Usar mutación sobre lógica nueva o crítica para detectar aserciones vacuas e implementaciones triviales. |
| C8 | Los TODO y otros marcadores en comentarios requieren una búsqueda deliberada: el análisis semántico de lenguajes suele ignorarlos; Semgrep ofrece modo genérico o regex de fichero. | Semgrep [S8] | Combinar búsqueda textual de marcadores con reglas AST/semánticas del stack. |
| C9 | La seguridad no aparece automáticamente por pasar tests funcionales. SSDF integra prácticas de seguridad en el SDLC y ASVS 5.0.0 ofrece requisitos de verificación versionados y consumibles programáticamente. | NIST [S10], OWASP [S11] | Seleccionar controles de seguridad por perfil de riesgo y registrar los IDs/versiones comprobados. |
| C10 | La evidencia sin identidad e integridad no basta. SLSA verifica firma, digest del artefacto, builder, fuente y parámetros frente a expectativas conocidas. | SLSA [S12] | Ligar cada evidencia al commit/diff, entorno, comando y artefacto exactos. |
| C11 | SOLID es un conjunto de principios de cambio y dependencias, no una puntuación universal. Sus fuentes enfatizan razones de cambio, sustitución, interfaces por cliente y dependencias hacia abstracciones; Google advierte además contra el sobrediseño. | Robert C. Martin [S13], Google [S6] | Evaluar violaciones concretas y su impacto, no exigir patrones o capas de forma mecánica. |

## Gates recomendados

1. **Contrato verificable antes de ejecutar.** Construir una matriz `requisito -> escenario -> oráculo -> evidencia`. Emitir `INCONCLUSIVE`, nunca `PASS`, si un requisito obligatorio es ambiguo, contradice los tests o no tiene observación verificable. Comprobar que una solución de referencia o el estado esperado puede satisfacer el oráculo.

2. **Evidencia fresca y reproducible.** Para cada comando registrar `commit`, digest del diff si el árbol está sucio, CWD, versiones relevantes, comando exacto, timestamp, código de salida y ruta/hash del log. Rechazar resultados heredados, resumidos sin salida verificable o producidos sobre otro estado.

3. **Oráculos deterministas primero.** Priorizar compilación, tests, schemas, invariantes, consultas de estado, linters y analizadores. Usar un grader LLM solo para propiedades semánticas como adecuación del diseño, legibilidad o cobertura de intención; nunca permitir que reinterprete como éxito un comando fallido.

4. **Estado final sobre narrativa y trayectoria sobre afirmaciones.** Verificar archivos, binarios, API, base de datos o UI resultantes. Conservar la traza de herramientas para detectar comandos no ejecutados, rutas equivocadas o evidencia fabricada, pero no imponer una única trayectoria cuando varias soluciones sean válidas.

5. **Tests de cambio, regresión y controles negativos.** Ejecutar tanto pruebas que pasan de rojo a verde como la suite preexistente. Exigir evidencia de que los tests nuevos fallan contra el comportamiento defectuoso o una mutación equivalente; revisar que no codifiquen detalles de implementación ausentes del criterio de aceptación.

6. **Evaluar la estabilidad del propio agente.** Mantener suites separadas de capacidad y regresión, alimentadas con fallos reales, casos límite y adversariales. Ejecutar varios trials en entorno limpio; para un gate usar una métrica de consistencia tipo `pass^k`, no `pass@k`, porque no basta con acertar una vez entre varios intentos.

7. **Separar implementador, verificador y calibración.** El agente que produjo el cambio no debe ser la única autoridad que lo aprueba. Calibrar el grader con un conjunto retenido y etiquetado por expertos, midiendo por separado detección de fallos y reconocimiento de pases; escalar desacuerdos o baja confianza a revisión humana.

8. **Gate explícito contra stubs y placeholders.** Escanear el diff de producción por marcadores (`TODO`, `FIXME`, `XXX`, `HACK`), primitivas (`todo!`, `unimplemented!`, `NotImplemented`, cuerpos vacíos), retornos constantes sospechosos, handlers que siempre responden éxito y wiring de fakes fuera de tests. Cada hallazgo debe bloquear o tener una exención localizada, justificada y trazable; la búsqueda textual sola no constituye prueba de defecto.

9. **Fidelidad y fuerza de los tests.** Aplicar mutation testing al código nuevo o de alto riesgo y bloquear mutantes supervivientes no justificados. Si se usan mocks, stubs o fakes, exigir al menos una prueba de contrato o integración contra la implementación real para los límites modificados.

10. **Perfil de preparación productiva.** Además de build/test, seleccionar por riesgo controles obligatorios de formato/lint, errores y recuperación, concurrencia, seguridad, rendimiento, observabilidad, migraciones, compatibilidad y empaquetado. Para aplicaciones web, referenciar requisitos ASVS con versión, por ejemplo `v5.0.0-1.2.5`; `N/A` necesita razón verificable.

11. **Diseño mantenible con evidencia, no dogma.** Revisar si el cambio introduce múltiples razones de cambio, dependencias que atraviesan límites hacia detalles, interfaces más amplias de lo que usan sus clientes, sustituciones que rompen contratos, ciclos o abstracciones especulativas. Bloquear solo violaciones concretas que degraden comprensión, cambio local o salud del código; no asignar un "SOLID score" ni exigir una interfaz por cada tipo.

12. **Veredicto no compensatorio.** Emitir solo `PASS`, `FAIL` o `INCONCLUSIVE`, con cada gate obligatorio visible. Un promedio alto no puede ocultar un fallo de seguridad, regresión, criterio de aceptación o evidencia. `PASS` requiere cero fallos obligatorios y cero evidencias pendientes; `INCONCLUSIVE` debe enumerar exactamente qué falta para decidir.

## Contrato mínimo de salida del verificador

```yaml
verdict: PASS | FAIL | INCONCLUSIVE
subject:
  commit: <sha>
  diff_digest: <sha256-or-null>
gates:
  - id: acceptance|regression|anti-placeholder|production|design
    status: PASS|FAIL|INCONCLUSIVE|N/A
    claim: <afirmación comprobada>
    evidence:
      command: <comando exacto o null>
      exit_code: <numero o null>
      artifact: <ruta o URL>
      digest: <sha256 o null>
    findings: []
unverified: []
```

## Fuentes primarias

- **[S1] OpenAI, Evaluation best practices.** Objetivos, datasets, métricas, evaluación continua y calibración humana. https://developers.openai.com/api/docs/guides/evaluation-best-practices
- **[S2] OpenAI, Graders.** Tipos de grader, multigrader, grader hacking y calibración. https://developers.openai.com/api/docs/guides/graders
- **[S3] OpenAI, Separating signal from noise in coding evaluations (2026-07-08).** Auditoría de prompts y tests rotos en SWE-Bench Pro. https://openai.com/index/separating-signal-from-noise-coding-evaluations/
- **[S4] Anthropic, Demystifying evals for AI agents (2026-01-09).** Trials, trazas, outcomes, graders múltiples y entornos aislados. https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents
- **[S5] Anthropic, Harness design for long-running application development (2026-03-24).** Separación entre generador y evaluador y criterios graduables. https://www.anthropic.com/engineering/harness-design-long-running-apps
- **[S6] Google Engineering Practices, What to look for in a code review.** Diseño, funcionalidad, complejidad, tests y salud del código. https://google.github.io/eng-practices/review/reviewer/looking-for.html
- **[S7] Google, Software Engineering at Google, Test Doubles.** Fidelidad, riesgos de mocks/stubs/fakes y pruebas de mayor alcance. https://abseil.io/resources/swe-book/html/ch13.html
- **[S8] Semgrep, Match comments with Semgrep.** Detección de TODO y comentarios mediante modo genérico o regex. https://semgrep.dev/docs/kb/rules/match-comments
- **[S9] cargo-mutants, documentación oficial.** Mutation testing para comprobar que los tests detectan cambios de comportamiento. https://docs.rs/crate/cargo-mutants/latest
- **[S10] NIST SP 800-218, SSDF 1.1 (final).** Prácticas de seguridad integradas en el ciclo de desarrollo. https://csrc.nist.gov/pubs/sp/800/218/final
- **[S11] OWASP ASVS 5.0.0.** Requisitos versionados para verificar controles de seguridad web. https://owasp.org/www-project-application-security-verification-standard/
- **[S12] SLSA v1.2, Verifying artifacts.** Identidad, firma, digest, fuente y parámetros frente a expectativas. https://slsa.dev/spec/v1.2/verifying-artifacts
- **[S13] Robert C. Martin, Design Principles and Design Patterns.** OCP, LSP, DIP e ISP como principios de dependencias y cambio. https://objectmentor.com/resources/articles/Principles_and_Patterns.pdf

Todas las URLs fueron consultadas el **2026-08-21**.
