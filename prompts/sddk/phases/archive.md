# SDDK Archive Phase

## Role And Boundary

Close a released SDDK cycle. Archive consumes release receipts, syncs durable
specifications and knowledge, produces the closing report and archive manifest,
then applies `archive.complete`. It performs no release Git effects and launches
no subagents.

## Required Inputs

- `cycle_id`, `path`, `{cycle-artifacts-dir}`, and CLI-resolved `{vault}`.
- `{framework-root}`, resolved from the loaded SDDK bundle/source root that
  contains `MANIFEST.sha256`, never from the adopted workspace.
- Cycle state `status=RELEASED`, `phase=archive`.
- Successful `release-report.md`, `merge-receipt`, and `release-receipt` bound
  to the published main SHA and annotated tag.
- Passing `verify-report.md` bound to the published SHA.
- On A-* paths, `debt-report.json` plus its outer-envelope SHA-256, with verdict
  `PASS | PASS_WITH_WARNINGS` and subject equal to the published SHA.
- Delta specs and durable knowledge links produced by prior phases.

## Hard Rules

- Preserve requirements absent from a delta; match modified/removed
  requirements by canonical requirement name.
- Treat the vault and cycle artifact directory as authorities. Never write SDDK
  state into an adopted product repository.
- Preserve the audit trail. Archive is logical closure; do not delete source
  evidence or invent a repository-local archive folder.
- Block destructive or ambiguous spec merges for human confirmation.
- Do not claim cycle closure until `archive.complete` returns `CLOSED` and the
  ledger verifies.
- Do not assume the release lease remains active. `release.complete` normally
  auto-releases it when runtime phase changes.

## Decision Gates

| Condition | Action |
|---|---|
| Release report/receipt missing or SHA/tag mismatch | `blocked` |
| A-* debt evidence missing, mismatched, FAIL, or INCONCLUSIVE | `blocked` |
| Delta merge is destructive or ambiguous | `blocked`, request confirmation |
| Vault validation or ledger validation fails | `blocked` |
| All closure evidence is valid | Apply `archive.complete` |

## Procedure

1. Refresh `sddk cycle status --root . --scope . --cycle {cycle_id} --format json`
   and consume `{vault}` from the validated `cli_context`. Do not repeat the
   immutable knowledge bootstrap query.
2. Validate release, verify, and required debt artifact hashes and subject
   binding.
3. Merge each delta spec into the durable main spec:
   - `ADDED`: append the complete new requirement.
   - `MODIFIED`: replace the complete matching requirement.
   - `REMOVED`: remove only the matching requirement.
   - Missing main spec: persist the complete delta as the initial main spec.
4. Finalize knowledge graph nodes for the cycle, milestone, affected ADRs,
   requirements, and incidences. Record published SHA/tag, verify/debt verdicts,
   release receipt, artifact links, and closure date.
5. Run `sddk vault validate --root . --scope . --vault {vault} --format json` and retain its evidence.
6. Generate the self-contained closing HTML defined by
   `prompts/sddk/HTML-REPORT.md` under `{cycle-artifacts-dir}`; `/tmp` may hold a
   disposable presentation copy.
7. Persist the initial `archive-report.md` and `archive-manifest`. The initial
   manifest references the release receipt, published SHA/tag, synced specs,
   knowledge nodes, report hashes, and vault-validation evidence. Leave final
   ledger evidence out of this initial manifest: it does not exist until
   `archive.complete` appends the closing ledger event.
8. Apply the ledger contract below and return the archive envelope.

### Incidence Sync

For each debt finding whose canonical status is `open`, `in-progress`, or
`deferred`, compute the incidence identity from its stable fingerprint and title
slug. Create a missing INC from
`{framework-root}/docs/debt/INCIDENCE-TEMPLATE.md` under the resolved
`{vault}/incs/`; update an existing incidence rather than duplicating it. Verify
the template against the framework manifest before use. If the framework path
or manifest entry is unavailable, block; never resolve the template relative to
the adopted project's CWD. `resolved` and `superseded` findings may only update
an existing incidence. Record the resulting paths/hashes in the archive
manifest. This is a knowledge sync operation, not a new runtime transition.

## Managed Closure (vault-only) Branch

For cycles with `DeliveryKind = ManagedClosureDelivery`, the standard
`release.complete` route is bypassed. Instead, the cycle enters
`archive.vault.complete` directly from `BLOCKED` status.

### Entry Conditions (REQ-DKA-002, REQ-DKA-005)

| Condition | Required |
|---|---|
| `cycle.status == BLOCKED` | Yes |
| `cycle.delivery_kind == ManagedClosureDelivery` | Yes |
| `release-receipt.json` absent | Yes |
| `vault-receipt.json` absent (before emission) | Yes |

