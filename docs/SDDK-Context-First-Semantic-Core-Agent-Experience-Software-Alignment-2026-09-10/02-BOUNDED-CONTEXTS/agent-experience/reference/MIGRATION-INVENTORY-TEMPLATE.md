# Agent asset migration inventory template

Use one row per active asset.

| Asset | Path | Owner today | Encodes role? | workflow? | CLI? | policy? | context? | output? | Disposition | Target contract | Removal trigger |
|---|---|---|---:|---:|---:|---:|---:|---:|---|---|---|
| example | `prompts/reviewer.md` | engine | yes | yes | yes | no | yes | yes | SPLIT | AgentProfile + Task + Skill + CommandSurface | parity UAT green |

## Required review question

> If this text disappeared tomorrow, which typed component would lose behavior?

If the answer is “the product would stop knowing how workflow/authority/CLI works”, the behavior is in the wrong layer.
