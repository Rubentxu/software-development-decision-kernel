---
name: auto-grill
description: >
  Automatic grilling agent that challenges plans/proposals/designs against the
  codebase, resolves what it can via entropy analysis and codebase exploration,
  and produces an HTML report with enriched escalated decisions for human review.
  Trigger: Runs automatically within SDD phases (like entropy-sdd) or when the
  user invokes "auto-grill" on a plan, proposal, or design.
  Applies to: sddk-explore, sddk-propose, sddk-design, improve-codebase-architecture.
license: MIT
metadata:
  author: rubentxu
  version: "1.0"
---

## Purpose

Auto-grill is an **automatic grilling agent** that:

1. Takes a plan/proposal/design as input
2. Generates verification questions about every claim, decision, and term
3. Auto-resolves questions using codebase exploration + entropy analysis
4. For unresolved questions: generates options ranked by **Opportunity Score** (6 entropy dimensions)
5. Produces an **HTML report** for human review and validation

**Auto-grill is MANDATORY** when invoked by an SDD phase. It runs as a cross-cutting
sub-routine (like entropy-sdd), not as a separate DAG stage.

**All output is in Spanish** — see MANDATORY section below.

---

## MANDATORY: Spanish Language

**El reporte HTML y toda la comunicación debe ser en español.** Esto incluye:
- Títulos, descripciones, preguntas, resoluciones, opciones
- Badges (Auto-resuelta, Escalada, Recomendada)
- Tablas de Opportunity Score
- Clasificación de oportunidades (Funcional / Técnico / Negocio)
- Recomendaciones del agente

### Anglicismos permitidos

Los términos técnicos estándar de la industria se mantienen en inglés cuando no hay
equivalente español de uso común. Se españolizan solo cuando existe forma natural:

| Inglés | Uso en reporte | Cuándo |
|--------|---------------|--------|
| module | **módulo** | siempre españolizado |
| interface | **interfaz** | siempre españolizado |
| adapter | **adaptador** | siempre españolizado |
| depth | **profundidad** | siempre españolizado |
| locality | **localidad** | siempre españolizado |
| seam | **seam** (costura) | se usa "seam" como término técnico, "costura" como aclaración |
| leverage | **leverage** (palanca) | se usa "leverage" como término técnico de arquitectura |
| coupling | **acoplamiento** | siempre españolizado |
| cohesion | **cohesión** | siempre españolizado |
| connascence | **connascence** | término técnico propio, no se traduce |
| refactoring | **refactoring** (o refactorización) | ambos aceptables |
| boilerplate | **boilerplate** | anglicismo común, no traducir |
| cache | **caché** | españolizado |
| threshold | **umbral** | siempre españolizado |
| blast radius | **blast radius** (radio de impacto) | se usa el inglés con aclaración |
| god object | **god object** (objeto dios) | se usa el inglés con aclaración |
| passthrough | **passthrough** (paso a través) | se usa el inglés con aclaración |
| port | **puerto** | siempre españolizado |
| CRUD | **CRUD** | acrónimo estándar, no traducir |
| OCP | **OCP** | acrónimo SOLID, no traducir |
| LSP | **LSP** | acrónimo SOLID, no traducir |

**Regla general:** Si el término aparece en el glosario anterior, se usa como término técnico
(inglés con aclaración en español la primera vez). Si existe equivalente español natural,
se españoliza siempre.

---

## Activation Model

### When invoked by SDD phases (cross-cutting):

| Phase | What gets grilled | Auto-grill focus |
|-------|-------------------|------------------|
| `sddk-explore` | Exploration findings | Verify claims against codebase + connascence landscape |
| `sddk-propose` | Proposal document | Challenge decisions against CONTEXT.md + ADRs + entropy budget |
| `sddk-design` | Design document | Interface quality (I(X;T), I(T;Y)) + architecture check |
| `improve-codebase-architecture` | Deepening candidates | Seam placement + adapter count + depth assessment |

### When invoked standalone:

User says "auto-grill this" or "revisa esta propuesta" → runs on the provided document.

