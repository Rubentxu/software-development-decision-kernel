# Plantilla de ADR editorial

`book-context/adr/NN-{slug}.md`. Las decisiones editoriales y de diseño se registran aquí para no depender de la memoria de nadie.

```markdown
# ADR-NN: {Título de la decisión}

**Fecha**: YYYY-MM-DD
**Estado**: Propuesta | Aceptada | Reemplazada por ADR-XX
**Decidido por**: {autor/agente}

## Contexto
Por qué surge esta decisión. Qué problema resuelve. Qué alternativas se consideraron.

## Decisión
Qué se decidió, concretamente.

## Consecuencias
- Positivas: ...
- Negativas / trade-offs: ...
- Reversible: sí/no, y cómo.

## Relacionado
- ADR-XX (si reemplaza o complementa)
- evidence card / claim del corpus: {id}
```

## Cuándo escribir un ADR
- Elección de stack editorial (AsciiDoc vs mdBook vs Quarto).
- Elección de arquetipo editorial (cero-a-experto, para-dummies...).
- Simplificación pedagógica deliberada (ej. "enseñamos ChildOf en vez de bsn! directo").
- Decisión de alcance (qué se excluye del libro y por qué).
- Cualquier decisión que un futuro tú o un coautor preguntaría "¿por qué?".

## Cuándo NO escribir un ADR
- Detalles mecánicos (formato de un include).
- Correcciones puntuales (eso es errata, no decisión).

## Persistencia en Engram
Tras escribir el ADR, `mem_save` type=decision con resumen + ruta, para que sea recuperable por significado en sesiones futuras.
