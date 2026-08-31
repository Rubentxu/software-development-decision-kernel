---
name: uat-form-quality
description: "Trigger: uat quality, quality gate, anti-test-smells, smell audit, validar UAT. Run the UAT Form Quality Agent against a uat-plan.yaml to detect test smells and gate the pipeline."
disable-model-invocation: false
user-invocable: true
license: Apache-2.0
metadata:
  author: sddk-framework
  version: "1.0"
---

## Purpose

Run the UAT Form Quality Agent against a `uat-plan.yaml` to detect test smells (ambiguous instructions, missing expected results, leading questions, etc.). Produces a `UatFormQualityReport` in YAML. Gates the UAT pipeline based on severity threshold.

## Invocation

```bash
# Gate the pipeline with default threshold (BLOCKER)
sddk uat quality --plan uat-plan.yaml

# Show all warnings
sddk uat quality --plan uat-plan.yaml --threshold WARNING --verbose

# Standalone analysis
sddk uat quality --plan uat-plan.yaml --output quality-report.yaml
```

## Heuristic Detection (no LLM needed)

The skill runs these rule-based detectors against each scenario's `form.items[]`:

| Smell | Rule | Severity |
|-------|------|----------|
| `AMBIGUOUS_INSTRUCTION` | `/(?:correcto\|adecuado\|bien\|normal)/i` in action text | WARNING |
| `EXPECTED_ABSENT` | `kind == 'check'` without `expected` or `oracle` | BLOCKER |
| `MACHINE_OBSERVABLE` | question contains DOM/HTTP terms AND machine oracle exists | WARNING |
| `DUPLICATED_CHECK` | same `oracle`+`expected` in consecutive items | WARNING |
| `LEADING_QUESTION` | `/\bno\s+.*\?/i` or coaching phrases in question | WARNING |
| `SUBJECTIVE_NO_SCALE` | `human_confirmation` without `scale` | WARNING |
| `FAIL_NO_EVIDENCE` | `on_fail` present but `evidence_requirement.required != true` | BLOCKER |
| `STEP_TOO_LARGE` | action contains >3 comma-separated verbs | WARNING |
| `EXCESSIVE_STEPS` | >12 items in scenario with no checkpoint | WARNING |
| `HIDDEN_PREREQUISITE` | env var or config in action not in preconditions | WARNING |
| `NO_BRANCHING` | >3 error-visible outcomes with no `flow.goto` | WARNING |
| `BLIND_CHECK_WITHOUT_HIDDEN` | `blind_observation` with `visibility != 'blind'` | WARNING |

## Pipeline Integration

In the E14 pipeline (E14.5), this skill is the quality gate between `ux-form-agent` and `schema-validation`:

```
uat-planner → ux-form-agent → uat-form-quality → [gate] → schema-validation → human-approval
```

Gate behavior:
- `threshold: BLOCKER` (default): continue if no BLOCKER smells; stop if any BLOCKER found
- `threshold: WARNING`: stop if any WARNING or worse found

## Output

`UatFormQualityReport` written to:
- `uat-quality-report.yaml` in the same directory as the plan
- Or path specified by `--output`

Report contains: `smells[]` with `id, smell_id, severity, location, snippet, suggestion`, plus `summary` and `verdict`.

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Quality gate passed — no blocking smells at the applied threshold |
| 1 | Quality gate failed — BLOCKER/ERROR smells found or threshold exceeded |

## References

- `agents/uat-form-quality.md` — the agent definition with full smell catalog
- `agents/uat-ux-form.md` — the agent that remediates detected smells (E14.3)
- `specs/E14-uat-guided-pipeline/E14.2-FORM-QUALITY-AGENT.md` — full spec
