# ADR-019 — Workflow Self-Discovery, Indexing and Dynamic Extension

**Status:** Proposed · **Date:** 2026-08-16

## Context

SDDK has 4 canonical workflows (B-direct, A-min, A-lite, A-full) hardcoded in the orchestrator. Adding a new workflow requires editing the orchestrator prompt. There's no registry analogous to skill-registry.

## Decision

Adopt the **Workflow Self-Discovery Pattern** following skill conventions.

### Workflow Directory Structure (mirror of `skills/`)

```
workflows/                           ← NEW: extension surface
├── sddk-kernel/                    ← Engine FSM (formerly workflow/workflow.yaml)
│   ├── WORKFLOW.yaml
│   └── references/                  ← Sub-workflows (the 4 canonical paths)
│       ├── sddk-a-min/WORKFLOW.yaml
│       ├── sddk-a-lite/WORKFLOW.yaml
│       ├── sddk-a-full/WORKFLOW.yaml
│       └── sddk-b-direct/WORKFLOW.yaml
├── sddk-b-research/                 ← Independent workflow
│   ├── WORKFLOW.yaml
│   ├── references/                  ← Sub-workflows scoped here
│   └── assets/
└── _shared/                         ← Cross-workflow conventions (not invokable)
```

### Pattern parity with skills

- `workflows/sddk-kernel/WORKFLOW.yaml` (engine) → `Cargo.toml`/`package.json`
- `workflows/sddk-kernel/references/<path>/` → `docs/` (internal, not separately published)
- `workflows/sddk-b-research/WORKFLOW.yaml` → `skills/<name>/SKILL.md` (discoverable)
- `workflows/_shared/` → `skills/_shared/` (not invokable)

### Sub-workflows

Live inside parent's `references/`. Not independently registered. If shared across workflows → move to `_shared/`.

### Workflow Metadata

```yaml
schema_version: 1
workflow:
  id: sddk-b-research
  version: "0.1.0"
meta:
  name: "Deep Research"
  trigger:
    goal_pattern: ["research", "investigate"]
    context_quality: [C0, C1, C2]
    domains: [any]
  rationale: "..."
  precedence: 50
  requires:
    agents: [deep-research-orchestrator]
    skills: [...]
  provides: [research-report, evidence-cards]
  provenance: {...}
phases: [...]
```

### CLI Surface

| Command | Function |
|---------|----------|
| `sddk workflow list` | List workflows found under `workflows/` |
| `sddk workflow show <name>` | Print resolved YAML with provenance |
| `sddk workflow validate <path>` | Schema + engine validation |
| `sddk workflow write-registry` | Mirror skill-registry: scan, dedupe, render, XDG cache |

### Auto-Discovery

Create `skills/_shared/workflow-resolver.md` mirroring `skill-resolver.md`.

### Pack-Bundled Workflows

Packs MAY include `workflows/<name>/`. Pack validator detects them.

### Dynamic Composition as Fallback

`/sddk-custom <goal>` implemented; generates workflows under `workflows/<name>/` with provenance.

## Consequences

### Positive
- Workflows follow skill pattern exactly.
- Self-describing with meta block.
- Sub-workflows scoped to parent.
- Cross-workflow shared via _shared/.
- Packs can ship workflows.

### Negative
- Migration cost (4 workflows restructured).
- Sub-workflow discoverability (only via parent).

## Implementation (4 phases)

- **Phase 1 (SDDK2-906)**: Schema + canonical files. Move 4 canonical to `workflows/sddk-kernel/references/`. Create `sddk-b-research/`.
- **Phase 2 (SDDK2-907)**: CLI + workflow-registry + resolver.
- **Phase 3 (SDDK2-908)**: Pack integration.
- **Phase 4 (SDDK2-909)**: `/sddk-custom` command.

## Compatibility

Engine decoupling preserved: `workflow/workflow.yaml` (engine) and `workflows/` (extension) are separate files. 4 canonical paths inside engine; new independent workflows in `workflows/`.

## References

- `schemas/workflow.schema.json`
- `prompts/sddk/dynamic-workflow.md`
- `crates/sddk-cli/src/dev/registry.rs`
- `skills/_shared/skill-resolver.md`
