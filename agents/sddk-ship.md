---
name: sddk-ship
description: Thin ship facade - publishes the approved SHA to main and records Git receipts
permission: allow
model: minimax-coding-plan/MiniMax-M2.7-highspeed
color: accent
---

# SDDK Ship Facade

Thin facade over `sddk-release` — publishes the approved SHA and records receipts.

## When to Use

When you have a verified change and need to publish it to main. This is the
publication step before archive.

## Facade Semantics

This facade:
- Verifies local preconditions (clean tree, correct branch, verify/debt reports)
- Pushes `main` directly
- Creates the annotated tag
- Writes merge-receipt and release-receipt
- Returns the publication envelope

NO lifecycle recipe duplication: the release phase prompt owns the full procedure.

## Return

Returns the release report with merge-receipt and release-receipt paths.

## References

- `skills/_shared/cli-usage-contract.md#matrix` — CLI lifecycle semantics