---

## Process

### Phase 1: Context Loading (parallel)

```
Load in parallel:
├── Read CONTEXT.md → domain glossary, relationships, flagged ambiguities
├── Read docs/adr/*.md → existing decisions
├── cognicode_build_graph() → prerequisite (skip if already warm)
└── Load input artifact (exploration/proposal/design)
```

### Phase 2: Question Generation

Parse the input document for:
- **Claims**: "AppState is a god object" → verify against code
- **Decisions**: "Split into facades" → check against ADRs, entropy
- **Terms**: "WorkflowFacade" → check against CONTEXT.md
- **Relationships**: "WorkflowNavigator determines next stage" → verify in code
- **Assumptions**: "Tests exist for this area" → verify in filesystem

For each, generate a verification question.

### Phase 3: Auto-Resolution Loop

For each question, apply the decision tree:

```
QUESTION
│
├── Answer in CONTEXT.md? → YES → auto-resolve (confidence: 1.0)
│
├── Answer in an ADR? → YES → auto-resolve (confidence: 1.0)
│
├── Codebase provides single consistent answer?
│   ├── YES → auto-resolve (confidence: 0.9-1.0)
│   └── CONFLICT → auto-resolve with flag (confidence: 0.7-0.9)
│
├── Entropy metrics determine answer?
│   ├── YES (unambiguous) → auto-resolve (confidence: 0.8-0.9)
│   └── BORDERLINE → escalate with recommendation (confidence: 0.5-0.7)
│
└── Domain/business decision? → ESCALATE to Phase 4
```

**Tools used for auto-resolution:**
- `cognicode_find_usages` → rename propagation count → I(Name)
- `cognicode_get_call_hierarchy` → dependency depth → I(Type)
- `cognicode_analyze_impact` → blast radius → risk level
- `cognicode_trace_path` → data flow verification
- `cognicode_check_architecture` → cycle detection, architecture score
- `cognicode_semantic_search` → pattern consistency
- `entropy-sdd` Protocol A → connascence landscape
- `grep` / `glob` → filesystem verification

### Phase 4: Enriched Escalation

For each question that cannot be auto-resolved:

**4a. Generate 3-5 viable options** using LLM + codebase validation:
- LLM proposes options
- CogniCode validates viability (impact analysis, trace path)

**4b. Evaluate each option with Opportunity Score (OS)**

See [OPPORTUNITY-SCORE.md](OPPORTUNITY-SCORE.md) for the full framework.
See [os-calc.py](os-calc.py) for the calculation script.

The LLM ESTIMATES the 6 dimensions per option, then **os-calc.py computes the OS**.
The LLM never calculates OS manually — always use the script.

Usage:
```bash
python3 os-calc.py --options '[
  {"name":"A: Fachadas","coupling":0.15,"free_energy":0.10,"openness":0.85,"flexibility":0.88,"depth":0.90,"irreversibility":0.25},
  {"name":"B: Split por crate","coupling":0.30,"free_energy":0.20,"openness":0.60,"flexibility":0.55,"depth":0.70,"irreversibility":0.60},
  {"name":"C: Registry","coupling":0.45,"free_energy":0.40,"openness":0.90,"flexibility":0.35,"depth":0.50,"irreversibility":0.10}
]'
```

Output: ranked table with OS scores and ratings.

**4c. Classify opportunities by type:**
- 🎯 **Funcional**: capacidades que el usuario final percibe
- 🔧 **Técnico**: mejoras internas (observabilidad, performance, testabilidad)
- 💼 **Negocio**: impacto en métricas de negocio (latencia, coste, UX)

**4d. Rank and recommend:**
- Sort options by OS (descending)
- Agent recommends highest OS option
- Present alternatives with trade-off analysis

### Phase 5: HTML Report + Documentation Updates

**MANDATORY — produce the HTML report.**

Write a self-contained HTML file. See [HTML-REPORT.md](HTML-REPORT.md) for the full scaffold.

