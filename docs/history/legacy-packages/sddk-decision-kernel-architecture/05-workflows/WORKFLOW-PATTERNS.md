# Workflow Patterns for SDDK

## Principle
Patterns are reusable compositions over the runtime algebra. Prefer the simplest pattern that preserves the required guarantees.

## Control
- **Sequence** — known linear dependency.
- **Choice/Router** — classify and select one/more paths.
- **Loop** — bounded iteration.
- **Gate/Wait** — policy/human/external event.

## Concurrency
- **Parallel Split** — known independent branches.
- **Map/Fan-out** — item count discovered at runtime.
- **Join** — all/quorum/partial aggregation.
- **Race** — first valid result; cancel losers where safe.

## Agentic
- **Orchestrator/Workers** — dynamic decomposition then Map/Join.
- **Generator/Evaluator** — iterative output/review.
- **Convergence** — verify gaps, remediate, repeat until invariant satisfied.
- **Adversarial Verification** — independent reviewers try to refute output.

## Reliability/governance
- **Circuit Breaker** — provider/tool health.
- **Saga/Compensation** — multi-effect operations.
- **Human Gate** — irreversible/high-risk decisions.
- **Event Reaction** — ActiveGraph-style behavior triggered from canonical events.

## Selection examples
| Problem | Composition |
|---|---|
| rename API flag | Sequence |
| migrate hundreds of files | Discover → Map → Join → Verify |
| architecture proposal | Parallel reviewers → adjudicate |
| unknown bug | Orchestrator/Workers + convergence |
| release | Gate + Sequence + Saga/receipts |
| provider outage | Event Reaction + Circuit Breaker + reroute |

## Anti-patterns
- using an LLM call for a deterministic join/loop;
- forcing a human gate between every generated artifact;
- recursive uncontrolled worker spawning;
- dynamic workflow generated as unrestricted shell/JS;
- parallel agents writing the same worktree;
- fixed verifier fan-out independent of change risk.
