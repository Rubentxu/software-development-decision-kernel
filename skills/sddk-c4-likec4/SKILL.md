---
name: sddk-c4-likec4
description: "Trigger: SDDK C4 view, architecture report, LikeC4 render. Produce evidence-bound architecture views with validated rich output or a table fallback."
disable-model-invocation: true
user-invocable: false
license: MIT
metadata:
  author: SDDK Team
  version: "1.0"
---

## Activation Contract

Activate only for architecture-impacting work or an explicit C4 request. Consume
subject identity, artifact paths, requested levels, and evidence references.

## Hard Rules

- Treat `references/model-contract.md` as semantic authority.
- Write only under `{cycle-artifacts-dir}/reports/architecture/`.
- Use installed tools only; never install, download, invoke `npx`, or alter the workspace.
- Keep semantic status independent from render status and verification verdict.
- Open a browser only when explicitly requested.

## Decision Gates

| Condition | Action |
|---|---|
| Missing subject/evidence/stable IDs | Return `insufficient_evidence`; do not invent an edge |
| Installed LikeC4 and Node `>=22.22.3` | Validate, then render rich output |
| LikeC4 absent or Node absent/older than `22.22.3` | Record exact versions/reason and produce Markdown and table-HTML fallback |
| Validation/render failure | Preserve semantic result, record failure, emit fallback |

## Execution Steps

1. Build one semantic manifest under the model contract. Completion: every node
   and relation has state, stable ID, subject, and evidence coverage.
2. Preflight installed `archctl`, `node`, and `likec4` versions without network
   access. Require Node `>=22.22.3` for the declared LikeC4 baseline. Completion:
   exact versions, compatibility, and fallback reason are recorded.
3. When rich rendering is available, write LikeC4 sources beneath the output
   directory and run `likec4 validate --json --file <edited-file> <project-dir>`.
   Accept rich output only after successful validation; build with
   `likec4 build -o <output-dir> <project-dir>`.
4. Always generate Markdown and table-HTML from the same manifest. Completion:
   `planned_but_missing`, `unplanned_actual`, and evidence gaps remain visible.
5. Hash manifest, sources, validation output, and renders. Completion: output
   envelope names every path/hash and no workspace file changed.

## Output Contract

Return the complete manifest from `references/model-contract.md`, plus risks and
next action. Renderer failure cannot change `semantic_status` or verify verdict.

## References

- `references/model-contract.md`
- `../../prompts/sddk/phase-contracts.md`
- `../../prompts/sddk/HTML-REPORT.md`
