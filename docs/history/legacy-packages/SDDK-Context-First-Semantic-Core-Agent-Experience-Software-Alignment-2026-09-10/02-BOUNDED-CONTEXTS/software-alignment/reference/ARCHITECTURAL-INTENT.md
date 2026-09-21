# Architectural Intent

ArchitecturalIntent is compiled from existing sources rather than authored as a second architecture file.

Possible contributors:

- module/context declarations;
- ADR/Decision Memory;
- dependency rules;
- invariants;
- active tradeoffs and waivers;
- architecture profiles/lenses;
- project-specific conventions.

The compiler should explain provenance:

```text
application must not import infrastructure
  <- project architecture contract ARCH-04
  <- Decision DEC-21
```

Generic lens advice has lower precedence than explicit intent.
