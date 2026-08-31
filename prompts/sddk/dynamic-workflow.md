# Dynamic Workflow Generation (NEW v3.3 — compose on-demand)

When triage cannot match a goal to a canonical SDDK path (B-direct / A-min / A-lite / A-full), compose a new workflow YAML on-demand. Inspired by Anthropic Dynamic Workflows (Jun 2026) but constrained to our SDDK shape. Generated workflows are saved to disk and cached in Engram.

## Trigger conditions

Generate a workflow when ANY of the following:

- `goal_pattern` does not match any canonical path's `trigger.goal_pattern`
- User explicitly invokes `/sddk-custom <goal>` or asks for a "custom flow"
- Triage classifies a novel domain not covered by A-min/A-lite/A-full
- User wants to combine multiple paths (e.g., "spec only, no apply" or "refactor debt first, then implement")

If you can match a canonical path, **always prefer it** — generated workflows are for genuinely novel goals.

## Algorithm (8 steps)

1. **Goal analysis**: extract intent, scope, key concerns from user input. Identify if goal is `investigation`, `implementation`, `verification`, `refactor`, `documentation`, `migration`, or `unknown`.

2. **Capability survey**: query these for available primitives:
   - The installed skill registry (via the `skill-registry` skill) — available skills
   - `~/.config/opencode/opencode.json` — registered agents
   - `~/.config/opencode/workflows/*.yaml` — existing workflows
   - `~/.config/opencode/docs/sddk-evolution/agentic-workflow-patterns-catalog.md` — pattern vocabulary

3. **Pattern composition**: select patterns from catalog based on goal characteristics:

   | Goal characteristic | Pattern to add |
   |---------------------|----------------|
   | Well-defined linear stages | prompt-chain |
   | Need classification/branching | routing |
   | Independent subtasks (≥4) | parallel-sectioning |
   | High-stakes verification | parallel-voting |
   | Dynamic decomposition | orchestrator-worker |
   | Iterative refinement | evaluator-optimizer |
   | Consensus / debate | group-chat |
   | Open-ended no plan | magentic-adaptive-plan |
   | Irreversible action | hitl-gate |
   | >10 subtasks | hierarchical-teams |
   | Unstable dependency | circuit-breaker |
   | Long-running distributed | saga |
   | Shared mutable state | blackboard |
   | **SDDK code-change path** | spec-driven-decomposition, multi-lens-verification, trunk-sync-gate |

4. **Phase design**: compose phases[] with these defaults:
   - Always start with `trunk-sync-start` (preflight)
   - Always end with `trunk-sync-end` + `result-contract`
   - Include `branch-creation` if any agent produces commits
   - Include `verify` if agents produce code/output
   - Include `debt-verify` for A-* code-change paths using `prompts/sddk/phases/debt-verify.md`; its depth is path-derived and B-direct disables it
   - Include `update-knowledge-graph` if milestone tracking is enabled
   - Include `release` (mandatory before archive, NOT opt-in) if git workflow applies
   - Knowledge pipeline preflight (`with_knowledge: true`): run scan → review plan → import → verify. Pass only explicitly reviewed changed-entry IDs to `--approve`; unapproved changed entries remain `NeedsReview` rather than becoming trusted.

5. **YAML composition**: emit workflow YAML following the schema in `~/.config/opencode/workflows/README.md`. Required fields:
   - `name` (kebab-case, descriptive)
   - `version: "0.1.0"` (always start at 0.1.0 for generated)
   - `status: experimental` (always experimental for generated)
   - `description` (1-2 sentences)
   - `pattern_composition` (array of catalog names)
   - `trigger` (goal_pattern, context_quality, path)
   - `phases` (array)
   - `success_criteria`
   - `provenance.generated_by: orchestrator`
   - `provenance.generated_at: <ISO timestamp>`
   - `provenance.goal: <original goal string>`

6. **Schema validation**: verify the YAML has all required fields. If missing, fall back to closest canonical path and log `dynamic-workflow-invalid <reason>`.

7. **User approval (HITL)**: present the generated YAML to the user via the `question` tool:
   - "¿Apruebas este workflow custom o querés editarlo?"
   - Options: `approve`, `edit`, `reject`
   - **Mandatory** — never execute a generated workflow without approval. This is the safety net.

8. **Persist + execute**:
   - On approve → write YAML to `~/.config/opencode/workflows/<name>.yaml`
   - Cache to Engram with `topic_key: generated-workflow/<hash-of-goal-pattern>`
   - Execute via the same Phase B algorithm (walk phases[])
   - On edit → write user's edited version, execute
   - On reject → fall back to closest canonical path

## Caching and reuse

On next invocation with same `goal_pattern`:

```bash
# Check Engram cache before composing
engram_mem_search(scope=project, query="generated-workflow <goal_pattern-hash>")
```

If cached and `status != stale`, reuse it instead of composing from scratch. Skip step 4-7 if cache hit.

## Safety rails

- **Max 16 phases**: if generated workflow has >16 phases, reject and fall back.
- **No destructive agents**: generated workflows cannot spawn `sddk-apply` or other commit-producing agents unless they include `branch-creation` + a `release` step owned by `sddk-release`.
- **No skip of git**: generated workflows MUST include `trunk-sync-start` and `trunk-sync-end`.
- **No contradiction with prose**: if generated workflow contradicts MCW or git-contract, log `dynamic-workflow-contradicts-prose` and reject.

## Telemetry

After each generated workflow execution, save to Engram:

```yaml
type: discovery
topic_key: dynamic-workflow-metrics/<name>
content: |
  Generated workflow: <name>
  Triggered by: <goal>
  Phases count: <n>
  User approved: <bool>
  Execution verdict: PASS|PW|FAIL
  Tokens used: <n>
  Lead time: <h>h
```

This feeds F3 self-tuning to learn which generated workflows are useful.

## Failure modes

| Condition | Action |
|-----------|--------|
| Schema invalid | Fall back to closest canonical path, log `dynamic-workflow-invalid` |
| User rejects | Fall back to closest canonical path |
| >16 phases | Reject, fall back, log `dynamic-workflow-too-large` |
| Contradicts prose | Reject, fall back, log `dynamic-workflow-contradicts-prose` |
| No matching agents/skills | Fall back to closest canonical path |
| Mid-execution failure | Same as canonical (BLOCK, retry, escalate per phase.failure_mode) |
