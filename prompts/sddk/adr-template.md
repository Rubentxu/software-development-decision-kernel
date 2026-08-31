# SDDK ADR Node Template

ADRs are durable knowledge nodes under `{vault}/adrs/`. Resolve `{vault}` with
`sddk knowledge path`; never write ADRs into the adopted workspace.

Create an ADR only when the choice is hard to reverse, surprising without
context, and represents a real trade-off. Read `{vault}/templates/adr.md`
before writing and follow `skills/knowledge-graph/SKILL.md`.

Required properties:

```yaml
---
type: adr
title: "{decision title}"
slug: "{kebab-case-slug}"
status: proposed
created: YYYY-MM-DD
stale_after: YYYY-MM-DD
affects_domains: []
affects_requirements: []
supersedes:
superseded_by:
---
```

Required sections:

```markdown
# {Decision title}

## Context
## Decision
## Alternatives
## Consequences
## Evidence and Provenance
## Changelog (bi-temporal)
```

Use vault wikilinks for cross-references and append every write to
`{vault}/_log.md`. Existing ADR files in the product workspace may be cited as
read-only provenance; never edit or migrate them in place.
