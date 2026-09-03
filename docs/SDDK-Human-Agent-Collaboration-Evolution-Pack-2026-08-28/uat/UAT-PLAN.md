# UAT Plan — Human-Agent Collaboration

## Participantes
- novice developer;
- experienced developer;
- framework maintainer.

## Método
Cada participante ejecuta escenarios sin leer internals. Después de cada output responde:
1. ¿En qué fase está?
2. ¿Qué acaba de ocurrir?
3. ¿Qué viene después?
4. ¿Necesitas hacer algo?

## Gate global
>=90% respuestas correctas.

## Dimensiones
orientation, clarity, interruption friction, trust, resume, persona usefulness, memory correctness.

## Hard fails
- usuario cree success cuando hay blocker;
- required action no visible;
- persona cambia significado;
- resume inventa estado;
- high-risk action continúa sin approval;
- memory no se puede corregir/eliminar.

## Dogfood
>=20 ciclos reales antes de stable.
