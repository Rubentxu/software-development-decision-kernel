# Escalation Policy

Use `grill-with-docs`, `auto-grill`, or `auto-grill-loop` only when launch plan justifies it.

Escalate for:
- Ambiguous domain language
- Code/docs/user claim contradiction
- Hard-to-reverse decision with real trade-off
- Critical connascence or poor design-quality score
- Context stuck at C0/C1

## Specialized Agent Delegation (within SDDK cycle)

| Agent | Trigger | Purpose |
|-------|---------|---------|
| `auto-grill-loop-orchestrator` | Proposal/design needs validation | Multi-pass adversarial |
| `jd-judge-a` + `jd-judge-b` | Pre-merge blind review (judgment-day) | Dual adversarial review |
| `sddk-coherence` | At path-selected MCW handoff checks | Read-only coherence score; no runtime transition |
| **`impeccable-primary`** | Frontend design request (any UI/UX/craft work) | Primary design agent — declares register, routes to 23 impeccable commands, integrates with SDDK via Path D |
| **`sddk-debt-verify`** | After verify PASS/PW on A-* | Coordinator; its phase prompt owns path-derived worker fan-out |

## Debt-Verify Policy

The complete activation, path/depth, worker, decision, report, failure, and
trade-off contract lives only in `prompts/sddk/phases/debt-verify.md`. There is
no mid-cycle depth or skip prompt.

## SDDK Artifacts Live in User Space (ADR-0011)

SDDK **never writes inside a project repo** (zero intrusion, ADR-0011). All working paths created during a cycle live in XDG user directories:

- Cycle artifacts: `$SDDK_DATA_DIR/projects/<project_id>/cycle-artifacts/{cycle_id}/`
- Generated docs: `$SDDK_DATA_DIR/projects/<project_id>/generated/`
- Knowledge vault: `<vault>` from `sddk knowledge path`

**Rules:**
- Never commit a repo-local SDDK working path or copy vault knowledge into `docs/`.
- Never create `.gitignore`, `.ignore`, `.atl/`, `sddk/`, or checkpoint files inside a project repo to hold SDDK state (ADR-0011).
- Never refuse to read an SDDK path because it lives outside the repo.
- `sddk-init` never plants ignore files or repo-local state. Persistence is Engram-memory + XDG + vault only.

## Release → Archive: Release Is Mandatory Before Archive (v3.7, no opt-out)

The workflow enforces `release.complete` before `archive.complete`. After all
required verify, A-* debt, review, and UAT gates pass, the orchestrator invokes
`sddk-release` without an opt-in prompt.

**Mandatory transition sequence:**
```
required quality gates → RELEASE_PENDING/release
    ↓  (next tick, no questions, no opt-in)
sddk-release(route=local)        → produces merge-receipt + release-receipt
    ↓  (only on status=success)
sddk-archive                     → produces archive-manifest linked to release-receipt
    ↓
trunk-sync-end (Phase 4.1)
```

**Recovery on blocker:** if `sddk-release` returns `status=blocked`, the orchestrator surfaces the blockers[] and instructs the user to re-run `/sddk-release <change>` (idempotent resume).

**Skill gate:** when `sddk-release/SKILL.md` is loaded, it is **delegate-only** — re-delegate to the executor agent.

## Auto Mode = Complete Cycle, No Mid-Cycle Pauses

When launched with `mode=auto` (the default), the orchestrator must run **the entire MCW from explore (Phase 1) through trunk-sync-end (Phase 4.1)** without pausing.

Forbidden mid-cycle pauses:
- "Do you want me to run debt-verify?" — debt-verify is mandatory
- "Do you want me to archive?" — archive is mandatory after release success
- "Do you want me to release?" — release is mandatory before archive
- "Should I continue?" — never asked in auto mode

The **only** legitimate mid-cycle user interaction is `escalation_needed=true` from a phase agent.

## Interactive Mode — Phase Checkpoints

In interactive mode, the orchestrator pauses after reversible planning phases.
Release and archive remain separate runtime transitions but form one automatic
closure chain with no user prompt between them.

| After phase | Interactive pause? | Rationale |
|---|---|---|
| explore, propose, spec, design, tasks | YES | Planning is reversible |
| apply (Phase 2.1 start) | **YES — last checkpoint** | Once commits exist, cycle must close |
| verify, debt-verify | NO | Automatic gate |
| release | **NO — automatic handoff to archive** | Publication is settled; archive still must close runtime state |
| trunk-sync-end | NO | Final gate, automatic |

Capability selection and skill loading live in `prompts/sddk/arsenal.md`. This
policy only decides when escalation is justified.
