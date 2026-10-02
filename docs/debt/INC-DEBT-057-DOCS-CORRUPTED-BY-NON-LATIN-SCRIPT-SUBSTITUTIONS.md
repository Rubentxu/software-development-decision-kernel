---
id: INC-DEBT-057-DOCS-CORRUPTED-BY-NON-LATIN-SCRIPT-SUBSTITUTIONS
title: 13 documentos de docs/ tienen caracteres CJK, cirílicos o de reemplazo sustituyendo palabras españolas, y la palabra original no se puede reconstruir
status: open
severity: medium
priority: P2
detected_at: 2026-10-02
detected_in_session: session-66
component: documentation
surface: docs/
fingerprint: "docs_spanish_prose_spliced_with_cjk_cyrillic_and_replacement_chars"
related: [INC-DEBT-052]
references:
  - docs/adr/ADR-0072-secretary-budgets.md
  - docs/adr/ADR-0068-bounded-execution.md
  - docs/adr/ADR-0002-atomic-gate-receipt-seq-allocation.md
  - docs/debt/INC-DEBT-040-PREPUSH-BUMP-PREDICATE-UNSATISFIABLE-FOR-DECLARED-RELEASE.md
  - docs/debt/INC-DEBT-054-DOCTOR-STRICT-MEASURES-NOTHING.md
  - docs/debt/INC-DEBT-056-BUNDLE-STAGING-DERIVED-FROM-SURFACE-LIST-NOT-THE-MANIFEST.md
  - docs/debt/INC-DEBT-020.md
  - docs/architecture/adrs/ADR-0139-STATIC-ENHANCED-COVERAGE-CONTRACT.md
---

## Qué es

En trece documentos de `docs/` hay caracteres de un **sistema de escritura
distinto del español** pegados en mitad de una palabra o de una frase, en
documentos que están íntegramente en español o en inglés. No son citas ni
nombres propios: ocupan exactamente la posición de un palabra.

Medido en session-66, recorriendo `docs/**/*.md` con un
rango de bloques no latinos (U+0370-03FF cirílico, U+0590-08FF,
U+0900-0DFF, U+0E00-0FFF, U+1100-11FF, U+2E80-9FFF, U+AC00-D7AF, U+FFFD de
reemplazo). Trece ficheros, 27 ocurrencias en total.

| fichero | dónde | caracteres |
|---|---|---|
| `docs/adr/ADR-0072-secretary-budgets.md` | L47, «…`p-w0` colisionan» | U+4E24 U+8005 |
| `docs/adr/ADR-0068-bounded-execution.md` | L195, «`PartialEq` en snapshot» + | U+505A U+6765 |
| `docs/adr/ADR-0002-atomic-gate-receipt-seq-allocation.md` | L51, «El método» + | U+5185 U+5E55 |
| `docs/debt/INC-DEBT-040-…` | L251, «la sesion-45» + | U+6267 U+529B U+884C |
| `docs/debt/INC-DEBT-054-…` | L43, entrecomillado numérico | U+5370 U+53D1 |
| `docs/debt/INC-DEBT-056-…` | L181, «la prueba» + | U+043F U+043E U+043D U+044F |
| `docs/debt/INC-DEBT-020.md` | L87, «más reciente» + | U+5E78 |
| `docs/architecture/adrs/ADR-0139-…` | L89, bloque de código, comentario | U+FFFD U+FFFD |
| `docs/roadmap/SESSION-JOURNAL.md` | 27 ocurrencias en entradas antiguas | varios |
| `docs/history/…` (4 ficheros) | dentro de paquetes históricos | varios |

Todos preexistentes a session-66. Ninguno introducido por los commits de esta
sesión: la comprobación se hizo antes de cada commit precisamente porque la
clase se había repetido ya ocho veces en la sesión.

## Por qué NO se corrige automáticamente

Es la parte de esta entrada que la hace deuda y no una tarea de limpieza.

**La corrupción se detecta con fiabilidad. La palabra original no.** En
`ADR-0072` los dos caracteres ocupan el sitio de «ambos» y eso se deduce del
contexto. En `INC-DEBT-054` ocupan el sitio de un adjetivo dentro de un
entremillado: no hay forma de saber si la frase era «0<termino corrupto><termino corrupto>» → «0 de superficie»
→ «0 ineliminables». Sustituir unos caracteres por otros sería **fabricar texto
normativo** — y algunos de esos ficheros son ADRs `accepted` y entradas del
índice de deuda.

La corrupción es simétrica con un sentido: se puede medir, no se puede deshacer
sin quien escribió.

## Por qué no es cosmético

Tres razones, en orden de peso:

1. **Un ADR `accepted` con una palabra corrupta no se puede citar con
   confianza.** Quien lo lea no sabe si el término corrupto es ruido o si
   carga significado. Es la misma clase que «una cita que no resuelve»: una
   autoridad que no se puede leer no gobierna nada.
2. **El índice de deuda es la superficie de navegación del operador.** Una
   entrada ilegible es una entrada que no se lee.
3. **No hay guard.** Nada impedía que esto pasara y nada lo detecta hoy: es la
   misma clase de hallazgo que `test_uat_authority_citations.py` o que el guard
   de cobertura de gates — *un invariante que nadie ejecuta*.

## Criterio de cierre

- (a) Los trece ficheros quedan con la prosa correcta, decided por quien
  escribió cada frase; **y** un guard nuevo (`tests/test_docs_script_contamination.py`)
  recorre `docs/` con el mismo rango y falla en cualquier newcomers, con una
  allowlist explícita de lo preexistente y el motivo de cada línea.
- (b) Sólo el guard, con los trece preexistentes en la allowlist por nombre de
  fichero y por contexto. El inventario queda tracked y la corrupción nueva es
  imposible; la vieja sigue visible pero no bloquea.

**(a) es el cierre real. (b) es el cierre parcial honesto**: convierte un
inventario que nadie tenía en un inventario que nadie puede dejar crecer.

Falsificado: quitar una línea de la allowlist tiene que poner el guard en rojo,
y mutar un fichero limpio tiene que producir `[FAIL]`. Un guard que acepta
todo lo que hay hoy y no muerde con lo que mañana es el INC-DEBT-054 de nuevo.

## Lo que esta sesión ya hizo

- Inventario completo y medido (esta tabla).
- `SESSION-JOURNAL.md` y `docs/roadmap/CURRENT.md` lo tienen escrito por
  punto de codigo Unicode, para que la entrada que reporta la corrupción no la
  contenga.
- `STATE.yaml` igual: citar los caracteres habría inyectado la corrupción en el
  puntero.
