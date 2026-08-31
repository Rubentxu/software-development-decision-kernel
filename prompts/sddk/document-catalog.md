# SDDK Document Catalog

This catalog defines the authoritative location and ownership of SDDK output.
Resolve paths before work with `sddk knowledge status --format json`; never
infer `project_id` from the checkout name or repeat a path query.

## Authority Matrix

| Class | Location | Content | Authority |
|---|---|---|---|
| Durable knowledge | `{vault}` | Milestones, ADRs, requirements, cycle nodes, incidences, terms | Canonical project knowledge |
| Cycle artifacts | `{cycle-artifacts-dir}` | Explore, proposal, spec, design, tasks, progress, verification, debt, archive, release, reports | Canonical operational record for one cycle |
| Generated docs | `$SDDK_DATA_DIR/projects/{project_id}/generated/` | Inventory and workflow renderings | Generated output |
| CAS and receipts | `$SDDK_DATA_DIR/projects/{project_id}/` | Artifact blobs, adoption receipt, workspace records | Operational state |
| Engram | Profile-controlled | Optional `sddk/...` mirrors and episodic memory | Never authoritative alone |
| `/tmp` | Temporary | Optional presentation copies | Disposable, never authoritative |

## Cycle Artifacts

Every phase writes beneath the CLI-resolved `{cycle-artifacts-dir}`:

| Artifact | Producer | Consumers |
|---|---|---|
| `explore-report.md` | `sddk-explore` | propose, coherence |
| `proposal.md` | `sddk-propose` | spec, design |
| `spec.md` or `specs/` | `sddk-spec` | design, tasks, verify, archive |
| `design.md` | `sddk-design` | tasks, apply, verify |
| `tasks.md` | `sddk-tasks` | apply, verify |
| `apply-progress.yaml` | `sddk-apply` | verify, orchestrator |
| `coherence/{trigger}.md` | `sddk-coherence` | orchestrator, archive report |
| `verify-report.md` | `sddk-verify` | debt-verify, release, archive |
| `inventory.json` | `sddk cycle inventory` | verify, release, archive; `sddk.inventory/v1` (embeds `.gitignore` matches in `ignored_by_project`) |
| `debt-report.json` | `sddk-debt-verify` | release, archive; machine authority |
| `debt-report.md` | `sddk-debt-verify` | human review; derived from JSON |
| `release-report.md` | `sddk-release` | archive, cycle audit |
| `archive-report.md` | `sddk-archive` | cycle audit |
| `reports/cierre.html` | `sddk-archive` | human reviewer; derived projection only |

Optional Engram mirrors use `sddk/{change-name}/{artifact-type}` only when
`sddk knowledge status` reports `engram_enabled: true`.

HTML never overrides source reports, manifests, ledger receipts, artifact
hashes, or evidence. Its recorded path/hash proves render identity, not the
truth of claims rendered inside it.

## Durable Nodes

| Node | Vault location | Owner |
|---|---|---|
| Milestone and serialization lock | `{vault}/milestones/` | orchestrator acquires; release transition releases; archive finalizes milestone |
| ADR | `{vault}/adrs/` | spec/design proposal; orchestrator-approved write |
| Requirement | `{vault}/specs/{domain}/` | spec, archive |
| Cycle | `{vault}/cycles/` | archive |
| Incidence | `{vault}/incs/` | verify/debt evidence; archive finalization |
| Term | `{vault}/terms/` | explore, spec |

All vault writes follow `skills/knowledge-graph/SKILL.md`, preserve provenance,
and append to `{vault}/_log.md`.

## Adopted Workspace

The workspace contains product code and product-owned files. SDDK may read
existing `README`, architecture documents, ADRs, roadmap, `CONTEXT.md`, or
other product documentation as **read-only evidence**. Those files are not
SDDK authority and SDDK never creates, updates, migrates, or indexes them in
place.

SDDK never writes workspace-local `docs/`, ROADMAP, ADRs, specs, context files,
`sddk/`, `.sddk/`, `.atl/`, workflow manifests, ignore files, checkpoints, or
reports. Missing SDDK knowledge is repaired in `{vault}`; missing operational
state is repaired under `$SDDK_DATA_DIR/projects/{project_id}/`.

## Cross-References

- Cycle artifacts reference neighbouring artifacts by paths under
  `{cycle-artifacts-dir}`.
- Durable nodes use vault wikilinks.
- The cycle node links the PR/tag, verdicts, relevant durable nodes, and
  operational artifact paths.
- A temporary HTML copy may point back to
  `{cycle-artifacts-dir}/reports/cierre.html`; it is not retained as authority.

## Dogfooding Exception

`sddk generate docs|inventory --in-repo` is allowed only when explicitly run
against the `sddk-framework` development repository. It is never used for an
adopted product workspace.

## Vault Layout and Ownership

Full vault structure (resolved via `sddk knowledge path`):

```
$VAULT_PATH (via `sddk knowledge path`, e.g. ~/.sddk-knowledge/<project_id>/)
                                        ← KNOWLEDGE GRAPH (outside repo)
├── milestones/                         ← serialization lock + milestones
│   ├── _active.md                      ← lock file (LOCKED/AVAILABLE)
│   └── M-NNN-{slug}.md                 ← one node per cycle
├── adrs/                               ← architectural decisions
│   └── ADR-NNN-{slug}.md               ← linked to REQ nodes + cycle
├── specs/{domain}/                     ← system requirements
│   └── REQ-{Slug}.md                   ← linked to ADR + cycle + tests
├── cycles/                             ← cycle manifests (traceability hub)
│   └── CYC-{date}-{slug}.md            ← links to ALL artifacts of a cycle
├── incs/                               ← problems found
│   └── INC-NNN-{slug}.md               ← linked to ADR + REQ
├── terms/                              ← glossary
│   └── TERM-{Slug}.md                  ← linked to ADR + REQ
├── _index.md                           ← MOC raíz (Dataview queries)
└── _log.md                             ← append-only activity log
```

| Node type | Owner | When |
|-----------|-------|------|
| `milestone` (M-NNN) | Orchestrator (open) → Archive (finalize) | Step 0.2 / Step 3.4 |
| `active_lock` (_active) | Orchestrator (acquire) / `release.complete` (runtime auto-release) | Step 0.2 / Step 3.3 |
| `adr` (ADR-NNN) | sddk-spec / sddk-design (create) → Archive (implementation status) | Step 1.4 / Step 3.4 |
| `requirement` (REQ-Slug) | sddk-spec (create) → Archive (sync/version) | Step 1.4 / Step 3.4 |
| `cycle` (CYC-date-slug) | sddk-archive | Step 3.4 |
| `incidence` (INC-NNN) | verify/debt-verify (evidence) → Archive (persist/finalize) | Phase 2 / Step 3.4 |
| `term` (TERM-Slug) | sddk-explore / sddk-spec | Phase 1 |
| proposal, spec delta, design, tasks | phase agents (working state in `{cycle-artifacts-dir}/`) | Phase 1 |
| verify-report, debt-report JSON/Markdown | verify/debt agents (working state) | Phase 2 |
| release-report | sddk-release | Phase 3 |
| archive-report | sddk-archive | Phase 3.4 |
