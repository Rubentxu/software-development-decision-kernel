# SPEC-001 — Human Interaction Plane

## Purpose
Definir el bounded context responsable de convertir estado/evidencia SDDK en comprensión humana.

## Inputs
Runtime state, artifacts, decisions, project knowledge, interaction profile.

## Outputs
InteractionEvent, StageReport, DecisionRequired, ResumeSummary.

## Invariants
- no lifecycle mutation;
- no invented state;
- no personality semantics in domain;
- failure to render != lifecycle failure, pero must surface degraded presentation.

## Acceptance
Given a valid cycle state, when BuildCurrentRunView executes, then all visible lifecycle facts are traceable to an authority reference.
