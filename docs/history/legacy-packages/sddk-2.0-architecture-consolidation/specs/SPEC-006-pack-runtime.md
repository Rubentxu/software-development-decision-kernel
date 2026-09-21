# SPEC-006 — Pack Runtime and Modularity

**Status:** Proposed

## 1. Goal

Turn the existing PackManifest direction into the primary mechanism for keeping SDDK extensible without expanding the kernel.

## 2. Pack categories

- **core/kernel:** runtime contracts only;
- **infrastructure:** evidence, tool/capability adapters, identity bridges, storage projections;
- **domain:** UAT, architecture, testing, research, docs;
- **bridge:** translates foreign ontology/events to SDDK core primitives;
- **bundle:** named composition of packs, not a new ontology.

## 3. Dependency semantics

A pack manifest MUST distinguish:

- `requires`: hard dependency; pack cannot load without it;
- `integrates_with`: optional capability that improves behavior; absence must degrade gracefully;
- `conflicts_with`: explicit incompatible combination;
- `provides`: capabilities/event schemas/view types exported by the pack.

## 4. Pack contents

A pack MAY include:

- domain schemas/types;
- behaviors;
- agent definitions;
- prompts/skills;
- policy fragments;
- tools/capability declarations;
- projections/views;
- fixtures;
- docs;
- migrations.

Every pack MUST include deterministic validation metadata and content hashes.

## 5. Suggested first-party packs

```text
sddk-pack-sdd
sddk-pack-uat
sddk-pack-testing
sddk-pack-architecture
sddk-pack-research
sddk-pack-docs
sddk-pack-ui
sddk-pack-cognicode
sddk-pack-auto-grill
```

Not all must ship in the first 2.0 release. UAT is the recommended first extraction because it already has substantial domain depth and UI.

## 6. Bridge pattern

Example: Cognicode retains its own rich code graph, while `sddk-bridge-cognicode` maps selected outputs to universal SDDK types such as artifact, observation, evidence and decision. This prevents the SDDK kernel from absorbing language-specific AST ontology.

## 7. CLI

Target commands:

```text
sddk pack list
sddk pack inspect <id>
sddk pack install <source>
sddk pack verify <path>
sddk pack enable <id>
sddk pack disable <id>
```

## 8. Core-stays-small invariant

Adding a new domain noun to core requires an ADR that proves it is cross-domain, stable and impossible to model as a layered pack. Default decision is rejection.
