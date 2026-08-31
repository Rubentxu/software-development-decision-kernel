# Protocolo de checkpoint de sesión

Qué hace `book-memory-keeper` al cerrar una sesión (o al alcanzar un punto de parada natural). Garantiza que la próxima sesión pueda continuar sin pérdida.

## Checkpoint obligatorio (siempre, al cerrar)

1. **LEDGER.md**: actualizar estado de cada capítulo, `blocked_on`, `cycle`, `next_action`.
2. **SESSION-LOG.md**: reescribir con:
   - Qué hicimos esta sesión.
   - Dónde quedamos exactamente.
   - Qué hacer la próxima vez (lista accionable).
3. **Engram `mem_session_summary`**: resumen ejecutivo (Goal/Discoveries/Accomplished/Next Steps/Relevant Files).
4. **Engram `mem_save`** type=decision: una observación por decisión clave tomada en la sesión.
5. **Engram upsert** de voz/glosario si cambiaron (topic_key estable: `voice-{libro}`, `glossary-{libro}`).

## Checkpoint por macro-fase (al cerrar A/R/B/C/D)

Además del obligatorio:

| Macro-fase | Extra a persistir |
|------------|-------------------|
| A | `book-config.yml` final + voice-profile + code-map |
| R | `corpus.yml` + snapshot + gaps |
| B | capítulo completado marcado, code cards actualizadas |
| C | `verify-report.jsonl` + revisión consolidada |
| D | `manifest.json` + CHANGELOG de la edición |

## Recuperación garantizada
Al iniciar la siguiente sesión, `recall` devuelve:
- Proyecto detectado (`mem_current_project`).
- Últimas observaciones (`mem_context`).
- LEDGER + SESSION-LOG leídos.
- Síntesis: "continúa en X, bloqueado por Y, voz Z".

## Anti-patrón
Cerrrar la sesión tras un trabajo largo sin checkpoint. Si la sesión se interrumpe bruscamente, el último checkpoint (máximo una macro-fase atrás) es lo que se recupera. Por eso el checkpoint es **por macro-fase**, no solo al final de sesión.

## Frecuencia mínima
Aunque la sesión sea corta: siempre al cerrar, aunque solo sea para escribir "no avanzamos mucho, seguimos en el mismo punto". Un SESSION-LOG vacío es mejor que ninguno.
