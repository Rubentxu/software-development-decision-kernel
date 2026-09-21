# CLI UX — porcelain for humans, contract surface for agents

## Daily human commands

```text
sddk status
sddk plan
sddk run change
sddk run verify
sddk run ship
sddk next
sddk why <entity>
sddk diff
sddk memory ...
sddk context ...
sddk doctor
```

## Advanced/plumbing groups

Low-level capabilities remain for scripts/debugging but are not taught as the primary workflow:

```text
ledger, artifact, capability, git, graph, fork,
pack, telemetry/observability, raw authority diagnostics
```

## Agent contract surface

Agents should normally receive command contracts directly from the execution adapter. Diagnostic/export UX:

```text
sddk help agent
sddk help agent --target verify
sddk help agent --target verify --format json
sddk context explain --instructions --commands
```

`help agent` is generated from CommandRegistry. It is not a second handwritten documentation source.

## `next`

Composes Run state, ExecutionFrontier (legal), ContinuationOptions (advisory), blockers/pending authority and context/memory freshness. Legal vs advisory is always labelled.

## `why`

One top-level explanation entry point with typed profiles rather than proliferating graph/debt/decision-specific public command families.

## Machine output

Agent/script-facing commands expose stable machine schemas. Human text is presentation and must not be parsed by agents when JSON/schema output exists.

## Progressive disclosure

Default output concise; `--explain` adds rule/provenance; JSON exposes stable contract; verbose diagnostics are optional. Routine agents receive only relevant command subset and examples.
