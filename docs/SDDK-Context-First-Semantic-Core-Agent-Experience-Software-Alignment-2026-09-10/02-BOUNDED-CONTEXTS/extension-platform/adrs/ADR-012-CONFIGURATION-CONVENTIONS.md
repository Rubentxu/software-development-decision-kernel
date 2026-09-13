# ADR-012 — Convention-first hierarchical configuration

**Status:** Proposed

## Decision

Configuration precedence is deterministic:

```text
built-in defaults
< user/global
< project
< scoped module/pack
< local ignored override
< environment
< CLI
```

Every effective setting is explainable with source provenance (`sddk config explain <key>`).

A lock file may pin schema/pack/workflow/policy contract versions, never secrets.

## Principle

Prefer useful defaults and discovered conventions; require configuration only when ambiguity or risk is material.
