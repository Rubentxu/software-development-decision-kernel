# Métricas, experimentos opcionales e ideas laterales útiles

## Antes/después medible (mismo fixture, scope y calidad)

| Señal | Medida/base | Éxito/fallo a observar |
|---|---|---|
| Adopción real | productor→normalizador→writer→consumidor CLI/host real; nº ejecuciones | Salida real usada por Verify/agenda; `pub trait` sin invocador = no entregado. |
| Calidad epistémica | claims con evidence valid/partial/unknown por scope, falsa aprobación | 0 PASS fabricados; Unknown explícito cuando no hay evidencia o cobertura. |
| Fiabilidad | UAT negativos, replay, duplicados, ref missing/stale | 0 efectos duplicados y 0 mandatory evidence refs perdidas. |
| Eficiencia de tokens | tokens por tarea igual con prompts/baseline controlados | Medir diferencia; no asumir ahorro porque hay más CLI. |
| Latencia/coste | tiempo de preflight, consulta, run, normalización y fin de tarea, p50/p95 | Automatización opcional debe aportar valor neto; detener scan caro sin consumidor. |
| Complejidad | owner stores/AGG/ADTs/puertos añadidos, aliases eliminados, wiring usado | Justificación explícita; nuevos modelos solo con UAT imposible de otro modo. |
| Privacidad | pruebas canary stdout/stderr/denial/CAS/host | 0 secreto expuesto o persistido sin permiso. |
| CLI futuros | segundo consumidor, costo de build/install/version drift | Posponer split si aumenta compat o duplica composición/authority. |

## Spike 1: Tool-first decision routing

Sin añadir tabla, usar colección de tareas históricas anonimizadas/fixtures con etiqueta de mejor herramienta/skill obtenida de contrato experto y outcomes cuando sean comparables. Probar baseline: reglas+capability filtering → recuperación semántica → LLM existente → Jev/TypeSafe **opcional**. No medir «agente terminó» como certeza de mejor elección contrafactual; randomizar/ensayar A/B solo en tareas sin riesgos y con presupuesto. Métricas top-k, tasa de abstención, falsos positivos, calibración de clases etiquetadas, coste p95. Jev produce evaluación consultiva; no truth, rights, EIG ni scheduling. Si la clasificación determinista resuelve la mayoría, dejar Jev fuera.

## Spike 2: Detección de información que merece producirse

«Demand-driven evidence»: cada claim o decisión tiene requisitos explícitos. Comparar tiempo/coste de capturar **solo evidencias faltantes** frente a escanear todo el proyecto. El Secretary consulta outputs reales para construir agenda; producer selection y scheduling se basan primero en reglas/capability/allowlist y presupuesto. No convertir cada observación en WorkItem. Fallo medible: análisis ejecutados sin consumidor o que no cambian contexto/decisión.

## Spike 3: Diff de decisiones antes/después de herramienta

Una propuesta de Secretary puede incluir refs antes/después de ejecutar un experimento; contrastar la incertidumbre **de las alternativas** sin confundir probabilidad de `Choice` con veracidad de hechos. Comparar observable: decisiones cambiadas, investigaciones abortadas por no aportar evidencia y omisiones detectadas. No crear posterior bayesiano universal ni auto-cerrar por entropía baja.

## Spike 4: Proyección de atención sin nuevo grafo persistente

Componer GraphStore/Planning/Verify/WHY por refs, visualizar qué observación desbloquea qué WorkItem y por qué se propone una rama. La visualización es derivada, por eso reconstruible. Solo considerar materializar checkpoint si el coste de reconstrucción medido exige algo más que consulta pura.

## Spike 5: Normalización adaptativa incremental

Escoger UNA familia de reporte nativo (versión fijada). Parsear con herramienta de la propia familia si ofrece JSON estable; si no, escribir un adaptador estrecho que preserve errores/partial. Antes de generalizar a seis families, demostrar que un segundo consumidor necesita el mismo resultado y que el primer parser puede componerse sin introducir un «universal test AST».

## Regla final de descarte

Un spike que no aporta beneficio medible, no identifica un consumidor, exige duplicar autoridad/estado o solo produce dashboards sin modificar el comportamiento de un caso de uso NO se promociona a feature. Reutilizar o retirar documentación exploratoria, pero conservar evidencia de por qué se descartó.
