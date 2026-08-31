# SDDK Persistence Contract

This contract applies to every active SDDK agent, skill, and prompt.

## Directory Authorities

| Context | Authority | Write policy |
|---|---|---|
| Framework development repo | Current `sddk-framework/` checkout | Framework sources, commits, releases, and explicit `--in-repo` dogfooding only |
| Runtime bundle | `$SDDK_DATA_DIR/framework/<version>/` | Installed snapshot and editor links; never edit as source |
| Adopted workspace | Product repository | Read product code and existing product docs as evidence; SDDK writes nothing |
| Durable knowledge | `{vault}` from `sddk knowledge status --format json` | Milestones, ADRs, requirements, cycles, incidences, and terms |
| Operational state | `$SDDK_DATA_DIR/projects/<project_id>/` | Receipt, CAS, `{cycle-artifacts-dir}`, generated output, and project operational data |
| Engram | Optional parallel memory | Mirror only when the knowledge profile enables it; never artifact authority |

The only optional SDDK configuration in an adopted workspace is
`.sddk-versions`. The developer owns it; SDDK never creates or edits it.

## Zero Intrusion

SDDK MUST NOT create or modify repository-local framework state in an adopted
workspace. This includes `docs/`, `CONTEXT.md`, `CONTEXT-MAP.md`, ROADMAPs,
ADRs, specs, `sddk/`, `.sddk/`, `.atl/`, `.ignore`, `.gitignore`, workflow
manifests, checkpoints, reports, or cycle artifacts.

Pre-existing product documentation may be read as evidence. It is read-only
input and never becomes SDDK authority. Generated documentation goes to
`$SDDK_DATA_DIR/projects/<project_id>/generated/`. `--in-repo` is reserved for
explicit dogfooding of the framework development repo, never an adopted
workspace.

## Resolve Once

The orchestrator resolves the knowledge authority once:

```bash
sddk knowledge status --root . --scope . --format json
```

This command already returns project identity, vault path, profile presence,
vault presence, and Engram status. Phase executors consume those validated
fields from `cli_context` and do not repeat the query.

Never reconstruct `{vault}` or `<project_id>` from a directory basename, git
checkout name, environment guess, or hard-coded home path. The orchestrator
passes the CLI-resolved `{vault}`, `{project_id}`, and
`{cycle-artifacts-dir}` to phase executors.

All CLI invocation, ownership, freshness, machine-output, evidence, and error
rules come from `skills/_shared/cli-usage-contract.md`.

## Artifact Routing

- Durable project knowledge is written under `{vault}`.
- Proposal, spec, design, tasks, apply progress, verification, debt, archive,
  release, and HTML reports are written under `{cycle-artifacts-dir}`.
- CLI transitions receive those XDG paths as artifact inputs.
- `/tmp` may hold a disposable presentation copy. It is never authoritative.
- Engram mirrors use `sddk/{change-name}/{artifact-type}` topic keys only when
  `sddk knowledge status` reports `engram_enabled: true`.

## CLI Persistence

The orchestrator opens or resumes a cycle and passes `cycle_id`,
`{cycle-artifacts-dir}`, and validated `cli_context` to phase executors. The
canonical phase prompt alone declares whether that phase performs a CAS store,
a filesystem-only validation, a gate/transition sequence, or no runtime
mutation. Do not invent a universal ledger recipe.

All automated commands use `--format json` when available and follow
`skills/_shared/cli-usage-contract.md`. Evidence binds subject and artifact
hashes and includes material check results. Boolean-only evidence is invalid.
Persistence never falls back to repository-local files or Engram-only state.

## Optional Engram Mirror

If and only if the resolved knowledge profile enables Engram, mirror the full
artifact after the XDG write:

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

Downstream phases always read `{cycle-artifacts-dir}` and `{vault}` first.
Engram is for recovery and search, not pipeline authority.
