# Puntuación de confianza (confidence_score)

Fórmula reproducible para asignar 0.0–1.0 a una afirmación triangulada. El score determina el `status` publicable.

## Componentes

```
confidence = w1 * authority + w2 * convergence + w3 * independence + w4 * freshness - penalties
```

Pesos por defecto: `w1=0.35, w2=0.25, w3=0.25, w4=0.15`.

### 1. authority (0–1)
Nivel de la mejor fuente que confirma:
- L1-exp/L1 → 1.0
- L2 → 0.9
- L3 → 0.8
- L4 → 0.75
- L5 → 0.6
- L6 → 0.5
- L7 → 0.3

### 2. convergence (0–1)
- `consensus` (todas coinciden) → 1.0
- `partial-agreement` (mayoría, matices) → 0.7
- `contradiction` resuelta → 0.6
- `contradiction` no resuelta → 0.2

### 3. independence (0–1)
- ≥3 fuentes independientes → 1.0
- 2 independientes → 0.8
- 1 fuente L1/L2 indiscutible → 0.8
- 1 fuente L3–L7 sola → 0.4
- 0 independientes (todas derivan de una) → 0.2

### 4. freshness (0–1)
- `retrieved_at` < 6 meses → 1.0
- 6–12 meses → 0.8
- 1–2 años → 0.6
- > 2 años → 0.4
- Sin fecha → 0.2

### Penalties
- Conflicto de intereses no declarado: −0.2
- Fuente con sesgo comercial evidente: −0.2
- Link roto / sin archivar: −0.3
- Afirmación de versión sin `retrieved_at`: −0.3

## Umbral de status
- `confidence ≥ 0.8` → `verified` (publicable)
- `0.5 ≤ confidence < 0.8` → `disputed` (publicable SOLO con disclaimer explícito)
- `confidence < 0.5` → `unverified` (bloqueante)

## Anti-patrón
Una afirmación con 5 blogs L7 que dicen lo mismo pero todos derivan de la doc oficial: `independence` baja (0.2) porque no son independientes. El score cae por debajo de 0.5 aunque haya "muchas fuentes". La triangulación cuenta independencia, no cantidad.
