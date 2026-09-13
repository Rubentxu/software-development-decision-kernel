# Contextual CLI cheat sheets

## Why contextual

A reviewer should not receive release/fork/ledger plumbing by default. A release agent should not spend tokens on unrelated UAT internals.

## Resolver inputs

```text
Target/Task
AgentProfile
Enabled Packs
Project capabilities/platform
CommandRegistry
```

## Resolver output

`AgentCommandSurface` is a bounded projection with exact syntax/examples. It may include related commands that are currently unauthorized, but must label their effect/authority requirement.

## Recovery path

If runtime returns `UnknownCommandContract` or version mismatch, the adapter may refresh the surface or call the machine help/contract endpoint. Repeated help probing is treated as contract drift telemetry, not normal agent behavior.
