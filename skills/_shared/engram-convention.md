# Optional Engram Mirror Convention

Engram is parallel SDDK memory, never the authority for durable knowledge or
cycle artifacts. Use it only when `sddk knowledge status --root . --scope .
--format json` reports `engram_enabled: true`.

## Naming

```text
title:     sddk/{change-name}/{artifact-type}
topic_key: sddk/{change-name}/{artifact-type}
type:      architecture
project:   {Engram project resolved by Engram}
scope:     project
capture_prompt: false
```

Valid artifact types include `explore`, `proposal`, `spec`, `design`, `tasks`,
`apply-progress`, `verify-report`, `debt-report`, `archive-report`,
`release-report`, and `state`.

## Read Order

1. Read phase artifacts from `{cycle-artifacts-dir}`.
2. Read durable nodes from `{vault}`.
3. Use Engram only for optional recovery, search, and episodic context.

If an Engram result is used, retrieve full content with
`mem_get_observation`; never use a truncated search preview. A missing Engram
server or observation does not change the authoritative filesystem paths.

## Write

Write the authoritative artifact first, then optionally mirror it:

```text
mem_save(
  title: "sddk/{change-name}/{artifact-type}",
  topic_key: "sddk/{change-name}/{artifact-type}",
  type: "architecture",
  project: "{project}",
  capture_prompt: false,
  content: "{full artifact markdown}"
)
```

Same `topic_key` + project + scope is an upsert. This is useful for recovery,
but it is not an audit trail; the vault and XDG artifacts retain authority.
