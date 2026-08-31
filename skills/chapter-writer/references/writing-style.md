# Estilo editorial — castellano de España

Aplicado por `chapter-writer` y validado por `editorial-reviewer` (Vale).

## Voz
- Segunda persona del plural ("veis", "comprobáis") o impersonal ("se observa").
- Tuteo informal solo si el `audience-profile` lo indica.

## Terminología inglesa
- Mantener en inglés los términos sin traducción aceptada: *borrowing*, *ownership*, *scheduling*, *query*.
- Traducir cuando existe equivalente consolidado: *component* → *componente*, *entity* → *entidad*.
- Evitar spanglish ("chequear", "printear").

## Estructura
- Cada sección abre con 1-2 frases de contexto, no con un encabezado suelto.
- Cerrar cada capítulo con un resumen de 3-7 puntos.
- Glosario al final de cada capítulo con los términos nuevos.

## Prohibiciones (muletillas LLM)
- "En el mundo de...", "En el ecosistema de..."
- Introducciones genéricas de más de 2 párrafos.
- Repetir el título del capítulo como primera frase.
- "Como hemos visto" abusivo.
- Listas de viñetas donde bastaría un párrafo.

## Ejemplos de código
- Siempre incluidos desde `examples/` vía `include::`.
- Nunca copiados a mano en el `.adoc`.
- Etiquetar regiones con `tag::` para incluir solo lo relevante.
