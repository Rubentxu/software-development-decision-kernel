# Opportunity Score Framework

Every escalated decision presents multiple options ranked by a composite **Opportunity Score (OS)**. The OS quantifies which option delivers the most value — functional, technical, and business — per unit of coupling introduced.

## Calculation: os-calc.py

**The LLM NEVER calculates OS manually.** The LLM estimates the 6 dimensions per option (values 0.0–1.0), then calls `os-calc.py` to compute the actual OS.

```bash
# From skill directory:
python3 os-calc.py --options '[
  {"name":"A: Fachadas","coupling":0.15,"free_energy":0.10,"openness":0.85,"flexibility":0.88,"depth":0.90,"irreversibility":0.25},
  {"name":"B: Split crate","coupling":0.30,"free_energy":0.20,"openness":0.60,"flexibility":0.55,"depth":0.70,"irreversibility":0.60}
]'

# JSON output (for HTML injection):
python3 os-calc.py --json --options '[...]'

# HTML table rows (for report):
python3 os-calc.py --html --options '[...]'

# Custom weights:
python3 os-calc.py --weights '{"flexibility":0.35}' --options '[...]'

# Interactive mode:
python3 os-calc.py --interactive --names "Opción A" "Opción B"
```

## Formula

```
OS = w₁ × (1 - norm_coupling)
   + w₂ × (1 - norm_free_energy)
   + w₃ × openness_ratio
   + w₄ × norm_flexibility
   + w₅ × norm_depth
   + w₆ × (1 - norm_irreversibility)
```

### Weights (default, adjustable per project)

| Dimension | Weight | Rationale |
|-----------|--------|-----------|
| Acoplamiento (coupling) | 0.20 | Less new coupling = more maintainable |
| Cohesión (free energy) | 0.15 | Lower F = elements share purpose |
| Apertura (OCP) | 0.20 | Extension > modification |
| Flexibilidad (scenarios) | 0.25 | More future options = more valuable |
| Profundidad (depth) | 0.10 | Deep modules > shallow wrappers |
| Reversibilidad | 0.10 | Reversible decisions are safer |

**Why flexibility has the highest weight (0.25):** The user explicitly requested that escalated decisions show "oportunidades funcionales útiles y de negocio, y mejoras evidentes." Flexibility measures exactly this — how many future scenarios each option enables.

## Dimensions

### 1. Acoplamiento — ΔI(A;B)

```
For each option:
  1. Identify new dependencies introduced
  2. Estimate I(Name) = log2(rename_propagation_count)
  3. Estimate I(Type) = log2(type_dependency_depth)
  4. Sum: ΔI = I(new_pairs)
  5. Normalize: norm_coupling = ΔI / max_across_options

Lower is better → (1 - norm_coupling) in formula.
```

**Interpretation:**
- ΔI = 0: option adds no coupling
- ΔI < 1 bit: minimal coupling
- ΔI 1-3 bits: moderate — acceptable for high-value options
- ΔI > 3 bits: HIGH — only justified if flexibility compensates

### 2. Cohesión — ΔF

```
For each option:
  1. Identify modules affected
  2. Estimate F_before = H(methods) - H(methods|purpose) for each module
  3. Estimate F_after for the same modules under this option
  4. ΔF = F_after - F_before
  5. Normalize across options

ΔF < 0: cohesion improves (split justified)
ΔF = 0: no change
ΔF > 0: cohesion worsens (elements lose shared purpose)
```

### 3. Apertura — H(Δ_new) / H(Δ_existing)

```
OCP ratio:
  H(Δ_new) = bits of NEW code/types/functions
  H(Δ_existing) = bits that must CHANGE in existing code

  openness = H(Δ_new) / (H(Δ_new) + H(Δ_existing))

  openness ≈ 1.0: pure extension (ideal OCP)
  openness ≈ 0.5: balanced
  openness < 0.3: mostly modification (OCP violated)
```

### 4. Flexibilidad — H(escenarios_futuros)

This is the **core innovation** requested by the user.

```
For each option:
  1. Generate future scenarios enabled by this option
     (use LLM + CogniCode semantic_search to find related patterns)
  
  2. Classify each scenario:
     🎯 Funcional: end-user visible capability
     🔧 Técnico: internal improvement (observability, perf, testability)
     💼 Negocio: business metric impact (latency, cost, UX, retention)
  
  3. Count scenarios by type:
     N_func, N_tech, N_biz
  
  4. H(flexibility) = log2(1 + N_func + N_tech + N_biz)
  
  5. Normalize across options: norm_flexibility = H / max_H
```

**Scenario generation heuristics:**

For each option, ask:
- "If we choose this, what new capabilities become trivial to add?"
- "What patterns does this option align with in the existing codebase?"
- "What business metrics could improve with this option?"
- "What technical debt does this option resolve as a side effect?"

### 5. Profundidad — leverage / interfaz

```
For each option:
  1. Estimate interface surface: number of public methods/types exposed
  2. Estimate implementation richness: behaviors hidden behind interface
  3. depth = implementation_richness / interface_surface

  depth > 0.7: deep module (high leverage)
  depth 0.3-0.7: moderate
  depth < 0.3: shallow (wrapper pattern)

  Normalize across options.
```

### 6. Reversibilidad — H(revert)

```
For each option:
  1. Count steps to revert this decision:
     - Files to delete
     - Files to modify back
     - Dependencies to remove
     - Tests to update
  2. H(revert) = log2(steps_to_revert)
  3. Normalize: lower H = more reversible = better

  (1 - norm_irreversibility) in formula → more reversible scores higher.
```

## Interpretation

| OS | Rating | Badge | Meaning |
|----|--------|-------|---------|
| > 0.7 | EXCELENTE | 🟢 | Low coupling, high flexibility, opens many futures |
| 0.4 – 0.7 | BUENO | 🟡 | Balanced, some trade-offs |
| 0.2 – 0.4 | REGULAR | 🟠 | Introduces coupling or limits futures |
| < 0.2 | POBRE | 🔴 | High coupling, breaks OCP, closes options |

## Example

```
Decision: "¿Cómo dividir el AppState God Object?"

Opción A: Fachadas de dominio
  ΔI = -2.1 bits (removes coupling from god object)
  ΔF = -0.8 (improves cohesion)
  Apertura = 0.85 (mostly extension)
  H(flex) = log2(6) = 2.58 → 5 scenarios: 
    🎯 WorkspaceFacade CRUD sin tocar AppState
    🎯 WorkflowFacade operaciones de workflow
    🔧 Cada fachada testeable independientemente
    🔧 Seam real: 2 adapters (prod + test)
    💼 Handler nuevo = método en fachada
  Depth = 0.9 (rich behavior behind small interface)
  H(revert) = log2(3) = 1.58 (3 pasos para revertir)

OS_A = 0.20×0.85 + 0.15×0.90 + 0.20×0.85 + 0.25×0.88 + 0.10×0.90 + 0.10×0.75
     = 0.17 + 0.14 + 0.17 + 0.22 + 0.09 + 0.08
     = 0.87 🟢
```
