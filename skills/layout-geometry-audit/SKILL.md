---
name: layout-geometry-audit
description: Trigger: alignment issue, spacing issue, button inconsistency, broken cards, odd layout, overflow, geometry audit. Audits UI layout objectively using bounding boxes, gaps, dimensions, and viewport-fit assertions.
license: MIT
metadata:
  author: OpenCode
  version: "2.0"
---

## Activation Contract

Use when the question is whether a layout is objectively aligned, spaced, clipped, or visually inconsistent.

## Hard Rules

- Prefer measured geometry over opinion.
- Report the specific element group or selector when possible.
- Use tolerances intentionally; do not label 1px noise as a defect unless it is repeated and meaningful.
- Treat viewport overflow as a real bug unless explicitly intended.

## Measurement Targets

Measure:

- left/right edges of sibling cards
- gap consistency in button rows and form controls
- equal heights for equivalent controls
- overflow width vs viewport width
- text container width vs rendered content
- overlay position relative to viewport edges
- visible clipping on headers, CTAs, list rows, or dialogs

## Suggested Tolerances

- edge alignment: `<= 2px`
- control height consistency in same row: `<= 2px`
- gap consistency for repeated siblings: `<= 4px`
- viewport fit: no horizontal overflow

## Execution Steps

1. Capture a snapshot with boxes or gather `boundingBox()` data.
2. Group elements that should behave consistently.
3. Compare dimensions, offsets, and gaps.
4. Check for overflow, clipping, or overlap.
5. Translate measurements into a concrete finding.

## Output Contract

For each issue return:

- affected area
- measured problem
- expected rule
- severity
- recommended fix

## References

- `../ui-audit-protocol/references/evidence-checklist.md`
