# Adversarial Entropy Metrics

Inteligencia entrópica basada en métricas para el juicio adversarial en sddk-verify.

## Concepto

Cuando el juicio adversarial (2 jueces) encuentra deficiencias, cada una se evalúa con
métricas entrópicas cuantificables. Los jueces no "opinan" — **miden**:

- Cuánto acoplamiento introduce la deficiencia (entropía de implementación)
- Cuánto del comportamiento esperado no está cubierto por specs (spec coverage)
- Cuántos módulos se ven afectados (blast radius)
- Cuánta información interna se filtra (information leakage)
- Cuánto peor empeoró el sistema (entropy delta)
- Qué tan difícil es corregir (reversibility → effort)

## Dimensiones por Finding

| Dimensión | Rango | Qué mide |
|-----------|-------|----------|
| `spec_coverage` | 0.0–1.0 | Cuánto de lo implementado está cubierto por la spec (0=nada, 1=completo) |
| `impl_entropy` | 0.0–8.0 bits | I(A;B) acoplamiento introducido por la deficiencia |
| `blast_radius` | 0.0–1.0 | Módulos afectados normalizados (0=aislado, 1=sistema completo) |
| `reversibility` | 0.0–1.0 | Facilidad de corregir (0=rewrite, 1=trivial) |
| `entropy_delta` | -5.0–8.0 bits | ΔH introducido por esta deficiencia (negativo=mejora) |
| `information_loss` | 0.0–1.0 | I(X;T) leakage de estado interno a través de la interfaz |

## Fórmula AES (Adversarial Entropy Score)

```
AES = w_spec × (1 - spec_coverage)
    + w_entropy × min(impl_entropy / 5.0, 1.0)
    + w_blast × blast_radius
    + w_leak × information_loss
    + w_delta × max(entropy_delta, 0) / 5.0

Pesos:
  w_spec    = 0.20  — spec gap
  w_entropy = 0.25  — acoplamiento (máximo peso — es lo más caro)
  w_blast   = 0.25  — impacto (máximo peso — es lo que más importa)
  w_leak    = 0.15  — information leakage
  w_delta   = 0.15  — empeoramiento del sistema

Modificador de reversibilidad:
  AES_final = AES × (1.0 - 0.2 × (1 - reversibility))
  → Deficiencias difíciles de corregir obtienen boost (permanecen más tiempo)
```

**AES > 0.5** → CRITICAL (debe corregirse antes de archivar)
**AES 0.25–0.5** → WARNING (debería corregirse)
**AES < 0.25** → SUGGESTION (mejora opcional)

## Tipos de Finding

| Tipo | Descripción | Acción correctiva |
|------|-------------|-------------------|
| `spec_gap` | Spec no cubre comportamiento implementado | `add_missing_scenarios` |
| `spec_ambiguity` | Spec es vaga, múltiples interpretaciones | `clarify_language` |
| `spec_stale` | Spec describe comportamiento obsoleto | `update_to_current` |
| `code_bug` | Código no hace lo que la spec dice | `fix_behavior` |
| `code_missing` | Spec requiere algo no implementado | `implement_missing` |
| `design_drift` | Implementación se desvía del diseño | `realign_implementation` |
| `design_omission` | Diseño no anticipó un caso real | `add_design_decision` |
| `entropy_regression` | Acoplamiento/cohesión empeoró | `refactor_coupling` |

## Clasificación de Severidad

```python
# Entropy regressions siempre WARNING mínimo
if type == "entropy_regression" and AES >= 0.3: → CRITICAL

# Code bugs con blast radius alto → CRITICAL
if type in ["code_bug", "code_missing"] and AES >= 0.5: → CRITICAL

# Spec gaps con alta entropía → CRITICAL (ocultan acoplamiento)
if type == "spec_gap" and AES >= 0.6: → CRITICAL

# General
AES >= 0.5  → CRITICAL
AES >= 0.25 → WARNING
AES < 0.25  → SUGGESTION
```

## Esfuerzo de Corrección

