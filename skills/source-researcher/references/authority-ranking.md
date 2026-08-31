# Ranking de autoridad de fuentes

Orden de preferencia al construir evidence cards. Una afirmación sin fuente de nivel ≤3 es **riesgo**.

| Nivel | Tipo | Ejemplo |
|-------|------|---------|
| 1 | Especificación / estándar formal | RFC de IETF, spec oficial del lenguaje |
| 2 | Documentación oficial | docs.rs, doc.rust-lang.org, bevyengine.org/learn |
| 3 | Código fuente del proyecto | módulos, tests, changelog, release notes |
| 4 | Paper formal revisado por pares | arXiv, ACM, IEEE |
| 5 | Post/blog del mantenedor con fecha | blog oficial, discourse del proyecto |
| 6 | Libro de referencia canónico | "The Rust Programming Language" |
| 7 | Respuesta de foro con votación alta | Stack Overflow con respuesta aceptada |
| 8 | Blog de terceros | cualquier otro |

Reglas:
- Nivel 1-3 con `retrieved_at` y `version` → `status: verified`.
- Nivel 4-6 → `verified` solo si la afirmación no depende de versión concreta.
- Nivel 7-8 → `unverified`; requiere corroboración con nivel ≤3 antes de citar.
- Cualquier fuente sin fecha ni versión → descartar.
