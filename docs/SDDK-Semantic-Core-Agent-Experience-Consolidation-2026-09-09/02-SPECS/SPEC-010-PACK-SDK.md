# SPEC-010 — Pack SDK and extension boundaries

## Extension points

A pack may provide versioned contributions for:

- schemas/node/relation namespaces;
- workflow definitions;
- Targets/Tasks;
- evidence validators;
- projection contributors;
- context contributors;
- policy rules;
- explanation/query contributors;
- CLI subcommands under its namespace only when a Target is insufficient.

## Isolation requirements

- no concrete SQLite dependency from pack domain;
- no direct writes to SemanticGraph storage;
- no additions to core closed enums solely for one pack;
- explicit pack compatibility range and schema version;
- disable/unload degrades gracefully when optional.

## Conformance suite

Every pack passes: manifest validation, deterministic registration, namespace collision tests, projection rebuild tests, disabled-pack behavior and Authority Engine bypass test.


## Agent Experience extension points

Packs MAY contribute namespaced:

```text
SkillDefinition
AgentProfile supplement (rare; prefer core roles + skills)
Task templates
ContextContributor
InstructionSource fragments
Command examples for commands/tasks the pack owns
Output/Evidence schemas
```

Packs MUST NOT grant Capabilities, override KernelInvariant/PolicyConstraint instructions, inject unregistered CLI syntax or mutate Decision Memory/projection storage directly.
