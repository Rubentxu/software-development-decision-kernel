# Debt Cluster Output Envelope (shared contract)

> Single source of truth for the `cluster_run` envelope emitted by every
> `debt-*-cluster` subagent of `sddk-debt-verify`. Clusters reference this
> contract instead of duplicating it (AX-S5 finding: the envelope was
> copy-pasted across 5 agents; AGENTS.md §2.7 — one canonical authority per
> concept).

Every cluster MUST emit exactly one `cluster_run` envelope with this shape:

```yaml
cluster_run:
  cluster: <cluster-name>          # e.g. debt-coupling-cluster
  status: completed | failed | timed_out
  attempts: 1..3
  analyzer: {name, version}
  subject_sha: {head_commit}
  started_at: {RFC3339}
  finished_at: {RFC3339}
  findings: [Common Finding]       # normalized per the Common Finding Contract
                                   # in prompts/sddk/phases/debt-verify.md
  errors: [{code, message}]
  details: <cluster-specific block — see the cluster's own agent file>
```

Rules:

- `cluster` must be the agent's own registered name.
- Every issue goes through the Common Finding Contract; domain specifics live
  in `finding.details`, never by replacing envelope fields.
- `details` is the ONLY cluster-defined field. Each cluster agent file
  declares its required `details` keys under "Output Contract".
- Do not emit a cluster verdict. The parent coordinator owns the only
  Decision Contract, applied after validating and deduplicating all
  Common Findings.
- On failure or timeout, still emit the envelope: `status` reflects the
  failure, `errors` carries codes, `findings` may be empty.
