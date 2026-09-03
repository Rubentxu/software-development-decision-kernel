# ADR-042 — Separate reasoning skills, semantic capabilities and technology profiles

**Status:** Proposed

## Context

Mixing how to reason, what work is requested, which technology rules apply and who executes the work produces giant skills and language-specific workflows.

## Decision

Use four layers:

```text
Reasoning Skill
      ↓
Semantic Capability
      ↓
Technology Profile
      ↓
Provider / Tool Route
```

- **Reasoning Skill:** how to think; no authority. Example `systems-reasoning`.
- **Semantic Capability:** stable workflow intent. Example `systems.review`.
- **Technology Profile:** contextual specialization. Example `engineering.rust.v1`.
- **Provider/Route:** agent, deterministic tool, human or service selected by normal routing.

## Rules

- deterministic repository signals take precedence over LLM guesses for profile resolution;
- polyglot scopes may activate several profiles;
- new profiles must not require kernel enum/operator changes;
- `rust-patterns` remains broad Rust knowledge; `rust-systems-reasoning` only adds assurance deltas.

## Consequence

SDDK stays technology-neutral while reviews become technology-aware.
