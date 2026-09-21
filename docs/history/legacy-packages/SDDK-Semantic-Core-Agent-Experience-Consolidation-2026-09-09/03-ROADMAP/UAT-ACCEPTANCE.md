# End-to-end UAT acceptance suite

## UAT-01 Fresh project adoption

An empty compatible repository is adopted without creating duplicate authorities.

## UAT-02 Planning reconciliation

Given desired planning input, dry-run shows deterministic WorkItem delta; apply is idempotent.

## UAT-03 Multi-run Cycle

Run A fails, Run B waits approval, Run C passes under one Cycle. Cycle summary is derived and contains no competing runtime truth.

## UAT-04 Governed side effect

Action requiring approval produces zero effect before approval; after approval Capability executes, verifies postcondition and emits Receipt/Evidence.

## UAT-05 Agent handoff

Multiple conflicting Contributions include high-risk dissent and distinct Evidence. Synthesis preserves material dispositions audibly.

## UAT-06 Decision Memory recovery

Fresh process reconstructs Memory HEAD, semantic diff and bounded ContextCapsule from project state without replaying chat transcripts.

## UAT-07 Graph rebuild and WHY

Delete SemanticGraph storage, rebuild, then equivalent why query returns equivalent proof/source refs.

## UAT-08 Vault boundary

Editing Vault Markdown may change search/context candidates but cannot mutate canonical Decision or AdmissionDecision.

## UAT-09 Pack isolation

Disable UAT pack; core planning/runtime/memory/why remain functional and pack-specific contributions disappear gracefully.

## UAT-10 Legacy compatibility

Representative supported legacy commands route to equivalent application use cases and emit planned deprecation guidance.

## UAT-11 Architecture drift guard

Attempting to introduce a second authoritative graph/evidence/revision/event path fails architecture validation.

## UAT-12 Long-session recovery

After decisions/retries/agent changes and long simulated time, status/next/why/context explain are enough to resume without transcript replay.

## UAT-13 Agent CLI without discovery

Given Target `verify`, the adapter injects an AgentCommandSurface. The agent completes the fixture using only supplied command contracts and makes zero exploratory `--help` calls.

## UAT-14 Cheat sheet syntax integrity

Every example in the generated `verify` cheat sheet parses against the same binary CommandRegistry; dry-run/sandbox examples produce expected output schemas.

## UAT-15 CLI drift detection

Rename/remove an option in a fixture without updating its CommandSpec example. Contract/golden tests fail before release.

## UAT-16 Skill does not grant Capability

Enable a Skill requiring `filesystem.write` while actor lacks write admission. Skill is selected, but write attempt is denied and no external mutation occurs.

## UAT-17 Instruction conflict fail-closed

Project instruction requires action X while policy forbids X. InstructionCompiler emits a typed conflict/constraint result; provider is never sent an instruction set that ambiguously authorizes X.

## UAT-18 Provider portability

Same AgentProfile + Task + Context + Skill selection executes through two fake/provider adapters. Rendered provider payload differs, but semantic contract refs/hashes and normalized output schema remain valid.

## UAT-19 Agent execution provenance

Given an execution id, query returns exact profile/task/context/instruction/skills/command-surface/tool/policy/config hashes and output refs without exposing chain-of-thought.

## UAT-20 Agent asset migration guard

A migrated prompt/skill fixture containing a deprecated internal command or direct canonical-store mutation instruction is rejected by static architecture/agent-contract checks.

## UAT-21 Contextual command minimization

Reviewer task receives review/verify/status/why/context commands but not unrelated release/plumbing commands unless explicitly relevant. The omitted commands remain discoverable through global diagnostics but do not consume routine agent context.

## UAT-22 Command knowledge vs authority

AgentCommandSurface documents `run ship` as a known related command while current authority denies shipping. Attempt is rejected by AuthorityEngine, proving surface knowledge is not permission.
