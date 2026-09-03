# Plan de implementación

## Slice 0 — Contract reconciliation
Cambios sólo docs/tests:
- authority matrix;
- prompt lint rule;
- status-query fix;
- golden contradictory fixture.

## Slice 1 — Domain types
Añadir tipos puros a `sddk-domain::interaction`.

Evitar persistence/CLI hasta tener schemas y tests.

## Slice 2 — CurrentRunView
Implementar application service que consuma state/artifacts existentes.
No renderizar personalidad todavía.

## Slice 3 — Neutral/Novice StageReport
Renderer determinista mínimo.
Dogfood 3 ciclos.

## Slice 4 — Resume
Cold-start/resume desde CLI + artifacts.
Dogfood reiniciando proceso/chat.

## Slice 5 — Decision/Reframe/Assumption
Añadir records y `HumanDecisionPort`.
Integrar balanced policy.

## Slice 6 — Semantic commands
Preferir:
- `sddk status` existente como base;
- `sddk explain --why <id|topic>`;
- `sddk decisions`;
- `sddk memory`.

No crear alias redundantes si el editor puede mapear `/where` a `sddk status`.

## Slice 7 — Audience/persona
Añadir transforms.
Golden tests de parity antes de habilitar preset por defecto.

## Slice 8 — Preferences
Store XDG + commands.
Sin Engram obligatorio.

## Slice 9 — Telemetry
Friction events → control plane.
Dashboard y F3.

## Slice 10 — UAT integration
Reuse `HumanDecisionPort` en UAT Guided Runner/Review Queue.

## Slice 11 — Stabilize
Docs, migrations, release, dogfood report.

## Regla de slice

Cada slice:
- compila;
- tiene tests;
- no deja placeholder;
- genera artifact/report esperado;
- mantiene trunk/release contract;
- tiene rollback sencillo.
