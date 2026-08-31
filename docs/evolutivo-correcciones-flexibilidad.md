---
type: deferred-backlog
status: captured
captured_by_cycle: p-63676b11dc0ef88f/phase-c-test-boundary-cleanup
captured_at: 2026-08-31T19:00:00Z
principle: "Fail closed para seguridad; recover forward para proceso."
future_scope: "Incorporación futura al framework como operación de ciclo (supersede), modo replan-in-place, gates de proceso autorreparables, validación de ubicación de artefactos, y separación de ciclo vs hipótesis de diseño."
not_in_scope: "Este documento NO es requisito, ADR, ni código entregado por el ciclo phase-c-test-boundary-cleanup. Es backlog diferido para decisión futura."
---

# Ideas diferidas de recuperación del framework

> **Principio rector:** Fail closed para seguridad; recover forward para proceso.

Sí. Los problemas que vimos revelan una diferencia clave:
Un framework debe bloquear cambios peligrosos, no bloquear el aprendizaje ni la entrega.
Insights concretos para SDDK:
1. Añadir cycle supersede como operación de primera clase
  Debe cerrar o congelar el ciclo anterior con motivo, enlazarlo al nuevo ciclo y conservar evidencia. Sin edición manual del ledger ni ciclos competidores.
2. Distinguir gates de seguridad de gates de proceso
  - Seguridad: secretos, regresiones, integridad Git, datos. Bloquean.
  - Proceso: fase desalineada, artefacto desubicado, planificación obsoleta. Deben abrir una ruta de recuperación automática.
3. Añadir un modo "replan in place"
  Si verify descubre que el alcance es equivocado, el ciclo debe poder volver formalmente a propose/spec/design/tasks, con una razón y una relación de supersesión. Hoy el framework detecta bien la contradicción, pero no ofrece salida.
4. Convertir los bloqueos en acciones, no diagnósticos
  Un bloqueo debe devolver siempre una única recuperación ejecutable:
  open/verify + scope invalid
  → replan-in-place
  → genera nuevo plan y preserva el anterior como superseded
5. Validar la ubicación de artefactos al escribirlos
  Los agentes no deberían poder crear v2/vault/... dentro del repo. El escritor de artefactos debe resolver siempre la ruta XDG y rechazar rutas del workspace.
6. Separar "ciclo" de "hipótesis de diseño"
  El ciclo representa el objetivo de producto; una decisión mínima fallida es sólo una hipótesis descartada. No debería obligar a abandonar el objetivo ni a abrir otro ciclo.
7. Presupuestar complejidad contra valor
  El framework debe preguntar: "¿este gate reduce riesgo real o sólo impide avanzar?". Si no protege una invariante observable, debe ser advisory o autorreparable.
La regla rectora sería:
Fail closed para seguridad; recover forward para proceso.
