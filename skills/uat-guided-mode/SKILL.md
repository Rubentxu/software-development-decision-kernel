---
name: uat-guided-mode
description: "Trigger: uat-guided-mode, modo guiado, guion para torpes. Patterns for the junior-guided UAT wizard: one scenario per screen, pre-written steps, PASS/FAIL/BLOCKED, evidence paste, no getting lost."
disable-model-invocation: true
user-invocable: false
license: MIT
metadata:
  author: sddk-framework
  version: "1.0"
  delegate_only: true
---

> **ORCHESTRATOR GATE**: If you loaded this skill, STOP. Delegate to `uat-guide`.

## Purpose

The "guion para torpes": a wizard a junior can complete alone. One scenario per screen, steps pre-written by `uat-guide`, big verdict buttons, screenshot paste with Ctrl+V, visible progress.

## Wizard UX contract

1. **One scenario per screen.** Never show the matrix to a junior first.
2. **Step 1/4 → 2/4** indicator always visible; `← Anterior / Siguiente →` at the bottom.
3. **`plain_steps` rendered one per screen**: action (with copy button when `copy_hint`), then `Esperado:`.
4. **`rationale` shown as a callout** ("Por qué importa") — keeps the tester engaged and catches wrong assumptions.
5. **Verdict buttons**: ✅ Pasó / ❌ Falló / ⏸ Bloqueado — one tap, immediately persisted to localStorage.
6. **Evidence prompt shown before the verdict row**: "Evidencia requerida: <evidence_prompt> (pega screenshot con Ctrl+V)".
7. **Progress bar + counter** ("Progreso: 12/20 escenarios valorados") — a junior must never wonder where they are.
8. **Free-text comment box** per scenario (optional).

## Data contract

The wizard reads `plain_steps`, `rationale`, `evidence_prompt`, `priority`, `title` from `uat-plan.yaml`. If a field is missing, the wizard degrades gracefully (skips the callout, shows the step only).

## References

- `agents/uat-guide.md` — the field author
- ADR-012 §7 (guided mode design) in the knowledge vault
