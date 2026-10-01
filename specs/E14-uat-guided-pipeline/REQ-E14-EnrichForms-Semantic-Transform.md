---
type: requirement
title: "REQ-E14-EnrichForms-Semantic-Transform"
slug: "REQ-E14-EnrichForms-Semantic-Transform"
domain: "[[E14-uat-guided-pipeline]]"
status: active
created: 2026-08-12
created_in_cycle: "[[CYC-2026-08-12-e14-uat-guided-pipeline-stabilization-deferred]]"
last_modified_cycle: "[[CYC-2026-08-12-e14-uat-guided-pipeline-stabilization-deferred]]"
last_modified_version: v1.9.1
decision_authority: "TBD by sddk-design"
tested_by: "sddk uat enrich-forms + integration test test_uat_enrich_forms_*.rs"
verified_in_cycle:
incidences: []
rfc2119: MUST
stale_after: 2026-11-10
---

# REQ-E14-EnrichForms-Semantic-Transform

## Requirement

The `sddk uat enrich-forms` subcommand MUST NOT emit generic
`Info + Check(prompt: "Verify this scenario passes")` items. For each
scenario without a `form` spec, it MUST decide an optimal interaction
set per the decision tree in `agents/uat-ux-form.md`:

- **machine**: HTTP / DOM / JSON / console / file verifiable; no
  human prompt required.
- **blind_observation**: human reads / counts without seeing
  `expected`; `visibility: blind`.
- **human_confirmation**: open evaluation; `expected` visible.
- **human_rating**: UX / subjective; `scale: {min, max, anchors}`
  required, `require_comment_below` set.
- **form_action**: when the criterion involves user action
  (click, form submit, typing); includes field checks + oracle +
  human confirmation.

Each scenario's enriched form MUST:
- Include a `checkpoint` every 5 items when total > 5.
- Set `evidence_requirement: [Screenshot]` for blocking P0/P1
  checks.
- Keep `visibility` consistent (`blind_observation` ⇒
  `visibility: blind`).
- Emit provenance per scenario:
  `{ generated_by: "uat-ux-form", model, based_on:
  [scenario_id, ...], confidence: number, human_reviewed: false }`.

The agent MUST NEVER generate HTML / JS; output is YAML / JSON only.

## Scenarios

### Scenario: HTTP-verifiable criterion emits machine oracle only
- **GIVEN** a scenario `criterion: "API returns 200 on /health"`
- **WHEN** `sddk uat enrich-forms --plan p.yaml` runs
- **THEN** the enriched plan adds an item with `oracle.kind = http,
  target: /health, expected: 200`, blocking=true

### Scenario: UX criterion emits rating with scale
- **GIVEN** a scenario `criterion: "Error message is helpful"`
- **WHEN** enrichment runs
- **THEN** the enriched form includes `kind: human_rating` with
  `scale: {min:1, max:5, anchors:{1:Confuso, 3:Neutral, 5:Claro}}`
  and `require_comment_below: 3`

### Scenario: Long scenario inserts checkpoint
- **GIVEN** a scenario with 8 items after enrichment
- **WHEN** enrichment runs
- **THEN** at least one `kind: checkpoint` item is present
  between items 5 and 6

### Scenario: Blind visibility matches blind_observation kind
- **GIVEN** a scenario enriched with `kind: blind_observation`
- **WHEN** the form is serialized
- **THEN** `visibility == "blind"` and `check.expected` is hidden
  from the rendered prompt

### Scenario: Provenance block per scenario
- **GIVEN** any scenario enriched
- **WHEN** the enriched plan is written
- **THEN** each scenario carries
  `provenance: { generated_by: "uat-ux-form", model, based_on:
  [...], confidence: 0..1, human_reviewed: false }`

## Traceability

- **Decision authority:** TBD by sddk-design
- **Created in cycle:** [[CYC-2026-08-12-e14-uat-guided-pipeline-stabilization-deferred]]
- **Supersedes:** `specs/E14-uat-guided-pipeline/E14.3-UX-FORM-AGENT.md`
  (kept for history; the E14.3 ↔ E14.2 order contradiction is
  resolved by adopting E14.5's enrich→quality→schema→approval order
  and reinterpreting E14.3's "recibe" as the BLOCKER remediation
  feedback loop)
- **Last modified:** 2026-08-12
- **Tested by:** `sddk uat enrich-forms` + integration test in
  `tests/test_uat_enrich_forms_*.rs` (TDD, ≥1 test required)
- **Incidences:** none

## Changelog (bi-temporal)

- 2026-08-12T07:31 | created | cycle=[[CYC-2026-08-12-e14-uat-guided-pipeline-stabilization-deferred]] | valid_from=2026-08-12 | valid_to=∞
- 2026-08-12T07:31 | supersedes | E14.3-UX-FORM-AGENT.md | contradiction C-2 (FormQuality/UXForm order) resolved: canonical order is enrich→quality→schema→approval per E14.5; UX Form Agent "receives the quality report" only as a remediation loop on BLOCKER smells | valid_from=2026-08-12 | valid_to=∞