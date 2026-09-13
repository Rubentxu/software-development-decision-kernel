# Context ownership checklist

Antes de crear/mover un tipo preguntar:

1. ¿Cuál es su ubiquitous language?
2. ¿Quién cambia su lifecycle?
3. ¿Es canonical o projection?
4. ¿Quién tiene autoridad para modificarlo?
5. ¿Es advice o policy?
6. ¿Necesita provider-specific knowledge? Si sí, probablemente está en adapter.
7. ¿Puede representarse como view sobre un owner existente?
8. ¿Qué contexto debe depender de cuál?
9. ¿Qué fichero actual reemplaza/consolida?
10. ¿Qué UAT demuestra la frontera?
