---
name: accessibility-reviewer
description: "Trigger: revisión de accesibilidad, texto alternativo, diagramas sin color, tablas accesibles, jerarquía de encabezados, contraste, a11y, lectores de pantalla. Comprueba texto alternativo, diagramas comprensibles sin color, tablas legibles, jerarquía, contraste y explicación textual de la información visual."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.0"
---

## Activation Contract

Úsalo como sub-pase de `editorial-reviewer`/`technical-reviewer` sobre el render HTML del libro. Garantiza que el contenido técnico es accesible.

No la uses para contenido puramente textual sin soporte visual.

## Hard Rules

- Toda imagen/diagrama tiene **texto alternativo** descriptivo (no "imagen").
- La información de un diagrama debe ser **comprensible sin color** (usar formas/etiquetas).
- Jerarquía de encabezados correcta (un solo `h1`, sin saltos de nivel).
- Contraste suficiente (WCAG AA mínimo).
- Información visual clave tiene **explicación textual** equivalente.

## Checklist

| Categoría | Qué comprobar |
|-----------|---------------|
| Texto alternativo | `alt=` descriptivo en cada imagen/diagrama |
| Color | información no depende solo del color (formas/etiquetas) |
| Tablas | encabezados `<th>`, caption, no usar tablas para maquetación |
| Jerarquía | orden `h1→h2→h3` sin saltos |
| Contraste | ratio ≥ 4.5:1 para texto normal (WCAG AA) |
| Texto ↔ visual | cada diagrama con explicación textual de su mensaje |
| Enlaces | texto de enlace descriptivo (no "haz clic aquí") |
| Código | bloques con `role="code"` y legibles (no solo color) |

## Execution Steps

1. Renderizar el capítulo a HTML (`book-builder` parcial).
2. Escanear imágenes/diagramas → verificar `alt`.
3. Analizar diagramas → ¿requieren color para entenderse?
4. Comprobar jerarquía de encabezados.
5. Medir contraste del texto (herramientas o heurística).
6. Verificar que cada diagrama tiene párrafo textual equivalente.
7. Emitir `build/reviews/{chapter-id}.a11y.yml`.

## Output Contract

- `build/reviews/{chapter-id}.a11y.yml`.
- `verdict`: `PASS` | `PASS_WITH_REMEDIATION` | `BLOCKED`.
- Falta de `alt` en diagrama informativo → `BLOCKED`.

## References

- `references/wcag-quick.md` — referencia rápida WCAG 2.1 AA.
