# Conditional Capabilities Arsenal

The orchestrator has access to ALL these capabilities but the **triage gate** decides when to deploy each one. Default OFF; the launch plan's `adaptive_lenses` and `context_quality` opt in.

## MCP / External Tools

| Tool | Detected by | Inject into phase when |
|------|-------------|-------------------------|
| **CogniCode** (`cognicode-sdd` skill) | Tool availability check | `taxonomy` has `coupling_connascence` or `boundary_seam`, OR `context_quality ≤ C2` |
| **Chronos** (`chronos-sdd` skill) | Tool availability check | `taxonomy` includes runtime bug / perf / race, OR topic involves existing bug |
| **`impeccable`** (frontend design primary, 23 commands) | Auto-installed skill at `.opencode/skills/impeccable/` | Request mentions design/redesign/UI/components/typography/color/motion/a11y/critique. Routes 23 commands: craft, shape, audit, critique, polish, bolder, quieter, distill, harden, animate, colorize, typeset, layout, delight, overdrive, clarify, adapt, optimize, live, extract, document, init, onboard |
| **cognicode-quality** | Tool availability check | Architectural change in A-full path |
| **Engram** | MCP tool availability | Memory persistence |
| **Web Search Multi-Provider** | When phase requires external research | Proposal with external APIs/libraries, explore with ambiguous tech |
| **Entropy_sdd heuristics** | `entropy_sdd` skill available | `recommended_effort ≥ deepen` OR `context_quality ≤ C2` |

**Provider priority for Web Search:**
1. Tavily (`tavily_tavily_search`, `tavily_tavily_extract`) — technical docs, RFCs, vendor specs
2. Exa (`exa_web_search_exa`) — news, recent changes, community
3. MiniMax (`minimax_web_search`) — general purpose fallback
4. z.ai (curl fallback) — tertiary, GitHub repo analysis

## Multi-Lens Verification (deployed based on path)

All paths run the mandatory verify gates. Lens count changes depth, never the anti-placeholder, production-readiness, regression, or changed-scope SOLID baseline.

| Path | Verify depth | Lenses launched |
|------|--------------|-----------------|
| **B-direct** | Light verify | 1 direct-acceptance check |
| **A-min** | Standard | 2 lenses (spec + test quality) |
| **A-lite** | Standard | 3 lenses (spec + test + production readiness) |
| **A-full** | **Multi-lens** | 6 parallel lenses + 1 synthesis |

Lenses for A-full:
1. Spec Compliance
2. Architecture + Connascence
3. Test Quality
4. Design Coherence
5. `jd-judge-a`
6. `jd-judge-b`
7. Parent `sddk-verify` synthesis (merges evidence; cannot override deterministic failures)

## Model Assignments (phase → model)

Pass via `model` parameter in `task()` calls. If assigned model unavailable, substitute and continue.

| Phase | Default Model | Reason |
|-------|---------------|--------|
| orchestrator | MiniMax M2.7 | Coordination, decisions |
| `sddk-init` | DeepSeek V4 Pro | Bootstrap, stack detection |
| `sddk-explore` | GLM-5.1 | Reads code, structural |
| `sddk-propose` | DeepSeek V4 Pro | Architectural decisions |
| `sddk-spec` | DeepSeek V4 Pro | Structured writing |
| `sddk-design` | MiniMax M2.7 | Architecture decisions |
| `sddk-tasks` | MiniMax M2.7 | Mechanical breakdown |
| `sddk-apply` | MiniMax M2.7 | Implementation |
| `sddk-verify` (lens) | GLM-4.7 | Specialized verification lens |
| `sddk-verify` (synthesis) | GLM-4.7 | Merge + verdict |
| `sddk-debt-verify` (phase) | MiniMax M2.7 | Post-verify debt audit orchestration |
| `debt-*-cluster` | MiniMax M2.7 | Same as sddk-verify |
| `sddk-archive` | GLM-4.7 | Copy and close |
| default | MiniMax M2.7 | Non-SDD general delegation |

## Workdir Isolation (CRITICAL — prevents parallel contamination)

**Never launch `sddk-apply` on the same filesystem without branch isolation.** Past sessions lost hours when 5 parallel apply agents clobbered each other's edits.

**Mandatory rules** when applying:

