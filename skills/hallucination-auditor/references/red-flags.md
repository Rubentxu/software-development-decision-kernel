# Red flags de fabricación LLM en libros técnicos

Patrones recurrentes (extraídos de auditorías reales, incluida la del libro Bevy) que `hallucination-auditor` busca activamente.

## APIs inventadas
- Macros/derives que suenan plausibles pero no existen: `bsn!`, `#[derive(AppSettings)]`, `EntityEvent`.
- Métodos con naming inconsistente con el resto del framework.
- Tipos que mezclan conceptos (`On<E>` como sinónimo de `Trigger`).

## Crates inexistentes o versiones rotas
- Asumir que el crate sigue la numeración del framework principal (Bevy 0.19 → crate "0.19" falso).
- Crates fabricados con nombre plausible (`bevy_navigation`, `berrycode`).
- Crates abandonados presentados como compatibles con la versión actual.

## Referencias falsas
- URLs que no resuelven (siempre verificar fetch).
- RFC/issue numbers inventados.
- Papers con DOI inválido.

## Resultados inventados
- Salidas de comandos/cargo que no se han ejecutado.
- Benchmarks sin metodología ni reproducción.
- Logs/stack traces fabricados para "ilustrar" un error.

## Confusiones conceptuales
- Afirmar que un trait es subtrait de otro sin fundamento (`Resource` subtrait de `Component`).
- Presentar características nightly como estables.
- Atribuir a una versión algo que se retiró o aún no existe.

## Regla de decisión
Ante la duda, tratar como `UNVERIFIED_CLAIM` y exigir evidence card. Nunca dejar pasar un `critical` "porque suena bien".
