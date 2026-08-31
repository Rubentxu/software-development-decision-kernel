# WCAG 2.1 AA — referencia rápida (lo que aplica a un libro técnico)

## 1. Perceptible
- **1.1.1 Non-text Content**: todo gráfico/diagrama con `alt` equivalente.
- **1.3.1 Info and Relationships**: usar marcas semánticas (`<th>`, listas, encabezados).
- **1.4.3 Contrast (Minimum)**: ≥ 4.5:1 texto normal, ≥ 3:1 texto grande.
- **1.4.1 Use of Color**: el color no es el único medio de información.

## 2. Operable
- **2.4.6 Headings and Labels**: encabezados descriptivos.
- **2.4.10 Section Headings**: secciones con encabezado.

## 3. Comprensible
- **3.1.4 Abbreviations**: expandir siglas en primera aparición.

## 4. Robusto
- HTML válido y semántico para que lectores de pantalla lo procesen.

## Aplicación a diagramas técnicos
- Un diagrama de flujo donde "el rojo = error" falla 1.4.1 si no hay también un icono/etiqueta.
- Un grafo de dependencias debe poder leerse en escala de grises.
- Un diagrama de secuencia debe tener equivalente textual (prosa que describa el orden).

## Herramientas
- axe-core / Lighthouse para el render HTML.
- Comprobación manual de escala de grises (modo del navegador).
