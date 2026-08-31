# Evidence-Bound Architecture Model

## Manifest

```yaml
schema_version: sddk-architecture-view/v1
cycle_id: string
phase: propose | design | verify | archive
subject_sha: sha
baseline_sha: sha|null
graph_revision: string
views: [{id: string, level: context|container|component, root_id: string}]
elements:
  - {id: string, kind: person|system|container|component, name: string, state: observed|planned|actual, evidence_refs: []}
relationships:
  - {id: string, source_id: string, target_id: string, description: string, state: observed|planned|actual, evidence_refs: []}
delta:
  - {relationship_id: string, status: unchanged|implemented_as_planned|planned_but_missing|unplanned_actual|insufficient_evidence}
semantic_status: valid | insufficient_evidence | invalid
render_status: rendered | unavailable | failed
elements_total: integer
relationships_total: integer
accepted_evidence_coverage: number
intent_ref: artifact-ref|null
delta_ref: artifact-ref|null
outputs:
  - {kind: likec4-html|svg|json|markdown|table-html, path: string, sha256: string}
tool_versions: {node: string|null, likec4: string|null, archctl: string|null}
diagnostics: []
```

IDs remain stable across views, states, locales, and renderers. A renderer may
change layout, labels, and visual grouping; it may not create semantic elements
or relationships.

## State Semantics

- `observed`: baseline fact directly supported at `baseline_sha`.
- `planned`: design intent supported by proposal/spec/design evidence.
- `actual`: post-apply fact directly supported at `subject_sha`.

Do not treat planned intent as observed or actual. Compare planned and actual
relations by stable ID:

- planned + actual: `implemented_as_planned`;
- planned without supported actual: `planned_but_missing`;
- actual without planned: `unplanned_actual`;
- missing subject/evidence: `insufficient_evidence`.

## Evidence

Each evidence reference includes `kind`, authoritative path or command, subject
SHA/diff digest, observation, and SHA-256/output digest. Prose without a source
is an explicit gap. Evidence coverage counts supported semantic records over all
records; `accepted_evidence_coverage` is that ratio in the closed interval
`0..1`. Layout and rendered pixels do not count as semantic evidence.

## Fallback

Markdown and table HTML list the same IDs, endpoints, states, deltas, and
evidence references as rich output. Missing LikeC4 changes only `render_status`.
The fallback is valid even when no diagram can be rendered.