### Transition: `archive.vault.complete`

```
from:   { status: BLOCKED, phase: null }
to:     { status: CLOSED, phase: archive }
paths:  A-min, A-lite, A-full, B-direct
```

Required artifacts and gates:
- `vault-receipt` artifact (emitted by `sddk release vault`)
- `archive-manifest` artifact
- `vault-receipt-verified` gate (HMAC signature check)
- `vault-index-current` gate
- `release-bypass-declared` gate

### Procedure

1. Run `sddk release vault --root . --scope . --cycle {cycle_id} --format json`
   to emit `vault-receipt.json`. This fails if:
   - `delivery_kind != ManagedClosureDelivery`
   - `release-receipt.json` already exists
   - `cycle.status != BLOCKED`
2. Evaluate `vault-receipt-verified` by re-computing the HMAC-SHA256 payload
   and comparing against the `signature` field in `vault-receipt.json`.
3. Evaluate `vault-index-current` against the resolved vault path.
4. Evaluate `release-bypass-declared` by checking `delivery_kind` is present
   and equals `ManagedClosureDelivery` in the cycle manifest.
5. Run `sddk ledger verify --root . --scope . --format json` and retain evidence.
6. When all gates pass, transition `archive.vault.complete`:
   `sddk cycle transition --root . --scope . --cycle {cycle_id} --transition archive.vault.complete --artifact vault-receipt={vault_receipt_path} --artifact archive-manifest={manifest_path} --gate-receipt {vault_receipt_receipt_id} --gate-receipt {ledger_receipt_id} --format json`
7. Require `outcome=succeeded`, `status=CLOSED`, `phase=archive`.
8. Refresh `sddk cycle status --root . --scope . --cycle {cycle_id} --format json`
   after successful transition. Require `status=CLOSED`, `phase=archive`.
9. Run `sddk ledger verify --root . --scope . --format json` again to cover the
   closing append. Finalize `archive-manifest.md` with the post-transition evidence.

**Note:** Managed-closure cycles MUST still persist incidences for debt findings
with status `open`, `in-progress`, or `deferred` exactly as described in the
Incidence Sync section above. The vault route does not waive the debt incidence
tracking requirement.

### Ledger Contract

Transition reference:
```
Transition:   archive.vault.complete
Matrix row:   lifecycle.cycle.transition.archive
Artifact:     {cycle_artifacts_dir}/vault-receipt.json
On failure:   blocked — runtime remains BLOCKED; do not retry from cache
```

## Ledger Contract (standard archive)

Transition reference:
```
Transition:   archive.complete
Matrix row:   lifecycle.cycle.transition.archive
Artifact:     {cycle_artifacts_dir}/archive-manifest.md
On failure:   blocked — runtime remains OPEN/archive; do not retry from cache
```

Full procedure (from `cli-usage-contract.md#matrix`):

1. Run `sddk ledger verify --root . --scope . --format json`, capture its exact
   argv, exit code, and output digest, then evaluate:
   `sddk cycle evaluate-gate --root . --scope . --cycle {cycle_id} --transition archive.complete --gate ledger-valid --outcome {outcome} --evaluator sddk.cli --evidence {ledger_evidence_json_arg} --timestamp {now} --actor sddk --format json`
2. Evaluate `vault-index-current` with vault path, validation result/output
   digest, archive-manifest path/SHA-256, and published subject:
   `sddk cycle evaluate-gate --root . --scope . --cycle {cycle_id} --transition archive.complete --gate vault-index-current --outcome {outcome} --evaluator sddk.cli --evidence {vault_evidence_json_arg} --timestamp {now} --actor sddk --format json`
3. When both gates pass, transition `archive.complete` with the manifest and
   both receipt IDs:
   `sddk cycle transition --root . --scope . --cycle {cycle_id} --transition archive.complete --artifact archive-manifest={manifest_path} --gate-receipt {ledger_receipt_id} --gate-receipt {vault_receipt_id} {lease_flags_if_present} --format json`
   Include lease owner/token only if the fresh cycle status actually contains a
   live lease; otherwise omit both flags.
4. Require transition `outcome=succeeded`, `status=CLOSED`, `phase=archive`.
5. Refresh `sddk cycle status --root . --scope . --cycle {cycle_id} --format json`
   after the successful transition. Require `status=CLOSED`, `phase=archive`,
   and an observed `updated_at` valid as an RFC 3339 timestamp. This
   post-transition status value is the manifest's concrete `closed_at`; never
   use `XXZ`, a guessed timestamp, or a timestamp observed before
   `archive.complete`.
6. Run `sddk ledger verify --root . --scope . --format json` again after the
   successful transition to cover the closing append. Capture the exact JSON
   output and use its observed `event_count` and `last_hash` values.
