# SDDK Phase Contracts

This document owns cross-phase interfaces: router context, artifact handoff,
and Git interleaving. Each `prompts/sddk/phases/{phase}.md` file owns that
phase's operational semantics. Agent wrappers and skills reference those
contracts instead of restating them.

## Router Context Contract

Every phase consumes the `SDDK Launch Plan` from the orchestrator. Do not rediscover this context unless a required field is missing or contradicted by code/docs.

Every phase also follows `prompts/sddk/decision-model.md` (Knowledge Layers, Source Hierarchy, Knowledge States, Jurisprudence sections) for retrieval order, authority, knowledge states, promotion, and verify feedback.
Git operations follow `prompts/sddk/git-contract.md` — phases are interleaved with git, not separate from it.

Required router fields:
- Knowledge Coverage: roadmap/work items/architecture/ownership/learnings presence.
- Context Quality: C0/C1/C2/C3.
- Problem Taxonomy: dominant axes and evidence.
- Domain Language: resolved terms and unresolved ambiguities.
- Invariants: known rules or explicit unknowns.
- Recommended Effort: skip, verify, deepen, or recommend-lenses.
- Git Checkpoints: branch, base/head SHA, clean state, publication owner, and semver tag plan.
- Report Presentation: resolved locale, fallback reason, and audience.

Required knowledge behavior:
- Preserve provenance for important claims.
- Record knowledge gaps as first-class artifacts when they block progress.
- Keep workflow state, durable knowledge, and Engram memory separate.

### Git Phase Interleaving

The orchestrator owns git operations, but phases must respect the interleaving:

| Phase | Git State | Phase Responsibility |
|-------|-----------|---------------------|
| `sddk-tasks` | Branch NOT yet created | Produce tasks. Orchestrator creates branch after this phase. |
| `sddk-apply` | Branch exists, pushed to remote | Produce atomic conventional commits per task slice. Never commit broken code. |
| `sddk-verify` | Commits exist on branch | Fix commits follow conventional format. |
| **`sddk-debt-verify`** (mandatory on A-*, disabled on B-direct) | Commits exist on the cycle branch; verify passed | **Read-only audit.** Apply `prompts/sddk/phases/debt-verify.md`, emit authoritative JSON plus Markdown, and remediate failures on the same cycle branch. `INCONCLUSIVE` blocks release. |
| **`sddk-release`** | Verify and required debt evidence passed | Own direct main push, SHA verification, annotated tag, local receipts, and release report. Its successful transition moves the runtime to `RELEASED/archive` and auto-releases the phase lease. |
| `sddk-archive` | Release report succeeded and release receipt exists | Sync delta specs, finalize durable knowledge and the closing report, then close the cycle with an archive manifest linked to the release receipt. It must not assume a live lease after `release.complete`. |

Phases must not perform Git operations unless their canonical phase prompt owns
them. The orchestrator owns branch setup and dispatch; `sddk-release` owns the
Phase 3 publication effects: direct main push, SHA verification, annotated tag,
and local receipts. `sddk-archive` owns durable closure after publication. No
phase depends on a PR or CI/CD system.

The ROADMAP, ADRs, archive folders, and HTML reports live in user space (XDG + knowledge vault, ADR-0011): `$SDDK_DATA_DIR/projects/<project_id>/` and `<vault>` from `sddk knowledge path`. They are never written into the project repo and never committed.

The `sddk-debt-verify` capability runs on the same cycle branch as apply and
verify. The orchestrator accepts only `PASS` or `PASS_WITH_WARNINGS` before
handing off to release. This declarative contract does not claim a dedicated
runtime phase or CLI transition.

## Report Presentation

Resolve presentation once in the launch plan:

```yaml
report_locale_requested: string | null
report_locale: BCP-47 string
report_locale_fallback: none | project | parent-language | es
report_audience: novice | standard | expert
```

Resolution order is explicit valid `report_locale_requested`, valid project
preference, then `es`. Localized labels fall back from the requested tag to its
parent language and then `es`. An invalid tag is skipped and its fallback reason
is recorded. Never infer a persistent report locale from `$LANG` or isolated
chat text.

Localize presentation prose and headings only. Keep machine keys, IDs, enums,
verdicts, commands, paths, hashes, API names, and evidence unchanged. Reports
use four disclosure layers: **Summary**, **Guide**, **Technical detail**, and
**Evidence**. Audience controls initial expansion only; every locale/audience
receives the same facts. A failure states impact and one recovery action. `N/A`
states why; missing required evidence stays visible.

## Explore

Explore produces evidence for routing and proposal work.

