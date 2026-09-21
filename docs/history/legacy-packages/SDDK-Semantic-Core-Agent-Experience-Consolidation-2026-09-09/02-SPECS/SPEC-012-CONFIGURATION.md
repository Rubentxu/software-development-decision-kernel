# SPEC-012 — Configuration, conventions and lock semantics

## Project file

Recommended minimal `sddk.toml`:

```toml
[project]
kind = "rust-service"

[workflow]
default_target = "change"

[packs]
use = ["sdd", "uat"]

[memory]
hot_budget = 2000
context_budget = 16000

[policy]
profile = "team-default"
```

## Requirements

- deterministic precedence;
- `sddk config explain <key>` returns source chain;
- unknown keys fail or warn according to schema/version, never silently drift;
- secrets use external secret providers/environment, not lock/config files;
- `sddk.lock` optionally pins pack/workflow/policy/schema contract versions;
- scoped configuration inherits from project and can only override declared keys.


## Agent configuration

Configuration may select provider/model routing, enable/disable Skills, choose AgentProfile defaults and add project instruction sources. Resolved configuration is immutable/hashable for a Run/execution and referenced by AgentExecutionReceipt.

Secrets never enter instruction/cheat-sheet material. Provider credentials remain in secret handling boundaries.
