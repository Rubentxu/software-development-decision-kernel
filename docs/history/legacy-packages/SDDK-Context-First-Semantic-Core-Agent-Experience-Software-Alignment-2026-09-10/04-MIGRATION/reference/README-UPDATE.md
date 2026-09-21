# Proposed repository README architecture update

Add a concise “Architecture and roadmap” section that points to exactly one canonical entry point and explains the four authorities:

```markdown
## Architecture and roadmap

SDDK is a deterministic decision kernel for agent-assisted software development.

Current architecture and roadmap: `docs/architecture/README.md`.

Core rules:
- canonical state is Facts + immutable Objects;
- graphs/views/indexes/context are derived;
- side effects require Authority admission;
- agent prompts/skills/CLI cheat sheets are compiled from typed contracts and do not own architecture.

Historical evolution packs remain available for rationale, but are not normative.
```

Do not place milestone detail or duplicated command cheat sheets in the root README. Generate command examples from CommandRegistry into dedicated reference docs.