7. Finalize `archive-manifest.md` and `archive-report.md` with the concrete
   post-transition `closed_at` and ledger evidence. Recompute and record every affected digest,
   including the manifest and report digests, after finalization. Do not retain or create
   placeholder ledger values;
   `event_count: 931` and `sha256:<post-transition-hash>` are invalid evidence.

Any CLI failure blocks archive. Never reacquire a lease merely to satisfy an
outdated command template.

## Output Contract

```yaml
status: success | blocked
executive_summary: 1-3 evidence-bound sentences
cycle_id: string
published_subject: {main_sha: sha, tag: semver}
artifacts:
  - {kind: archive-report, path: string, sha256: string}
  - {kind: archive-manifest, path: string, sha256: string}
  - {kind: closing-html, path: string, sha256: string}
release_receipt: string
report_locale_requested: string | null
report_locale: string
report_locale_fallback: none | project | parent-language | es
report_audience: novice | standard | expert
specs_synced: [{domain: string, added: N, modified: N, removed: N}]
knowledge_nodes_updated: [string]
runtime_status: CLOSED | RELEASED
next_recommended: ready-for-next-cycle | resolve-blocker
risks: []
context_quality: C0 | C1 | C2 | C3
skill_resolution: paths-injected | fallback-registry | fallback-path | none
follow_up_incidences:
  - {path: docs/debt/INC-NNN-{slug}.md, fingerprint: hex, cluster_id: string,
     severity: critical|high|medium|low, priority: P0|P1|P2|P3}
```

## References

- `skills/sddk-archive/SKILL.md`
- `skills/_shared/sddk-phase-common.md`
- `skills/_shared/persistence-contract.md`
- `skills/knowledge-graph/SKILL.md`
- `prompts/sddk/HTML-REPORT.md`
- `prompts/sddk/phases/release.md`
- `prompts/sddk/phases/debt-verify.md` (for follow-up incidence creation)
- `docs/debt/SEVERITY.md`
- `docs/debt/PRIORITY.md`
- `docs/debt/debt-report.schema.json`
- `docs/debt/INCIDENCE-TEMPLATE.md`

## Follow-up Incidences (cycle-7b)

When `debt-report.json` from the previous phase contains one finding with
`attribution: pre_existing` and the verdict was `PASS_WITH_WARNINGS`, the
archive phase MUST persist one `INC-NNN-{slug}.md` file per distinct
`fingerprint`. The incidence is rendered from
[`docs/debt/INCIDENCE-TEMPLATE.md`](../../../docs/debt/INCIDENCE-TEMPLATE.md)
and uses the canonical severity/priority taxonomies:

| Field | Source |
|---|---|
| `id` | `INC-NNN-{slug}` allocated by `sddk-archive`; slug derived from `finding.title` |
| `severity` | `finding.severity` lowercased per `docs/debt/SEVERITY.md` |
| `priority` | `finding.priority` per `docs/debt/PRIORITY.md` |
| `fingerprint` | `finding.fingerprint` (16-64 hex) |
| `fingerprint_aliases` | any prior cycle aliases for the same rule |
| `cluster_id` | `finding.cluster_id` |
| `owner` | human owner declared in the report or `unassigned` |
| `evidence_refs` | `finding.evidence_refs[]` from `debt-report.json` |

Block `archive.complete` if any expected `INC-NNN` cannot be persisted
(working-tree write failure, slug collision not resolvable, or a
`fingerprint` already claimed by an open incidence with a different
`cluster_id` — that is a fingerprint collision and signals a cluster
mapping regression that must be fixed before archive can complete).

## Files Inventory

The archive report always carries a `## Files Inventory` section, sourced from
`{cycle-artifacts-dir}/inventory.json` (`sddk.inventory/v1`). Render a single
`## Files Inventory` block with bucket counters and the top-N paths table
exactly as defined in `prompts/sddk/phases/verify.md` § Files Inventory. Block
`archive.complete` if `summary.unavailable_reason` is `git-not-initialized` or
`invalid_rev`; persist the inventory artifact regardless so the next attempt
has a reference. The closing HTML projection includes the same section.

When the reducer returns `summary.unavailable_reason`, render the marker
`inventory-unavailable: <reason>` at the top of the block and list the canonical
source path (`inventory.json`) so the human reviewer can locate the artifact.
The artifact embeds the project's `.gitignore` matches inside
`ignored_by_project`; no sidecar file is produced.

The archive envelope must also include the persistence of `inventory.json`:

```yaml
inventory:
  path: string
  sha256: string
  unavailable_reason: git-not-initialized | git-context-missing | io-error | invalid_rev | null
```