1. **One apply per branch.** Each `sddk-apply` subagent operates on its own `<type>/<description>` branch. No two apply agents share a branch.
2. **One apply per working tree** (preferred for true parallelism). If the project supports git worktrees, give each parallel apply its own worktree rooted at the same commit on `main`.
3. **Each apply commits atomically** (single commit per task slice) — see `git-contract.md`. This way if a branch gets reset, no work is lost.
4. **Conflict detection before merging.** After apply completes, `git diff main...<branch>` should be reviewed before merge. Auto-merge requires this diff to be empty of conflicting files.

**When parallelism is NOT needed** (most cases): serialize applies on a single branch. The default SDDK flow is single-agent, single-branch. Only parallelize when:
- Independent task slices that touch different files
- Test runs that can happen while implementation continues
- Verification that can run in parallel with apply

If unsure, serialize. Parallelism gains time but loses safety.

## Lateral Thinking Patterns

| Pattern | Trigger | Default |
|---------|---------|---------|
| **F1 (Crystallize)** | 2+ valid approaches in propose/design | OFF — opt-in when triggered |
| **F3 (Self-Improving)** | After every cycle, consumes metrics → tunes next | **ON** — always |
| **F4 (Speculative)** | 2+ architecturally distinct approaches in design | OFF — opt-in |

## Strict TDD Forwarding (MANDATORY when active)

When launching `sddk-apply` or `sddk-verify`:

1. Search for testing capabilities: `mem_search("sddk/{project}/testing-capabilities")`
2. If result contains `strict_tdd: true` AND `strict_tdd_mode: true` in launch plan:
   - Inject into sub-agent prompt: `"STRICT TDD MODE IS ACTIVE. Test runner: {test_command}. You MUST follow strict-tdd-{apply|verify}.md. Do NOT fall back to Standard Mode."`
   - **NON-NEGOTIABLE.** Don't rely on sub-agent discovering independently.
3. Cache TDD status for the session.

## Apply-Progress Continuity (MANDATORY for continuation batches)

When launching `sddk-apply` for a continuation (not first batch):

1. Search: `mem_search("sddk/{change-name}/apply-progress")`
2. If found, inject: `"PREVIOUS APPLY-PROGRESS EXISTS at topic_key 'sddk/{change-name}/apply-progress'. You MUST read it first, merge your new progress with the existing progress, and save the combined result. Do NOT overwrite — MERGE."`
3. If not found, no special instruction.

This prevents progress loss across batches.

## Skill Resolver Protocol

At session start (or first delegation):

1. Search for compact rules: `mem_search("sddk/{project}/init")` → extract `Compact Rules` section
2. If not found: `mem_search("skill-registry")` or read the installed skill registry via `skill-registry` skill
3. Cache as `project_compact_rules`
4. For each sub-agent launch: inject matched rules as `## Project Standards (auto-resolved)` BEFORE task-specific instructions
5. Add model alias from Model Assignments to Agent tool call

**Skill Resolution Feedback:**
After every delegation, check the result's `skill_resolution` field:
- `injected` → OK
- `fallback-registry`, `fallback-path`, `none` → cache was lost (compaction). Re-read registry immediately, inject in subsequent calls.

## Debt-Lifecycle Gates

- `debt-severity-assigned` — every open finding in `debt-report.json` has severity ∈ {critical, high, medium, low}.
- `debt-priority-assigned` — every open finding in `debt-report.json` has priority ∈ {P0, P1, P2, P3}.

Both wired into `phase.verify.complete*` on A-* paths; B-direct skips debt-verify.

## Post-Subagent Validation

After EACH sub-agent returns (BEFORE next phase):

1. **Verify Engram persistence** — confirm the sub-agent saved its artifacts
2. **Verify artifact completeness** — check that all required artifacts (proposal/spec/design/tasks/verify-report) are present
3. **Write cycle checkpoint** to Engram if this is a phase boundary

## Web Search Multi-Provider (when delegating research)

When YOU call search directly (not delegating to `auto-grill-*`):

```
Broad research: tavily_tavily_search + exa_web_search_exa simultaneously
Targeted query: Tavily first
Recent/breaking: Exa first
Quota fallback chain: Tavily → Exa → MiniMax → z.ai via curl
Same URL multiple results: keep highest-quality source
```
