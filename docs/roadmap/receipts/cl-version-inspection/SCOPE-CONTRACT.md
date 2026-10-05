# cl-version-inspection — SCOPE-CONTRACT

**Cycle:** `p-63676b11dc0ef88f/version-coherence-068` (VA7 del bloque C4.1)
**Date:** 2026-10-05T18:40:00Z

---

## Objetivo, en una frase y falsable

Un agente puede responder **por qué** SDDK resolvió o no la versión de un
target sin abrir ningún fichero del repositorio, y el informe dice **qué no se
comprobó** además de lo que sí.

Falsable porque tiene una forma concreta: un comando nuevo que,
sobre un árbol real, imprime para cada provider considerado qué respuesta dio, con
qué evidencia, y termina con una lista de lo que **no** se comprobó — y la
conformance suite demuestra que resolver no modifica el árbol.

## Qué NO es objetivo, y por qué

- **No es un validador de releases.** Encontrar la `ProductVersion` no
  certifica un release, y el informe dice eso en voz alta. Un comando que
  pareciera decir «el release está bien» sería peor que no tenerlo.
- **No recomienda cambios al proyecto del usuario.** El bloque prohíbe
  expresamente decir «añade `version=` a X». El informe describe; quien decide
  qué hacer con el diagnóstico es el operador.
- **No añade capacidades al registry.** La inspección **pregunta** la
  capability que ya existe. Un provider nuevo se sigue añadiendo por el SPI de
  VA2, no por un caso especial aquí.
- **No es una segunda reducción.** El informe se construye **después** de
  `reduce()`, nunca en vez de él. Dos reducciones distintas divergen.

## Riesgo de datos

**Cero.** La resolución es read-only por construcción (VA1) y esta superficie no
añade I/O: lee lo que el registry ya leyó. La conformance suite lo demuestra
midiendo el sha256 del árbol antes y después, no declarándolo.

## STOP conditions

1. Que el informe necesite un concepto que el dominio ya tenga y esté reutilizado
   como autoridad nueva.
2. Que la conformance suite no pueda demostrar read-only sin tocar el producto.
3. Que `--json` tenga que cambiar de forma un consumidor existente (MEDIDO antes:
   `version_authority` no lo deserializa nadie, comprobado en VA3).

## Rejilla de la inspección, y por qué son seis y no cinco

El bloque nombra `OBSERVED`, `CROSS_VALIDATED`, `NOT_CHECKED`, `NOT_APPLICABLE`,
`CONFLICT`, `INVALID`. Se conservan los seis nombres, pero **dos son niveles
distintos y una lista plana los escondería**:

- **por observación**: `OBSERVED` (declaró), `NOT_APPLICABLE` (no es su
  asunto), `INVALID` (no se pudo leer);
- **por veredicto**: `CROSS_VALIDATED`, `CONFLICT`, `NOT_CHECKED`.

Una columna con los seis mezcla «este provider no tenía nada que ver» con
«este release no se comprobó con nadie más», que son preguntas distintas y una
de ellas es una advertencia.

## El riesgo de este bloque, escrito antes de empezar

El modo de fallo no es que el informe falte: es que **mienta con naturalidad**.
Un informe que enumera providers y sus razones es muy fácil de hacer verosímil
mientras omite que no cross-validó nada, y un informe verosímil que omite una
limitación es peor que no tener informe. Por eso `NOT_CHECKED` es un campo
**calculado**, no un texto fijo: sale de lo que de verdad pasó, y si el código
cambia y el `NOT_CHECKED` no, el test lo nota.