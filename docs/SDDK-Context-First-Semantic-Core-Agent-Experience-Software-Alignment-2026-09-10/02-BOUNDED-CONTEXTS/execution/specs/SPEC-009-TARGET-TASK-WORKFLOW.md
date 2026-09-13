# SPEC-009 — Target/Task workflow model

## Goal

Reduce CLI bureaucracy by grouping lower-level capabilities into declarative, explainable execution graphs inspired by Maven lifecycle phases and Gradle task graphs.

## Target

A named user intent such as `change`, `verify`, `ship`, `recover`, `audit`.

## Task contract

Each Task declares:

```text
id
inputs
outputs
depends_on
side_effect_class
authority_requirement
determinism
cacheability
retry_policy
evidence_contract
memory_effects
```

## UX

```text
sddk run verify --dry-run

context.resolve        UP-TO-DATE
plan.validate          UP-TO-DATE
build                  EXECUTE
test                   EXECUTE
assurance              WAITING
evidence.collect       WAITING
```

## Rules

- target names are not domain `Goal`s;
- target resolution is deterministic under effective config/pack versions;
- LLM execution output is never silently reused as deterministic cache truth;
- side-effect tasks route through Authority Engine.
