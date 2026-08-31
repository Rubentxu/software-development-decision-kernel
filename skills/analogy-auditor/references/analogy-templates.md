# Plantilla para declarar límites de analogías

Toda analogía en el libro debería poder completar esta ficha:

```yaml
analogy:
  concept: ownership de Rust
  analogy: "ser dueño de un coche"
  maps_well:
    - exclusividad (un único dueño)
    - transferencia (vender = move)
  breaks_at:
    - "no hay transacción/precio al mover"
    - "borrow no existe en coches (prestar sin transferir)"
    - "el compilador lo hace cumplir, no la ley"
  disclaimer_required: true
```

## Reglas
- Si `breaks_at` está vacío, la analogía probablemente es demasiado vaga o es literalmente el concepto (no hace falta analogía).
- El `disclaimer_required` se resuelve añadiendo una frase en el capítulo: "Esta analogía ayuda con X, pero ojo: no aplica a Y".

## Anti-patrón
Analogía larga y florida sin disclaimer, que el lector internaliza como modelo completo y luego choca con el comportamiento real. Mejor analogía corta + aclaración técnica que analogía autosuficiente.
