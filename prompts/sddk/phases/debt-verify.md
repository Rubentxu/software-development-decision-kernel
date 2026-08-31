# SDDK Debt-Verify Gate Contract

This document is the single declarative authority for the post-verify technical
debt gate. Agents and skills point here instead of copying its decision tables
or report schemas.

Debt-verify is a workflow **capability/gate** between functional verify and
release. It is not a new value in the legacy runtime `Phase` enum. Runtime and
CLI enforcement are intentionally outside this specification change.

## Canonical References (cycle-7b)

The debt lifecycle is anchored to four canonical artifacts. This phase contract
**must** align with them; do not duplicate their taxonomies inline.

| Artifact | Authority | Used by |
|---|---|---|
| [`docs/debt/SEVERITY.md`](../../../docs/debt/SEVERITY.md) | Severity taxonomy `critical \| high \| medium \| low` | Common Finding `severity`, report `summary.by_severity`, Decision Contract |
| [`docs/debt/PRIORITY.md`](../../../docs/debt/PRIORITY.md) | Remediation priority `P0 \| P1 \| P2 \| P3` | Common Finding `priority` (mapped from severity + context), `follow_up[].priority` |
| [`docs/debt/debt-report.schema.json`](../../../docs/debt/debt-report.schema.json) | JSON Schema `1.0.0/1.1.0` for persisted `debt-report.json` | Authoritative outer envelope; Markdown is derived |
| [`docs/debt/INCIDENCE-TEMPLATE.md`](../../../docs/debt/INCIDENCE-TEMPLATE.md) | Cross-cycle durable record template | Pre-existing `PASS_WITH_WARNINGS` produces `INC-NNN-{slug}.md` (cycle-7b) |

The Common Finding shape below is the **working internal contract** used by
the gate and the clusters. It is mapped to the persisted `debt-report.json`
shape (the canonical schema) by the projection layer that produces the
authoritative artifact. See the **Schema Mapping** table below.

## Activation

Run after `sddk-verify` returns `PASS` or `PASS_WITH_WARNINGS`:

| Path | Policy | Depth | Required clusters |
|---|---|---|---|
| A-min | mandatory | smoke | coupling, overeng |
| A-lite | mandatory | standard | coupling, overeng, smells, duplication |
| A-full | mandatory | deep | architecture, coupling, overeng, smells, duplication |
| B-direct | disabled | n/a | none |

Depth is path-derived and locked. Reversibility may influence triage into a
different path; it does not skip or deepen debt-verify after the cycle starts.

## Policy Trade-offs

| Choice | Benefit | Cost / Risk | Mitigation |
|---|---|---|---|
| Path-derived depth | Predictable cost and no mid-cycle negotiation | Smoke and standard may miss dimensions outside their cluster set | Triage irreversible or architectural work into A-full before the cycle starts |
| Fail closed on incomplete coverage | Prevents a partial audit from becoming a false PASS | Analyzer outages can delay release without proving debt | Retry transient failures up to the bounded limit, then require human review |
| Block only introduced/updated debt | Makes adoption viable in repositories with legacy debt | Pre-existing debt can remain indefinitely | Keep it visible and create owned, prioritized follow-up incidences |
| Reproducible evidence and stable fingerprints | Enables deduplication, comparison, and audit | Adds hashing, normalization, and provenance overhead | Scale cluster count by path and reject unsupported numeric precision |
| JSON authority plus Markdown projection | Gives machines deterministic input and humans a readable report | Two artifacts can drift | Generate Markdown from persisted JSON; bind both hashes in the outer envelope |
| Specification-only runtime handoff | Avoids claiming CLI enforcement that does not exist | The declarative gate can be bypassed by current runtime integrations | Track typed runtime enforcement as deferred roadmap work |

## Required Input

The orchestrator supplies one immutable audit packet:

