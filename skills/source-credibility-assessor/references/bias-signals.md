# Señales de sesgo y conflicto de intereses

`source-credibility-assessor` busca estos patrones. Su presencia aplica penalty o rechazo.

## Sesgo comercial
- El autor vende un producto relacionado con la afirmación.
- El blog es content marketing de una empresa (ej. "X es mejor que Y" donde el autor trabaja en X).
- Afirman superioridad sin benchmark reproducible.

## Sesgo de autopromoción
- El autor cita mayoritariamente su propio trabajo.
- Una librería se presenta como "estándar" cuando es marginal.
- Recomienda su curso/libro/servicio de pago.

## Conflicto de intereses (COI)
- El autor es mantenedor del crate que evalúa positivamente sin declararlo.
- Comparativas donde el autor tiene interés financiero en el ganador.
- **Regla**: el COI declarado es tolerable (con penalty); el COI oculto es rechazo.

## Señales positivas (aumentan confianza)
- Declaración explícita de conflictos ("no tengo relación comercial con...").
- El autor cambia de opinión pública cuando nueva evidencia aparece.
- Cita fuentes que lo contradicen y las refuta con evidencia.

## Link rot y accesibilidad
- 404 / 410 → `rot` (rejected salvo archivo).
- Paywall → `paywall` (admitted si hay resumen verificable, penalty si no).
- Redirección sospechosa (dominio comprado) → verificar destino.
- Sin archivo en Wayback → más frágil.

## Anti-patrón
Asumir que un libro editado por una editorial grande es neutro. La editorial tiene interés en venderlo; evaluar al autor y al contenido, no solo el sello.
