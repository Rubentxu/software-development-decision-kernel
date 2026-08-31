# Research Sources — Dynamic Workflows & Harness Simplification

Research snapshot: **2026-08-19**.

## Anthropic / Claude

### Introducing dynamic workflows in Claude Code — 2026-05-28
https://claude.com/blog/introducing-dynamic-workflows-in-claude-code

Relevant ideas:
- dynamically generated orchestration scripts;
- tens/hundreds of parallel subagents for appropriate tasks;
- independent verification before surfacing results;
- move orchestration from repeated conversational turns into programmatic execution.

### Harness design for long-running application development — 2026-03-24
https://www.anthropic.com/engineering/harness-design-long-running-apps

Relevant ideas:
- planner/generator/evaluator architecture;
- structured artifacts for long-running handoff;
- harness components encode assumptions about model weakness;
- remove scaffolding incrementally and evaluate whether it remains load-bearing.

### Advanced tool use / Programmatic Tool Calling — 2025-11-24
https://www.anthropic.com/engineering/advanced-tool-use

Relevant ideas:
- keep large/intermediate tool results out of model context when code/runtime can process them;
- reduce inference overhead for deterministic loops/filtering/aggregation;
- let the model focus on decisions rather than raw plumbing.

## Workflow Patterns
https://www.workflowpatterns.com/patterns/control/

Used as conceptual grounding for sequence, parallel split, synchronization/join, choices, dynamic multiple instances and related control-flow patterns.

## SDDK interpretation
SDDK adopts the architectural ideas, not a Claude-specific implementation:
- provider-neutral WorkflowIR instead of generated JavaScript as authority;
- validated dynamic graph expansion;
- event sourcing/replay;
- governed capabilities;
- cross-provider/IDE routing;
- durable human waits and local Control Plane.
