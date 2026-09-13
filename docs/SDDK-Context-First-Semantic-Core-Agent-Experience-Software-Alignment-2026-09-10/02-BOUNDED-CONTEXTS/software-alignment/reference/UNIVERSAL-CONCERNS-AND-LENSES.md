# Universal concerns and paradigm lenses

## Universal layer

The core asks questions that make sense in many paradigms:

- Is responsibility understandable?
- Are changes local or surprisingly coupled?
- Are boundaries explicit?
- Is state complexity controlled?
- Are effects and failures visible?
- Are invariants documented and testable?
- Does knowledge match current software?

## Lenses

Lenses answer those questions using paradigm-specific vocabulary.

### OO lens
SOLID/GRASP, interfaces, polymorphism, encapsulation, responsibility clusters.

### Functional lens
purity/effects, totality, ADTs, composition, explicit state transitions.

### Hexagonal lens
ports/adapters, dependency direction, side-effect boundaries.

### DDD lens
bounded contexts, ownership, context maps, ubiquitous language/semantic leakage.

### Data-oriented/ECS lens
layout/data flow/systems/components, locality and mutation boundaries.

No lens is universally correct. Project decisions override generic heuristic preferences.