Required behavior:
- Knowledge Coverage: present/missing/stale classes and why they matter.
- Context Quality: C0/C1/C2/C3.
- Problem Taxonomy: dominant axes and evidence.
- Domain Language: resolved terms and unresolved ambiguities.
- Invariants: known rules or explicit unknowns.
- Recommended Effort: skip, verify, deepen, or recommend-lenses.
- Knowledge Gaps: explicit gaps that should be persisted for later phases.

## Propose

Propose converts evidence into WHAT and WHY without guessing.

Required behavior:
- Knowledge Alignment: which durable artifacts define scope, ownership, and acceptance.
- Context Gate: quality, taxonomy, and effort decision.
- Invariants: rules that must survive the change, with verification target.
- Capabilities named in domain language.
- Recommended lenses only when context quality and risk justify them.
- Promotion Notes: what stays memory-only vs what should become durable knowledge.

If C0/C1 gaps affect scope, ownership, or capabilities, propose returns partial/blocked.

## Design

Design converts proposal/specs into HOW without repeating exploration.

Required behavior:
- Knowledge Reuse Check: roadmap/work item/ADR/ownership/learnings reused vs missing.
- Context Reuse Check: artifacts reused, gaps, code verification, quality level.
- Applied Lenses: only lenses that affected the design.
- Invariants and Constraints: enforcement point and verification.
- Entropy envelope: interface/coupling risk at the depth selected by the kernel.
- Knowledge Impact: what design choices may supersede or stale earlier knowledge.

If C0/C1 gaps affect boundaries, invariants, or contracts, design returns partial/blocked.

## Verify

Verify is a coordinator/worker phase. The coordinator runs deterministic gates
once, dispatches only the path-selected lenses, and owns the verdict. Lens
selection, evidence requirements, and output schemas are defined only in
`prompts/sddk/phases/verify.md`.

## Coherence

Coherence is a path-selected MCW handoff check, not a runtime phase. The
`sddk-coherence` leaf evaluator reads declared XDG artifacts, writes
`coherence/{trigger}.md`, and never applies a cycle transition. Triggers,
thresholds, hard blocks, and output are defined only in
`prompts/sddk/phases/coherence.md`.

## Independent Pass Evidence (SC-POL-1 carrier)

Gate evidence requirements are defined in
`skills/_shared/cli-usage-contract.md#matrix` (see the **Gate Evidence** section).
Do not re-state the evidence shape, validation function, or validation conditions
inline; reference `#matrix` instead.

## Apply

Apply implements approved SDDK tasks safely, preserving progress and verifying each slice.

Required behavior:
- Follow `prompts/sddk/git-contract.md` for commit format and atomicity.
- One atomic conventional commit per completed task slice.
- Report git checkpoints in apply-progress: branch, base/head SHA, and clean state.
- Never commit broken code. Every commit must build and pass tests.
- If unexpected blast radius appears, stop and report partial.

## Archive

Archive closes a released SDDK cycle. It syncs delta specs and updates durable knowledge.

Required behavior:
- Confirm `release-report.md` succeeded and `release-receipt` exists.
- On A-* paths, confirm `debt-report.json` exists, its outer-envelope hash
  matches, its subject SHA equals the released SHA, and its verdict is `PASS`
  or `PASS_WITH_WARNINGS`.
- Block when debt evidence is missing, FAIL, or INCONCLUSIVE.
- Link the archive manifest to the release receipt.
- Do not delete a development branch as part of archive.
- Generate the self-contained derived HTML projection using `prompts/sddk/HTML-REPORT.md`.
- Report path: `{cycle-artifacts-dir}/reports/cierre.html`; `$TMPDIR` may hold a disposable copy.
- Open the report only when explicitly requested.

## Debt-Verify

Debt-verify is the mandatory A-* handoff between passing functional verification
and release; B-direct disables it. Its path/depth mapping, worker fan-out,
finding contract, aggregation, remediation, and report schemas are defined only
in `prompts/sddk/phases/debt-verify.md`.

### `debt-report.json` (per-cycle artifact)

Producer: `sddk-debt-verify`. Schema: `docs/debt/debt-report.schema.json` (draft-07). Required: schema_version, cycle_id, generated_at, findings. Each finding: id, title, severity, priority, status, fingerprint, fingerprint_aliases, cluster_id, category, description.

### `INC-NNN-{slug}.md` (cross-cycle artifact)

Producer: `sddk-archive` (cycle-8+). Template:
`{framework-root}/docs/debt/INCIDENCE-TEMPLATE.md`, resolved from the verified
framework bundle/source manifest, never from the adopted workspace. Location:
`{vault}/incs/` (zero-intrusion). Lifecycle append-only; ADR-0047 §3.2.
