---
name: uat-ux-form
description: "Trigger: uat form, ux-form, enrich forms, enrich-forms, optimal interaction. Transform acceptance criteria into optimal UatFormSpec: blind observation vs. machine check vs. human confirmation."
disable-model-invocation: false
user-invocable: true
license: Apache-2.0
metadata:
  author: sddk-framework
  version: "1.0"
---

## Purpose

Transform acceptance criteria in a `uat-plan.yaml` into optimal `UatFormSpec` interaction patterns. For each scenario, decide: blind observation vs. machine check vs. human confirmation. Apply evidence requirements and branching rules per priority.

## Invocation

```bash
# Enrich an entire plan with forms (E14.3 primary use)
sddk uat enrich-forms --plan uat-plan.yaml --output uat-plan-enriched.yaml

# Test decision logic with a single criterion
sddk uat form \
  --criterion "After saving, the project appears in the project list" \
  --priority P0 \
  --context scenario-ctx.yaml

# Dry-run: show what forms would be generated without writing
sddk uat enrich-forms --plan uat-plan.yaml --dry-run --verbose
```

## Decision Rules (embedded)

The skill applies these rules deterministically (no LLM needed for the core logic):

### Machine verifiable?

Terms that indicate machine-verifiable: `visible`, `displayed`, `http`, `api`, `status`, `dom`, `aria`, `console`, `element exists`, `200`, `404`, `button`, `input`

If criterion contains these → use `machine` oracle (no human question).

### Blind observation?

Terms that suggest blind-friendly: `count`, `how many`, `appears`, `visible`, `displayed`, `shown`, `see`, `ves`, `veces`, ` número `, `list`

If machine can't verify AND criterion matches blind hints → use `blind_observation` (expected hidden from human).

### UX / Rating?

Terms that suggest subjective rating: `usability`, `usabilidad`, `ease`, `clarity`, `claridad`, `satisfaction`, `intuitive`, `design`, `experience`, `helpful`, `confusing`

If matches → use `human_rating` with scale 1-5 + `require_comment_below=3`.

### Evidence requirement

- P0/P1: `evidence_requirement.required = true`, `accepted: [screenshot, annotation]`
- P2: `evidence_requirement.required = false`, `accepted: [screenshot]`

### Branching

- P0: `on_fail.goto = diagnose` (requires a `diagnose-*` step or checkpoint)
- P1: `on_fail.goto = stop`
- P2: `on_fail.goto = stop`

### Checkpoints

Insert a `kind: checkpoint` item every 5 form items.

## Pipeline Integration

In the E14 pipeline (E14.5):

```
uat-planner → uat-ux-form → uat-form-quality → [loop if errors] → schema-validation
```

The `enrich-forms` command enriches `form.items[]` in each scenario. If a scenario already has a `form` with items, the skill merges (adds missing interaction types without removing existing human-designed items).

## Output

Modified `uat-plan.yaml` with `form.items[]` added to each scenario that doesn't have them, or enriched if they exist.

Each enriched scenario includes provenance:

```yaml
provenance:
  enriched_by: uat-ux-form
  enriched_at: "2026-08-11T12:00:00Z"
  decisions:
    - criterion_id: crit-1
      interaction_type: blind_observation
      reasoning: "Count is blind-friendly; DOM confirms"
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Enrichment complete |
| 1 | Validation error in generated forms |

## References

- `agents/uat-ux-form.md` — full agent definition with few-shot examples
- `agents/uat-form-quality.md` — quality gate (downstream)
- `agents/uat-planner.md` — scenario generator (upstream)
- `specs/E14-uat-guided-pipeline/E14.3-UX-FORM-AGENT.md` — full spec
- `crates/sddk-domain/src/uat.rs` — UatFormSpec definitive schema
