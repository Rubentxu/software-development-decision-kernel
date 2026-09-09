# Agent-facing asset migration plan

## Scope

Inventory and migrate all behavior-bearing assets, including:

```text
AGENTS.md and nested agent instruction files
prompts/**
agents/**
skills/**
templates/**
rules/**
instructions/**
pack-provided agent assets
embedded prompt strings in Rust
CLI examples copied into docs/prompts
provider-specific tool descriptions
```

## Classification

Every asset receives exactly one disposition:

| Disposition | Meaning |
|---|---|
| KEEP | already fits a stable typed contract |
| MIGRATE | same responsibility, new schema/compiler path |
| SPLIT | monolith contains multiple responsibilities |
| ABSORB | responsibility moves into core Task/Policy/Command/Context contract |
| DEPRECATE | compatibility only for defined window |
| REMOVE | redundant/unsafe/no remaining owner |

## Smells to detect automatically

- hard-coded deprecated CLI command names;
- direct mention of internal SQLite/table/store paths;
- Vault described as authority or memory;
- raw ledger reads prescribed to ordinary agents;
- Cycle runtime states used after Run consolidation;
- instructions that grant/assume permission;
- duplicated architecture conventions across multiple skills;
- free-form output where Contribution/ExecutionOutcome schema exists;
- prompt text implementing workflow transitions;
- undocumented provider-specific behavior;
- command examples not present in CommandRegistry.

## Migration sequence

```text
Inventory
  ↓
Extract semantic responsibilities
  ↓
Move workflow logic → Target/Task
Move policy/permission → Authority/Policy
Move context selection → ContextCompiler
Move procedural method → SkillDefinition
Move role semantics → AgentProfile
Move CLI syntax/examples → CommandRegistry
  ↓
Compile EffectiveInstructions
  ↓
Parity/UAT against representative tasks
  ↓
Deprecate/remove old prompt asset
```

## No transcript dependency

Long-lived recovery should use Decision Memory + canonical facts + Evidence + ContextCompiler. Chat transcripts may be retained externally for UX/history, but are not the durable agent execution contract.

## Definition of done per migrated asset

- owner and contract identified;
- no internal-store authority assumption;
- provider-neutral semantics where possible;
- examples tested;
- output schema validated;
- authority requirements explicit;
- old path has deprecation/removal disposition;
- execution provenance can identify the migrated version/hash.