```yaml
contract_version: debt-gate/v1
cycle:
  cycle_id: {cycle-id}
  change_name: {change-name}
  path: A-min | A-lite | A-full
  remediation_round: 0..3
subject:
  branch: {feature-branch}
  base_commit: {full SHA}
  head_commit: {full SHA}
  diff_digest: {sha256 of normalized base...head diff}
scope:
  effective_depth: smoke | standard | deep
  changed_paths: [repo-relative paths]
  one_hop_dependencies: [repo-relative paths]
verify_evidence:
  path: {cycle-artifacts-dir}/verify-report.md
  sha256: {64 lowercase hex}
  subject_sha: {same head_commit}
  verdict: PASS | PASS_WITH_WARNINGS
router:
  context_quality: C0 | C1 | C2 | C3
  strict_tdd: true | false
  engram_memory: true | false
```

## Preflight

Block before launching clusters when any condition fails:

1. Verify evidence is missing, malformed, not PASS/PW, or bound to another SHA.
2. `base_commit` or `head_commit` is unavailable.
3. The worktree contains changes not represented by `head_commit`.
4. Path and depth do not match the Activation table.
5. `remediation_round > 3`.

Use `base_commit...head_commit` for scope. Never assume the default branch is
named `main`. Remote push status is release evidence, not a debt-analysis
precondition.

## Cluster Run Contract

Every required cluster returns:

```yaml
cluster_run:
  cluster: debt-{dimension}-cluster
  status: completed | failed | timed_out
  attempts: 1..3
  analyzer:
    name: {agent or tool}
    version: {model, skill hash, or tool version}
  subject_sha: {head_commit}
  started_at: {RFC3339}
  finished_at: {RFC3339}
  findings: [Common Finding]
  errors: [{code, message}]
```

A required run that is not `completed`, or whose subject differs, makes the
global verdict `INCONCLUSIVE`.

## Common Finding Contract

All clusters normalize findings to this shape. Cluster-specific payloads may be
added under `details`.

```yaml
finding:
  finding_id: {cluster-local stable id}
  fingerprint: {sha256 of normalized canonical rule_id + path + symbol/context}
  rule_id: {canonical rule identifier shared across clusters}
  cluster: architecture | smells | duplication | coupling | overeng
  category: {stable category}
  severity: CRITICAL | HIGH | MEDIUM | LOW
  confidence: HIGH | MEDIUM | LOW
  baseline_state: new | updated | unchanged | unknown
  attribution: introduced | pre_existing | unknown
  locations:
    - path: {repo-relative path}
      start_line: {positive integer}
      end_line: {positive integer}
      symbol: {optional symbol}
  evidence:
    - kind: command | source | graph | test | analyzer
      observation: {what was observed, not an inference}
      command: {optional exact argv/string}
      tool: {tool name}
      tool_version: {version or unknown}
      exit_code: {integer or null}
      output_digest: {sha256 or null}
  impact: {concrete failure/change cost}
  remediation:
    target: apply | replan | backlog
    action: {specific next action}
  details: {}
```

Rules:

- `severity` measures impact; `confidence` measures evidentiary certainty.
- Corroboration by multiple clusters raises confidence only.
- `rule_id` identifies the issue independently of the analyzer. Clusters that
  observe the same issue use the same canonical rule id, so `fingerprint` does
  not include cluster identity. `finding_id` remains cluster-local.
- A finding without a repository-relative location and observable evidence is
  invalid unless its category is repository-wide; repository-wide findings
  must cite the analyzed scope and command/tool evidence.
- Numeric estimates require a reproducible method. Otherwise emit qualitative
  bands and raw counts.
- `baseline_state` is computed against the supplied base/head pair. `git blame`
  may add provenance but does not decide attribution by itself.
- Normalize paths, ordering, and fingerprints before hashing so identical input
  produces identical output.

## Schema Mapping (Common Finding → persisted `debt-report.json`)

The persisted artifact conforms to [`docs/debt/debt-report.schema.json`](../../../docs/debt/debt-report.schema.json).
The projection layer that produces the authoritative file MUST apply this
mapping; clusters and the gate MUST NOT emit Common Finding fields that do not
have a target in the schema.

