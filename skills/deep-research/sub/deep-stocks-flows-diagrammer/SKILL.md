---
name: deep-stocks-flows-diagrammer
description: "Trigger: stocks and flows, diagrama Forrester, simulación de sistemas, ecuaciones de stocks, World3 simulación, modelo dinámico cuantitativo. Modela cuantitativamente un sistema con diagramas stocks-and-flows (Forrester). Produce ecuaciones diferenciales/algebraicas, simulaciones en Python (PySD/BPTK-Py/from-scratch), validación contra World3 standard run. Núcleo para SOFTWARE (simulación); LIBRO produce explicación visual."
license: Apache-2.0
metadata:
  category: deep-research
  subcategory: systems-thinking
  author: rubentxu
  version: "1.0"
  domain: deep-research, systems-thinking
  based_on: "Jay Forrester (Industrial Dynamics 1961), Donella Meadows (Thinking in Systems 2008 cap. 2), John Sterman (Business Dynamics 2000)"
  consumers: [book-orchestrator, orchestrator]
---

## Activation Contract

Úsalo en **Fase 2 cuantitativa del análisis sistémico**, cuando hay datos suficientes para ir más allá del CLD cualitativo. Modela stocks (acumulaciones) y flows (tasas) con ecuaciones.

No lo uses para: análisis puramente cualitativo (`deep-feedback-loops-analyzer`), implementación completa del modelo (`deep-software-research` + `deep-pattern-extractor`).

## Hard Rules (Forrester / Meadows)

- **"All systems, everywhere, consist of these two kinds of concepts — stocks and flows — and none other."** (Forrester, 1971).
- **Stocks se miden en unidades básicas** (personas, dólares, galones).
- **Flows se miden en unidades/tiempo** (personas/año, dólares/mes).
- **Toda ecuación debe tener consistencia dimensional**: las unidades deben cuadrar en ambos lados.
- **Validación contra datos históricos**: el modelo debe reproducir el reference mode observado.
- **No inventar parámetros**: cada parámetro tiene fuente (paper, dato primario, calibración documentada).

## Execution Steps

### Modo A: Construir el diagrama SFD

1. Lee `research/cld/{topic}.yml` (CLD cualitativo).
2. Para cada variable, clasifica como:
   - **Stock**: acumulación (rectángulo).
   - **Flow**: tasa que cambia el stock (tubería con válvula).
   - **Auxiliary**: variable que computa la tasa.
   - **Parameter**: constante.
3. Define las **unidades** explícitamente.
4. Conecta stocks con flows (inflows + outflows).
5. Marca clouds (fuente/sumidero infinito).
6. Documenta **ecuaciones** (ver §5).

### Modo B: Simular

1. Define las ecuaciones en formato legible (e.g., equations Python).
2. Implementa con:
   - **PySD** (traduce modelos Vensim/STELLA a Python).
   - **BPTK-Py** (Business Prototyping Toolkit).
   - **From-scratch** con `scipy.integrate.solve_ivp` o Euler simple.
3. Validar contra:
   - **World3 Standard Run** (1900-2100): valores esperados en `research/test-fixtures/world3-standard-run.json`.
   - Datos históricos (si disponibles).
4. Análisis de sensibilidad: ¿qué parámetros más afectan el comportamiento?

### Modo LIBRO: producir explicación visual

- `research/diagrams/{topic}-sfd.mmd` (Mermaid con stocks/flows/auxiliaries).
- `research/drafts/{topic}-sfd-section.md` (borrador con ecuaciones + interpretación).

### Modo SOFTWARE: producir blueprint de simulación

- `research/blueprints/{topic}-simulator.yml` (API para correr el modelo).
- `research/code-patterns/stock-flow-simulator.py`.
- `research/test-fixtures/world3-standard-run.json` (valores esperados).

## Símbolos canónicos (Forrester/Meadows)

```
[Stock] = rectángulo (acumulación)
───►[Flow] = tubería con válvula (tasa)
(Cloud) = fuente/sumidero (fuera del modelo)
○ = auxiliary (computa la tasa)
```

## Ejemplo de ecuación (World3 Population loop)

```
Population(t+dt) = Population(t) + (Births(t) - Deaths(t)) * dt
Births(t) = Population(t) * fertility_rate(t)
Deaths(t) = Population(t) * mortality_rate(t)
fertility_rate(t) = f(fertility, max_fertility, desired_family_size)
mortality_rate(t) = f(mortality, life_expectancy)
```

## Esquema de SFD

```yaml
sfd:
  topic: "World3 Population"
  units:
    population: people
    time: year
    fertility: 1/year
  stocks:
    - {id: population, units: people, initial: 1.6e9, equation: "integ(births - deaths)"}
  flows:
    - {id: births, units: people/year, equation: "population * fertility"}
    - {id: deaths, units: people/year, equation: "population * mortality"}
  auxiliaries:
    - {id: fertility, units: 1/year, equation: "min(max_fertility, fertility_function)"}
  parameters:
    - {id: max_fertility, units: 1/year, value: 0.05, source: src-limits-to-growth-1972}
  cloud:
    births_source: "outside model"
    deaths_sink: "outside model"
  test_acceptance:
    - {year: 1970, expected: 3.7e9, tolerance: 0.05}
    - {year: 2000, expected: 6.1e9, tolerance: 0.05}
```

## Librerías válidas

| Librería | Uso |
|----------|-----|
| PySD | Traduce modelos Vensim/STELLA a Python |
| BPTK-Py | Modelos SD + agent-based |
| pysd-jax | PySD con backend JAX |
| Tellurium/libSEDML | Systems Biology Markup Language |
| Vensim | Editor + simulador (propietario) |
| STELLA/iThink | Editor + simulador (propietario) |

## Decision Gates

| Situación | Acción |
|-----------|--------|
| Parámetro sin fuente | Marcar `parameter: assumed`; no usar para claims `critical` |
| Ecuación sin consistencia dimensional | STOP; corregir antes de simular |
| Modelo no reproduce reference mode | Iterar: revisar estructura, parámetros, ecuaciones |
| > 50 stocks | Dividir en subsistemas; o usar abstracción |
| Validación contra World3 falla | Re-evaluar parámetros; verificar unidades; puede ser un bug, no un fallo del modelo |

## Output Contract

### LIBRO
- `research/diagrams/{topic}-sfd.mmd`.
- `research/sfd/{topic}.yml` (modelo formal).
- `research/drafts/{topic}-sfd-section.md`.

### SOFTWARE
- `research/blueprints/{topic}-simulator.yml`.
- `research/code-patterns/stock-flow-simulator.py`.
- `research/test-fixtures/{topic}-expected.json`.

## References

- **Forrester, J. W.** (1961). *Industrial Dynamics*.
- **Forrester, J. W.** (1971). *World Dynamics*.
- **Meadows, D. H.** (2008). *Thinking in Systems*. Cap. 2.
- **Sterman, J.** (2000). *Business Dynamics*. McGraw-Hill.
- **PySD**: `https://github.com/SDXorg/pysd`.
- **BPTK-Py**: `https://github.com/transentis/bptk-py`.
- **MIT OCW 15.988**: tutoriales.
