# Checklist adversarial completo

Preguntas que el revisor adversarial debe responder para cada capítulo.
Cualquier hallazgo confirmado es un bloqueo de publicación.

## Versiones y APIs
- [ ] ¿Cada API nombrada existe en la versión declarada del libro?
- [ ] ¿Las firmas de función coinciden con la documentación de esa versión?
- [ ] ¿Hay alguna API marcada como estable que en realidad es nightly/experimental?
- [ ] ¿Las versiones de crates del texto coinciden con las del `Cargo.toml` del ejemplo?

## Plataformas
- [ ] ¿El ejemplo compila y corre en las plataformas declaradas (linux/windows/macos)?
- [ ] ¿Hay rutas, permisos o dependencias específicas de un SO sin avisar?

## Simplificaciones peligrosas
- [ ] ¿Hay alguna analogía que deja de representar el comportamiento real? (→ `analogy-auditor`)
- [ ] ¿Hay alguna simplificación presentada como regla universal?
- [ ] ¿Se omite un caso límite importante para la versión?

## Resultados
- [ ] ¿Los resultados mostrados (salida de comandos, logs, benchmark) están verificados?
- [ ] ¿Los benchmarks tienen metodología, o parecen inventados?

## Evidencia
- [ ] ¿Cada afirmación técnica tiene evidence card con `status: verified`?
- [ ] ¿Hay afirmaciones que dependen de fuente nivel >6 (foro/blog)?

## Código
- [ ] ¿El `.adoc` incluye el código desde `examples/` (no copia a mano)?
- [ ] ¿El ejemplo pasa `code-example-verifier` con `ALL_GREEN`?