```
effort = (1 - reversibility) × type_modifier + blast_radius × 0.3

type_modifier:
  spec_gap: 0.2          # Escribir specs es barato
  spec_ambiguity: 0.15   # Aclarar es barato
  spec_stale: 0.3        # Actualizar necesita investigación
  code_bug: 0.4          # Corregir bugs necesita cuidado
  code_missing: 0.6      # Implementar desde cero es caro
  design_drift: 0.5      # Realinear requiere pensamiento
  design_omission: 0.4   # Agregar decisiones de diseño
  entropy_regression: 0.7 # Refactorizar acoplamiento es caro
```

## Spec Alignment

Métrica compuesta que evalúa qué tan alineadas están specs e implementación:

```
coverage_score = (covered + 0.5 × partial) / total
alignment_score = 1.0 - (missing + ambiguous + stale) / total

≥ 0.8 → 🟢 ALINEADO
0.5–0.8 → 🟡 PARCIAL
< 0.5 → 🔴 DESALINEADO
```

## Correction Plan

El plan de corrección prioriza:
1. **Primero por severidad**: CRITICAL → WARNING → SUGGESTION
2. **Dentro de severidad, por AES** descendente (peor primero)
3. **Desempate por esfuerzo** ascendente (easy wins primero)

Clasificación de acciones:
- 📋 **SPEC UPDATES**: spec_gap, spec_ambiguity, spec_stale
- 🔧 **CODE FIXES**: code_bug, code_missing, entropy_regression
- 📐 **DESIGN UPDATES**: design_drift, design_omission

## Uso del Script

```bash
# Evaluar findings individuales:
python3 adversarial-metrics.py --findings '[
  {"id":"F1","type":"spec_gap","description":"...",
   "spec_coverage":0.4,"impl_entropy":1.8,"blast_radius":0.3,
   "reversibility":0.8,"entropy_delta":0.5,"information_loss":0.1}
]'

# Con plan de corrección:
python3 adversarial-metrics.py --correction-plan --file findings.json

# Con spec alignment:
python3 adversarial-metrics.py --spec-alignment --file findings.json

# Output JSON (para pipelines):
python3 adversarial-metrics.py --json --file findings.json
```

## Integración con sddk-verify

El flujo completo:

```
sddk-verify Step 7 (Adversarial Entropy Judgment):
│
├── 7.1 Lanzar 2 jueces adversariales (delegación paralela)
│    Cada juez:
│    ├── Review contra specs + design + tasks
│    ├── Para CADA deficiencia: estimar 6 dimensiones entrópicas
│    ├── Generar lista de findings con métricas
│    └── Retornar findings como JSON
│
├── 7.2 Sintetizar findings (orchestrator)
│    ├── Merge findings de ambos jueces
│    ├── Confirmed: ambos encontraron = alta confianza
│    ├── Suspect: solo un juez = necesita triaje
│    └── Contradiction: desacuerdo = escalar
│
├── 7.3 Calcular AES con adversarial-metrics.py
│    ├── python3 adversarial-metrics.py --json --findings '{merged}'
│    ├── Obtener: AES scores, severidad, prioridad, esfuerzo
│    └── Generar correction plan
│
├── 7.4 Spec Alignment
│    ├── python3 adversarial-metrics.py --spec-alignment --file findings.json
│    └── Añadir al reporte
│
├── 7.5 Correction Cycle (orchestrator)
│    ├── IF CRITICAL findings → delegate sddk-apply with fix list
│    ├── After fix → re-run sddk-verify (solo Steps 7.1-7.3)
│    ├── Max 2 iterations → then ESCALATE
│    └── Clean → APPROVED
│
└── 7.6 HTML Report (Spanish)
     ├── Tabla de findings con AES
     ├── Spec alignment score
     ├── Correction plan priorizado
     └── Veredicto: APPROVED / ESCALATED
```

## Diferencia con Opportunity Score

| Métrica | Oportunidad (auto-grill) | Deficiencia (adversarial) |
|---------|--------------------------|--------------------------|
| **Contexto** | Decisiones de diseño/architecture | Bugs, gaps, desviaciones |
| **Objetivo** | Rankear opciones para escalar al humano | Priorizar correcciones |
| **Dimensiones** | coupling, free_energy, openness, flexibility, depth, reversibility | spec_coverage, impl_entropy, blast_radius, information_loss, entropy_delta, reversibility |
| **Output** | OS score por opción | AES score por finding |
| **Acción** | Human elige opción | Auto-fix o escalar |
| **Script** | `os-calc.py` | `adversarial-metrics.py` |