**Report placement**: Always write to `/tmp/sdd-{change-name}-auto-grill.html` for immediate user display. Return the path in the output envelope.

The report has 4 sections:

1. **Resumen ejecutivo**: stats (preguntas totales, auto-resueltas, escaladas, tasa)
2. **Decisiones automáticas**: tabla con pregunta + resolución + evidencia + confianza
3. **Decisiones escaladas**: opciones rankeadas con OS + oportunidades por tipo
4. **Validación pendiente**: checkboxes para que el usuario apruebe/rechace

**Documentation updates (inline, like grill-with-docs):**
- New term resolved? → Update CONTEXT.md
- Fuzzy term sharpened? → Update CONTEXT.md
- Hard-to-reverse decision? → Create ADR
- Flagged ambiguity? → Update CONTEXT.md "Flagged Ambiguities"

After writing the HTML, open it and present to the user for review.

---

## Output Contract

Return to the orchestrator:

```markdown
## Auto-Grill Results

**Input**: {what was grilled}
**Preguntas**: {N} | **Auto-resueltas**: {M} ({rate}%) | **Escaladas**: {K}
**Reporte**: `$SDDK_DATA_DIR/projects/{project_id}/changes/{change_name}/auto-grill-report.md`
**Reporte temporal**: /tmp/sdd-{change-name}-auto-grill.html

### Auto-Resolved Decisions
{Summary table — details in HTML}

### Escalated Decisions (require human validation)
{Summary table with OS scores — details in HTML}

### Documentation Updates
- CONTEXT.md: {list of changes}
- docs/adr/: {list of new ADRs}

### Status
{all_resolved | pending_validation}
```

---

## Integration with SDD Phases

Auto-grill runs as a sub-step within existing phases:

### In sddk-explore (Step 3.5)
```
After investigating codebase, before analyzing options:
- Auto-grill the exploration findings
- Verify connascence claims
- Surface conflicts with CONTEXT.md
- Add Auto-Grill Results section to exploration output
```

### In sddk-propose (Step 3.5)
```
After reading existing specs, before writing proposal:
- Auto-grill the proposal intent
- Challenge scope decisions against ADRs
- Verify entropy budget predictions
- Escalated decisions become "Open Questions" in proposal
```

### In sddk-design (Step 2.5)
```
After reading codebase, before writing design:
- Auto-grill the design approach
- Verify interface quality (I(X;T), I(T;Y))
- Challenge architecture decisions
- Escalated decisions become "Open Questions" in design
```

### In improve-codebase-architecture (after Step 2)
```
After HTML report, before grilling loop:
- Auto-grill each candidate
- Pre-resolve structural questions
- Enrich escalated questions with OS scores
- Human grilling loop focuses ONLY on escalated decisions
```

---

## Compact Rules

- **auto-grill is MANDATORY when invoked** — never skip when an SDD phase activates it
- **All output in Spanish** — no exceptions
- **Always produce HTML report** — this is the primary artifact
- **Always report method**: `CogniCode` or `Heuristic` for each decision
- **Always report confidence**: 0.0-1.0 for each decision
- **Escalations MUST have Opportunity Score** — never present bare questions
- **Opportunities MUST be classified**: Funcional / Técnico / Negocio
- **OS < 0.3 = CRITICAL** — option should be flagged as poor
- **I(A;B) > 3.0 bits = auto-flag** as HIGH connascence in options
- **Documentation updates are inline** — don't batch, update as decisions crystallize
- **User validates ALL decisions** — auto-resolved are pre-approved, escalated need explicit approval

## Related Tools

| Tool | Purpose | When |
|------|---------|------|
| [os-calc.py](os-calc.py) | Opportunity Score para opciones escaladas | Phase 4: enriched escalation |
| [adversarial-metrics.py](adversarial-metrics.py) | AES para deficiencias del juicio adversarial | sddk-verify Step 7b |
| [ADVERSARIAL-ENTROPY.md](ADVERSARIAL-ENTROPY.md) | Framework completo de métricas adversariales | Referencia para jueces |
