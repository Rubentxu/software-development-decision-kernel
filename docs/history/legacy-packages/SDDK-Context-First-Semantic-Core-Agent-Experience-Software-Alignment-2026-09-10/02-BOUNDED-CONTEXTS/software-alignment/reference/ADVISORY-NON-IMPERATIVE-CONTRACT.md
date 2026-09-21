# Advisory / non-imperative contract

Agent-visible Alignment data MUST be rendered with semantic labels:

```text
FACT
OBSERVATION
ASSESSMENT
DECISION
SUGGESTION
UNKNOWN
```

Example:

```text
OBSERVATION
planner.rs imports SqliteStore

ASSESSMENT [hexagonal]
possible dependency-direction tension

DECISION
DEC-18 accepts direct dependency during migration

SUGGESTION
if this boundary changes, revisit DEC-18 assumptions
```

Forbidden presentation:

```text
YOU MUST refactor planner.rs because SOLID says so.
```

Only explicit project contracts can appear as MUST/MUST NOT obligations.