| Common Finding | Persisted field | Transformation |
|---|---|---|
| `cluster` + `finding_id` | `id` | `FIND-{NNNNNN}` derived from a stable hash of `(cluster, finding_id)`; kept as cluster-local alias under `finding.finding_id` |
| `finding.rule_id` (one-line human description) | `title` | Verbatim, trimmed to 120 chars |
| `severity` (UPPERCASE) | `severity` | Lowercased: `CRITICAL → critical`, etc. |
| `priority` (assigned by gate, see Decision Contract) | `priority` | `P0\|P1\|P2\|P3`; defaults to `P2` for `attribution: pre_existing` and `P1` for `attribution: introduced` with severity `HIGH` or below |
| `cluster` | `category` | `cluster ∈ {smells, coupling, duplication, overeng, architecture}` mapped to schema enum |
| `baseline_state` + `attribution` | `status` | `status: open` always (the gate is per-cycle; superseded/resolved live in INC files) |
| `fingerprint` | `fingerprint` | Verbatim 16-64 lowercase hex |
| (none) | `fingerprint_aliases` | Merged from any prior cycle's same-rule observation; empty on first emit |
| (cluster-local sequence) | `cluster_id` | `CL-NN` derived from the rule's cluster |
| (rule description) | `description` | First 2000 chars of the rule text |
| (none) | `remediation_pr` | Set to the gate's `next_recommended` decision (`sddk-apply \| replan`) when `verdict=FAIL`, else `null` |
| (gate's `remediation.target`) | `remediation_cycle` | `apply \| replan \| backlog`; `null` for `verdict=PASS` |
| `evidence[]`, `locations[]`, `details{}` | `evidence_refs[]` | One entry per evidence kind with `kind`, `path`, `command`, `tool`, `tool_version`, `exit_code`, `output_digest` |
| `confidence` | (dropped) | Persisted as `summary.by_confidence` aggregate only; not per-finding in the schema |
| `attribution`, `baseline_state` | (dropped) | Persisted as `summary.by_attribution` aggregate; per-finding details live in the Markdown projection |

`remediation_pr`, `remediation_cycle`, and `evidence_refs` are optional in the
schema. The projection MUST set `remediation_cycle` to `null` when the gate
verdict is `PASS` and the finding has no follow-up action; it MUST set
`remediation_cycle` to `backlog` when verdict is `PASS_WITH_WARNINGS` and the
finding is `attribution: pre_existing`.

## Baseline And Suppressions

The gate follows a new-code policy:

- Findings with `attribution: introduced` and `baseline_state: new | updated`
  participate in blocking decisions.
- `pre_existing` findings remain visible and create follow-up debt, but do not
  become newly introduced merely because a cluster rediscovered them.
- `unknown` attribution on a would-be blocker yields `INCONCLUSIVE`.
- A suppression requires a finding fingerprint, human owner, justification,
  creation date, and expiry. Agent-authored or expired suppressions do not waive
  a finding.
- A valid suppression removes a finding from blocking counts but not from the
  report. The decision reasons must name every applied suppression.

## Deterministic Aggregation

1. Sort cluster runs by cluster name.
2. Reject malformed or wrong-subject results.
3. Normalize and sort findings by fingerprint.
4. Merge identical fingerprints; retain all evidence and source clusters.
5. Raise confidence when independent evidence corroborates a finding.
6. Count findings once after deduplication.
7. Apply the Decision Contract in table order.

## Decision Contract

| First matching condition | Verdict | Remediation |
|---|---|---|
| Required cluster missing/failed/timed out; invalid subject; malformed evidence; unknown attribution or LOW confidence on a potential blocker | `INCONCLUSIVE` | retry gate or human review |
| Any unsuppressed introduced CRITICAL finding in baseline state new/updated with confidence HIGH or MEDIUM | `FAIL` | `apply` unless structural signals require `replan` |
| Circular dependency, unencapsulated shared mutable state, or contract-breaking LSP violation introduced by the change | `FAIL` | `replan` for boundary/design failure; otherwise `apply` |
| Three or more unsuppressed introduced HIGH findings in baseline state new/updated with confidence HIGH/MEDIUM | `FAIL` | `apply` |
| One or two unsuppressed introduced HIGH findings, or three or more introduced MEDIUM findings, with no blocker | `PASS_WITH_WARNINGS` | `none`; attach backlog |
| Only pre-existing HIGH/CRITICAL findings, with complete evidence and no introduced blocker | `PASS_WITH_WARNINGS` | `none`; create follow-up incidence |
| No warning or blocking condition | `PASS` | `none` |

`re_iterate_from: replan` blocks automatic progression and recommends new
exploration/proposal work while preserving the current cycle branch and
evidence. It does not claim the current CLI can rewind phase state. The
orchestrator must surface the blocker and obtain an explicit recovery/new-cycle
decision before dispatching planning work.

## Authoritative Report

Persist `debt-report.json` as the machine authority. Render
`debt-report.md` from the same data for humans; Markdown never overrides JSON.

```yaml
contract_version: debt-gate/v1
report_id: {stable id}
generated_at: {RFC3339}
cycle: {cycle_id, change_name, path, remediation_round}
subject: {branch, base_commit, head_commit, diff_digest}
verify_evidence: {path, sha256, subject_sha, verdict}
coverage:
  required_clusters: [names]
  completed_clusters: [names]
  failed_clusters: [{name, status, attempts, errors}]
findings: [Common Finding, deduplicated, with source_clusters]
summary:
  total: {n}
  by_severity: {critical: n, high: n, medium: n, low: n}
  by_confidence: {high: n, medium: n, low: n}
  by_attribution: {introduced: n, pre_existing: n, unknown: n}
  by_cluster: {cluster: n}
decision:
  verdict: PASS | PASS_WITH_WARNINGS | FAIL | INCONCLUSIVE
  re_iterate_from: replan | apply | none
  reasons: [{rule, finding_fingerprints, explanation}]
  fail_closed: true
waivers: [{fingerprint, owner, justification, created_at, expires_at}]
follow_up: [{finding_fingerprint, action, owner, priority}]
artifact_paths:
  json: {path}
  markdown: {path}
runtime_handoff:
  status: specification_only
  desired_artifact_kind: debt-report
  desired_gate: debt-approved
  note: "No debt-specific CLI transition is declared by this documentation change."
```

The JSON report cannot contain its own digest or the Markdown digest without a
hash cycle. Persist and hash JSON first, render Markdown with
`source_json_sha256`, hash Markdown, then place both digests only in the outer
orchestrator envelope.

## Markdown Projection

The human report leads with:

1. Verdict, subject SHAs, and coverage completeness.
2. Counts by severity, confidence, attribution, and cluster.
3. Blocking/warning findings with `path:line`, evidence, and remediation.
4. Pre-existing findings and follow-up ownership.
5. Cluster failures or uncertainty.
6. Runtime handoff status and source JSON hash. The Markdown digest exists only
   in the outer envelope because an artifact cannot contain its own stable hash.

## Follow-up Incidences (cycle-7b)

When the verdict is `PASS_WITH_WARNINGS` and at least one finding has
`attribution: pre_existing`, the gate MUST emit a follow-up incidence per
distinct `fingerprint` so the durable record survives across cycles (ADR-0047
§3.2). Each incidence is rendered from
[`docs/debt/INCIDENCE-TEMPLATE.md`](../../../docs/debt/INCIDENCE-TEMPLATE.md):

```yaml
- destination: docs/debt/INC-NNN-{slug}.md
  derived_from: cycle {cycle_id} / debt-report.json
  fingerprint: <finding.fingerprint>
  cluster_id: <finding.cluster_id>
  severity: <finding.severity, lowercased per docs/debt/SEVERITY.md>
  priority: <finding.priority per docs/debt/PRIORITY.md>
  owner: <human owner or "unassigned">
  rationale: <one-sentence quote from finding.description>
```

The gate never creates or modifies INC files for `attribution: introduced`
findings — those are remediated inside the current cycle (Decision Contract
table above) and would re-open once introduced debt is allowed to age.

## Orchestrator Envelope

```yaml
contract_version: debt-gate/v1
status: success | partial | blocked
executive_summary: {1-3 evidence-bound sentences}
artifacts:
  - {kind: debt-report-json, path: ..., sha256: ...}
  - {kind: debt-report-markdown, path: ..., sha256: ...}
subject: {cycle_id, base_commit, head_commit, diff_digest}
verdict: PASS | PASS_WITH_WARNINGS | FAIL | INCONCLUSIVE
re_iterate_from: replan | apply | none
cluster_coverage: {required: n, completed: n, failed: n}
findings_by_severity: {critical: n, high: n, medium: n, low: n}
findings_by_attribution: {introduced: n, pre_existing: n, unknown: n}
next_recommended: sddk-release | sddk-apply | sddk-explore | retry-debt-verify | human-review
runtime_handoff: specification_only
risks: []
context_quality: C0 | C1 | C2 | C3
```

Mapping:

| Verdict | status | next_recommended |
|---|---|---|
| PASS/PASS_WITH_WARNINGS | success | sddk-release |
| FAIL + apply | blocked | sddk-apply |
| FAIL + replan | blocked | human-review |
| INCONCLUSIVE, retryable | partial | retry-debt-verify |
| INCONCLUSIVE, non-retryable | blocked | human-review |

## CLI Ledger Contract

Debt-verify is a declarative gate between verify and release; it has no standalone
runtime transition. Its outcome is consumed by `release.complete`.

Transition reference:
```
Transition:   release.complete
Matrix row:   lifecycle.cycle.transition.release
Artifact:     {cycle_artifacts_dir}/debt-report.json
On failure:   blocked — runtime remains OPEN/release; debt-severity/priority gates unsatisfied
```

Full procedure (from `cli-usage-contract.md#matrix`):
1. `sddk cycle status --root . --scope . --cycle {cycle_id} --format json` → confirm phase is `verify` or `release`.
2. Build `{evidence_json}` with debt-report path/SHA-256, verdict, subject SHA, and
   one result per required cluster. Set `{outcome}` to `passed` only when every
   required cluster returned `completed` and no introduced HIGH/CRITICAL finding
   has `confidence HIGH or MEDIUM`.
3. `sddk cycle evaluate-gate --root . --scope . --cycle {cycle_id}
   --transition release.complete --gate debt-severity-assigned
   --outcome {outcome} --evaluator sddk.cli --evidence {evidence_json}
   --timestamp {now} --actor sddk --format json`
4. `sddk cycle evaluate-gate --root . --scope . --cycle {cycle_id}
   --transition release.complete --gate debt-priority-assigned
   --outcome {outcome} --evaluator sddk.cli --evidence {evidence_json}
   --timestamp {now} --actor sddk --format json`
5. `sddk ledger verify --root . --scope . --format json`

On failure: blocked — `release.complete` gates cannot be satisfied. A failed CLI
invocation or ledger verification is a blocker.

- `agents/sddk-debt-verify.md`
- `agents/debt-architecture-cluster.md`
- `agents/debt-smells-cluster.md`
- `agents/debt-duplication-cluster.md`
- `agents/debt-coupling-cluster.md`
- `agents/debt-overeng-cluster.md`
- `skills/sddk-debt-verify/SKILL.md`
- `prompts/sddk/orchestrator.md`
- `prompts/sddk/mcw.md`
- `prompts/sddk/git-contract.md`
- `docs/debt/SEVERITY.md`
- `docs/debt/PRIORITY.md`
- `docs/debt/debt-report.schema.json`
- `docs/debt/INCIDENCE-TEMPLATE.md`
- `prompts/sddk/phases/archive.md` (for follow-up incidence creation)
