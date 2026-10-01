# Proposal: E14 — UAT Guided Pipeline (AI-Generated, Human-Validated)

## Intent

Cerrar los gaps residuales del milestone E13 (UAT v3) implementando el pipeline completo de generación asistida por IA: desde un requerimiento hasta un UAT guiado ejecutable, con el renderer determinista como única superficie de rendering. Los agentes especifican en DSL declarativo; nunca generan HTML/JS arbitrario.

## Scope

### In Scope
- E14.1: Evidence kinds `video` + `annotation` añadidos al DSL + UI en `evidenceCaptureUI()`
- E14.1: Inbox enriquecido con metadata completa (est_time, preflight checks, human judgment count, coverage)
- E14.2: Form Quality Agent — detecta anti-patrones de tests manuales (test smells, arXiv:2308.01386)
- E14.3: UX Form Agent — transforma criterio semántico en interacción óptima (blind observation vs. machine check vs. human confirmation)
- E14.4: Test Discovery Agent — usa Fara (CUA) para explorar la app real y generar UAT desde flujos descubiertos
- E14.5: Pipeline wiring — todos los agentes integrados en el flujo: ScenarioAgent → CoverageAgent → TestabilityAgent → FormQualityAgent → UXFormAgent → SchemaValidator → HumanApproval → Published

### Out of Scope
- Motor de ejecución automática de UAT (PlaywrightExecutor, ComputerUseExecutor — ya en backlog separado)
- Oracles deterministas adicionales (más allá de los ya definidos en el DSL v3)
- Integración conissue trackers externos (Jira, Linear) — futuro trabajo

## Approach

Arquitectura de pipeline agéntico inspirada en el §10-11 de GUIDED-UAT-DESIGN.md:

```
Requirement
    ↓
┌─────────────────┐
│ ScenarioAgent    │ (existente: uat-planner)
└────────┬────────┘
    ↓
┌─────────────────┐
│ CoverageAgent    │ (existente: gap analysis en uat-planner)
└────────┬────────┘
    ↓
┌─────────────────┐
│ TestabilityAgent │ (existente: uat-testability skill)
└────────┬────────┘
    ↓
┌─────────────────┐
│ FormQualityAgent │ (NUEVO: anti-test-smells)
└────────┬────────┘
    ↓
┌─────────────────┐
│ UXFormAgent     │ (NUEVO: criterion → optimal interaction)
└────────┬────────┘
    ↓
┌─────────────────┐
│ SchemaValidator  │ (existente: validate_form_dsl en domain)
└────────┬────────┘
    ↓
HUMAN APPROVAL
    ↓
Published UAT (con provenance + staleness tracking)
```

Para E14.4 (Test Discovery): Fara ejecuta acciones en la app real mientras Playwright observa el DOM y las API calls. El output es un "Actual Application Model" que alimenta al ScenarioAgent con flujos reales descubiertos (no inventados).

## Capabilities

### New Capabilities
- `uat-form-quality-agent`: Agent que analiza specs de UAT contra el catálogo de test smells y emite advertencias accionables
- `uat-ux-form-agent`: Agent que transforma una acceptance criterion semántica en una `UatFormSpec` óptima
- `uat-discovery-agent`: Agent que usa Fara + Playwright para explorar la app real y descubrir flujos de UI

### Modified Capabilities
- `UatFormEvidenceKind`: Añadir `video` y `annotation` al vocabulario cerrado
- `uat-planner`: Enriquecer con pipeline de generación asistida; añadir output de `UatFormSpec` por escenario
- `uat-guided-mode`: Soportar evidence kinds `video` y `annotation` en la UI

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/sddk-domain/src/uat.rs` | Modified | `UatFormEvidenceKind`: añadir `Video`, `Annotation` |
| `assets/uat-dashboard/kit/components.js` | Modified | `evidenceCaptureUI()`: video + annotation capture; `inboxView()`: richer metadata |
| `agents/uat-planner.md` | Modified | Añadir paso de pipeline + output `form` por escenario |
| `agents/uat-form-quality.md` | New | Form Quality Agent (anti-test-smells) |
| `agents/uat-ux-form.md` | New | UX Form Agent (semantic → optimal interaction) |
| `agents/uat-discovery.md` | New | Test Discovery Agent (Fara → discovered flows) |
| `skills/uat-form-quality/SKILL.md` | New | Skill para Form Quality Agent |
| `skills/uat-ux-form/SKILL.md` | New | Skill para UX Form Agent |
| `skills/uat-discovery/SKILL.md` | New | Skill para Test Discovery Agent |
| `skills/cua-test-orchestrator/SKILL.md` | Modified | Wire Test Discovery Agent a Fara CUA pipeline |
| `docs/uat/GUIDED-UAT-DESIGN.md` | Modified | Actualizar §10-11 con specs implementadas |
| `docs/sddk-stabilization-plan/BACKLOG.md` | Modified | Añadir Épica E14 con SDDK-1401..1407 |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Fara no disponible en el entorno | Medium | Test Discovery es opcional; el pipeline funciona sin él (requiere que uat-planner genere los flows manualmente) |
| Video capture overflow en storage | Low | Policy de retención en evidence; video solo cuando `evidence_kinds` lo incluye explícitamente |
| Over-engineering del pipeline | Medium | Empezar por E14.1 (impl directa), luego E14.2-14.3 con feedback real deldogfooding |

## Rollback Plan

Si el pipeline degrada la calidad del UAT generado:
1. `sddk uat plan --no-ai-assist` desactiva todos los agentes de pipeline y usa solo uat-planner básico
2. Los escenarios ya publicados con provenance retain su metadata; no se regeneran automáticamente
3. Las skills nuevas (`*-form-quality`, `*-ux-form`, `*-discovery`) se desactivan quitando los symlinks del bundle runtime

## Dependencies
- Fara 1.5 o equivalente (ComputerUseExecutor) corriendo en `localhost:8082` para E14.4
- Bundle runtime v1.9.1+ (base actual)
- Schema v3 del plan (ya implementado en E13)

## Success Criteria
- [ ] `video` y `annotation` son aceptados por `uat validate` y renderizados en la guided UI
- [ ] El inbox muestra: est_time, preflight_checks count, human_judgment count, coverage %
- [ ] Form Quality Agent detecta al menos 8 categorías de test smells y emite advertenciasataszoneadas
- [ ] UX Form Agent genera una `UatFormSpec` con al menos: blind observation + machine check + human confirmation por criterio
- [ ] Test Discovery Agent produce un "Actual Application Model" con los flujos descubiertos de la app real
- [ ] Pipeline completo: requirement → scenario candidate → form spec validada → human approval → published UAT
- [ ] 358+ tests workspace verde post-implementación; 0 clippy errors
